// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Connectivity health checking — monitors external endpoint availability.

use crate::config::{get_config_clone, with_config};
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::monitors::{
    DexScreenerMonitor, GeckoTerminalMonitor, InternetMonitor, RpcMonitor, RugcheckMonitor,
};
use crate::connectivity::state;
use crate::connectivity::types::{EndpointCriticality, EndpointHealth, ProbeFailure};
use crate::events::{record_connectivity_event, Severity};
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::sync::Notify;
use tokio::time::Duration;

/// Chain-owned monitors appended to the neutral set, registered by the
/// composition root; the neutral checker cannot construct them.
type ChainMonitorsFn = fn() -> Vec<Box<dyn EndpointMonitor>>;
static CHAIN_MONITORS: OnceLock<ChainMonitorsFn> = OnceLock::new();

/// Register the chain-contributed endpoint monitors (composition root only).
pub fn set_chain_monitors(factory: ChainMonitorsFn) {
    let _ = CHAIN_MONITORS.set(factory);
}

/// ConnectivityChecker - business logic for monitoring health of all external endpoints
///
/// This checker runs continuous health checks on:
/// - Internet connectivity (DNS, HTTP)
/// - RPC endpoints
/// - API endpoints (DexScreener, GeckoTerminal, Rugcheck, Jupiter, Raptor)
///
/// Critical endpoints (Internet, RPC) will cause system pause when unavailable.
/// Important endpoints (DexScreener, Jupiter) will trigger warnings and degraded mode.
/// Optional endpoints (Rugcheck) will silently fallback when unavailable.
pub struct ConnectivityChecker {
    pub monitors: Vec<Box<dyn EndpointMonitor>>,
}

impl ConnectivityChecker {
    /// Initialize all endpoint monitors for connectivity checking
    pub fn new() -> Self {
        // Initialize all monitors
        let mut monitors: Vec<Box<dyn EndpointMonitor>> = vec![
            Box::new(InternetMonitor::new()),
            Box::new(RpcMonitor::new()),
            Box::new(DexScreenerMonitor::new()),
            Box::new(GeckoTerminalMonitor::new()),
            Box::new(RugcheckMonitor::new()),
        ];
        if let Some(chain_monitors) = CHAIN_MONITORS.get() {
            monitors.extend(chain_monitors());
        }
        Self { monitors }
    }

    /// Register all monitors with global state
    pub async fn register_monitors(&self) {
        for monitor in &self.monitors {
            if monitor.is_enabled() {
                state::register_endpoint(
                    monitor.name(),
                    monitor.criticality(),
                    monitor.fallback_strategy(),
                )
                .await;

                logger::info(
                    LogTag::Connectivity,
                    &format!(
                        "Registered endpoint monitor: {} (criticality={:?}, enabled=true)",
                        monitor.name(),
                        monitor.criticality()
                    ),
                );
            } else {
                logger::debug(
                    LogTag::Connectivity,
                    &format!("Endpoint monitor disabled: {}", monitor.name()),
                );
            }
        }
    }

