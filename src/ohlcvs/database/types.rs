//! OHLCV database types — row structs for SQLite serialization.

/// Status information for a single OHLCV token
#[derive(Debug, Clone)]
pub struct OhlcvTokenStatus {
    pub mint: String,
    pub priority: String,
    pub fetch_interval_seconds: i64,
    pub is_active: bool,
    pub last_fetch: Option<String>,
    pub last_activity: String,
    pub consecutive_empty_fetches: i64,
    pub consecutive_pool_failures: i64,
    pub backfill_1m: bool,
    pub backfill_5m: bool,
    pub backfill_15m: bool,
    pub backfill_1h: bool,
    pub backfill_4h: bool,
    pub backfill_12h: bool,
    pub backfill_1d: bool,
    pub created_at: String,
    pub updated_at: String,
    pub candle_count: i64,
    pub earliest_timestamp: i64,
    pub latest_timestamp: i64,
    pub open_gaps: i64,
    pub pool_count: i64,
}

/// Result of a delete operation
#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub candles_deleted: usize,
    pub gaps_deleted: usize,
    pub pools_deleted: usize,
    pub config_deleted: usize,
}

/// Result of clearing ALL cached OHLCV candle data (manual clear or a data
/// version bump). Pools and the monitoring list are preserved.
#[derive(Debug, Clone, Default)]
pub struct ClearAllResult {
    pub candles_deleted: usize,
    pub gaps_deleted: usize,
    pub tokens_reset: usize,
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_candles: usize,
    pub total_gaps: usize,
    pub total_pools: usize,
    pub total_configs: usize,
    pub active_configs: usize,
    pub database_size_bytes: u64,
}
