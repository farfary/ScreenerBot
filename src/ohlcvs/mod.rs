// OHLCV Data Module
// Provides comprehensive OHLCV (Open, High, Low, Close, Volume) data management
// with multi-timeframe support, intelligent caching, and smart monitoring.

mod types;
mod database;
mod cache;
mod fetcher;
mod manager;
mod aggregator;
mod gaps;
mod priorities;
mod monitor;
mod service;

pub use types::{
    Timeframe,
    OhlcvDataPoint,
    TokenOhlcvConfig,
    PoolConfig,
    Priority,
    OhlcvMetrics,
    OhlcvResult,
    OhlcvError,
    PoolMetadata,
};

pub use service::OhlcvService;

use database::OhlcvDatabase;
use cache::OhlcvCache;
use fetcher::OhlcvFetcher;
use manager::PoolManager;
use monitor::OhlcvMonitor;

// Public API for accessing OHLCV data
pub async fn get_ohlcv_data(
    mint: &str,
    timeframe: Timeframe,
    pool_address: Option<&str>,
    limit: usize,
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>
) -> OhlcvResult<Vec<OhlcvDataPoint>> {
    service::get_ohlcv_data(
        mint,
        timeframe,
        pool_address,
        limit,
        from_timestamp,
        to_timestamp
    ).await
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