    /// Run health check for a single monitor. Returns whether this check was
    /// healthy, so the caller can tighten the polling cadence on the very first
    /// failed round instead of waiting for the failure threshold to be crossed.
    pub async fn check_monitor(monitor: &Box<dyn EndpointMonitor>) -> bool {
        if !monitor.is_enabled() {
            return true;
        }

        let name = monitor.name();
        let criticality = monitor.criticality();
        let fallback = monitor.fallback_strategy();

        state::ensure_endpoint_registered(name, criticality, fallback).await;

        // Capture previous health state BEFORE updating
        let previous_health = state::get_endpoint_health(name).await;

        let result = monitor.check_health().await;
        let check_healthy = result.healthy;

        let cfg = get_config_clone();
        let failure_threshold = cfg.connectivity.failure_threshold;
        let recovery_threshold = cfg.connectivity.recovery_threshold;

        // Update global state
        state::update_health(
            name,
            result.healthy,
            result.latency_ms,
            result.error,
            failure_threshold,
            recovery_threshold,
        )
        .await;

        // Get new health state after update
        let new_health = match state::get_endpoint_health(name).await {
            Some(h) => h,
            None => return check_healthy,
        };

        let previous_kind = previous_health
            .as_ref()
            .map(health_kind)
            .unwrap_or("unknown");
        let new_kind = health_kind(&new_health);

        // Only log and record events on state transitions
        let state_changed = previous_kind != new_kind;

        match &new_health {
            crate::connectivity::types::EndpointHealth::Healthy { latency_ms, .. } => {
                logger::debug(
                    LogTag::Connectivity,
                    &format!("{name} endpoint healthy (latency={latency_ms}ms)"),
                );

                if state_changed && previous_kind == "unhealthy" {
                    logger::info(
                        LogTag::Connectivity,
                        &format!("{name} endpoint recovered (latency={latency_ms}ms)"),
                    );
                }

                // Only record recovery event on transition TO healthy
                if state_changed
                    && (previous_kind == "unhealthy"
                        || previous_kind == "degraded"
                        || previous_kind == "unknown")
                {
                    tokio::spawn({
                        let name = name.to_string();
                        let latency = *latency_ms;
                        let from_state = previous_kind.to_string();
                        async move {
                            record_connectivity_event(
                                &name,
                                "healthy",
                                Severity::Info,
                                crate::events::with_text(
                                    serde_json::json!({
                                        "latency_ms": latency,
                                        "previous_state": from_state,
                                    }),
                                    &UiText::new(ids::EVENTS_CONNECTIVITY_ENDPOINT_RECOVERED)
                                        .arg("from", UiArg::Text(from_state.to_string())),
                                ),
                            )
                            .await;
                        }
                    });
                }
            }
            crate::connectivity::types::EndpointHealth::Degraded {
                latency_ms, reason, ..
            } => {
                // Only log warning on state transition
                if state_changed {
                    logger::warning(
                        LogTag::Connectivity,
                        &format!(
                            "{} endpoint degraded (latency={}ms): {}",
                            name,
                            latency_ms,
                            reason.render_source_plain()
                        ),
                    );

                    tokio::spawn({
                        let name = name.to_string();
                        let reason = reason.render_source_plain();
                        let latency = *latency_ms;
                        let from_state = previous_kind.to_string();
                        async move {
                            record_connectivity_event(
                                &name,
                                "degraded",
                                Severity::Warn,
                                serde_json::json!({
                                    "latency_ms": latency,
                                    "reason": reason,
                                    "previous_state": from_state,
                                }),
                            )
                            .await;
                        }
                    });
                }
            }
            crate::connectivity::types::EndpointHealth::Unhealthy {
                reason,
                consecutive_failures,
                ..
            } => {
                // Only log on state transition (but always at appropriate level for critical)
                if state_changed {
                    announce_unhealthy(
                        name,
                        criticality,
                        reason,
                        *consecutive_failures,
                        previous_kind,
                    );
                }
            }
            _ => {}
        }

        check_healthy
    }

