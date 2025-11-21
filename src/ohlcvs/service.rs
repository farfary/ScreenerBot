// Main OHLCV service implementation

use crate::logger::{self, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::monitor::{MonitorStats, OhlcvMonitor};
use crate::ohlcvs::priorities::ActivityType;
use crate::ohlcvs::types::{
    Candle, OhlcvError, OhlcvMetrics, OhlcvResult, PoolMetadata, Priority, Timeframe,
    TimeframeBundle, TokenOhlcvConfig,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Notify, OnceCell, RwLock};
use tokio::task::JoinHandle;

// Bundle cache constants (Phase 2)
const BUNDLE_CACHE_TTL_SECONDS: u64 = 30;
const BUNDLE_CACHE_MAX_SIZE: usize = 150;
const BUNDLE_CANDLE_COUNT: usize = 100;
const PARALLEL_FETCH_LIMIT: usize = 10;
const BUNDLE_REFRESH_INTERVAL_SECONDS: u64 = 5;

static OHLCV_SERVICE: OnceCell<Arc<OhlcvServiceImpl>> = OnceCell::const_new();

pub struct OhlcvService;

struct OhlcvServiceImpl {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
    cache: Arc<OhlcvCache>,
    pool_manager: Arc<PoolManager>,
    gap_manager: Arc<GapManager>,
    monitor: Arc<OhlcvMonitor>,
    
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

    async fn get_ohlcv_data(
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

        // Fetch from unified candles table
        let mut candles = self.db.get_candles(
            mint,
            Some(&pool),
            timeframe,
            from_timestamp,
            to_timestamp,
            Some(limit),
        )?;

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

    fn has_data(&self, mint: &str) -> OhlcvResult<bool> {
        self.db.has_data_for_mint(mint)
    }

    fn get_mints_with_data(&self, mints: &[String]) -> OhlcvResult<HashSet<String>> {
        self.db.get_mints_with_data(mints)
    }
    
    /// Get timeframe bundle from cache (non-blocking, cache-only)
    /// Returns None if bundle is stale or missing (triggers background refresh)
    async fn get_timeframe_bundle(&self, mint: &str) -> OhlcvResult<Option<TimeframeBundle>> {
        let cache = self.bundle_cache.read().await;
        
        if let Some((bundle, cached_at)) = cache.get(mint) {
            let age_secs = cached_at.elapsed().as_secs();
            
            if age_secs < BUNDLE_CACHE_TTL_SECONDS {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!("CACHE_HIT: Bundle for {} (age: {}s)", mint, age_secs),
                );
                
                // Create result with correct metadata - don't modify cached bundle
                let mut result = bundle.clone();
                result.cache_hit = true;
                result.cache_age_seconds = age_secs;
                return Ok(Some(result));
            }
            
            logger::debug(
                LogTag::Ohlcv,
                &format!("CACHE_STALE: Bundle for {} (age: {}s > {}s TTL)", mint, age_secs, BUNDLE_CACHE_TTL_SECONDS),
            );
        } else {
            logger::debug(
                LogTag::Ohlcv,
                &format!("CACHE_MISS: No bundle for {}", mint),
            );
        }
        
        Ok(None)
    }
    
    /// Build complete timeframe bundle by fetching all 7 timeframes
    /// Fetches in parallel with PARALLEL_FETCH_LIMIT concurrency
    /// Coordinates to prevent duplicate concurrent builds for same token
    async fn build_timeframe_bundle(&self, mint: &str) -> OhlcvResult<TimeframeBundle> {
        // Use single write transaction to atomically check and mark as building
        {
            let mut in_progress = self.build_in_progress.write().await;
            
            // Try to insert - if already present, another task is building
            if !in_progress.insert(mint.to_string()) {
                logger::debug(
                    LogTag::Ohlcv,
                    &format!("BUNDLE_BUILD_SKIP: Another task already building bundle for {}, waiting...", mint),
                );
                drop(in_progress);
                
                // Wait briefly for the other build to complete
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                
                // Check cache again - the other build should have stored it
                if let Ok(Some(bundle)) = self.get_timeframe_bundle(mint).await {
                    logger::debug(
                        LogTag::Ohlcv,
                        &format!("BUNDLE_BUILD_REUSE: Found bundle built by another task for {}", mint),
                    );
                    return Ok(bundle);
                }
                // If still not in cache, proceed with build (other task may have failed)
                // Re-acquire write lock and mark as building
                let mut in_progress = self.build_in_progress.write().await;
                in_progress.insert(mint.to_string());
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
                        cached.into_iter().skip(start_idx).collect()
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
                .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {}", e)))??;
                
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
            let candles = result
                .map_err(|e| OhlcvError::ApiError(format!("Task join error: {}", e)))??;
            
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
                &format!("BUNDLE_BUILD_SLOW: Built bundle for {} in {}ms", mint, elapsed_ms),
            );
        } else {
            logger::debug(
                LogTag::Ohlcv,
                &format!("BUNDLE_BUILD: Built bundle for {} in {}ms", mint, elapsed_ms),
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
            cache_age_seconds: 0,  // Fresh build
            cache_hit: false,
        })
    }
    
    /// Store bundle in cache with LRU eviction
    /// Takes bundle by value to avoid unnecessary cloning
    async fn store_bundle(&self, mint: String, bundle: TimeframeBundle) -> OhlcvResult<()> {
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
                    &format!("BUNDLE_EVICT: Removed {} from cache (LRU)", oldest_mint),
                );
            }
        }
        
        cache.insert(mint.clone(), (bundle, Instant::now()));
        logger::debug(
            LogTag::Ohlcv,
            &format!("BUNDLE_STORE: Stored bundle for {} (cache size: {})", mint, cache.len()),
        );
        
        Ok(())
    }
}

