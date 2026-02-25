//! OHLCV service — public API functions for querying candle data.

use super::database::{DatabaseStats, DeleteResult, OhlcvTokenStatus};
use super::priorities::ActivityType;
use super::service::{get_or_init_service, OhlcvServiceImpl, OHLCV_SERVICE};
use super::types::{
    Candle, MonitorStats, OhlcvError, OhlcvMetrics, OhlcvResult, PoolMetadata, Priority, Timeframe,
    TimeframeBundle,
};
use std::collections::HashSet;
use std::sync::Arc;

// ==================== Public API Functions ====================

/// Fetch OHLCV candle data for a token with optional pool and time range filters.
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

/// List available pools for a token that have OHLCV data.
pub async fn get_available_pools(mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
    let service = get_or_init_service().await?;

    service.pool_manager.get_pool_metadata(mint).await
}

/// Get unfilled data gaps for a token's OHLCV timeline.
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

pub async fn update_token_priority(mint: &str, priority: Priority) -> OhlcvResult<()> {
    let service = get_or_init_service().await?;

    service.monitor.update_priority(mint, priority).await
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
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
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
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

async fn get_metrics_impl(service: &OhlcvServiceImpl) -> OhlcvMetrics {
    let stats = service.monitor.get_stats().await;

    let tokens_monitored = stats.total_tokens;
    // Offload synchronous DB calls to blocking threads to avoid stalling async runtime
    let db = Arc::clone(&service.db);
    let (pools_tracked, data_points_stored, gaps_detected, gaps_filled) =
        tokio::task::spawn_blocking(move || {
            let pools = db.get_pool_count().unwrap_or_default();
            let points = db.get_data_point_count().unwrap_or_default();
            let gaps_det = db.get_gap_count(false).unwrap_or_default();
            let gaps_fill = db.get_gap_count(true).unwrap_or_default();
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

// ==================== OHLCV Listing and Management API ====================

/// Get all OHLCV tokens with their status information
pub async fn get_all_tokens_with_status() -> OhlcvResult<Vec<OhlcvTokenStatus>> {
    let service = get_or_init_service().await?;
    let db = Arc::clone(&service.db);

    tokio::task::spawn_blocking(move || db.get_all_tokens_with_status())
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

/// Delete all OHLCV data for a specific token
pub async fn delete_token_data(mint: &str) -> OhlcvResult<DeleteResult> {
    let service = get_or_init_service().await?;
    let db = Arc::clone(&service.db);
    let mint_owned = mint.to_string();

    // Also remove from monitoring
    let _ = service.monitor.remove_token(&mint_owned).await;

    tokio::task::spawn_blocking(move || db.delete_token_data(&mint_owned))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

/// Delete OHLCV data for tokens that have been inactive for specified hours
pub async fn delete_inactive_tokens(inactive_hours: i64) -> OhlcvResult<Vec<String>> {
    let service = get_or_init_service().await?;
    let db = Arc::clone(&service.db);

    tokio::task::spawn_blocking(move || db.delete_inactive_tokens(inactive_hours))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

/// Get database statistics
pub async fn get_database_stats() -> OhlcvResult<DatabaseStats> {
    let service = get_or_init_service().await?;
    let db = Arc::clone(&service.db);

    tokio::task::spawn_blocking(move || db.get_database_stats())
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}
