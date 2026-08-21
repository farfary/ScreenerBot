//! OHLCV service — public API for querying candle data with cache and database fallback.

use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::monitor::OhlcvMonitor;
use crate::ohlcvs::types::{
    Candle, OhlcvError, OhlcvResult, OhlcvStatus, OhlcvTimeframeStatus, Timeframe, TimeframeBundle,
    BUNDLE_CANDLE_COUNT,
};
use crate::{
    chains::ChainId,
    logger::{self, LogTag},
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
        let db = Arc::new(OhlcvDatabase::new(db_path, ChainId::Solana)?);
        let fetcher = Arc::new(OhlcvFetcher::new());
        let cache = Arc::new(OhlcvCache::new(ChainId::Solana));
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

    /// Wipe ALL cached candle data (DB) AND both in-memory caches. Clearing only
    /// the DB left the hot cache and the bundle cache serving stale candles until
    /// their TTL (up to 24h / 30s), so a manual "clear cache" appeared to do
    /// nothing on charts that were already cached. Invalidate them together.
    pub(super) async fn clear_all_data(
        &self,
    ) -> OhlcvResult<crate::ohlcvs::database::ClearAllResult> {
        let db = Arc::clone(&self.db);
        let result = tokio::task::spawn_blocking(move || db.clear_all_ohlcv_data())
            .await
            .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;
        self.cache.clear()?;
        self.bundle_cache.write().await.clear();
        Ok(result)
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
                // limit == 0 means "return the full stored history" (no cap);
                // otherwise take the last N entries (most recent).
                let start_idx = if limit == 0 {
                    0
                } else {
                    filtered.len().saturating_sub(limit)
                };
                return Ok(filtered.into_iter().skip(start_idx).collect());
            }
        }

        // limit == 0 => no SQL LIMIT (return everything for this timeframe).
        let db_limit = if limit == 0 { None } else { Some(limit) };

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
                db_limit,
            )
        })
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;

        // NOTE: deliberately NO "query any pool" fallback here. Candles from a
        // different pool are a different price series; merging them would combine
        // pools (the token-details chart must only ever show the single resolved
        // pool). If the resolved pool has no candles yet the chart shows an empty
        // "collecting" state until the backfill fills THIS pool.

        // Fallback: if still empty and requested timeframe is not 1m, try
        // aggregating from 1m data — scoped to the SAME resolved pool (never
        // across pools).
        if candles.is_empty() && timeframe != Timeframe::Minute1 {
            let db = Arc::clone(&self.db);
            let mint_owned = mint.to_string();
            let pool_owned = pool.clone();
            // Fetch enough 1m candles to aggregate into requested limit
            // For 5m we need 5x more 1m candles, etc.
            let multiplier = timeframe.to_seconds() / Timeframe::Minute1.to_seconds();
            // limit == 0 => aggregate the full 1m history (no cap).
            let raw_limit = if limit == 0 {
                None
            } else {
                Some(limit * multiplier as usize)
            };

            let raw_candles = tokio::task::spawn_blocking(move || {
                db.get_candles(
                    &mint_owned,
                    Some(&pool_owned),
                    Timeframe::Minute1,
                    from_timestamp,
                    to_timestamp,
                    raw_limit,
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

        // Only cache the FULL series (limit == 0 => db_limit is None). A limited
        // read — e.g. the limit=1 existence probes fired by the chart status
        // indicator and the timeframe fallback — returns just a slice; caching
        // that slice under the (mint, pool, timeframe) key would make the chart's
        // next full-history read cache-hit the slice and render only a few
        // candles. The full-series read below repopulates the cache correctly.
        if db_limit.is_none() {
            let _ = self
                .cache
                .put(mint, Some(&pool), timeframe, candles.clone());
        }

        // limit == 0 => return everything; otherwise the most recent N.
        let start_idx = if limit == 0 {
            0
        } else {
            candles.len().saturating_sub(limit)
        };
        Ok(candles.into_iter().skip(start_idx).collect())
    }

    pub(super) fn has_data(&self, mint: &str) -> OhlcvResult<bool> {
        self.db.has_data_for_mint(mint)
    }

    pub(super) fn get_mints_with_data(&self, mints: &[String]) -> OhlcvResult<HashSet<String>> {
        self.db.get_mints_with_data(mints)
    }

    /// Assemble the per-timeframe OHLCV process status for a token (monitoring
    /// state + candle counts + backfill flags). Cheap: one grouped candle query
    /// plus per-timeframe backfill flags from the monitor config row.
    pub(super) async fn get_status(&self, mint: &str) -> OhlcvResult<OhlcvStatus> {
        // Scope the summary to the SAME single pool the chart reads
        // (`get_ohlcv_data`): default pool, else the best available. Without this
        // the counts would sum every pool_address and diverge from the chart the
        // moment a token has candles under more than one pool. No pool yet =>
        // no data (empty summary), which is the correct "collecting" state.
        let mut selected_pool = self.pool_manager.get_default_pool(mint).await?;
        if selected_pool.is_none() {
            selected_pool = self.pool_manager.get_best_pool(mint).await?;
        }
        let summary = match selected_pool.as_ref() {
            Some(pool) => self.db.get_timeframe_summary(mint, &pool.address)?,
            None => Vec::new(),
        };
        let last_new_by_tf = self
            .db
            .get_timeframe_last_new_data(mint)
            .unwrap_or_default();
        let last_checked_at = self.db.get_last_checked_at(mint).unwrap_or(None);
        let monitored = self.monitor.is_monitored(mint).await;

        let mut total_candles = 0i64;
        let mut best_timeframe: Option<String> = None;
        let mut all_backfilled = true;
        let mut overall_last_new: Option<i64> = None;
        let mut timeframes = Vec::with_capacity(7);

        // Timeframe::all() is finest→coarsest, so the first one with candles is
        // the finest available — the chart's preferred default target.
        for tf in Timeframe::all() {
            let tf_str = tf.as_str().to_string();
            let (candles, latest) = summary
                .iter()
                .find(|(name, _, _)| name == &tf_str)
                .map(|(_, count, latest)| (*count, *latest))
                .unwrap_or((0, None));

            let backfill_complete = self.db.is_backfill_complete(mint, tf).unwrap_or(false);
            if !backfill_complete {
                all_backfilled = false;
            }

            total_candles += candles;
            if candles > 0 && best_timeframe.is_none() {
                best_timeframe = Some(tf_str.clone());
            }

            let last_new_data_at = last_new_by_tf
                .iter()
                .find(|(name, _)| name == &tf_str)
                .map(|(_, ts)| *ts);
            if let Some(ts) = last_new_data_at {
                overall_last_new = Some(overall_last_new.map_or(ts, |cur| cur.max(ts)));
            }

            timeframes.push(OhlcvTimeframeStatus {
                timeframe: tf_str,
                candles,
                backfill_complete,
                latest_timestamp: latest,
                last_new_data_at,
            });
        }

        Ok(OhlcvStatus {
            mint: mint.to_string(),
            monitored,
            has_data: total_candles > 0,
            total_candles,
            best_timeframe,
            backfill_complete: all_backfilled,
            last_checked_at,
            last_new_data_at: overall_last_new,
            timeframes,
        })
    }

    /// Get timeframe bundle from cache (non-blocking, cache-only)
    /// Returns None if bundle is stale or missing (triggers background refresh)
    pub(super) async fn get_timeframe_bundle(
        &self,
        mint: &str,
    ) -> OhlcvResult<Option<TimeframeBundle>> {
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
    pub(super) async fn store_bundle(
        &self,
        mint: String,
        bundle: TimeframeBundle,
    ) -> OhlcvResult<()> {
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
