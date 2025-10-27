use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use async_trait::async_trait;
use std::time::Instant;
use tokio::time::{timeout, Duration};

/// Jupiter API monitor
pub struct JupiterMonitor;

impl JupiterMonitor {
    pub fn new() -> Self {
        Self
    }

    const BASE_URL: &'static str = "https://lite-api.jup.ag";
}

#[async_trait]
impl EndpointMonitor for JupiterMonitor {
    fn name(&self) -> &'static str {
        "jupiter"
    }

    fn criticality(&self) -> EndpointCriticality {
        EndpointCriticality::Important
    }

    fn fallback_strategy(&self) -> Option<FallbackStrategy> {
        Some(FallbackStrategy::UseAlternative {
            endpoint_name: "gmgn".to_string(),
        })
    }

    fn is_enabled(&self) -> bool {
        let cfg = get_config_clone();
        cfg.connectivity.enabled && cfg.connectivity.endpoints.jupiter.enabled
    }

    async fn check_health(&self) -> HealthCheckResult {
        let cfg = get_config_clone();
        let timeout_secs = cfg.connectivity.health_check_timeout_secs;

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(e) => return HealthCheckResult::failure(format!("Failed to create client: {}", e)),
        };

        // Use token search endpoint for health check (lightweight, no auth required)
        let url = format!("{}/tokens/v2/search?query=SOL", Self::BASE_URL);
        let start = Instant::now();

        match timeout(Duration::from_secs(timeout_secs), client.get(&url).send()).await {
            Ok(Ok(response)) => {
                let latency = start.elapsed().as_millis() as u64;

                if response.status().is_success() {
                    HealthCheckResult::success(latency)
                } else {
                    HealthCheckResult::failure(format!("HTTP {}", response.status()))
                }
            }
            Ok(Err(e)) => HealthCheckResult::failure(format!("Request failed: {}", e)),
            Err(_) => HealthCheckResult::failure(format!("Timeout after {}s", timeout_secs)),
        }
    }

    fn description(&self) -> &'static str {
        "Jupiter Aggregator API"
    }
}
