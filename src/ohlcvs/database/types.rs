// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV database types — row structs for SQLite serialization.

use crate::ohlcvs::types::{PoolConfig, Timeframe};

/// An unfilled gap row of one pool, with its retry bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapRecord {
    pub timeframe: Timeframe,
    /// First missing bucket (unix secs).
    pub start_timestamp: i64,
    /// Last missing bucket (unix secs, inclusive).
    pub end_timestamp: i64,
    pub attempts: u32,
    /// Unix secs of the last fill attempt.
    pub last_attempt: Option<i64>,
}

/// The stored row of one bucket of a series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredBucket {
    /// Written by a native source, not derived locally from 1m rows.
    pub native: bool,
    /// When the row's values last changed (unix secs).
    pub fetched_at: Option<i64>,
}

/// Stored candles of one timeframe on one pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeframeSummary {
    pub timeframe: String,
    pub candles: i64,
    pub earliest: Option<i64>,
    pub latest: Option<i64>,
}

/// Status information for a single OHLCV token
#[derive(Debug, Clone)]
pub struct OhlcvTokenStatus {
    pub chain: crate::chains::ChainId,
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

/// A token's pools to register and the one series (default) pool among them, planned from the
/// pool rows read inside the write transaction that applies it.
#[derive(Debug, Clone)]
pub struct SeriesPoolPlan {
    /// Every pool to keep registered; registered pools missing from it are deleted.
    pub pools: Vec<PoolConfig>,
    /// The address of the series pool, one of `pools`.
    pub series: String,
}

/// Result of writing a token's registered pools (`OhlcvDatabase::write_series_pools`).
#[derive(Debug, Clone)]
pub struct SeriesPoolWrite {
    /// The plan that was written.
    pub plan: SeriesPoolPlan,
    /// Registered pools that were not in the new set, deleted with their candles and gaps.
    pub removed_pools: Vec<String>,
    /// Set when the series pool moved and the token's series was reset.
    pub reset: Option<SeriesPoolReset>,
}

/// Result of moving a token's candle series onto a new pool: the rows of every
/// other pool removed. The token's backfill flags are reset with them.
#[derive(Debug, Clone, Default)]
pub struct SeriesPoolReset {
    /// The series pool before the move; `None` when the token had no default pool.
    pub previous_pool: Option<String>,
    /// Candle rows deleted by the write, removed pools included.
    pub candles_deleted: usize,
    /// Gap rows deleted by the write, removed pools included.
    pub gaps_deleted: usize,
}

/// Database statistics
#[derive(Debug, Clone, Default)]
pub struct DatabaseStats {
    pub total_candles: usize,
    pub total_gaps: usize,
    pub total_pools: usize,
    pub total_configs: usize,
    pub active_configs: usize,
    pub database_size_bytes: u64,
}
