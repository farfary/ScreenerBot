// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Connectivity state management — tracks and aggregates endpoint health across services.

use super::types::{EndpointCriticality, EndpointHealth, FallbackStrategy, ProbeFailure};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Lock-free mirror of "is the `internet` endpoint confirmed offline?", updated
/// by `ConnectivityState::update_health`. Lets hot fetch loops gate with a cheap
/// atomic load (see `is_network_offline`) instead of taking the async RwLock.
static INTERNET_OFFLINE: AtomicBool = AtomicBool::new(false);

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
    /// Map of endpoint name -> when it was registered, the start of its first-probe window
    registered_at: HashMap<&'static str, Instant>,
}

/// The critical endpoints, sorted by name within each group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CriticalEndpointStatus {
    /// Confirmed down: consecutive failures crossed the threshold.
    pub down: Vec<&'static str>,
    /// Still unknown after the first-probe window: no probe has settled their state since
    /// registration, which is a fault of its own and never a startup state.
    pub unchecked: Vec<&'static str>,
    /// Some critical endpoint is still unknown inside its first-probe window.
    pub awaiting_first_probe: bool,
}

impl CriticalEndpointStatus {
    /// Whether any critical endpoint is a fault: confirmed down or never checked.
    pub fn is_faulted(&self) -> bool {
        !self.down.is_empty() || !self.unchecked.is_empty()
    }
}

impl ConnectivityState {
    fn new() -> Self {
        Self {
            health: HashMap::new(),
            criticality: HashMap::new(),
            fallback: HashMap::new(),
            failures: HashMap::new(),
            successes: HashMap::new(),
            registered_at: HashMap::new(),
        }
    }

    /// Forget every registered endpoint and its health/failure/success counters.
    /// Test-only: production code discovers endpoints once at monitor startup
    /// and never needs to un-know one. Integration tests that drive real
    /// endpoint names (e.g. "rpc", "dexscreener", "rugcheck") through this
    /// global state must call this before each test to avoid inheriting
    /// registration/health left behind by a previous test — this state is
    /// process-global and outlives any single `#[tokio::test]`.
    pub fn reset_all_for_tests(&mut self) {
        self.health.clear();
        self.criticality.clear();
        self.fallback.clear();
        self.failures.clear();
        self.successes.clear();
        self.registered_at.clear();
        INTERNET_OFFLINE.store(false, Ordering::Relaxed);
    }

    /// Register an endpoint with its metadata
    pub fn register_endpoint(
        &mut self,
        name: &'static str,
        criticality: EndpointCriticality,
        fallback: Option<FallbackStrategy>,
    ) {
        self.health.insert(name, EndpointHealth::Unknown);
        self.criticality.insert(name, criticality);
        self.fallback.insert(name, fallback);
        self.failures.insert(name, 0);
        self.successes.insert(name, 0);
        self.registered_at.insert(name, Instant::now());
    }

    /// Ensure endpoint metadata exists without resetting current health
    pub fn ensure_endpoint_metadata(
        &mut self,
        name: &'static str,
        criticality: EndpointCriticality,
        fallback: Option<FallbackStrategy>,
    ) {
        self.health.entry(name).or_insert(EndpointHealth::Unknown);
        self.registered_at.entry(name).or_insert_with(Instant::now);
        self.failures.entry(name).or_default();
        self.successes.entry(name).or_default();

        let should_update_criticality = self
            .criticality
            .get(name)
            .map(|current| *current != criticality)
            .unwrap_or(true);

        if should_update_criticality {
            self.criticality.insert(name, criticality);
        }

        let should_update_fallback = self
            .fallback
            .get(name)
            .map(|current| current != &fallback)
            .unwrap_or(true);

        if should_update_fallback {
            self.fallback.insert(name, fallback);
        }
    }

