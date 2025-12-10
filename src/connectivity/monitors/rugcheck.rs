use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use async_trait::async_trait;
use std::time::Instant;
use tokio::time::Duration;

/// Rugcheck API monitor
pub struct RugcheckMonitor;

impl RugcheckMonitor {
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

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(e) => return HealthCheckResult::failure(format!("Failed to create client: {}", e)),
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
                    HealthCheckResult::failure(format!("HTTP {}", response.status()))
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    HealthCheckResult::failure(format!("Timeout after {}s", timeout_secs))
                } else {
                    HealthCheckResult::failure(format!("Request failed: {}", e))
                }
            }
        }
    }

    fn description(&self) -> &'static str {
        "Rugcheck API"
    }
}
