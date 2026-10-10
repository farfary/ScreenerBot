// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV data types — candles, timeframes, priorities, and fetch configuration.

use crate::config::with_config;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// First pause between fetches of an unhealthy pool (see `PoolConfig::fetch_retry_delay`).
pub const UNHEALTHY_POOL_RETRY_BASE: Duration = Duration::from_secs(60);
/// Longest pause between fetches of an unhealthy pool.
pub const UNHEALTHY_POOL_RETRY_MAX: Duration = Duration::from_secs(30 * 60);

/// Timeframes whose kept history (`Timeframe::max_history_candles`) reaches past the
/// backfill page, so the monitor pages them further back from the Data Server.
pub const DEEP_HISTORY_TIMEFRAMES: [Timeframe; 4] = [
    Timeframe::Hour1,
    Timeframe::Hour4,
    Timeframe::Hour12,
    Timeframe::Day1,
];

/// Supported timeframes for OHLCV data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timeframe {
    #[serde(rename = "1m")]
    Minute1,
    #[serde(rename = "5m")]
    Minute5,
    #[serde(rename = "15m")]
    Minute15,
    #[serde(rename = "1h")]
    Hour1,
    #[serde(rename = "4h")]
    Hour4,
    #[serde(rename = "12h")]
    Hour12,
    #[serde(rename = "1d")]
    Day1,
}

impl Timeframe {
    /// Returns the duration in seconds for this timeframe
    pub fn to_seconds(&self) -> i64 {
        match self {
            Timeframe::Minute1 => 60,
            Timeframe::Minute5 => 300,
            Timeframe::Minute15 => 900,
            Timeframe::Hour1 => 3600,
            Timeframe::Hour4 => 14400,
            Timeframe::Hour12 => 43200,
            Timeframe::Day1 => 86400,
        }
    }

