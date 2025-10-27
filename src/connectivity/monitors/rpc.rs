use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use crate::config::get_config_clone;
use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use std::time::Instant;

/// RPC endpoint monitor - checks health of all configured RPC URLs
pub struct RpcMonitor;

impl RpcMonitor {
    pub fn new() -> Self {
        Self
    }

    /// Check health of a single RPC URL
    async fn check_rpc_url(&self, url: &str, timeout_secs: u64) -> Result<u64, String> {
        let start = Instant::now();

        // Create temporary RPC client for health check
        let rpc_client = RpcClient::new_with_timeout(
            url.to_string(),
            std::time::Duration::from_secs(timeout_secs),
        );

        // Use getHealth RPC method (lightweight and fast)
        match tokio::task::spawn_blocking(move || rpc_client.get_health()).await {
            Ok(Ok(_)) => {
                let latency = start.elapsed().as_millis() as u64;
                Ok(latency)
            }
            Ok(Err(e)) => Err(format!("RPC health check failed: {}", e)),
            Err(e) => Err(format!("RPC health check task failed: {}", e)),
        }
    }
}

#[async_trait]
impl EndpointMonitor for RpcMonitor {
    fn name(&self) -> &'static str {
        "rpc"
    }

    fn criticality(&self) -> EndpointCriticality {
        EndpointCriticality::Critical
    }

    fn fallback_strategy(&self) -> Option<FallbackStrategy> {
        Some(FallbackStrategy::Fail)
    }

    fn is_enabled(&self) -> bool {
        let cfg = get_config_clone();
        cfg.connectivity.enabled && cfg.connectivity.endpoints.rpc.enabled
    }

    async fn check_health(&self) -> HealthCheckResult {
        let cfg = get_config_clone();
        let timeout_secs = cfg.connectivity.health_check_timeout_secs;
        let rpc_urls = &cfg.rpc.urls;

        if rpc_urls.is_empty() {
            return HealthCheckResult::failure("No RPC URLs configured".to_string());
        }

        let mut successful_checks = 0;
        let mut total_latency = 0u64;
        let mut errors = Vec::new();

        // Check all RPC URLs
        for url in rpc_urls {
            match self.check_rpc_url(url, timeout_secs).await {
                Ok(latency) => {
                    successful_checks += 1;
                    total_latency += latency;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", url, e));
                }
            }
        }

        // If at least one RPC is healthy, consider the endpoint available
        if successful_checks > 0 {
            let avg_latency = total_latency / successful_checks;
            
            if (successful_checks as usize) < rpc_urls.len() {
                // Some RPCs failed
                HealthCheckResult::degraded(
                    avg_latency,
                    format!(
                        "{}/{} RPC URLs healthy. Failures: {}",
                        successful_checks,
                        rpc_urls.len(),
                        errors.join("; ")
                    ),
                )
            } else {
                // All RPCs healthy
                HealthCheckResult::success(avg_latency)
            }
        } else {
            // All RPCs failed
            HealthCheckResult::failure(format!(
                "All {} RPC URLs unreachable: {}",
                rpc_urls.len(),
                errors.join("; ")
            ))
        }
    }

    fn description(&self) -> &'static str {
        "Solana RPC endpoints"
    }
}
