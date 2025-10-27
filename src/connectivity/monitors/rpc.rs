use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use async_trait::async_trait;

/// RPC endpoint monitor - checks health of all configured RPC URLs
pub struct RpcMonitor;

impl RpcMonitor {
    pub fn new() -> Self {
        Self
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
        let timeout_secs = cfg.connectivity.endpoints.rpc.timeout_secs.max(1);
        let rpc_client = crate::rpc::get_rpc_client();
        let rpc_urls = rpc_client.get_all_urls();

        if rpc_urls.is_empty() {
            return HealthCheckResult::failure("No RPC URLs configured".to_string());
        }

        let mut successful_checks = 0;
        let mut total_latency = 0u64;
        let mut errors = Vec::new();

        // Check all RPC URLs
        for url in &rpc_urls {
            match rpc_client.probe_get_health(url, timeout_secs).await {
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