    /// Returns the GeckoTerminal API parameter for this timeframe
    ///
    /// GeckoTerminal API only supports: "minute", "hour", "day", "second"
    /// We fetch base granularity and aggregate locally for finer timeframes
    pub fn to_api_param(&self) -> &'static str {
        match self {
            // All minute-based timeframes fetch from "minute" endpoint (1m candles)
            Timeframe::Minute1 | Timeframe::Minute5 | Timeframe::Minute15 => "minute",
            // All hour-based timeframes fetch from "hour" endpoint (1h candles)
            Timeframe::Hour1 | Timeframe::Hour4 | Timeframe::Hour12 => "hour",
            // Day timeframe
            Timeframe::Day1 => "day",
        }
    }

    /// Returns GeckoTerminal API params: (endpoint, aggregate)
    /// This properly leverages native timeframe support from the API
    pub fn to_api_params(&self) -> (&'static str, u32) {
        match self {
            Timeframe::Minute1 => ("minute", 1),
            Timeframe::Minute5 => ("minute", 5),
            Timeframe::Minute15 => ("minute", 15),
            Timeframe::Hour1 => ("hour", 1),
            Timeframe::Hour4 => ("hour", 4),
            Timeframe::Hour12 => ("hour", 12),
            Timeframe::Day1 => ("day", 1),
        }
    }

    /// The timeframe whose GeckoTerminal API params are `(endpoint, aggregate)`;
    /// the inverse of [`Self::to_api_params`]. `None` for a pair no timeframe uses.
    pub fn from_api_params(endpoint: &str, aggregate: u32) -> Option<Timeframe> {
        Timeframe::all()
            .into_iter()
            .find(|tf| tf.to_api_params() == (endpoint, aggregate))
    }

    /// Maximum candles available from API for 30 days
    /// How many candles to request per backfill call for this timeframe. The
    /// fetcher clamps each call to MAX_CANDLES_PER_REQUEST (1000) and the server
    /// returns however much history it holds, so requesting 1000 gets the DEEPEST
    /// available series in one call: coarse frames span years (1d×1000 ≈ 2.7y,
    /// 4h×1000 ≈ 166d, 1h×1000 ≈ 41d), fine frames as far as providers retain
    /// (1m ≈ 16h, 5m ≈ 3.5d). The old per-tf "30-day" caps needlessly truncated
    /// coarse charts (1d was 30 candles) even though the server had far more.
    pub fn max_backfill_candles(&self) -> usize {
        1000
    }

    /// How many candles of this timeframe the app keeps for a series. Fine timeframes keep
    /// the backfill page; coarse timeframes are paged further back from the Data Server
    /// (see `DEEP_HISTORY_TIMEFRAMES`) up to 1h 365 days, 4h and 12h 730 days, and 1d 3650
    /// days, which covers every day a Solana token can have traded. The coarse caps sum to
    /// about 18k rows per token at most; a token holds fewer when its history is shorter.
    pub fn max_history_candles(&self) -> usize {
        match self {
            Timeframe::Minute1 | Timeframe::Minute5 | Timeframe::Minute15 => {
                self.max_backfill_candles()
            }
            Timeframe::Hour1 => 8_760,
            Timeframe::Hour4 => 4_380,
            Timeframe::Hour12 => 1_460,
            Timeframe::Day1 => 3_650,
        }
    }

    /// Priority order for backfilling (fastest first)
    /// Lower number = higher priority (faster to fetch)
    pub fn backfill_priority(&self) -> u8 {
        match self {
            Timeframe::Day1 => 1,     // Fastest: 1 call
            Timeframe::Hour12 => 2,   // Fast: 1 call
            Timeframe::Hour4 => 3,    // Fast: 1 call
            Timeframe::Hour1 => 4,    // Medium: 1 call
            Timeframe::Minute15 => 5, // Medium: 3 calls
            Timeframe::Minute5 => 6,  // Slow: 9 calls
            Timeframe::Minute1 => 7,  // Slowest: 44 calls
        }
    }

    /// Returns all supported timeframes
    pub fn all() -> Vec<Timeframe> {
        vec![
            Timeframe::Minute1,
            Timeframe::Minute5,
            Timeframe::Minute15,
            Timeframe::Hour1,
            Timeframe::Hour4,
            Timeframe::Hour12,
            Timeframe::Day1,
        ]
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Timeframe> {
        match s {
            "1m" => Some(Timeframe::Minute1),
            "5m" => Some(Timeframe::Minute5),
            "15m" => Some(Timeframe::Minute15),
            "1h" => Some(Timeframe::Hour1),
            "4h" => Some(Timeframe::Hour4),
            "12h" => Some(Timeframe::Hour12),
            "1d" => Some(Timeframe::Day1),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Timeframe::Minute1 => "1m",
            Timeframe::Minute5 => "5m",
            Timeframe::Minute15 => "15m",
            Timeframe::Hour1 => "1h",
            Timeframe::Hour4 => "4h",
            Timeframe::Hour12 => "12h",
            Timeframe::Day1 => "1d",
        }
    }
}

impl fmt::Display for Timeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Universal candle type used across OHLCV and strategy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    pub fn new(timestamp: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Validates that the OHLCV data is consistent
    pub fn is_valid(&self) -> bool {
        self.high >= self.low
            && self.open >= self.low
            && self.open <= self.high
            && self.close >= self.low
            && self.close <= self.high
            && self.volume >= 0.0
    }
}

// Bundle system constants - hardcoded for performance and simplicity
/// Number of candles to fetch per timeframe for bundle creation
pub const BUNDLE_CANDLE_COUNT: usize = 100;

/// Multi-timeframe bundle containing all timeframes for a single token
/// This is the primary data structure for strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeframeBundle {
    pub mint: String,
    pub pool_address: String,
    pub timestamp: DateTime<Utc>,

    // All timeframes pre-loaded (each contains BUNDLE_CANDLE_COUNT candles)
    pub m1: Vec<Candle>,  // 1-minute (100 candles = 100 min = 1.67 hours)
    pub m5: Vec<Candle>,  // 5-minute (100 candles = 500 min = 8.33 hours)
    pub m15: Vec<Candle>, // 15-minute (100 candles = 1500 min = 25 hours)
    pub h1: Vec<Candle>,  // 1-hour (100 candles = 100 hours = 4.17 days)
    pub h4: Vec<Candle>,  // 4-hour (100 candles = 400 hours = 16.67 days)
    pub h12: Vec<Candle>, // 12-hour (100 candles = 1200 hours = 50 days)
    pub d1: Vec<Candle>,  // 1-day (100 candles = 100 days)

    // Metadata
    pub cache_age_seconds: u64, // Age of bundle in cache (0 for fresh build)
    pub cache_hit: bool,        // Whether this was served from cache
}

