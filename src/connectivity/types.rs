//! Connectivity data types — endpoint health status, criticality levels, and fallback strategies.

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
    /// Parse criticality level from a string (defaults to Optional for unknown values)
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
    /// Returns true if the endpoint is fully healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self, EndpointHealth::Healthy { .. })
    }

    /// Returns true if the endpoint is in degraded mode
    pub fn is_degraded(&self) -> bool {
        matches!(self, EndpointHealth::Degraded { .. })
    }

    /// Returns true if the endpoint is completely unavailable
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, EndpointHealth::Unhealthy { .. })
    }

    /// Returns true if the endpoint is usable (healthy or degraded)
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
    /// Parse a fallback strategy from a config string (defaults to Skip)
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
    /// Create a successful health check result with measured latency
    pub fn success(latency_ms: u64) -> Self {
        Self {
            healthy: true,
            latency_ms,
            error: None,
        }
    }

    /// Create a failed health check result with the error message
    pub fn failure(error: String) -> Self {
        Self {
            healthy: false,
            latency_ms: 0,
            error: Some(error),
        }
    }

    /// Create a degraded health check result (healthy but with performance issues)
    pub fn degraded(latency_ms: u64, reason: String) -> Self {
        Self {
            healthy: true,
            latency_ms,
            error: Some(reason),
        }
    }
}
