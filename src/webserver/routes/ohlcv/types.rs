//! OHLCV route types — request/response structs and query parameters.

use crate::ohlcvs::{Candle, PoolMetadata};
use serde::{Deserialize, Serialize};

// ==================== Response Types ====================

#[derive(Debug, Serialize)]
pub(super) struct OhlcvDataResponse {
    pub mint: String,
    pub pool_address: Option<String>,
    pub timeframe: String,
    pub data: Vec<Candle>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct PoolsResponse {
    pub mint: String,
    pub pools: Vec<PoolMetadata>,
    pub default_pool: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct GapsResponse {
    pub mint: String,
    pub timeframe: String,
    pub gaps: Vec<GapInfo>,
    pub total_gaps: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct GapInfo {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub duration_seconds: i64,
}

#[derive(Debug, Serialize)]
pub(super) struct DataStatusResponse {
    pub mint: String,
    pub has_data: bool,
    pub timeframes_available: Vec<String>,
    pub latest_timestamp: Option<i64>,
    pub data_quality: String,
}

#[derive(Debug, Serialize)]
pub(super) struct MetricsResponse {
    pub tokens_monitored: usize,
    pub pools_tracked: usize,
    pub api_calls_per_minute: f64,
    pub cache_hit_rate_percent: f64,
    pub average_fetch_latency_ms: f64,
    pub gaps_detected: usize,
    pub gaps_filled: usize,
    pub data_points_stored: usize,
    pub database_size_mb: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct OhlcvTokenListResponse {
    pub tokens: Vec<OhlcvTokenItem>,
    pub total_count: usize,
    pub stats: OhlcvStatsResponse,
}

#[derive(Debug, Serialize)]
pub(super) struct OhlcvTokenItem {
    pub mint: String,
    pub priority: String,
    pub status: String,
    pub is_active: bool,
    pub fetch_interval_seconds: i64,
    pub last_fetch: Option<String>,
    pub last_activity: String,
    pub consecutive_empty_fetches: i64,
    pub consecutive_pool_failures: i64,
    pub backfill_progress: BackfillProgress,
    pub candle_count: i64,
    pub earliest_timestamp: i64,
    pub latest_timestamp: i64,
    pub data_span_hours: f64,
    pub open_gaps: i64,
    pub pool_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub(super) struct BackfillProgress {
    pub completed: u8,
    pub total: u8,
    pub percent: f64,
    pub timeframes: BackfillTimeframes,
}

#[derive(Debug, Serialize)]
pub(super) struct BackfillTimeframes {
    #[serde(rename = "1m")]
    pub m1: bool,
    #[serde(rename = "5m")]
    pub m5: bool,
    #[serde(rename = "15m")]
    pub m15: bool,
    #[serde(rename = "1h")]
    pub h1: bool,
    #[serde(rename = "4h")]
    pub h4: bool,
    #[serde(rename = "12h")]
    pub h12: bool,
    #[serde(rename = "1d")]
    pub d1: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct OhlcvStatsResponse {
    pub total_tokens: usize,
    pub active_tokens: usize,
    pub total_candles: usize,
    pub total_gaps: usize,
    pub total_pools: usize,
    pub database_size_mb: f64,
}

#[derive(Debug, Serialize)]
pub(super) struct DeleteTokenResponse {
    pub mint: String,
    pub candles_deleted: usize,
    pub gaps_deleted: usize,
    pub pools_deleted: usize,
    pub config_deleted: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct CleanupResponse {
    pub deleted_count: usize,
    pub deleted_mints: Vec<String>,
}

// ==================== Query Parameters ====================

#[derive(Debug, Deserialize)]
pub(super) struct OhlcvQuery {
    pub timeframe: Option<String>,
    pub pool: Option<String>,
    pub limit: Option<usize>,
    pub from: Option<i64>,
    pub to: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GapsQuery {
    pub timeframe: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct MonitorRequest {
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CleanupRequest {
    pub inactive_hours: Option<i64>,
}
