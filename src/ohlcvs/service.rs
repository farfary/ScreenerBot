//! OHLCV service — public API for querying candle data with cache and database fallback.

use crate::logger::{self, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::monitor::OhlcvMonitor;
use crate::ohlcvs::types::{
    Candle, OhlcvError, OhlcvResult, Timeframe, TimeframeBundle, BUNDLE_CANDLE_COUNT,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Notify, OnceCell, RwLock};
use tokio::task::JoinHandle;

// Bundle cache constants (Phase 2)
const BUNDLE_CACHE_TTL_SECONDS: u64 = 30;
const BUNDLE_CACHE_MAX_SIZE: usize = 150;
const PARALLEL_FETCH_LIMIT: usize = 10;
const BUNDLE_REFRESH_INTERVAL_SECONDS: u64 = 5;

pub(super) static OHLCV_SERVICE: OnceCell<Arc<OhlcvServiceImpl>> = OnceCell::const_new();

pub struct OhlcvService;

pub(super) struct OhlcvServiceImpl {
    pub(super) db: Arc<OhlcvDatabase>,
    pub(super) fetcher: Arc<OhlcvFetcher>,
    pub(super) cache: Arc<OhlcvCache>,
    pub(super) pool_manager: Arc<PoolManager>,
    pub(super) gap_manager: Arc<GapManager>,
    pub(super) monitor: Arc<OhlcvMonitor>,

    // Phase 2: Bundle cache for strategy evaluation
    bundle_cache: Arc<RwLock<HashMap<String, (TimeframeBundle, Instant)>>>,

    // Track in-flight builds to prevent duplicate concurrent builds
    build_in_progress: Arc<RwLock<HashSet<String>>>,
}

impl OhlcvServiceImpl {
    fn new(db_path: PathBuf) -> OhlcvResult<Self> {
        let db = Arc::new(OhlcvDatabase::new(db_path)?);
        let fetcher = Arc::new(OhlcvFetcher::new());
        let cache = Arc::new(OhlcvCache::new());
        let pool_manager = Arc::new(PoolManager::new(Arc::clone(&db)));
        let gap_manager = Arc::new(GapManager::new(Arc::clone(&db), Arc::clone(&fetcher)));
        let monitor = Arc::new(OhlcvMonitor::new(
            Arc::clone(&db),
            Arc::clone(&fetcher),
            Arc::clone(&cache),
            Arc::clone(&pool_manager),
            Arc::clone(&gap_manager),
        ));

        let bundle_cache = Arc::new(RwLock::new(HashMap::new()));
        let build_in_progress = Arc::new(RwLock::new(HashSet::new()));

        Ok(Self {
            db,
            fetcher,
            cache,
            pool_manager,
            gap_manager,
            monitor,
            bundle_cache,
            build_in_progress,
        })
    }

    pub(super) async fn get_ohlcv_data(
        &self,
        mint: &str,
        timeframe: Timeframe,
        pool_address: Option<&str>,
        limit: usize,
        from_timestamp: Option<i64>,
        to_timestamp: Option<i64>,
    ) -> OhlcvResult<Vec<Candle>> {
        // Determine pool to use
        let pool = if let Some(addr) = pool_address {
            addr.to_string()
        } else {
            // Use default pool, falling back to best available option
            let mut selected_pool = self.pool_manager.get_default_pool(mint).await?;

            if selected_pool.is_none() {
                selected_pool = self.pool_manager.get_best_pool(mint).await?;
            }

            let default_pool =
                selected_pool.ok_or_else(|| OhlcvError::PoolNotFound(mint.to_string()))?;

            default_pool.address.clone()
        };

        // Try cache first
        if let Ok(Some(mut cached_data)) = self.cache.get(mint, Some(&pool), timeframe) {
            // Ensure ASC ordering
            cached_data.sort_by_key(|d| d.timestamp);

            // Filter by time range
            let filtered: Vec<Candle> = cached_data;

            if !filtered.is_empty() {
                // Take last N entries (most recent)
                let start_idx = filtered.len().saturating_sub(limit);
                return Ok(filtered.into_iter().skip(start_idx).collect());
            }
        }

        // Fetch from unified candles table (wrapped in spawn_blocking to avoid blocking async runtime)
        let db = Arc::clone(&self.db);
        let mint_owned = mint.to_string();
        let pool_owned = pool.clone();
        let mut candles = tokio::task::spawn_blocking(move || {
            db.get_candles(
                &mint_owned,
                Some(&pool_owned),
                timeframe,
                from_timestamp,
                to_timestamp,
                Some(limit),
            )
        })
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;

        // Fallback: if no candles found for specific pool, try querying without pool filter
        // This handles cases where candles were stored under a different pool address
        if candles.is_empty() {
            let db = Arc::clone(&self.db);
            let mint_owned = mint.to_string();
            candles = tokio::task::spawn_blocking(move || {
                db.get_candles(
                    &mint_owned,
                    None, // Query any pool for this mint
                    timeframe,
                    from_timestamp,
                    to_timestamp,
                    Some(limit),
                )
            })
            .await
            .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;
        }

        // Fallback: if still empty and requested timeframe is not 1m, try aggregating from 1m data
        if candles.is_empty() && timeframe != Timeframe::Minute1 {
            let db = Arc::clone(&self.db);
            let mint_owned = mint.to_string();
            // Fetch enough 1m candles to aggregate into requested limit
            // For 5m we need 5x more 1m candles, etc.
            let multiplier = timeframe.to_seconds() / Timeframe::Minute1.to_seconds();
            let raw_limit = limit * multiplier as usize;

            let raw_candles = tokio::task::spawn_blocking(move || {
                db.get_candles(
                    &mint_owned,
                    None,
                    Timeframe::Minute1,
                    from_timestamp,
                    to_timestamp,
                    Some(raw_limit),
                )
            })
            .await
            .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;

            if !raw_candles.is_empty() {
                // Aggregate 1m data to requested timeframe
                candles = OhlcvAggregator::aggregate(&raw_candles, Timeframe::Minute1, timeframe)?;
                logger::debug(
                    LogTag::Ohlcv,
                    &format!(
                        "Aggregated {} 1m candles to {} {} candles for {}",
                        raw_candles.len(),
                        candles.len(),
                        timeframe,
                        mint
                    ),
                );
            }
        }

        if candles.is_empty() {
            return Ok(Vec::new());
        }

        // Normalize to ASC ordering
        candles.sort_by_key(|d| d.timestamp);

        // Update cache
        let _ = self
            .cache
            .put(mint, Some(&pool), timeframe, candles.clone());

        // Take last N entries (most recent)
        let start_idx = candles.len().saturating_sub(limit);
        Ok(candles.into_iter().skip(start_idx).collect())
    }

    pub(super) fn has_data(&self, mint: &str) -> OhlcvResult<bool> {
        self.db.has_data_for_mint(mint)
    }

    pub(super) fn get_mints_with_data(&self, mints: &[String]) -> OhlcvResult<HashSet<String>> {
        self.db.get_mints_with_data(mints)
    }

    /// Get timeframe bundle from cache (non-blocking, cache-only)
    /// Returns None if bundle is stale or missing (triggers background refresh)
    pub(super) async fn get_timeframe_bundle(&self, mint: &str) -> OhlcvResult<Option<TimeframeBundle>> {
        let cache = self.bundle_cache.read().await;

        if let Some((bundle, cached_at)) = cache.get(mint) {
            let age_secs = cached_at.elapsed().as_secs();

            if age_secs < BUNDLE_CACHE_TTL_SECONDS {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!("CACHE_HIT: Bundle for {mint} (age: {age_secs}s)"),
                );

                // Create result with correct metadata - don't modify cached bundle
                let mut result = bundle.clone();
                result.cache_hit = true;
                result.cache_age_seconds = age_secs;
                return Ok(Some(result));
            }

            logger::debug(
                LogTag::Ohlcv,
                &format!(
                    "CACHE_STALE: Bundle for {} (age: {}s > {}s TTL)",
                    mint, age_secs, BUNDLE_CACHE_TTL_SECONDS
                ),
            );
        } else {
            logger::debug(LogTag::Ohlcv, &format!("CACHE_MISS: No bundle for {mint}"));
        }

        Ok(None)
    }

    /// Build complete timeframe bundle by fetching all 7 timeframes
    /// Fetches in parallel with PARALLEL_FETCH_LIMIT concurrency
    /// Coordinates to prevent duplicate concurrent builds for same token
    pub(super) async fn build_timeframe_bundle(&self, mint: &str) -> OhlcvResult<TimeframeBundle> {
        // Check if another task is already building this bundle
        {
            let in_progress = self.build_in_progress.read().await;
            if in_progress.contains(mint) {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!("BUNDLE_BUILD_WAIT: Another task already building bundle for {mint}, waiting..."),
                );
                drop(in_progress);

                // Wait for the other build to complete (poll cache with exponential backoff)
                let mut wait_ms = 50u64;
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_millis(wait_ms)).await;

                    // Check if build completed (no longer in progress)
                    let still_building = {
                        let in_progress = self.build_in_progress.read().await;
                        in_progress.contains(mint)
                    };

                    if !still_building {
                        // Check cache for the result
                        if let Ok(Some(bundle)) = self.get_timeframe_bundle(mint).await {
                            logger::debug(
                                LogTag::Ohlcv,
                                &format!(
                                    "BUNDLE_BUILD_REUSE: Found bundle built by another task for {}",
                                    mint
                                ),
                            );
                            return Ok(bundle);
                        }
                        break; // Build finished but no result - proceed to build ourselves
                    }

                    wait_ms = (wait_ms * 2).min(200); // Exponential backoff, max 200ms
                }

                // If we get here, either:
                // 1. The other build completed but failed to cache
                // 2. The other build is taking too long (>~1.2s)
                // In either case, we'll try to build ourselves
            }
        }

        // Mark as building (atomic check-and-set)
        {
            let mut in_progress = self.build_in_progress.write().await;
            if !in_progress.insert(mint.to_string()) {
                // Race condition: another task started building while we were waiting
                // Just return error to avoid duplicate work
                drop(in_progress);
                return Err(OhlcvError::NotFound(format!(
                    "Bundle build already in progress for {}",
                    mint
                )));
            }
        }

        let start = Instant::now();

        // Get default pool
        let pool = {
            let mut selected_pool = self.pool_manager.get_default_pool(mint).await?;
            if selected_pool.is_none() {
                selected_pool = self.pool_manager.get_best_pool(mint).await?;
            }
            selected_pool.ok_or_else(|| OhlcvError::PoolNotFound(mint.to_string()))?
        };

        let pool_address = pool.address.clone();

        // Fetch all 7 timeframes in parallel
        let timeframes = vec![
            Timeframe::Minute1,
            Timeframe::Minute5,
            Timeframe::Minute15,
            Timeframe::Hour1,
            Timeframe::Hour4,
            Timeframe::Hour12,
            Timeframe::Day1,
        ];

        let mut tasks = Vec::new();

        for tf in timeframes {
            let mint_owned = mint.to_string();
            let pool_owned = pool_address.clone();
            let db = Arc::clone(&self.db);
            let cache = Arc::clone(&self.cache);

            let task = tokio::spawn(async move {
                // Try cache first
                if let Ok(Some(mut cached)) = cache.get(&mint_owned, Some(&pool_owned), tf) {
                    cached.sort_by_key(|d| d.timestamp);
                    let start_idx = cached.len().saturating_sub(BUNDLE_CANDLE_COUNT);
                    return Ok::<Vec<Candle>, OhlcvError>(
                        cached.into_iter().skip(start_idx).collect(),
                    );
                }

                // Fetch from database
                let candles = tokio::task::spawn_blocking(move || {
                    db.get_candles(
                        &mint_owned,
                        Some(&pool_owned),
                        tf,
                        None,
                        None,
                        Some(BUNDLE_CANDLE_COUNT),
                    )
                })
                .await
                .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;

                Ok(candles)
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;

        // Extract results
        let mut m1 = Vec::new();
        let mut m5 = Vec::new();
        let mut m15 = Vec::new();
        let mut h1 = Vec::new();
        let mut h4 = Vec::new();
        let mut h12 = Vec::new();
        let mut d1 = Vec::new();

        for (idx, result) in results.into_iter().enumerate() {
            let candles =
                result.map_err(|e| OhlcvError::ApiError(format!("Task join error: {e}")))??;

            match idx {
                0 => m1 = candles,
                1 => m5 = candles,
                2 => m15 = candles,
                3 => h1 = candles,
                4 => h4 = candles,
                5 => h12 = candles,
                6 => d1 = candles,
                _ => {}
            }
        }

        let elapsed_ms = start.elapsed().as_millis();
        if elapsed_ms > 500 {
            logger::info(
                LogTag::Ohlcv,
                &format!(
                    "BUNDLE_BUILD_SLOW: Built bundle for {} in {}ms",
                    mint, elapsed_ms
                ),
            );
        } else {
            logger::debug(
                LogTag::Ohlcv,
                &format!(
                    "BUNDLE_BUILD: Built bundle for {} in {}ms",
                    mint, elapsed_ms
                ),
            );
        }

        // Remove from in-progress tracking
        {
            let mut in_progress = self.build_in_progress.write().await;
            in_progress.remove(mint);
        }

        Ok(TimeframeBundle {
            mint: mint.to_string(),
            pool_address,
            timestamp: Utc::now(),
            m1,
            m5,
            m15,
            h1,
            h4,
            h12,
            d1,
            cache_age_seconds: 0, // Fresh build
            cache_hit: false,
        })
    }

    /// Store bundle in cache with LRU eviction
    /// Takes bundle by value to avoid unnecessary cloning
    pub(super) async fn store_bundle(&self, mint: String, bundle: TimeframeBundle) -> OhlcvResult<()> {
        let mut cache = self.bundle_cache.write().await;

        // LRU eviction: if cache is full, remove oldest entry
        if cache.len() >= BUNDLE_CACHE_MAX_SIZE && !cache.contains_key(&mint) {
            if let Some(oldest_mint) = cache
                .iter()
                .min_by_key(|(_, (_, instant))| *instant)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_mint);
                logger::debug(
                    LogTag::Ohlcv,
                    &format!("BUNDLE_EVICT: Removed {oldest_mint} from cache (LRU)"),
                );
            }
        }

        cache.insert(mint.clone(), (bundle, Instant::now()));
        logger::debug(
            LogTag::Ohlcv,
            &format!(
                "BUNDLE_STORE: Stored bundle for {} (cache size: {})",
                mint,
                cache.len()
            ),
        );

        Ok(())
    }
}

