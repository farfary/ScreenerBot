use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Criticality level determines system behavior when endpoint is unavailable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointCriticality {
    /// System pauses completely if endpoint is down (e.g., Internet, RPC)
    Critical,
    /// System continues but with warnings and degraded mode (e.g., DexScreener, Jupiter)
    Important,
    /// System continues silently with fallback (e.g., Rugcheck, CoinGecko)
    Optional,
}

impl EndpointCriticality {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Self::Critical,
            "important" => Self::Important,
            "optional" => Self::Optional,
            _ => Self::Optional,
        }
    }
}

/// Health status of an endpoint with detailed information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum EndpointHealth {
    /// Endpoint is functioning normally
    Healthy {
        latency_ms: u64,
        last_check: DateTime<Utc>,
    },
    /// Endpoint is functioning but with degraded performance
    Degraded {
        latency_ms: u64,
        reason: String,
        last_check: DateTime<Utc>,
    },
    /// Endpoint is not functioning
    Unhealthy {
        reason: String,
        last_check: DateTime<Utc>,
        last_success: Option<DateTime<Utc>>,
        consecutive_failures: u32,
    },
    /// Health status unknown (not checked yet)
    Unknown,
}

impl EndpointHealth {
    pub fn is_healthy(&self) -> bool {
        matches!(self, EndpointHealth::Healthy { .. })
    }

    pub fn is_degraded(&self) -> bool {
        matches!(self, EndpointHealth::Degraded { .. })
    }

    pub fn is_unhealthy(&self) -> bool {
        matches!(self, EndpointHealth::Unhealthy { .. })
    }

    pub fn is_available(&self) -> bool {
        matches!(
            self,
            EndpointHealth::Healthy { .. } | EndpointHealth::Degraded { .. }
        )
    }
}

/// Fallback strategy when endpoint is unavailable
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FallbackStrategy {
    /// Use cached data if available and not older than max_age_secs
    UseCache { max_age_secs: u64 },
    /// Use alternative endpoint
    UseAlternative { endpoint_name: String },
    /// Skip the operation silently
    Skip,
    /// Fail the operation with error
    Fail,
}

impl FallbackStrategy {
    pub fn from_config(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cache" => Self::UseCache {
                max_age_secs: 86400,
            }, // 24h default
            "skip" => Self::Skip,
            "fail" => Self::Fail,
            _ => Self::Skip,
        }
    }
}

/// Check result for health monitoring
#[derive(Debug)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
}

impl HealthCheckResult {
    pub fn success(latency_ms: u64) -> Self {
        Self {
            healthy: true,
            latency_ms,
            error: None,
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            healthy: false,
            latency_ms: 0,
            error: Some(error),
        }
    }

    pub fn degraded(latency_ms: u64, reason: String) -> Self {
        Self {
            healthy: true,
            latency_ms,
            error: Some(reason),
        }
    }
}
