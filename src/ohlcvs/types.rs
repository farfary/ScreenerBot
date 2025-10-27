// Core types for OHLCV module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

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
    /// DEPRECATED: Use to_api_params() for proper native timeframe support
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

    /// Maximum candles available from API for 30 days
    pub fn max_candles_30d(&self) -> usize {
        match self {
            Timeframe::Minute1 => 43_200,  // 30 * 24 * 60
            Timeframe::Minute5 => 8_640,   // 30 * 24 * 12
            Timeframe::Minute15 => 2_880,  // 30 * 24 * 4
            Timeframe::Hour1 => 720,       // 30 * 24
            Timeframe::Hour4 => 180,       // 30 * 6
            Timeframe::Hour12 => 60,       // 30 * 2
            Timeframe::Day1 => 30,         // 30
        }
    }

    /// Priority order for backfilling (fastest first)
    /// Lower number = higher priority (faster to fetch)
    pub fn backfill_priority(&self) -> u8 {
        match self {
            Timeframe::Day1 => 1,      // Fastest: 1 call
            Timeframe::Hour12 => 2,    // Fast: 1 call
            Timeframe::Hour4 => 3,     // Fast: 1 call
            Timeframe::Hour1 => 4,     // Medium: 1 call
            Timeframe::Minute15 => 5,  // Medium: 3 calls
            Timeframe::Minute5 => 6,   // Slow: 9 calls
            Timeframe::Minute1 => 7,   // Slowest: 44 calls
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

/// A single OHLCV data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OhlcvDataPoint {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl OhlcvDataPoint {
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

/// Configuration for a single pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub address: String,
    pub dex: String,
    pub liquidity: f64,
    pub is_default: bool,
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
            last_successful_fetch: None,
            failure_count: 0,
        }
    }

    pub fn mark_success(&mut self) {
        self.last_successful_fetch = Some(Utc::now());
        self.failure_count = 0;
    }

    pub fn mark_failure(&mut self) {
        self.failure_count += 1;
    }

    pub fn is_healthy(&self) -> bool {
        self.failure_count < 5
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
    pub pools: Vec<PoolConfig>,
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
            pools: Vec::new(),
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

    pub fn get_default_pool(&self) -> Option<&PoolConfig> {
        self.pools.iter().find(|p| p.is_default)
    }

    pub fn get_best_pool(&self) -> Option<&PoolConfig> {
        self.pools.iter().filter(|p| p.is_healthy()).max_by(|a, b| {
            a.liquidity
                .partial_cmp(&b.liquidity)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
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
            return "immediately".to_string();
        }

        match self.consecutive_pool_failures {
            1 => "5 minutes".to_string(),
            2 => "15 minutes".to_string(),
            3 => "1 hour".to_string(),
            4 => "6 hours".to_string(),
            _ => "24 hours".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintGapAggregate {
    pub mint: String,
    pub open_gaps: usize,
    pub largest_gap_seconds: Option<i64>,
    pub latest_gap_end: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct OhlcvTokenStatus {
    pub mint: String,
    pub priority: Priority,
    pub is_active: bool,
    pub has_data: bool,
    pub pools_monitored: usize,
    pub primary_pool: Option<String>,
    pub last_fetch_timestamp: Option<DateTime<Utc>>,
    pub next_fetch_timestamp: Option<DateTime<Utc>>,
    pub last_activity_timestamp: DateTime<Utc>,
    pub fetch_interval_seconds: u64,
    pub consecutive_empty_fetches: u32,
    pub retention_target_timestamp: i64,
    pub earliest_candle_timestamp: Option<i64>,
    pub latest_candle_timestamp: Option<i64>,
    pub coverage_percent: f64,
    pub backfill_in_progress: bool,
    pub open_gap_count: usize,
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
#[derive(Debug, Clone, Serialize)]
pub enum OhlcvError {
    DatabaseError(String),
    ApiError(String),
    RateLimitExceeded,
    PoolNotFound(String),
    InvalidTimeframe(String),
    DataGap { start: i64, end: i64 },
    CacheError(String),
    NotFound(String),
}

impl fmt::Display for OhlcvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OhlcvError::DatabaseError(e) => write!(f, "Database error: {}", e),
            OhlcvError::ApiError(e) => write!(f, "API error: {}", e),
            OhlcvError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            OhlcvError::PoolNotFound(pool) => write!(f, "Pool not found: {}", pool),
            OhlcvError::InvalidTimeframe(tf) => write!(f, "Invalid timeframe: {}", tf),
            OhlcvError::DataGap { start, end } => {
                write!(f, "Data gap detected: {} to {}", start, end)
            }
            OhlcvError::CacheError(e) => write!(f, "Cache error: {}", e),
            OhlcvError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for OhlcvError {}

pub type OhlcvResult<T> = Result<T, OhlcvError>;
