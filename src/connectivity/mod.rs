//! External service health monitoring — tracks API availability and latency.
pub mod checker;
pub mod monitor;
pub mod monitors;
pub mod state;
pub mod types;

pub use checker::ConnectivityChecker;
pub use monitor::EndpointMonitor;
pub use state::{
    are_critical_endpoints_healthy, get_all_health, get_endpoint_health, get_fallback_strategy,
    get_unhealthy_critical_endpoints, is_endpoint_healthy, is_endpoint_offline, is_network_offline,
};
pub use types::{EndpointCriticality, EndpointHealth, FallbackStrategy, HealthCheckResult};

/// Check that none of the named endpoints is CONFIRMED down.
///
/// Returns None if every endpoint may be used, Some(endpoint_names) for those that are
/// explicitly `Unhealthy`.
///
/// This is the gate for decisions that must not be blocked by the ABSENCE of a health
/// reading, only by a negative one. [`check_endpoints_healthy`] maps `Unknown` to
/// unhealthy, and an endpoint stays `Unknown` forever when its monitor is disabled in
/// config (a disabled monitor is never registered and never probed) — so requiring an
/// optional endpoint such as `rugcheck` there silently blocked EVERY automated entry for
/// as long as the operator left that monitor off, and blocked all of them at boot until
/// the first probe landed. The hard fail-closed check stays where the network is
/// actually used: the buy and sell executors.
pub async fn check_endpoints_usable(endpoint_names: &[&str]) -> Option<String> {
    let mut offline = Vec::new();

    for name in endpoint_names {
        if is_endpoint_offline(name).await {
            offline.push(name.to_string());
        }
    }

    if offline.is_empty() {
        None
    } else {
        Some(offline.join(", "))
    }
}

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
