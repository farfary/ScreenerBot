// Main OHLCV service implementation

use crate::arguments::is_debug_ohlcv_enabled;
use crate::logger::{log, LogTag};
use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::monitor::{MonitorStats, OhlcvMonitor};
use crate::ohlcvs::priorities::ActivityType;
use crate::ohlcvs::types::{
    OhlcvDataPoint, OhlcvError, OhlcvMetrics, OhlcvResult, PoolMetadata, Priority, Timeframe,
};
use chrono::Utc;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Notify, OnceCell};
use tokio::task::JoinHandle;

static OHLCV_SERVICE: OnceCell<Arc<OhlcvServiceImpl>> = OnceCell::const_new();

pub struct OhlcvService;

struct OhlcvServiceImpl {
    db: Arc<OhlcvDatabase>,
    fetcher: Arc<OhlcvFetcher>,
    cache: Arc<OhlcvCache>,
    pool_manager: Arc<PoolManager>,
    gap_manager: Arc<GapManager>,
    monitor: Arc<OhlcvMonitor>,
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

        Ok(Self {
            db,
            fetcher,
            cache,
            pool_manager,
            gap_manager,
            monitor,
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
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
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

            // Filter by timestamp if needed
            let filtered: Vec<OhlcvDataPoint> = cached_data
                .into_iter()
                .filter(|d| {
                    (from_timestamp.is_none() || d.timestamp >= from_timestamp.unwrap())
                        && (to_timestamp.is_none() || d.timestamp <= to_timestamp.unwrap())
                })
                .collect();

            if !filtered.is_empty() {
                // Take last N entries (most recent)
                let start_idx = filtered.len().saturating_sub(limit);
                return Ok(filtered.into_iter().skip(start_idx).collect());
            }
        }

        // Try aggregated cache in database
        if timeframe != Timeframe::Minute1 {
            let mut aggregated = self.db.get_aggregated_data(
                mint,
                &pool,
                timeframe,
                from_timestamp,
                to_timestamp,
                limit,
            )?;

            if !aggregated.is_empty() {
                // Normalize to ASC ordering
                aggregated.sort_by_key(|d| d.timestamp);

                // Update cache with normalized data
                let _ = self
                    .cache
                    .put(mint, Some(&pool), timeframe, aggregated.clone());

                // Take last N entries (most recent)
                let start_idx = aggregated.len().saturating_sub(limit);
                return Ok(aggregated.into_iter().skip(start_idx).collect());
            }
        }

        // Fall back to raw 1m data and aggregate
        let raw_data = self.db.get_1m_data(
            mint,
            Some(&pool),
            from_timestamp,
            to_timestamp,
            limit * 1000,
        )?; // Fetch more for aggregation

        if raw_data.is_empty() {
            return Ok(Vec::new());
        }

        // Aggregate if needed
        let mut result = if timeframe == Timeframe::Minute1 {
            raw_data
        } else {
            OhlcvAggregator::aggregate(&raw_data, timeframe)?
        };

        // Normalize to ASC ordering
        result.sort_by_key(|d| d.timestamp);

        // Cache the result
        let _ = self.cache.put(mint, Some(&pool), timeframe, result.clone());

        // Take only requested limit from the end (most recent)
        let start_idx = result.len().saturating_sub(limit);
        Ok(result.into_iter().skip(start_idx).collect())
    }

    fn has_data(&self, mint: &str) -> OhlcvResult<bool> {
        self.db.has_data_for_mint(mint)
    }

    fn get_mints_with_data(&self, mints: &[String]) -> OhlcvResult<HashSet<String>> {
        self.db.get_mints_with_data(mints)
    }
}

async fn get_or_init_service() -> OhlcvResult<Arc<OhlcvServiceImpl>> {
    let service = OHLCV_SERVICE
        .get_or_try_init(|| async {
            if is_debug_ohlcv_enabled() {
                log(LogTag::Ohlcv, "INIT", "Initializing OHLCV runtime");
            }

            // Use config for DB path
            let db_path = PathBuf::from("data").join("ohlcvs.db");
            let service_impl = OhlcvServiceImpl::new(db_path)?;

            if is_debug_ohlcv_enabled() {
                log(LogTag::Ohlcv, "SUCCESS", "OHLCV runtime ready");
            }
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
        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "TASK_START",
                "OHLCV monitoring tasks started",
            );
        }

        let shutdown_task = tokio::spawn(monitor.instrument(async move {
            shutdown.notified().await;
            if is_debug_ohlcv_enabled() {
                log(
                    LogTag::Ohlcv,
                    "TASK_STOP",
                    "Shutdown signal received for OHLCV monitoring",
                );
            }
            monitor_instance.stop().await;
            if is_debug_ohlcv_enabled() {
                log(LogTag::Ohlcv, "TASK_END", "OHLCV monitoring tasks stopped");
            }
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
) -> OhlcvResult<Vec<OhlcvDataPoint>> {
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
