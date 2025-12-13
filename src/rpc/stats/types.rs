//! Statistics types for RPC module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::rpc::types::{CircuitState, ProviderKind};

/// Individual RPC call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcCallRecord {
    /// Provider that handled the call
    pub provider_id: String,
    /// RPC method name
    pub method: String,
    /// Whether call succeeded
    pub success: bool,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Error code if failed
    pub error_code: Option<i64>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Whether this was a retry
    pub was_retried: bool,
    /// Retry attempt number
    pub retry_count: u32,
    /// Whether rate limited
    pub was_rate_limited: bool,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Aggregated statistics for a time bucket
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeBucketStats {
    /// Start of time bucket
    pub bucket_start: DateTime<Utc>,
    /// Total calls
    pub call_count: u64,
    /// Successful calls
    pub success_count: u64,
    /// Error count
    pub error_count: u64,
    /// Rate limit count
    pub rate_limit_count: u64,
    /// Total latency (for average calculation)
    pub latency_sum_ms: u64,
    /// Minimum latency
    pub latency_min_ms: Option<u64>,
    /// Maximum latency
    pub latency_max_ms: Option<u64>,
}

impl TimeBucketStats {
    /// Average latency in milliseconds
    pub fn avg_latency_ms(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.latency_sum_ms as f64 / self.call_count as f64
        }
    }

    /// Success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.call_count == 0 {
            100.0
        } else {
            100.0 * self.success_count as f64 / self.call_count as f64
        }
    }

    /// Merge another bucket into this one
    pub fn merge(&mut self, other: &TimeBucketStats) {
        self.call_count += other.call_count;
        self.success_count += other.success_count;
        self.error_count += other.error_count;
        self.rate_limit_count += other.rate_limit_count;
        self.latency_sum_ms += other.latency_sum_ms;

        self.latency_min_ms = match (self.latency_min_ms, other.latency_min_ms) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.latency_max_ms = match (self.latency_max_ms, other.latency_max_ms) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
    }
}

/// Provider statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStats {
    /// Provider ID
    pub provider_id: String,
    /// Provider URL (masked)
    pub url_masked: String,
    /// Provider kind
    pub kind: ProviderKind,
    /// Current circuit state
    pub circuit_state: CircuitState,
    /// Total calls
    pub total_calls: u64,
    /// Total errors
    pub total_errors: u64,
    /// Total rate limits
    pub total_rate_limits: u64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Current rate limit (may be reduced)
    pub current_rate_limit: u32,
    /// Base rate limit
    pub base_rate_limit: u32,
    /// Is provider healthy
    pub is_healthy: bool,
    /// Last success time
    pub last_success: Option<DateTime<Utc>>,
    /// Last error time
    pub last_error: Option<DateTime<Utc>>,
    /// Last error message
    pub last_error_message: Option<String>,
}

/// Method statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MethodStats {
    /// Method name
    pub method: String,
    /// Total calls
    pub total_calls: u64,
    /// Total errors
    pub total_errors: u64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Success rate
    pub success_rate: f64,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Session ID
    pub session_id: String,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Session end time (if ended)
    pub ended_at: Option<DateTime<Utc>>,
    /// Total calls
    pub total_calls: u64,
    /// Total errors
    pub total_errors: u64,
    /// Duration
    pub duration_secs: u64,
}

/// Overall statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Current session ID
    pub session_id: String,
    /// Session start time
    pub session_started_at: DateTime<Utc>,
    /// Total calls this session
    pub total_calls: u64,
    /// Total errors this session
    pub total_errors: u64,
    /// Total rate limits this session
    pub total_rate_limits: u64,
    /// Overall success rate
    pub success_rate: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Per-provider stats
    pub providers: Vec<ProviderStats>,
    /// Per-method stats (top 10)
    pub methods: Vec<MethodStats>,
    /// Calls per minute (last 60 minutes)
    pub calls_per_minute: Vec<TimeBucketStats>,
}

/// Stats for webserver/API response (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcStatsResponse {
    /// Session ID
    pub session_id: String,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Total calls
    pub total_calls: u64,
    /// Total errors
    pub total_errors: u64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Provider count
    pub provider_count: usize,
    /// Healthy provider count
    pub healthy_provider_count: usize,
    /// Calls in last minute
    pub calls_last_minute: u64,
}
