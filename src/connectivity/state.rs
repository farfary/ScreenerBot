use super::types::{EndpointCriticality, EndpointHealth, FallbackStrategy};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::RwLock;

/// Global endpoint health state storage
pub struct ConnectivityState {
    /// Map of endpoint name -> health status
    health: HashMap<&'static str, EndpointHealth>,
    /// Map of endpoint name -> criticality level
    criticality: HashMap<&'static str, EndpointCriticality>,
    /// Map of endpoint name -> fallback strategy
    fallback: HashMap<&'static str, Option<FallbackStrategy>>,
    /// Map of endpoint name -> consecutive failure count
    failures: HashMap<&'static str, u32>,
    /// Map of endpoint name -> consecutive success count
    successes: HashMap<&'static str, u32>,
}

impl ConnectivityState {
    fn new() -> Self {
        Self {
            health: HashMap::new(),
            criticality: HashMap::new(),
            fallback: HashMap::new(),
            failures: HashMap::new(),
            successes: HashMap::new(),
        }
    }

    /// Register an endpoint with its metadata
    pub fn register_endpoint(
        &mut self,
        name: &'static str,
        criticality: EndpointCriticality,
        fallback: Option<FallbackStrategy>,
    ) {
        self.health
            .insert(name, EndpointHealth::Unknown);
        self.criticality.insert(name, criticality);
        self.fallback.insert(name, fallback);
        self.failures.insert(name, 0);
        self.successes.insert(name, 0);
    }

    /// Update health status for an endpoint
    pub fn update_health(
        &mut self,
        name: &'static str,
        healthy: bool,
        latency_ms: u64,
        error: Option<String>,
        failure_threshold: u32,
        recovery_threshold: u32,
    ) {
        let now = Utc::now();

        if healthy {
            // Increment success counter
            let successes = self.successes.entry(name).or_insert(0);
            *successes += 1;

            // Check if we've recovered
            if *successes >= recovery_threshold {
                // Reset failure counter
                self.failures.insert(name, 0);

                // Update health status
                if let Some(reason) = error {
                    // Degraded (healthy but with warning)
                    self.health.insert(
                        name,
                        EndpointHealth::Degraded {
                            latency_ms,
                            reason,
                            last_check: now,
                        },
                    );
                } else {
                    // Fully healthy
                    self.health.insert(
                        name,
                        EndpointHealth::Healthy {
                            latency_ms,
                            last_check: now,
                        },
                    );
                }
            } else {
                // Still recovering, keep previous unhealthy status but update check time
                // Don't update health yet until we reach recovery_threshold
            }
        } else {
            // Increment failure counter
            let failures = self.failures.entry(name).or_insert(0);
            *failures += 1;

            // Reset success counter
            self.successes.insert(name, 0);

            // Check if we've crossed failure threshold
            if *failures >= failure_threshold {
                // Get last successful check time
                let last_success = match self.health.get(name) {
                    Some(EndpointHealth::Healthy { last_check, .. })
                    | Some(EndpointHealth::Degraded { last_check, .. }) => Some(*last_check),
                    Some(EndpointHealth::Unhealthy { last_success, .. }) => *last_success,
                    _ => None,
                };

                self.health.insert(
                    name,
                    EndpointHealth::Unhealthy {
                        reason: error.unwrap_or_else(|| "Unknown error".to_string()),
                        last_check: now,
                        last_success,
                        consecutive_failures: *failures,
                    },
                );
            }
        }
    }

    /// Get health status for an endpoint
    pub fn get_health(&self, name: &str) -> Option<EndpointHealth> {
        self.health.get(name).cloned()
    }

    /// Get criticality level for an endpoint
    pub fn get_criticality(&self, name: &str) -> Option<EndpointCriticality> {
        self.criticality.get(name).copied()
    }

    /// Get fallback strategy for an endpoint
    pub fn get_fallback(&self, name: &str) -> Option<FallbackStrategy> {
        self.fallback.get(name).and_then(|f| f.clone())
    }

    /// Get all endpoint health statuses
    pub fn get_all_health(&self) -> HashMap<&'static str, EndpointHealth> {
        self.health.clone()
    }

    /// Check if endpoint is healthy (available for use)
    pub fn is_healthy(&self, name: &str) -> bool {
        self.health
            .get(name)
            .map(|h| h.is_available())
            .unwrap_or(false)
    }

    /// Check if all critical endpoints are healthy
    pub fn are_critical_endpoints_healthy(&self) -> bool {
        for (name, criticality) in &self.criticality {
            if *criticality == EndpointCriticality::Critical {
                if !self.is_healthy(name) {
                    return false;
                }
            }
        }
        true
    }

    /// Get list of unhealthy critical endpoints
    pub fn get_unhealthy_critical_endpoints(&self) -> Vec<&'static str> {
        let mut unhealthy = Vec::new();
        for (name, criticality) in &self.criticality {
            if *criticality == EndpointCriticality::Critical && !self.is_healthy(name) {
                unhealthy.push(*name);
            }
        }
        unhealthy
    }
}

/// Global connectivity state instance
static GLOBAL_STATE: LazyLock<Arc<RwLock<ConnectivityState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(ConnectivityState::new())));

/// Get reference to global connectivity state
pub fn get_state() -> Arc<RwLock<ConnectivityState>> {
    GLOBAL_STATE.clone()
}

/// Register an endpoint with the global state
pub async fn register_endpoint(
    name: &'static str,
    criticality: EndpointCriticality,
    fallback: Option<FallbackStrategy>,
) {
    let state_arc = get_state();
    let mut state = state_arc.write().await;
    state.register_endpoint(name, criticality, fallback);
}

/// Update health status for an endpoint
pub async fn update_health(
    name: &'static str,
    healthy: bool,
    latency_ms: u64,
    error: Option<String>,
    failure_threshold: u32,
    recovery_threshold: u32,
) {
    let state_arc = get_state();
    let mut state = state_arc.write().await;
    state.update_health(
        name,
        healthy,
        latency_ms,
        error,
        failure_threshold,
        recovery_threshold,
    );
}

/// Check if an endpoint is healthy
pub async fn is_endpoint_healthy(name: &str) -> bool {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.is_healthy(name)
}

/// Get health status for an endpoint
pub async fn get_endpoint_health(name: &str) -> Option<EndpointHealth> {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.get_health(name)
}

/// Get fallback strategy for an endpoint
pub async fn get_fallback_strategy(name: &str) -> Option<FallbackStrategy> {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.get_fallback(name)
}

/// Check if all critical endpoints are healthy
pub async fn are_critical_endpoints_healthy() -> bool {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.are_critical_endpoints_healthy()
}

/// Get list of unhealthy critical endpoints
pub async fn get_unhealthy_critical_endpoints() -> Vec<&'static str> {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.get_unhealthy_critical_endpoints()
}

/// Get all endpoint health statuses
pub async fn get_all_health() -> HashMap<&'static str, EndpointHealth> {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.get_all_health()
}