impl TimeframeBundle {
    /// Create a new empty bundle
    pub fn new(mint: String, pool_address: String) -> Self {
        Self {
            mint,
            pool_address,
            timestamp: Utc::now(),
            m1: Vec::new(),
            m5: Vec::new(),
            m15: Vec::new(),
            h1: Vec::new(),
            h4: Vec::new(),
            h12: Vec::new(),
            d1: Vec::new(),
            cache_age_seconds: 0,
            cache_hit: false,
        }
    }

    /// Get candles for a specific timeframe by string
    pub fn get_timeframe(&self, timeframe: &str) -> Option<&Vec<Candle>> {
        match timeframe {
            "1m" => Some(&self.m1),
            "5m" => Some(&self.m5),
            "15m" => Some(&self.m15),
            "1h" => Some(&self.h1),
            "4h" => Some(&self.h4),
            "12h" => Some(&self.h12),
            "1d" => Some(&self.d1),
            _ => None,
        }
    }

    /// Check if bundle has data for all timeframes
    pub fn is_complete(&self) -> bool {
        !self.m1.is_empty()
            && !self.m5.is_empty()
            && !self.m15.is_empty()
            && !self.h1.is_empty()
            && !self.h4.is_empty()
            && !self.h12.is_empty()
            && !self.d1.is_empty()
    }

    /// Check if bundle is fresh enough (age in seconds)
    pub fn is_fresh(&self, max_age_seconds: u64) -> bool {
        let age = Utc::now()
            .signed_duration_since(self.timestamp)
            .num_seconds();
        age >= 0 && (age as u64) < max_age_seconds
    }

    /// Get total number of candles across all timeframes
    pub fn total_candles(&self) -> usize {
        self.m1.len()
            + self.m5.len()
            + self.m15.len()
            + self.h1.len()
            + self.h4.len()
            + self.h12.len()
            + self.d1.len()
    }
}

/// Configuration for a single pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub address: String,
    pub dex: String,
    pub liquidity: f64,
    pub is_default: bool,
    /// Whether this pool is wSOL-quoted. A USD-quoted pool must NOT be fetched via
    /// GeckoTerminal `currency=token` (that returns USD candles that poison the
    /// SOL-denominated series); the SOL-forcing sources (data server, the chain's candle feeds)
    /// are used instead. Defaults to true (legacy rows / unknown = assume SOL).
    pub is_native_pair: bool,
    pub last_successful_fetch: Option<DateTime<Utc>>,
    pub failure_count: u32,
}

impl PoolConfig {
    pub fn new(address: String, dex: String, liquidity: f64) -> Self {
        Self {
            address,
            dex,
            liquidity,
            is_default: false,
            is_native_pair: true,
            last_successful_fetch: None,
            failure_count: 0,
        }
    }

    /// Whether the pool's fetches succeed: fewer than `ohlcv.max_pool_failures` consecutive
    /// failures. An unhealthy series pool stays the series; it is fetched on the backoff of
    /// [`Self::fetch_retry_delay`] until a success resets its count.
    pub fn is_healthy(&self) -> bool {
        self.failure_count < with_config(|cfg| cfg.ohlcv.max_pool_failures)
    }

    /// The pause between fetches of an unhealthy pool, doubling with each failure past the
    /// health limit from `UNHEALTHY_POOL_RETRY_BASE` up to `UNHEALTHY_POOL_RETRY_MAX`; `None`
    /// for a healthy pool, which is fetched on its normal cadence.
    pub fn fetch_retry_delay(&self) -> Option<Duration> {
        if self.is_healthy() {
            return None;
        }
        let past_limit = self
            .failure_count
            .saturating_sub(with_config(|cfg| cfg.ohlcv.max_pool_failures));
        let delay = UNHEALTHY_POOL_RETRY_BASE
            .checked_mul(1u32.checked_shl(past_limit).unwrap_or(u32::MAX))
            .unwrap_or(UNHEALTHY_POOL_RETRY_MAX);
        Some(delay.min(UNHEALTHY_POOL_RETRY_MAX))
    }