    /// Update health status for an endpoint
    pub fn update_health(
        &mut self,
        name: &'static str,
        healthy: bool,
        latency_ms: u64,
        error: Option<ProbeFailure>,
        failure_threshold: u32,
        recovery_threshold: u32,
    ) {
        let now = Utc::now();

        if healthy {
            // Increment success counter
            let successes = self.successes.entry(name).or_default();
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
            let failures = self.failures.entry(name).or_default();
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
                        reason: error.unwrap_or(ProbeFailure::Unknown),
                        last_check: now,
                        last_success,
                        consecutive_failures: *failures,
                    },
                );
            }
        }

        // Keep the lock-free `is_network_offline()` mirror in sync with the
        // "internet" endpoint so hot fetch loops can gate with a cheap atomic
        // load instead of taking the async RwLock on every iteration.
        if name == "internet" {
            INTERNET_OFFLINE.store(self.is_confirmed_unhealthy("internet"), Ordering::Relaxed);
        }
    }

    /// Get health status for an endpoint
    pub fn get_health(&self, name: &str) -> Option<EndpointHealth> {
        self.health.get(name).cloned()
    }

    /// Get criticality level for an endpoint
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
            .unwrap_or_default()
    }

    /// Check if an endpoint is CONFIRMED unhealthy.
    ///
    /// Unlike `is_healthy`, this returns true only for the `Unhealthy` state
    /// (consecutive failures crossed the threshold). `Unknown` (not yet
    /// checked), `Degraded`, and `Healthy` all return false. This is the safe
    /// signal for "stop hammering the network" gates: it never blocks before
    /// the first health check has run, so startup behavior is unchanged.
    pub fn is_confirmed_unhealthy(&self, name: &str) -> bool {
        self.health
            .get(name)
            .map(|h| h.is_unhealthy())
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

    /// Critical endpoints that are CONFIRMED down — the aggregate counterpart
    /// of [`Self::is_confirmed_unhealthy`], and the only honest basis for an
    /// alarm that tells the operator to pause trading.
    ///
    /// [`Self::get_unhealthy_critical_endpoints`] negates `is_healthy`, which
    /// reports `Unknown` (registered, never probed) as unhealthy. That is the
    /// right reading for "is this endpoint usable yet?" and the wrong one for
    /// "has this endpoint failed?": on every boot the first pass ran before any
    /// probe had landed, so the checker logged
    /// `CRITICAL: 2 critical endpoint(s) unhealthy: ["rpc", "internet"]` one
    /// second into startup and never again, on a machine that was online the
    /// whole time. An alarm that cries wolf at every launch trains the operator
    /// to ignore the real one.
    pub fn get_confirmed_unhealthy_critical_endpoints(&self) -> Vec<&'static str> {
        let mut unhealthy = Vec::new();
        for (name, criticality) in &self.criticality {
            if *criticality == EndpointCriticality::Critical && self.is_confirmed_unhealthy(name) {
                unhealthy.push(*name);
            }
        }
        unhealthy
    }

    /// The critical endpoints as faults and pending probes at `now`. An endpoint still
    /// `Unknown` is pending only within `first_probe_window` of its registration; after
    /// that the absence of any settled probe is reported as `unchecked`, so a probe loop
    /// that never ran or never finishes cannot read as "starting" forever.
    pub fn critical_endpoint_status(
        &self,
        first_probe_window: Duration,
        now: Instant,
    ) -> CriticalEndpointStatus {
        let mut status = CriticalEndpointStatus::default();
        for (name, criticality) in &self.criticality {
            if *criticality != EndpointCriticality::Critical {
                continue;
            }
            match self.health.get(name) {
                Some(EndpointHealth::Unhealthy { .. }) => status.down.push(*name),
                Some(EndpointHealth::Unknown) | None => {
                    let in_window = self.registered_at.get(name).is_some_and(|registered| {
                        now.saturating_duration_since(*registered) < first_probe_window
                    });
                    if in_window {
                        status.awaiting_first_probe = true;
                    } else {
                        status.unchecked.push(*name);
                    }
                }
                Some(_) => {}
            }
        }
        status.down.sort_unstable();
        status.unchecked.sort_unstable();
        status
    }
}

/// How long a registered endpoint may stay unprobed before that is a fault: one steady-state
/// check interval. Every endpoint's first probe is due at once, and the checker runs at the
/// shorter degraded cadence while any critical endpoint is unknown, so a whole interval
/// without a settled probe means the probes are not landing.
pub fn first_probe_window() -> Duration {
    crate::config::with_config(|cfg| {
        Duration::from_secs(cfg.connectivity.check_interval_secs.max(1))
    })
}

/// Global connectivity state instance
static GLOBAL_STATE: LazyLock<Arc<RwLock<ConnectivityState>>> =
    LazyLock::new(|| Arc::new(RwLock::new(ConnectivityState::new())));