async fn get_or_init_service() -> OhlcvResult<Arc<OhlcvServiceImpl>> {
    let service = OHLCV_SERVICE
        .get_or_try_init(|| async {
            logger::info(
                LogTag::Ohlcv,
                &"INIT: Initializing OHLCV runtime".to_string(),
            );

            let db_path = crate::paths::get_ohlcvs_db_path();
            let service_impl = OhlcvServiceImpl::new(db_path)?;

            logger::info(LogTag::Ohlcv, &"SUCCESS: OHLCV runtime ready".to_string());
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
        let service = get_or_init_service().await?;

        let monitor_instance = Arc::clone(&service.monitor);

        // Start background monitoring tasks before awaiting shutdown
        monitor_instance.clone().start().await?;
        logger::info(
            LogTag::Ohlcv,
            &"TASK_START: OHLCV monitoring tasks started".to_string(),
        );

        let shutdown_task = tokio::spawn(monitor.instrument(async move {
            shutdown.notified().await;
            logger::info(
                LogTag::Ohlcv,
                &"TASK_STOP: Shutdown signal received for OHLCV monitoring".to_string(),
            );
            monitor_instance.stop().await;
            logger::info(
                LogTag::Ohlcv,
                &"TASK_END: OHLCV monitoring tasks stopped".to_string(),
            );
        }));

        Ok(vec![shutdown_task])
    }

    pub async fn has_data(mint: &str) -> OhlcvResult<bool> {
        let service = get_or_init_service().await?;
        service.has_data(mint)
    }
}

// ==================== Public API Functions ====================

pub async fn get_ohlcv_data(
    mint: &str,
    timeframe: Timeframe,
    pool_address: Option<&str>,
    limit: usize,
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>,
) -> OhlcvResult<Vec<Candle>> {
    let service = get_or_init_service().await?;

    service
        .get_ohlcv_data(
            mint,
            timeframe,
            pool_address,
            limit,
            from_timestamp,
            to_timestamp,
        )
        .await
}

pub async fn get_available_pools(mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
    let service = get_or_init_service().await?;

    service.pool_manager.get_pool_metadata(mint).await
}

pub async fn get_data_gaps(mint: &str, timeframe: Timeframe) -> OhlcvResult<Vec<(i64, i64)>> {
    let service = get_or_init_service().await?;

    let gaps = service
        .gap_manager
        .get_unfilled_gaps(mint, timeframe)
        .await?;

    Ok(gaps
        .into_iter()
        .map(|g| (g.start_timestamp, g.end_timestamp))
        .collect())
}

pub async fn request_refresh(mint: &str) -> OhlcvResult<()> {
    let service = get_or_init_service().await?;

    // Record activity
    service
        .monitor
        .record_activity(mint, ActivityType::DataRequested)
        .await?;

    // Force refresh
    service.monitor.force_refresh(mint).await
}

pub async fn add_token_monitoring(mint: &str, priority: Priority) -> OhlcvResult<()> {
    let service = get_or_init_service().await?;

    service.monitor.add_token(mint.to_string(), priority).await
}

pub async fn remove_token_monitoring(mint: &str) -> OhlcvResult<()> {
    let service = get_or_init_service().await?;

    service.monitor.remove_token(mint).await
}

pub async fn record_activity(mint: &str, activity_type: ActivityType) -> OhlcvResult<()> {
    let service = get_or_init_service().await?;

    service.monitor.record_activity(mint, activity_type).await
}

pub async fn get_metrics() -> OhlcvMetrics {
    if let Some(service) = OHLCV_SERVICE.get() {
        get_metrics_impl(service.as_ref()).await
    } else {
        OhlcvMetrics::default()
    }
}

pub async fn get_monitor_stats() -> Option<MonitorStats> {
    if let Some(service) = OHLCV_SERVICE.get() {
        Some(service.monitor.get_stats().await)
    } else {
        None
    }
}

pub async fn has_data(mint: &str) -> OhlcvResult<bool> {
    let service = get_or_init_service().await?;
    let service_clone = service.clone();
    let mint_owned = mint.to_string();

    // Wrap sync DB call in spawn_blocking to prevent blocking async runtime
    tokio::task::spawn_blocking(move || service_clone.has_data(&mint_owned))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {}", e)))?
}

pub async fn get_mints_with_data(mints: &[String]) -> OhlcvResult<HashSet<String>> {
    if mints.is_empty() {
        return Ok(HashSet::new());
    }

    let service = get_or_init_service().await?;
    let service_clone = service.clone();
    let owned = mints.to_vec();

    tokio::task::spawn_blocking(move || service_clone.get_mints_with_data(&owned))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {}", e)))?
}

async fn get_metrics_impl(service: &OhlcvServiceImpl) -> OhlcvMetrics {
    let stats = service.monitor.get_stats().await;

    let tokens_monitored = stats.total_tokens;
    // Offload synchronous DB calls to blocking threads to avoid stalling async runtime
    let db = Arc::clone(&service.db);
    let (pools_tracked, data_points_stored, gaps_detected, gaps_filled) =
        tokio::task::spawn_blocking(move || {
            let pools = db.get_pool_count().unwrap_or(0);
            let points = db.get_data_point_count().unwrap_or(0);
            let gaps_det = db.get_gap_count(false).unwrap_or(0);
            let gaps_fill = db.get_gap_count(true).unwrap_or(0);
            (pools, points, gaps_det, gaps_fill)
        })
        .await
        .unwrap_or((0, 0, 0, 0));

    // Calculate database size (rough estimate)
    let database_size_bytes = (data_points_stored as u128).saturating_mul(64);
    let database_size_mb = (database_size_bytes as f64) / (1024.0 * 1024.0); // ~64 bytes per point

    OhlcvMetrics {
        tokens_monitored,
        pools_tracked,
        api_calls_per_minute: service.fetcher.calls_per_minute(),
        cache_hit_rate: service.cache.hit_rate(),
        average_fetch_latency_ms: service.fetcher.average_latency_ms(),
        gaps_detected,
        gaps_filled,
        data_points_stored,
        database_size_mb,
        oldest_data_timestamp: None, // Could query DB for this if needed
    }
}

// ==================== Phase 2: Bundle Cache API ====================

/// Get timeframe bundle from cache for strategy evaluation (non-blocking)
/// Returns None if bundle is stale or missing - background worker will prepare it
pub async fn get_timeframe_bundle(mint: &str) -> OhlcvResult<Option<TimeframeBundle>> {
    let service = get_or_init_service().await?;
    service.get_timeframe_bundle(mint).await
}

/// Build complete timeframe bundle (used by background worker and on-demand)
pub async fn build_timeframe_bundle(mint: &str) -> OhlcvResult<TimeframeBundle> {
    let service = get_or_init_service().await?;
    service.build_timeframe_bundle(mint).await
}

/// Store bundle in cache with LRU eviction
/// Takes bundle by value to avoid unnecessary cloning
pub async fn store_bundle(mint: String, bundle: TimeframeBundle) -> OhlcvResult<()> {
    let service = get_or_init_service().await?;
    service.store_bundle(mint, bundle).await
}
