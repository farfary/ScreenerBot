// OHLCV Data Module
// Provides comprehensive OHLCV (Open, High, Low, Close, Volume) data management
// with multi-timeframe support, intelligent caching, and smart monitoring.

mod aggregator;
mod cache;
mod database;
mod fetcher;
mod gaps;
mod manager;
mod monitor;
mod priorities;
mod service;
mod types;

pub use types::{
    Candle, OhlcvError, OhlcvMetrics, OhlcvResult, PoolConfig, PoolMetadata, Priority,
    Timeframe, TimeframeBundle, TokenOhlcvConfig, BUNDLE_CANDLE_COUNT,
};

pub use monitor::{MonitorStats, MonitorTelemetrySnapshot};
pub use priorities::ActivityType;
pub use service::OhlcvService;

use cache::OhlcvCache;
use database::OhlcvDatabase;
use fetcher::OhlcvFetcher;
use manager::PoolManager;
use monitor::OhlcvMonitor;
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
    service::get_ohlcv_data(
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
    service::get_available_pools(mint).await
}

pub async fn get_data_gaps(mint: &str, timeframe: Timeframe) -> OhlcvResult<Vec<(i64, i64)>> {
    service::get_data_gaps(mint, timeframe).await
}

pub async fn request_refresh(mint: &str) -> OhlcvResult<()> {
    service::request_refresh(mint).await
}

pub async fn get_metrics() -> OhlcvMetrics {
    service::get_metrics().await
}

pub async fn get_monitor_stats() -> Option<MonitorStats> {
    service::get_monitor_stats().await
}

pub async fn has_data(mint: &str) -> OhlcvResult<bool> {
    service::has_data(mint).await
}

pub async fn get_mints_with_data(mints: &[String]) -> OhlcvResult<HashSet<String>> {
    service::get_mints_with_data(mints).await
}

pub async fn add_token_monitoring(mint: &str, priority: Priority) -> OhlcvResult<()> {
    service::add_token_monitoring(mint, priority).await
}

pub async fn remove_token_monitoring(mint: &str) -> OhlcvResult<()> {
    service::remove_token_monitoring(mint).await
}

pub async fn record_activity(mint: &str, activity_type: ActivityType) -> OhlcvResult<()> {
    service::record_activity(mint, activity_type).await
}

// Phase 2: Bundle Cache API for strategy evaluation
pub async fn get_timeframe_bundle(mint: &str) -> OhlcvResult<Option<TimeframeBundle>> {
    service::get_timeframe_bundle(mint).await
}

pub async fn build_timeframe_bundle(mint: &str) -> OhlcvResult<TimeframeBundle> {
    service::build_timeframe_bundle(mint).await
}

pub async fn store_bundle(mint: String, bundle: TimeframeBundle) -> OhlcvResult<()> {
    service::store_bundle(mint, bundle).await
}
