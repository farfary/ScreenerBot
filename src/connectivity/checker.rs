//! Connectivity health checking — monitors external endpoint availability.

use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::monitors::{
    DexScreenerMonitor, GeckoTerminalMonitor, InternetMonitor, JupiterMonitor, RpcMonitor,
    RugcheckMonitor,
};
use crate::connectivity::state;
use crate::events::{record_connectivity_event, Severity};
use crate::logger::{self, LogTag};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Notify;
use tokio::time::Duration;

/// ConnectivityChecker - business logic for monitoring health of all external endpoints
///
/// This checker runs continuous health checks on:
/// - Internet connectivity (DNS, HTTP)
/// - RPC endpoints
/// - API endpoints (DexScreener, GeckoTerminal, Rugcheck, Jupiter)
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
        let monitors: Vec<Box<dyn EndpointMonitor>> = vec![
            Box::new(InternetMonitor::new()),
            Box::new(RpcMonitor::new()),
            Box::new(DexScreenerMonitor::new()),
            Box::new(GeckoTerminalMonitor::new()),
            Box::new(RugcheckMonitor::new()),
            Box::new(JupiterMonitor::new()),
        ];

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

        // Helper to get health state discriminant for comparison
        let get_state_kind = |h: &crate::connectivity::types::EndpointHealth| -> &'static str {
            match h {
                crate::connectivity::types::EndpointHealth::Healthy { .. } => "healthy",
                crate::connectivity::types::EndpointHealth::Degraded { .. } => "degraded",
                crate::connectivity::types::EndpointHealth::Unhealthy { .. } => "unhealthy",
                crate::connectivity::types::EndpointHealth::Unknown => "unknown",
            }
        };

        let previous_kind = previous_health
            .as_ref()
            .map(get_state_kind)
            .unwrap_or("unknown");
        let new_kind = get_state_kind(&new_health);

        // Only log and record events on state transitions
        let state_changed = previous_kind != new_kind;

        match &new_health {
            crate::connectivity::types::EndpointHealth::Healthy { latency_ms, .. } => {
                logger::debug(
                    LogTag::Connectivity,
                    &format!("{name} endpoint healthy (latency={latency_ms}ms)"),
                );

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
                                serde_json::json!({
                                    "latency_ms": latency,
                                    "previous_state": from_state,
                                    "message": format!("Endpoint recovered from {from_state} to healthy"),
                                }),
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
                            name, latency_ms, reason
                        ),
                    );

                    tokio::spawn({
                        let name = name.to_string();
                        let reason = reason.clone();
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
                    let log_fn = match criticality {
                        crate::connectivity::types::EndpointCriticality::Critical => logger::error,
                        crate::connectivity::types::EndpointCriticality::Important => {
                            logger::warning
                        }
                        crate::connectivity::types::EndpointCriticality::Optional => logger::info,
                    };

                    log_fn(
                        LogTag::Connectivity,
                        &format!(
                            "{} endpoint unhealthy (failures={}, criticality={:?}): {}",
                            name, consecutive_failures, criticality, reason
                        ),
                    );

                    let severity = match criticality {
                        crate::connectivity::types::EndpointCriticality::Critical => {
                            Severity::Error
                        }
                        crate::connectivity::types::EndpointCriticality::Important => {
                            Severity::Warn
                        }
                        crate::connectivity::types::EndpointCriticality::Optional => Severity::Info,
                    };

                    tokio::spawn({
                        let name = name.to_string();
                        let reason = reason.clone();
                        let failures = *consecutive_failures;
                        let from_state = previous_kind.to_string();
                        let crit = criticality;
                        async move {
                            record_connectivity_event(
                                &name,
                                "unhealthy",
                                severity,
                                serde_json::json!({
                                    "reason": reason,
                                    "consecutive_failures": failures,
                                    "criticality": format!("{:?}", crit),
                                    "previous_state": from_state,
                                }),
                            )
                            .await;
                        }
                    });
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
                                serde_json::json!({
                                    "unhealthy_count": count,
                                    "unhealthy_endpoints": unhealthy_list,
                                    "message": format!("{count} critical endpoint(s) unhealthy - System should pause operations"),
                                }),
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