    /// Background task that periodically checks all endpoints.
    ///
    /// Two properties make detection fast without a separate "fast" checker:
    /// - **Adaptive cadence:** the loop tightens to `degraded_check_interval_secs`
    ///   whenever any critical endpoint is not healthy (including the Unknown
    ///   startup state), and relaxes to `check_interval_secs` once everything is
    ///   healthy. So an outage and its recovery are seen within a few seconds.
    /// - **Concurrent checks:** all monitors are probed in parallel, so a round
    ///   costs ~one timeout, not the sum of every monitor's timeout (critical
    ///   when offline, where each check would otherwise serialize its timeout).
    pub async fn run_health_checks(monitors: Vec<Box<dyn EndpointMonitor>>, shutdown: Arc<Notify>) {
        let initial_cfg = get_config_clone();
        logger::info(
            LogTag::Connectivity,
            &format!(
                "Starting connectivity health checks (critical every {}s, others every {}s, concurrent)",
                initial_cfg.connectivity.degraded_check_interval_secs,
                initial_cfg.connectivity.check_interval_secs
            ),
        );

        // Per-endpoint last-checked timestamps so each endpoint runs on its own
        // cadence. Critical reachability probes (internet = a TCP connect, rpc =
        // a local provider-health read) are cheap, so they run at the fast
        // cadence for near-real-time outage detection; the rate-limited API
        // health checks (Important/Optional) stay on the slow cadence so we don't
        // burn their quotas. The whole loop ticks at the fast cadence and simply
        // skips endpoints that are not due yet.
        let mut last_checked: HashMap<&'static str, Instant> = HashMap::new();

        // Signature of the last logged unhealthy set, so we log the CRITICAL line
        // only on change — not every fast tick.
        let mut last_unhealthy_sig: Option<String> = None;

        loop {
            let cfg = get_config_clone();
            let fast_secs = cfg.connectivity.degraded_check_interval_secs.max(1);
            let slow_secs = cfg.connectivity.check_interval_secs.max(1);
            let now = Instant::now();

            // Select endpoints whose own cadence is due this tick.
            let due: Vec<&Box<dyn EndpointMonitor>> = monitors
                .iter()
                .filter(|m| m.is_enabled())
                .filter(|m| {
                    let is_critical = m.criticality()
                        == crate::connectivity::types::EndpointCriticality::Critical;
                    let interval = if is_critical { fast_secs } else { slow_secs };
                    match last_checked.get(m.name()) {
                        Some(t) => now.duration_since(*t) >= Duration::from_secs(interval),
                        None => true, // never checked → check immediately
                    }
                })
                .collect();

            // Probe the due endpoints in parallel so a tick costs ~one timeout,
            // not the sum across endpoints (matters when offline).
            let checks = due.iter().map(|m| Self::check_monitor(*m));
            futures::future::join_all(checks).await;
            for m in &due {
                last_checked.insert(m.name(), now);
            }

            // Surface critical outages — only when the unhealthy set changes, so
            // the fast cadence doesn't spam an identical line every couple seconds.
            let unhealthy = state::get_confirmed_unhealthy_critical_endpoints().await;
            let sig = if unhealthy.is_empty() {
                None
            } else {
                let mut names: Vec<&str> = unhealthy.clone();
                names.sort_unstable();
                Some(names.join(","))
            };
            if sig != last_unhealthy_sig {
                last_unhealthy_sig = sig;
                if !unhealthy.is_empty() {
                    logger::error(
                        LogTag::Connectivity,
                        &format!(
                            "CRITICAL: {} critical endpoint(s) unhealthy: {:?} - System should pause operations",
                            unhealthy.len(),
                            unhealthy
                        ),
                    );

                    tokio::spawn({
                        let unhealthy_list: Vec<String> =
                            unhealthy.iter().map(|s| s.to_string()).collect();
                        let count = unhealthy.len();
                        async move {
                            record_connectivity_event(
                                "system",
                                "critical_endpoints_unhealthy",
                                Severity::Error,
                                crate::events::with_text(
                                    serde_json::json!({
                                        "unhealthy_count": count,
                                        "unhealthy_endpoints": unhealthy_list,
                                    }),
                                    &UiText::new(ids::EVENTS_CONNECTIVITY_CRITICAL_UNHEALTHY)
                                        .arg("count", UiArg::Text(count.to_string())),
                                ),
                            )
                            .await;
                        }
                    });
                }
            }

            // Tick at the fast cadence so critical endpoints are re-evaluated on
            // time; non-critical endpoints are simply skipped until their slower
            // cadence is due.
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::Connectivity, "Connectivity health checks shutting down");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(fast_secs)) => {}
            }
        }
    }
}

/// The state a health value is in, as the word the logs and events use for a transition.
fn health_kind(health: &EndpointHealth) -> &'static str {
    match health {
        EndpointHealth::Healthy { .. } => "healthy",
        EndpointHealth::Degraded { .. } => "degraded",
        EndpointHealth::Unhealthy { .. } => "unhealthy",
        EndpointHealth::Unknown => "unknown",
    }
}

/// Log and record that `name` became unhealthy: an error for a critical endpoint, a warning for
/// an important one, info for an optional one.
fn announce_unhealthy(
    name: &'static str,
    criticality: EndpointCriticality,
    reason: &ProbeFailure,
    consecutive_failures: u32,
    previous_kind: &str,
) {
    let log_fn = match criticality {
        EndpointCriticality::Critical => logger::error,
        EndpointCriticality::Important => logger::warning,
        EndpointCriticality::Optional => logger::info,
    };

    log_fn(
        LogTag::Connectivity,
        &format!(
            "{} endpoint unhealthy (failures={}, criticality={:?}): {}",
            name,
            consecutive_failures,
            criticality,
            reason.render_source_plain()
        ),
    );

    let severity = match criticality {
        EndpointCriticality::Critical => Severity::Error,
        EndpointCriticality::Important => Severity::Warn,
        EndpointCriticality::Optional => Severity::Info,
    };

    tokio::spawn({
        let name = name.to_string();
        let reason = reason.render_source_plain();
        let from_state = previous_kind.to_string();
        async move {
            record_connectivity_event(
                &name,
                "unhealthy",
                severity,
                serde_json::json!({
                    "reason": reason,
                    "consecutive_failures": consecutive_failures,
                    "criticality": format!("{:?}", criticality),
                    "previous_state": from_state,
                }),
            )
            .await;
        }
    });
}

