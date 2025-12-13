//! Circuit breaker configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,

    /// Number of consecutive successes in half-open state to close circuit
    pub success_threshold: u32,

    /// Duration to keep circuit open before transitioning to half-open
    pub open_duration: Duration,

    /// Timeout for probe requests in half-open state
    pub half_open_timeout: Duration,

    /// Maximum number of probe requests in half-open state
    pub half_open_max_requests: u32,

    /// Whether to track error types (some errors shouldn't trip circuit)
    pub ignore_rate_limits: bool,

    /// Minimum time between state transitions (prevents rapid flapping)
    pub min_state_duration: Duration,
}

impl CircuitBreakerConfig {
    /// Create with failure threshold
    pub fn with_threshold(failure_threshold: u32) -> Self {
        Self {
            failure_threshold,
            ..Default::default()
        }
    }

    /// Create for aggressive failure detection
    pub fn aggressive() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            open_duration: Duration::from_secs(15),
            half_open_timeout: Duration::from_secs(5),
            half_open_max_requests: 1,
            ignore_rate_limits: true,
            min_state_duration: Duration::from_secs(5),
        }
    }

    /// Create for conservative failure detection
    pub fn conservative() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 5,
            open_duration: Duration::from_secs(60),
            half_open_timeout: Duration::from_secs(10),
            half_open_max_requests: 3,
            ignore_rate_limits: true,
            min_state_duration: Duration::from_secs(10),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            open_duration: Duration::from_secs(30),
            half_open_timeout: Duration::from_secs(10),
            half_open_max_requests: 2,
            ignore_rate_limits: true,
            min_state_duration: Duration::from_secs(5),
        }
    }
}
