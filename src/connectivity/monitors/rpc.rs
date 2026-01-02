use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::types::{EndpointCriticality, FallbackStrategy, HealthCheckResult};
use crate::rpc::{get_rpc_client, RpcClientMethods};
use async_trait::async_trait;

/// RPC endpoint monitor - checks health of all configured RPC providers
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
        let rpc_client = get_rpc_client();

        // Get provider health info from new RPC architecture
        let provider_health = rpc_client.get_provider_health().await;

        if provider_health.is_empty() {
            return HealthCheckResult::failure("No RPC providers configured".to_string());
        }

        let total_providers = provider_health.len();
        let healthy_providers: Vec<_> = provider_health.iter().filter(|p| p.is_healthy).collect();
        let healthy_count = healthy_providers.len();

        // Calculate average latency from healthy providers
        let avg_latency = if !healthy_providers.is_empty() {
            let total_latency: f64 = healthy_providers.iter().map(|p| p.avg_latency_ms).sum();
            (total_latency / healthy_count as f64) as u64
        } else {
            0
        };

        // Collect errors from unhealthy providers
        let errors: Vec<String> = provider_health
            .iter()
            .filter(|p| !p.is_healthy)
            .map(|p| format!("{} ({}): {:?}", p.url_masked, p.kind, p.circuit_state))
            .collect();

        // If at least one RPC is healthy, consider the endpoint available
        if healthy_count > 0 {
            if healthy_count < total_providers {
                // Some providers failed
                HealthCheckResult::degraded(
                    avg_latency,
                    format!(
                        "{}/{} RPC providers healthy. Unhealthy: {}",
                        healthy_count,
                        total_providers,
                        errors.join("; ")
                    ),
                )
            } else {
                // All providers healthy
                HealthCheckResult::success(avg_latency)
            }
        } else {
            // All providers failed
            HealthCheckResult::failure(format!(
                "All {} RPC providers unreachable: {}",
                total_providers,
                errors.join("; ")
            ))
        }
    }

    fn description(&self) -> &'static str {
        "Solana RPC endpoints"
    }
}
