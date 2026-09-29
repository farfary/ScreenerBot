//! Rugcheck API health monitor — checks token safety analysis endpoint availability.

use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{
    EndpointCriticality, FallbackStrategy, HealthCheckResult, ProbeFailure,
};
use async_trait::async_trait;
use std::time::Instant;
use tokio::time::Duration;

/// Rugcheck API monitor
pub struct RugcheckMonitor;

impl RugcheckMonitor {
    /// Create a new monitor instance
    pub fn new() -> Self {
        Self
    }

    const BASE_URL: &'static str = "https://api.rugcheck.xyz";
}

#[async_trait]
impl EndpointMonitor for RugcheckMonitor {
    fn name(&self) -> &'static str {
        "rugcheck"
    }

    fn criticality(&self) -> EndpointCriticality {
        EndpointCriticality::Optional
    }

    fn fallback_strategy(&self) -> Option<FallbackStrategy> {
        Some(FallbackStrategy::Skip)
    }

    fn is_enabled(&self) -> bool {
        let cfg = get_config_clone();
        cfg.connectivity.enabled && cfg.connectivity.endpoints.rugcheck.enabled
    }

    async fn check_health(&self) -> HealthCheckResult {
        let cfg = get_config_clone();
        let timeout_secs = cfg.connectivity.endpoints.rugcheck.timeout_secs.max(1);

        let client = match crate::net::client_builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                return HealthCheckResult::failure(ProbeFailure::ClientSetupFailed {
                    detail: e.to_string(),
                })
            }
        };

        // Use ping endpoint for health check (lightweight, documented)
        let url = format!("{}/ping", Self::BASE_URL);
        let start = Instant::now();

        match client.get(&url).send().await {
            Ok(response) => {
                let latency = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    HealthCheckResult::success(latency)
                } else {
                    HealthCheckResult::failure(ProbeFailure::HttpStatus {
                        status: response.status().to_string(),
                    })
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    HealthCheckResult::failure(ProbeFailure::Timeout {
                        seconds: timeout_secs,
                    })
                } else {
                    HealthCheckResult::failure(ProbeFailure::RequestFailed {
                        detail: e.to_string(),
                    })
                }
            }
        }
    }

    fn description(&self) -> &'static str {
        "Rugcheck API"
    }
}