/// Report a failure of `monitor`'s endpoint seen outside its probe, such as a refusal that
/// answers a real request. The endpoint is unhealthy at once instead of after
/// `connectivity.failure_threshold` probe rounds, and the probe that finds it answering again
/// lifts it. Nothing happens for an endpoint that is not probed (its monitor is disabled, or not
/// registered yet), because nothing would ever lift the state.
pub async fn report_failure(monitor: &dyn EndpointMonitor, reason: ProbeFailure) {
    if !monitor.is_enabled() {
        return;
    }
    let name = monitor.name();
    let Some(criticality) = state::get_endpoint_criticality(name).await else {
        return;
    };
    let previous_health = state::get_endpoint_health(name).await;
    let recovery_threshold = with_config(|cfg| cfg.connectivity.recovery_threshold);
    state::update_health(name, false, 0, Some(reason.clone()), 1, recovery_threshold).await;

    if previous_health
        .as_ref()
        .is_some_and(EndpointHealth::is_unhealthy)
    {
        return;
    }
    if let Some(EndpointHealth::Unhealthy {
        consecutive_failures,
        ..
    }) = state::get_endpoint_health(name).await
    {
        let previous_kind = previous_health.as_ref().map_or("unknown", health_kind);
        announce_unhealthy(
            name,
            criticality,
            &reason,
            consecutive_failures,
            previous_kind,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectivity::types::{FallbackStrategy, HealthCheckResult};
    use async_trait::async_trait;

    const NAME: &str = "stub-endpoint";

    /// An endpoint whose probe finds it answering.
    struct AnsweringEndpoint {
        enabled: bool,
    }

    #[async_trait]
    impl EndpointMonitor for AnsweringEndpoint {
        fn name(&self) -> &'static str {
            NAME
        }

        fn criticality(&self) -> EndpointCriticality {
            EndpointCriticality::Important
        }

        fn fallback_strategy(&self) -> Option<FallbackStrategy> {
            None
        }

        fn is_enabled(&self) -> bool {
            self.enabled
        }

        async fn check_health(&self) -> HealthCheckResult {
            HealthCheckResult::success(5)
        }
    }

    fn refusal() -> ProbeFailure {
        ProbeFailure::HttpStatus {
            status: "403 Forbidden".to_string(),
        }
    }

    /// Clear the registry, then register `monitor` as the monitor start does.
    async fn registered(monitor: &AnsweringEndpoint) {
        crate::config::utils::install_default_config();
        state::reset_all_for_tests().await;
        state::register_endpoint(
            monitor.name(),
            monitor.criticality(),
            monitor.fallback_strategy(),
        )
        .await;
    }

    #[tokio::test]
    async fn a_reported_failure_makes_the_endpoint_unhealthy_at_once() {
        let endpoint = AnsweringEndpoint { enabled: true };
        registered(&endpoint).await;
        assert!(!state::is_endpoint_offline(NAME).await);

        report_failure(&endpoint, refusal()).await;
        assert!(state::is_endpoint_offline(NAME).await);

        report_failure(&endpoint, refusal()).await;
        assert!(state::is_endpoint_offline(NAME).await);
    }

    #[tokio::test]
    async fn a_report_changes_nothing_for_an_endpoint_that_is_not_probed() {
        let endpoint = AnsweringEndpoint { enabled: true };
        crate::config::utils::install_default_config();
        state::reset_all_for_tests().await;
        report_failure(&endpoint, refusal()).await;
        assert!(state::get_endpoint_health(NAME).await.is_none());

        let disabled = AnsweringEndpoint { enabled: false };
        registered(&disabled).await;
        report_failure(&disabled, refusal()).await;
        assert!(!state::is_endpoint_offline(NAME).await);
    }

    #[tokio::test]
    async fn probes_that_find_the_endpoint_answering_lift_a_reported_failure() {
        let endpoint = AnsweringEndpoint { enabled: true };
        registered(&endpoint).await;
        report_failure(&endpoint, refusal()).await;

        let probe: Box<dyn EndpointMonitor> = Box::new(AnsweringEndpoint { enabled: true });
        let recovery_threshold = with_config(|cfg| cfg.connectivity.recovery_threshold);
        for round in 1..recovery_threshold {
            assert!(
                ConnectivityChecker::check_monitor(&probe).await,
                "round {round}"
            );
            assert!(state::is_endpoint_offline(NAME).await, "round {round}");
        }

        assert!(ConnectivityChecker::check_monitor(&probe).await);
        assert!(!state::is_endpoint_offline(NAME).await);
        assert!(state::is_endpoint_healthy(NAME).await);
    }
}
