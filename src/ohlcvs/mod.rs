//! OHLCV candlestick data — fetching, caching, and gap filling for price charts.
mod aggregator;
mod cache;
mod database;
mod fetcher;
mod gaps;
mod manager;
mod monitor;
mod priorities;
mod service;
mod service_api;
pub mod sol_usd_chart;
mod types;

pub use types::{
    Candle, MonitorStats, MonitorTelemetrySnapshot, OhlcvError, OhlcvMetrics, OhlcvResult,
    OhlcvStatus, OhlcvTimeframeStatus, PoolConfig, PoolMetadata, Priority, Timeframe,
    TimeframeBundle, TokenOhlcvConfig, BUNDLE_CANDLE_COUNT,
};

pub use database::{ClearAllResult, DatabaseStats, DeleteResult, OhlcvTokenStatus};
pub use priorities::ActivityType;
pub use service::OhlcvService;

use std::collections::HashSet;

// Public API for accessing OHLCV data
pub async fn get_ohlcv_data(
    mint: &str,
    timeframe: Timeframe,
    pool_address: Option<&str>,
    limit: usize,
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>,
) -> OhlcvResult<Vec<Candle>> {
    service_api::get_ohlcv_data(
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
    service_api::get_available_pools(mint).await
}

pub async fn get_data_gaps(mint: &str, timeframe: Timeframe) -> OhlcvResult<Vec<(i64, i64)>> {
    service_api::get_data_gaps(mint, timeframe).await
}

pub async fn request_refresh(mint: &str) -> OhlcvResult<()> {
    service_api::request_refresh(mint).await
}

pub async fn get_metrics() -> OhlcvMetrics {
    service_api::get_metrics().await
}

pub async fn get_monitor_stats() -> Option<MonitorStats> {
    service_api::get_monitor_stats().await
}

pub async fn has_data(mint: &str) -> OhlcvResult<bool> {
    service_api::has_data(mint).await
}

pub async fn get_status(mint: &str) -> OhlcvResult<OhlcvStatus> {
    service_api::get_status(mint).await
}

pub async fn get_mints_with_data(mints: &[String]) -> OhlcvResult<HashSet<String>> {
    service_api::get_mints_with_data(mints).await
}

pub async fn add_token_monitoring(mint: &str, priority: Priority) -> OhlcvResult<()> {
    service_api::add_token_monitoring(mint, priority).await
}

pub async fn remove_token_monitoring(mint: &str) -> OhlcvResult<()> {
    service_api::remove_token_monitoring(mint).await
}

pub async fn update_token_priority(mint: &str, priority: Priority) -> OhlcvResult<()> {
    service_api::update_token_priority(mint, priority).await
}

pub async fn record_activity(mint: &str, activity_type: ActivityType) -> OhlcvResult<()> {
    service_api::record_activity(mint, activity_type).await
}

// Phase 2: Bundle Cache API for strategy evaluation
pub async fn get_timeframe_bundle(mint: &str) -> OhlcvResult<Option<TimeframeBundle>> {
    service_api::get_timeframe_bundle(mint).await
}

pub async fn build_timeframe_bundle(mint: &str) -> OhlcvResult<TimeframeBundle> {
    service_api::build_timeframe_bundle(mint).await
}

pub async fn store_bundle(mint: String, bundle: TimeframeBundle) -> OhlcvResult<()> {
    service_api::store_bundle(mint, bundle).await
}

// OHLCV listing and management API
pub async fn get_all_tokens_with_status() -> OhlcvResult<Vec<OhlcvTokenStatus>> {
    service_api::get_all_tokens_with_status().await
}

pub async fn delete_token_data(mint: &str) -> OhlcvResult<DeleteResult> {
    service_api::delete_token_data(mint).await
}

pub async fn delete_inactive_tokens(inactive_hours: i64) -> OhlcvResult<Vec<String>> {
    service_api::delete_inactive_tokens(inactive_hours).await
}

pub async fn clear_all_ohlcv_data() -> OhlcvResult<ClearAllResult> {
    service_api::clear_all_ohlcv_data().await
}

pub async fn get_database_stats() -> OhlcvResult<DatabaseStats> {
    service_api::get_database_stats().await
}