pub(super) async fn get_or_init_service() -> OhlcvResult<Arc<OhlcvServiceImpl>> {
    let service = OHLCV_SERVICE
        .get_or_try_init(|| async {
            logger::info(
                LogTag::Ohlcv,
                &"INIT: Initializing OHLCV runtime".to_owned(),
            );

            let db_path = crate::paths::get_ohlcvs_db_path();
            let service_impl = OhlcvServiceImpl::new(db_path)?;

            logger::info(LogTag::Ohlcv, &"SUCCESS: OHLCV runtime ready".to_owned());
            Ok::<Arc<OhlcvServiceImpl>, OhlcvError>(Arc::new(service_impl))
        })
        .await?;

    Ok(Arc::clone(service))
}

impl OhlcvService {
    pub async fn initialize() -> OhlcvResult<()> {
        get_or_init_service().await.map(|_| ())
    }

    pub async fn start(
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> OhlcvResult<Vec<JoinHandle<()>>> {
        let enabled = crate::config::with_config(|cfg| cfg.ohlcv.enabled);
        if !enabled {
            logger::info(
                LogTag::Ohlcv,
                &"OHLCV service is disabled via config, skipping start".to_owned(),
            );
            return Ok(vec![]);
        }

        let service = get_or_init_service().await?;

        let monitor_instance = Arc::clone(&service.monitor);

        // Start background monitoring tasks before awaiting shutdown
        monitor_instance.clone().start().await?;
        logger::info(
            LogTag::Ohlcv,
            &"TASK_START: OHLCV monitoring tasks started".to_owned(),
        );

        let shutdown_task = tokio::spawn(monitor.instrument(async move {
            shutdown.notified().await;
            logger::info(
                LogTag::Ohlcv,
                &"TASK_STOP: Shutdown signal received for OHLCV monitoring".to_owned(),
            );
            monitor_instance.stop().await;
            logger::info(
                LogTag::Ohlcv,
                &"TASK_END: OHLCV monitoring tasks stopped".to_owned(),
            );
        }));

        Ok(vec![shutdown_task])
    }

    pub async fn has_data(mint: &str) -> OhlcvResult<bool> {
        let service = get_or_init_service().await?;
        service.has_data(mint)
    }
}

