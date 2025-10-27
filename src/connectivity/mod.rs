pub mod monitor;
pub mod monitors;
pub mod service;
pub mod state;
pub mod types;

pub use monitor::EndpointMonitor;
pub use service::ConnectivityService;
pub use state::{
    are_critical_endpoints_healthy, get_all_health, get_endpoint_health, get_fallback_strategy,
    get_unhealthy_critical_endpoints, is_endpoint_healthy,
};
pub use types::{EndpointCriticality, EndpointHealth, FallbackStrategy, HealthCheckResult};

/// Check if specified endpoints are healthy
/// Returns None if all healthy, Some(endpoint_names) if any unhealthy
pub async fn check_endpoints_healthy(endpoint_names: &[&str]) -> Option<String> {
    let mut unhealthy = Vec::new();

    for name in endpoint_names {
        if !is_endpoint_healthy(name).await {
            unhealthy.push(name.to_string());
        }
    }

    if unhealthy.is_empty() {
        None
    } else {
        Some(unhealthy.join(", "))
    }
}