/// Get reference to global connectivity state
pub fn get_state() -> Arc<RwLock<ConnectivityState>> {
    GLOBAL_STATE.clone()
}

/// Forget every registered endpoint. Test-only — see
/// `ConnectivityState::reset_all_for_tests`.
pub async fn reset_all_for_tests() {
    let state_arc = get_state();
    let mut state = state_arc.write().await;
    state.reset_all_for_tests();
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

/// Ensure endpoint metadata exists without overwriting health state
pub async fn ensure_endpoint_registered(
    name: &'static str,
    criticality: EndpointCriticality,
    fallback: Option<FallbackStrategy>,
) {
    let state_arc = get_state();
    let mut state = state_arc.write().await;
    state.ensure_endpoint_metadata(name, criticality, fallback);
}

/// Update health status for an endpoint
pub async fn update_health(
    name: &'static str,
    healthy: bool,
    latency_ms: u64,
    error: Option<ProbeFailure>,
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

/// Check if a specific endpoint is CONFIRMED unhealthy (see
/// [`ConnectivityState::is_confirmed_unhealthy`]).
pub async fn is_endpoint_offline(name: &str) -> bool {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.is_confirmed_unhealthy(name)
}

/// Convenience gate for network-fetch loops: true only when the `internet`
/// endpoint is confirmed unhealthy. Use this to skip periodic API/RPC fetches
/// while offline instead of letting every task time out on DNS. Returns false
/// at startup (Unknown) so the first fetches still run and seed health state.
///
/// Lock-free (reads an atomic mirror kept in sync by the health checker), so it
/// is safe to call on every iteration of hot loops with no await/lock cost.
pub fn is_network_offline() -> bool {
    INTERNET_OFFLINE.load(Ordering::Relaxed)
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

/// Critical endpoints confirmed down — see
/// [`ConnectivityState::get_confirmed_unhealthy_critical_endpoints`].
pub async fn get_confirmed_unhealthy_critical_endpoints() -> Vec<&'static str> {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.get_confirmed_unhealthy_critical_endpoints()
}

/// The critical endpoints as faults and pending probes, under [`first_probe_window`] — see
/// [`ConnectivityState::critical_endpoint_status`].
pub async fn get_critical_endpoint_status() -> CriticalEndpointStatus {
    let window = first_probe_window();
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.critical_endpoint_status(window, Instant::now())
}

/// Get all endpoint health statuses
pub async fn get_all_health() -> HashMap<&'static str, EndpointHealth> {
    let state_arc = get_state();
    let state = state_arc.read().await;
    state.get_all_health()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Duration = Duration::from_secs(30);

    fn state_with(names: &[(&'static str, EndpointCriticality)]) -> ConnectivityState {
        let mut state = ConnectivityState::new();
        for (name, criticality) in names {
            state.register_endpoint(name, *criticality, None);
        }
        state
    }

    #[test]
    fn an_unprobed_critical_endpoint_is_pending_only_inside_its_window() {
        let state = state_with(&[
            ("rpc", EndpointCriticality::Critical),
            ("dexscreener", EndpointCriticality::Important),
        ]);
        let registered = state.registered_at["rpc"];

        let inside = state.critical_endpoint_status(WINDOW, registered + WINDOW / 2);
        assert!(inside.awaiting_first_probe);
        assert!(!inside.is_faulted());

        let at_bound = state.critical_endpoint_status(WINDOW, registered + WINDOW);
        assert!(!at_bound.awaiting_first_probe);
        assert_eq!(at_bound.unchecked, ["rpc"], "only critical endpoints count");
        assert!(at_bound.down.is_empty());
        assert!(at_bound.is_faulted());
    }

    #[test]
    fn a_settled_probe_ends_the_window_either_way() {
        let mut state = state_with(&[
            ("rpc", EndpointCriticality::Critical),
            ("internet", EndpointCriticality::Critical),
        ]);
        let late = state.registered_at["rpc"] + WINDOW * 2;
        state.update_health("rpc", true, 10, None, 3, 1);
        for _ in 0..3 {
            state.update_health("internet", false, 0, None, 3, 1);
        }

        let status = state.critical_endpoint_status(WINDOW, late);
        assert_eq!(status.down, ["internet"]);
        assert!(status.unchecked.is_empty());
        assert!(!status.awaiting_first_probe);
    }
}