    /// The ONE pool a token's candle series lives on: the default pool, whatever its fetch
    /// health. Every writer (monitor fetch, backfill) and every reader (chart, status)
    /// resolves through this, so stored candles stay readable while their pool's fetches
    /// fail. There is no fallback to another pool: the series moves only through
    /// `OhlcvDatabase::write_series_pools` or the failure handover of
    /// `OhlcvDatabase::mark_pool_failure`, which reset the token's rows and backfill flags with
    /// the move, so the flags always describe the pool read here.
    pub fn series_pool(pools: &[PoolConfig]) -> Option<&PoolConfig> {
        pools.iter().find(|p| p.is_default)
    }
}

/// Priority level for monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum Priority {
    Critical = 4,
    High = 3,
    Medium = 2,
    Low = 1,
}

impl Priority {
    /// Base fetch interval for this priority level
    pub fn base_interval(&self) -> Duration {
        match self {
            Priority::Critical => Duration::from_secs(30),
            Priority::High => Duration::from_secs(60),
            Priority::Medium => Duration::from_secs(300),
            Priority::Low => Duration::from_secs(900),
        }
    }

    pub fn from_str(s: &str) -> Option<Priority> {
        match s {
            "critical" => Some(Priority::Critical),
            "high" => Some(Priority::High),
            "medium" => Some(Priority::Medium),
            "low" => Some(Priority::Low),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Critical => "critical",
            Priority::High => "high",
            Priority::Medium => "medium",
            Priority::Low => "low",
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration for a token's OHLCV monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenOhlcvConfig {
    pub mint: String,
    pub priority: Priority,
    pub last_activity: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_fetch: Option<DateTime<Utc>>,
    pub fetch_frequency: Duration,
    pub consecutive_empty_fetches: u32,
    pub is_active: bool,
    // Pool discovery backoff fields
    pub last_pool_discovery_attempt: Option<i64>,
    pub consecutive_pool_failures: u32,
}

impl TokenOhlcvConfig {
    pub fn new(mint: String, priority: Priority) -> Self {
        Self {
            mint,
            priority,
            last_activity: Utc::now(),
            last_fetch: None,
            fetch_frequency: priority.base_interval(),
            consecutive_empty_fetches: 0,
            is_active: true,
            last_pool_discovery_attempt: None,
            consecutive_pool_failures: 0,
        }
    }

    pub fn mark_fetch(&mut self) {
        self.last_fetch = Some(Utc::now());
    }

    pub fn mark_activity(&mut self) {
        self.last_activity = Utc::now();
        self.consecutive_empty_fetches = 0;
    }

    pub fn mark_empty_fetch(&mut self) {
        self.consecutive_empty_fetches += 1;
    }

    pub fn calculate_adjusted_interval(&self) -> Duration {
        let base = self.priority.base_interval();
        let hours_inactive = (Utc::now() - self.last_activity).num_hours().max(0) as f64;
        let empty_factor = 1.0 + (self.consecutive_empty_fetches as f64) / 10.0;
        let time_factor = 1.0 + hours_inactive / 24.0;

        let adjusted_secs = ((base.as_secs() as f64) * empty_factor * time_factor) as u64;
        let max_secs = base.as_secs() * 10;

        Duration::from_secs(adjusted_secs.min(max_secs))
    }

    /// Check if enough time has passed to retry pool discovery
    /// Uses exponential backoff: 5m, 15m, 1h, 6h, 24h
    pub fn should_retry_pool_discovery(&self) -> bool {
        if self.consecutive_pool_failures == 0 {
            return true; // First attempt or no failures
        }

        let last_attempt = match self.last_pool_discovery_attempt {
            Some(t) => t,
            None => {
                return true;
            } // Never attempted
        };

        let now = Utc::now().timestamp();
        let elapsed = now - last_attempt;

        // Exponential backoff intervals
        let backoff_secs = match self.consecutive_pool_failures {
            1 => 300,   // 5 minutes
            2 => 900,   // 15 minutes
            3 => 3600,  // 1 hour
            4 => 21600, // 6 hours
            _ => 86400, // 24 hours (max)
        };

        elapsed >= backoff_secs
    }

    /// Mark pool discovery failure
    pub fn mark_pool_discovery_failure(&mut self) {
        self.consecutive_pool_failures += 1;
        self.last_pool_discovery_attempt = Some(Utc::now().timestamp());
    }

    /// Mark pool discovery success
    pub fn mark_pool_discovery_success(&mut self) {
        self.consecutive_pool_failures = 0;
        self.last_pool_discovery_attempt = Some(Utc::now().timestamp());
    }

    /// Get human-readable backoff time
    pub fn get_next_retry_description(&self) -> String {
        if self.consecutive_pool_failures == 0 {
            return "immediately".to_owned();
        }

        match self.consecutive_pool_failures {
            1 => "5 minutes".to_owned(),
            2 => "15 minutes".to_owned(),
            3 => "1 hour".to_owned(),
            4 => "6 hours".to_owned(),
            _ => "24 hours".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintGapAggregate {
    pub chain: crate::chains::ChainId,
    pub mint: String,
    pub open_gaps: usize,
    pub largest_gap_seconds: Option<i64>,
    pub latest_gap_end: Option<i64>,
}

/// Pool metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetadata {
    pub address: String,
    pub dex: String,
    pub liquidity: f64,
    pub is_default: bool,
    pub is_healthy: bool,
    pub last_successful_fetch: Option<DateTime<Utc>>,
    pub failure_count: u32,
}

impl From<&PoolConfig> for PoolMetadata {
    fn from(config: &PoolConfig) -> Self {
        Self {
            address: config.address.clone(),
            dex: config.dex.clone(),
            liquidity: config.liquidity,
            is_default: config.is_default,
            is_healthy: config.is_healthy(),
            last_successful_fetch: config.last_successful_fetch,
            failure_count: config.failure_count,
        }
    }
}

/// Metrics for the OHLCV system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvMetrics {
    pub tokens_monitored: usize,
    pub pools_tracked: usize,
    pub api_calls_per_minute: f64,
    pub cache_hit_rate: f64,
    pub average_fetch_latency_ms: f64,
    pub gaps_detected: usize,
    pub gaps_filled: usize,
    pub data_points_stored: usize,
    pub database_size_mb: f64,
    pub oldest_data_timestamp: Option<DateTime<Utc>>,
}

impl Default for OhlcvMetrics {
    fn default() -> Self {
        Self {
            tokens_monitored: 0,
            pools_tracked: 0,
            api_calls_per_minute: 0.0,
            cache_hit_rate: 0.0,
            average_fetch_latency_ms: 0.0,
            gaps_detected: 0,
            gaps_filled: 0,
            data_points_stored: 0,
            database_size_mb: 0.0,
            oldest_data_timestamp: None,
        }
    }
}

/// Error types for OHLCV operations
#[derive(Debug, Clone)]
pub enum OhlcvError {
    DatabaseError(String),
    ApiError(String),
    RateLimitExceeded,
    PoolNotFound(String),
    InvalidTimeframe(String),
    DataGap {
        start: i64,
        end: i64,
    },
    CacheError(String),
    NotFound(String),
    Chain(crate::chains::Error),
    /// A candle or backfill-flag write for a pool that is no longer the token's series pool.
    SeriesPoolMoved {
        mint: String,
        pool: String,
    },
}

impl From<crate::chains::Error> for OhlcvError {
    fn from(error: crate::chains::Error) -> Self {
        OhlcvError::Chain(error)
    }
}

impl fmt::Display for OhlcvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OhlcvError::DatabaseError(e) => write!(f, "Database error: {e}"),
            OhlcvError::ApiError(e) => write!(f, "API error: {e}"),
            OhlcvError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            OhlcvError::PoolNotFound(pool) => write!(f, "Pool not found: {pool}"),
            OhlcvError::InvalidTimeframe(tf) => write!(f, "Invalid timeframe: {tf}"),
            OhlcvError::DataGap { start, end } => {
                write!(f, "Data gap detected: {start} to {end}")
            }
            OhlcvError::CacheError(e) => write!(f, "Cache error: {e}"),
            OhlcvError::NotFound(msg) => write!(f, "Not found: {msg}"),
            OhlcvError::Chain(e) => write!(f, "Chain: {e}"),
            OhlcvError::SeriesPoolMoved { mint, pool } => {
                write!(f, "Pool {pool} is no longer the series pool of {mint}")
            }
        }
    }
}

