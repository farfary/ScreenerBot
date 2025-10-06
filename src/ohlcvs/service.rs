// Main OHLCV service implementation

use crate::ohlcvs::aggregator::OhlcvAggregator;
use crate::ohlcvs::cache::OhlcvCache;
use crate::ohlcvs::database::OhlcvDatabase;
use crate::ohlcvs::fetcher::OhlcvFetcher;
use crate::ohlcvs::gaps::GapManager;
use crate::ohlcvs::manager::PoolManager;
use crate::ohlcvs::monitor::OhlcvMonitor;
use crate::ohlcvs::priorities::ActivityType;
use crate::ohlcvs::types::{
    OhlcvDataPoint,
    OhlcvError,
    OhlcvMetrics,
    OhlcvResult,
    PoolMetadata,
    Priority,
    Timeframe,
};
use crate::services::Service;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

static OHLCV_SERVICE: RwLock<Option<Arc<OhlcvServiceImpl>>> = RwLock::const_new(None);

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
        let monitor = Arc::new(
            OhlcvMonitor::new(
                Arc::clone(&db),
                Arc::clone(&fetcher),
                Arc::clone(&cache),
                Arc::clone(&pool_manager),
                Arc::clone(&gap_manager)
            )
        );

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
        to_timestamp: Option<i64>
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        // Determine pool to use
        let pool = if let Some(addr) = pool_address {
            addr.to_string()
        } else {
            // Use default pool
            let default_pool = self.pool_manager
                .get_default_pool(mint).await?
                .or_else(|| {
                    // Fall back to best pool
                    futures::executor
                        ::block_on(self.pool_manager.get_best_pool(mint))
                        .ok()
                        .flatten()
                })
                .ok_or_else(|| OhlcvError::PoolNotFound(mint.to_string()))?;

            default_pool.address
        };

        // Try cache first
        if let Ok(Some(cached_data)) = self.cache.get(mint, Some(&pool), timeframe) {
            // Filter by timestamp if needed
            let filtered: Vec<OhlcvDataPoint> = cached_data
                .into_iter()
                .filter(|d| {
                    (from_timestamp.is_none() || d.timestamp >= from_timestamp.unwrap()) &&
                        (to_timestamp.is_none() || d.timestamp <= to_timestamp.unwrap())
                })
                .take(limit)
                .collect();

            if !filtered.is_empty() {
                return Ok(filtered);
            }
        }

        // Try aggregated cache in database
        if timeframe != Timeframe::Minute1 {
            let aggregated = self.db.get_aggregated_data(
                mint,
                &pool,
                timeframe,
                from_timestamp,
                to_timestamp,
                limit
            )?;

            if !aggregated.is_empty() {
                // Update cache
                let _ = self.cache.put(mint, Some(&pool), timeframe, aggregated.clone());
                return Ok(aggregated);
            }
        }

        // Fall back to raw 1m data and aggregate
        let raw_data = self.db.get_1m_data(
            mint,
            Some(&pool),
            from_timestamp,
            to_timestamp,
            limit * 1000
        )?; // Fetch more for aggregation

        if raw_data.is_empty() {
            return Ok(Vec::new());
        }

        // Aggregate if needed
        let result = if timeframe == Timeframe::Minute1 {
            raw_data
        } else {
            OhlcvAggregator::aggregate(&raw_data, timeframe)?
        };

        // Cache the result
        let _ = self.cache.put(mint, Some(&pool), timeframe, result.clone());

        // Take only requested limit
        Ok(result.into_iter().take(limit).collect())
    }
}

