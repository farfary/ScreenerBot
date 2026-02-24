//! Connectivity health checking — monitors external endpoint availability.

use crate::config::get_config_clone;
use crate::connectivity::monitor::EndpointMonitor;
use crate::connectivity::monitors::{
    DexScreenerMonitor, GeckoTerminalMonitor, GmgnMonitor, InternetMonitor, JupiterMonitor,
    RpcMonitor, RugcheckMonitor,
};
use crate::connectivity::state;
use crate::events::{record_connectivity_event, Severity};
use crate::logger::{self, LogTag};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{interval, Duration};

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
    pub fn new() -> Self {
        // Initialize all monitors
        let monitors: Vec<Box<dyn EndpointMonitor>> = vec![
            Box::new(InternetMonitor::new()),
            Box::new(RpcMonitor::new()),
            Box::new(DexScreenerMonitor::new()),
            Box::new(GeckoTerminalMonitor::new()),
            Box::new(RugcheckMonitor::new()),
            Box::new(GmgnMonitor::new()),
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

    /// Run health check for a single monitor
    pub async fn check_monitor(monitor: &Box<dyn EndpointMonitor>) {
        if !monitor.is_enabled() {
            return;
        }

        let name = monitor.name();
        let criticality = monitor.criticality();
        let fallback = monitor.fallback_strategy();

        state::ensure_endpoint_registered(name, criticality, fallback).await;

        // Capture previous health state BEFORE updating
        let previous_health = state::get_endpoint_health(name).await;

        let result = monitor.check_health().await;

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
            None => return,
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
                                    "message": format!("Endpoint recovered from {} to healthy", from_state),
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
    }

    /// Background task that periodically checks all endpoints
    pub async fn run_health_checks(monitors: Vec<Box<dyn EndpointMonitor>>, shutdown: Arc<Notify>) {
        let cfg = get_config_clone();
        let check_interval_secs = cfg.connectivity.check_interval_secs;

        let mut interval_timer = interval(Duration::from_secs(check_interval_secs));
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        logger::info(
            LogTag::Connectivity,
            &format!(
                "Starting connectivity health checks (interval={}s)",
                check_interval_secs
            ),
        );

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::Connectivity, "Connectivity health checks shutting down");
                    break;
                }
                _ = interval_timer.tick() => {
                    // Check all monitors sequentially (they're already async and lightweight)
                    for monitor in &monitors {
                        if monitor.is_enabled() {
                            Self::check_monitor(monitor).await;
                        }
                    }

                    // Check if critical endpoints are unhealthy
                    let unhealthy = state::get_unhealthy_critical_endpoints().await;
                    if !unhealthy.is_empty() {
                        logger::error(
                            LogTag::Connectivity,
                            &format!(
                                "CRITICAL: {} critical endpoint(s) unhealthy: {:?} - System should pause operations",
                                unhealthy.len(),
                                unhealthy
                            ),
                        );

                        // Record critical endpoints event
                        tokio::spawn({
                            let unhealthy_list: Vec<String> = unhealthy.iter().map(|s| s.to_string()).collect();
                            let count = unhealthy.len();
                            async move {
                                record_connectivity_event(
                                    "system",
                                    "critical_endpoints_unhealthy",
                                    Severity::Error,
                                    serde_json::json!({
                                        "unhealthy_count": count,
                                        "unhealthy_endpoints": unhealthy_list,
                                        "message": format!("{} critical endpoint(s) unhealthy - System should pause operations", count),
                                    }),
                                )
                                .await;
                            }
                        });
                    }
                }
            }
        }
    }
}