impl std::error::Error for OhlcvError {}

pub type OhlcvResult<T> = Result<T, OhlcvError>;

/// Snapshot of OHLCV monitor telemetry (public, read-only view)
#[derive(Debug, Clone, Default)]
pub struct MonitorTelemetrySnapshot {
    pub monitor_cycle_started_at: Option<DateTime<Utc>>,
    pub monitor_cycle_completed_at: Option<DateTime<Utc>>,
    pub monitor_cycle_duration_ms: Option<u64>,
    pub monitor_cycle_tokens_processed: usize,
    pub monitor_cycle_total: u64,
    pub gap_cycle_started_at: Option<DateTime<Utc>>,
    pub gap_cycle_completed_at: Option<DateTime<Utc>>,
    pub gap_cycle_duration_ms: Option<u64>,
    pub gap_cycle_tokens_processed: usize,
    pub gap_cycle_total: u64,
    pub last_rate_limit_at: Option<DateTime<Utc>>,
    pub rate_limit_events: u64,
    pub total_backfills_scheduled: u64,
    pub total_backfills_completed: u64,
    pub total_backfills_failed: u64,
    pub last_backfill_started_at: Option<DateTime<Utc>>,
    pub last_backfill_completed_at: Option<DateTime<Utc>>,
    pub last_backfill_duration_ms: Option<u64>,
    pub last_backfill_points: Option<usize>,
    pub last_backfill_error: Option<String>,
}