#[async_trait::async_trait]
impl Service for OhlcvService {
    fn name(&self) -> &'static str {
        "ohlcv"
    }

    fn priority(&self) -> u32 {
        50 // Medium priority
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["rpc_stats"] // May need RPC for pool discovery
    }

    async fn initialize(&self) -> Result<(), String> {
        let db_path = PathBuf::from("data/ohlcvs.db");

        let service_impl = OhlcvServiceImpl::new(db_path).map_err(|e|
            format!("Failed to initialize OHLCV service: {}", e)
        )?;

        let mut global = OHLCV_SERVICE.write().await;
        *global = Some(Arc::new(service_impl));

        println!("[OHLCV Service] Initialized");
        Ok(())
    }

    async fn start(
        &self,
        _monitor: crate::services::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let service = {
            let global = OHLCV_SERVICE.read().await;
            global.as_ref().ok_or("OHLCV service not initialized")?.clone()
        };

        // Start monitoring
        service.monitor.start().await.map_err(|e| format!("Failed to start monitor: {}", e))?;

        println!("[OHLCV Service] Started monitoring");

        // Return empty handles - monitor spawns its own tasks internally
        Ok(vec![])
    }

    async fn health(&self) -> Result<serde_json::Value, String> {
        let service = {
            let global = OHLCV_SERVICE.read().await;
            global.as_ref().ok_or("Service not initialized")?.clone()
        };

        let stats = service.monitor.get_stats().await;

        Ok(
            serde_json::json!({
            "status": "healthy",
            "tokens_monitored": stats.total_tokens,
            "cache_hit_rate": format!("{:.2}%", stats.cache_hit_rate * 100.0),
            "api_calls_per_minute": format!("{:.1}", stats.api_calls_per_minute),
            "queue_size": stats.queue_size,
        })
        )
    }

    async fn metrics(&self) -> Result<serde_json::Value, String> {
        let service = {
            let global = OHLCV_SERVICE.read().await;
            global.as_ref().ok_or("Service not initialized")?.clone()
        };

        let metrics = get_metrics_impl(&service).await;

        Ok(serde_json::to_value(metrics).map_err(|e| e.to_string())?)
    }
}

// ==================== Public API Functions ====================

pub async fn get_ohlcv_data(
    mint: &str,
    timeframe: Timeframe,
    pool_address: Option<&str>,
    limit: usize,
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>
) -> OhlcvResult<Vec<OhlcvDataPoint>> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    service.get_ohlcv_data(mint, timeframe, pool_address, limit, from_timestamp, to_timestamp).await
}

pub async fn get_available_pools(mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    service.pool_manager.get_pool_metadata(mint).await
}

pub async fn get_data_gaps(mint: &str, timeframe: Timeframe) -> OhlcvResult<Vec<(i64, i64)>> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    let gaps = service.gap_manager.get_unfilled_gaps(mint, timeframe).await?;

    Ok(
        gaps
            .into_iter()
            .map(|g| (g.start_timestamp, g.end_timestamp))
            .collect()
    )
}

pub async fn request_refresh(mint: &str) -> OhlcvResult<()> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    // Record activity
    service.monitor.record_activity(mint, ActivityType::DataRequested).await?;

    // Force refresh
    service.monitor.force_refresh(mint).await
}

pub async fn add_token_monitoring(mint: &str, priority: Priority) -> OhlcvResult<()> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    service.monitor.add_token(mint.to_string(), priority).await
}

pub async fn remove_token_monitoring(mint: &str) -> OhlcvResult<()> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    service.monitor.remove_token(mint).await
}

pub async fn record_activity(mint: &str, activity_type: ActivityType) -> OhlcvResult<()> {
    let service = {
        let global = OHLCV_SERVICE.read().await;
        global
            .as_ref()
            .ok_or_else(|| OhlcvError::NotFound("Service not initialized".to_string()))?
            .clone()
    };

    service.monitor.record_activity(mint, activity_type).await
}

pub async fn get_metrics() -> OhlcvMetrics {
    let service = match OHLCV_SERVICE.read().await.as_ref() {
        Some(s) => s.clone(),
        None => {
            return OhlcvMetrics::default();
        }
    };

    get_metrics_impl(&service).await
}

async fn get_metrics_impl(service: &OhlcvServiceImpl) -> OhlcvMetrics {
    let stats = service.monitor.get_stats().await;

    let tokens_monitored = stats.total_tokens;
    let pools_tracked = service.db.get_pool_count().unwrap_or(0);
    let data_points_stored = service.db.get_data_point_count().unwrap_or(0);
    let gaps_detected = service.db.get_gap_count(false).unwrap_or(0);
    let gaps_filled = service.db.get_gap_count(true).unwrap_or(0);

    // Calculate database size (rough estimate)
    let database_size_mb = ((data_points_stored as f64) * 64.0) / (1024.0 * 1024.0); // ~64 bytes per point

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_initialization() {
        let service = OhlcvService;
        assert_eq!(service.name(), "ohlcv");
        assert_eq!(service.priority(), 50);
    }
}