/// OHLCV monitor statistics
pub struct MonitorStats {
    pub total_tokens: usize,
    pub critical_tokens: usize,
    pub high_tokens: usize,
    pub medium_tokens: usize,
    pub low_tokens: usize,
    pub cache_hit_rate: f64,
    pub api_calls_per_minute: f64,
    pub queue_size: usize,
    pub telemetry: MonitorTelemetrySnapshot,
    pub backfills_in_progress: usize,
    pub open_gap_tokens: usize,
    pub open_gap_total: usize,
    pub top_open_gaps: Vec<MintGapAggregate>,
}

impl Default for MonitorStats {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            critical_tokens: 0,
            high_tokens: 0,
            medium_tokens: 0,
            low_tokens: 0,
            cache_hit_rate: 0.0,
            api_calls_per_minute: 0.0,
            queue_size: 0,
            telemetry: MonitorTelemetrySnapshot::default(),
            backfills_in_progress: 0,
            open_gap_tokens: 0,
            open_gap_total: 0,
            top_open_gaps: Vec::new(),
        }
    }
}

/// Per-timeframe data + backfill state for a token, surfaced to the chart
/// status indicator so the dialog can show exactly which timeframes have data
/// and whether the 30-day backfill finished.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvTimeframeStatus {
    pub timeframe: String,
    pub candles: i64,
    pub backfill_complete: bool,
    /// Timestamp (unix secs) of the oldest candle for this timeframe.
    pub earliest_timestamp: Option<i64>,
    /// Timestamp (unix secs) of the newest candle for this timeframe.
    pub latest_timestamp: Option<i64>,
    /// Candles whose bucket overlaps the requested `[from, to]` span, when the status was
    /// asked about one. Stored depth is capped per timeframe, so the newest candles of a
    /// timeframe say nothing about whether it still holds a position from weeks ago.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range_candles: Option<i64>,
    /// When new candles were last written for this timeframe (unix secs) — i.e.
    /// the last successful fetch that produced data.
    pub last_new_data_at: Option<i64>,
}

/// Overall OHLCV process status for one token (monitoring + per-timeframe data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvStatus {
    pub mint: String,
    /// Currently in the active monitoring set (data collection running).
    pub monitored: bool,
    /// Any timeframe has at least one stored candle.
    pub has_data: bool,
    /// Total candles across all timeframes.
    pub total_candles: i64,
    /// Finest timeframe that currently has candles (chart default target).
    pub best_timeframe: Option<String>,
    /// True once every timeframe's 30-day backfill is complete.
    pub backfill_complete: bool,
    /// When this token's OHLCV was last checked (any fetch attempt, unix secs).
    pub last_checked_at: Option<i64>,
    /// When new candles were last written across any timeframe (unix secs).
    pub last_new_data_at: Option<i64>,
    pub timeframes: Vec<OhlcvTimeframeStatus>,
}

#[cfg(test)]
mod tests {
    use super::{
        Duration, PoolConfig, Priority, Timeframe, DEEP_HISTORY_TIMEFRAMES,
        UNHEALTHY_POOL_RETRY_BASE, UNHEALTHY_POOL_RETRY_MAX,
    };

    /// The deep-history set is exactly the timeframes kept past the backfill page, and no
    /// timeframe keeps less than that page.
    #[test]
    fn deep_history_timeframes_are_those_kept_past_the_backfill_page() {
        let deep: Vec<Timeframe> = Timeframe::all()
            .into_iter()
            .filter(|tf| tf.max_history_candles() > tf.max_backfill_candles())
            .collect();
        assert_eq!(deep, DEEP_HISTORY_TIMEFRAMES.to_vec());
        for tf in Timeframe::all() {
            assert!(
                tf.max_history_candles() >= tf.max_backfill_candles(),
                "{tf:?}"
            );
        }
        let days = |tf: Timeframe| tf.max_history_candles() as i64 * tf.to_seconds() / 86_400;
        assert_eq!(days(Timeframe::Hour1), 365);
        assert_eq!(days(Timeframe::Hour4), 730);
        assert_eq!(days(Timeframe::Hour12), 730);
        assert_eq!(days(Timeframe::Day1), 3_650);
    }

    /// Catalog key of the label for each monitoring priority. The match is
    /// exhaustive, so a new variant fails to compile until it is mapped here and
    /// in `OHLCV_PRIORITY_LABELS` (pages/tokens/ohlcv.js).
    fn label_key(priority: Priority) -> &'static str {
        match priority {
            Priority::Critical => "tokens-ohlcv-priority-critical",
            Priority::High => "tokens-ohlcv-priority-high",
            Priority::Medium => "tokens-ohlcv-priority-medium",
            Priority::Low => "tokens-ohlcv-priority-low",
        }
    }

    #[test]
    fn priority_labels_exist_in_the_catalog() {
        for priority in [
            Priority::Critical,
            Priority::High,
            Priority::Medium,
            Priority::Low,
        ] {
            let key = label_key(priority);
            assert_eq!(
                key,
                format!("tokens-ohlcv-priority-{}", priority.as_str()),
                "key does not follow the id {}",
                priority.as_str()
            );
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
        }
    }

    fn pool(address: &str, liquidity: f64, is_default: bool, failure_count: u32) -> PoolConfig {
        PoolConfig {
            is_default,
            failure_count,
            ..PoolConfig::new(address.to_string(), "dex".to_string(), liquidity)
        }
    }

    fn resolved(pools: &[PoolConfig]) -> Option<&str> {
        PoolConfig::series_pool(pools).map(|p| p.address.as_str())
    }

    #[test]
    fn the_series_pool_is_the_default_whatever_its_health_and_never_another_pool() {
        crate::config::utils::install_default_config();
        // A shallow default wins over a deeper pool.
        let pools = [
            pool("deep", 61_364.0, false, 0),
            pool("default", 476.0, true, 0),
        ];
        assert_eq!(resolved(&pools), Some("default"));
        // A default whose fetches fail still carries the series: its stored candles stay
        // readable and no unreset pool stands in for it.
        let pools = [
            pool("default", 90_000.0, true, 50),
            pool("deep", 20_000.0, false, 0),
        ];
        assert!(!pools[0].is_healthy());
        assert_eq!(resolved(&pools), Some("default"));
        assert_eq!(resolved(&[pool("other", 1.0, false, 0)]), None);
        assert_eq!(resolved(&[]), None);
    }

    #[test]
    fn an_unhealthy_pool_is_retried_on_a_doubling_bounded_backoff() {
        crate::config::utils::install_default_config();
        let limit = crate::config::with_config(|cfg| cfg.ohlcv.max_pool_failures);
        let delay = |failures: u32| pool("p", 1.0, true, failures).fetch_retry_delay();
        assert_eq!(delay(0), None);
        assert_eq!(delay(limit - 1), None);
        assert_eq!(delay(limit), Some(UNHEALTHY_POOL_RETRY_BASE));
        assert_eq!(delay(limit + 1), Some(UNHEALTHY_POOL_RETRY_BASE * 2));
        let mut previous = Duration::ZERO;
        for failures in limit..limit + 64 {
            let current = delay(failures).expect("an unhealthy pool has a retry delay");
            assert!(current >= previous && current <= UNHEALTHY_POOL_RETRY_MAX);
            previous = current;
        }
        assert_eq!(previous, UNHEALTHY_POOL_RETRY_MAX);
        assert_eq!(delay(u32::MAX), Some(UNHEALTHY_POOL_RETRY_MAX));
    }
}
