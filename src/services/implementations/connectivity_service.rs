// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Connectivity service — monitors health of all external endpoints (RPC, APIs, internet).

use crate::config::get_config_clone;
use crate::connectivity::checker::ConnectivityChecker;
use crate::connectivity::state;
use crate::events::{record_connectivity_event, Severity};
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// The service's registered name, which the header uses to recognise its health entry.
pub const SERVICE_NAME: &str = "connectivity";

pub struct ConnectivityService {
    checker: Option<ConnectivityChecker>,
}

impl ConnectivityService {
    pub fn new() -> Self {
        Self {
            checker: Some(ConnectivityChecker::new()),
        }
    }
}

#[async_trait]
impl Service for ConnectivityService {
    fn name(&self) -> &'static str {
        SERVICE_NAME
    }

    fn priority(&self) -> i32 {
        5 // Very high priority - starts early, stops late
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![] // No dependencies - foundation service
    }

    fn is_enabled(&self) -> bool {
        // During pre-initialization (no config loaded), connectivity service should not
        // start. Explore tier: runs in full mode OR Explore Mode (wallet/RPC skipped).
        if !crate::global::is_explore_or_full() {
            return false;
        }

        let cfg = get_config_clone();
        cfg.connectivity.enabled
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::info(LogTag::Connectivity, "Initializing connectivity service");

        // Register all monitors with global state
        if let Some(checker) = &self.checker {
            checker.register_monitors().await;

            // Set readiness flag
            crate::global::CONNECTIVITY_SYSTEM_READY
                .store(true, std::sync::atomic::Ordering::SeqCst);

            logger::info(
                LogTag::Connectivity,
                &format!(
                    "Connectivity service initialized with {} monitors",
                    checker.monitors.len()
                ),
            );

            // Record initialization event
            tokio::spawn({
                let monitor_count = checker.monitors.len();
                async move {
                    record_connectivity_event(
                        "system",
                        "service_initialized",
                        Severity::Info,
                        crate::events::with_text(
                            serde_json::json!({
                                "monitor_count": monitor_count,
                            }),
                            &UiText::new(ids::EVENTS_CONNECTIVITY_SERVICE_INITIALIZED)
                                .arg("count", UiArg::Text(monitor_count.to_string())),
                        ),
                    )
                    .await;
                }
            });
        }

        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::info(LogTag::Connectivity, "Starting connectivity service");

        let cfg = get_config_clone();
        if !cfg.connectivity.enabled {
            logger::info(
                LogTag::Connectivity,
                "Connectivity monitoring disabled in config",
            );
            return Ok(vec![]);
        }

        // Record service start event
        tokio::spawn({
            let check_interval = cfg.connectivity.check_interval_secs;
            async move {
                record_connectivity_event(
                    "system",
                    "service_started",
                    Severity::Info,
                    crate::events::with_text(
                        serde_json::json!({
                            "check_interval_secs": check_interval,
                        }),
                        &UiText::new(ids::EVENTS_CONNECTIVITY_MONITORING_STARTED)
                            .arg("seconds", UiArg::Text(check_interval.to_string())),
                    ),
                )
                .await;
            }
        });

        // Move monitors out for the background task
        let checker = self.checker.take().ok_or_else(|| {
            crate::errors::Error::Service(crate::errors::ServiceError::Start {
                service: SERVICE_NAME.to_owned(),
                message: "Checker already taken".to_owned(),
            })
        })?;
        let monitors = checker.monitors;

        let handle = tokio::spawn(monitor.instrument(async move {
            ConnectivityChecker::run_health_checks(monitors, shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::Connectivity, "Connectivity service stopped");

        // Record service stop event
        tokio::spawn(async move {
            record_connectivity_event(
                "system",
                "service_stopped",
                Severity::Info,
                crate::events::with_text(
                    serde_json::json!({}),
                    &UiText::new(ids::EVENTS_CONNECTIVITY_MONITORING_STOPPED),
                ),
            )
            .await;
        });

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Service is healthy if not started yet (neither full nor Explore Mode)
        if !crate::global::is_explore_or_full() {
            return ServiceHealth::Healthy; // Not started yet, so healthy
        }

        let cfg = get_config_clone();

        if !cfg.connectivity.enabled {
            return ServiceHealth::Healthy;
        }

        endpoints_health(
            &state::get_critical_endpoint_status().await,
            state::first_probe_window(),
        )
    }
}

/// Health from the critical endpoints. An endpoint confirmed down is a fault, and so is one
/// never checked within `first_probe_window` of its registration; one still inside that
/// window leaves the service starting, as the header reads it, instead of reporting every
/// freshly started endpoint as unhealthy.
fn endpoints_health(status: &state::CriticalEndpointStatus, window: Duration) -> ServiceHealth {
    if !status.down.is_empty() {
        ServiceHealth::Unhealthy(
            UiText::new(ids::SERVICES_HEALTH_CONNECTIVITY_CRITICAL_UNHEALTHY)
                .arg("endpoints", UiArg::Text(format!("{:?}", status.down))),
        )
    } else if !status.unchecked.is_empty() {
        ServiceHealth::Unhealthy(
            UiText::new(ids::SERVICES_HEALTH_CONNECTIVITY_CRITICAL_UNCHECKED)
                .arg("endpoints", UiArg::Text(format!("{:?}", status.unchecked)))
                .arg("seconds", UiArg::Text(window.as_secs().to_string())),
        )
    } else if status.awaiting_first_probe {
        ServiceHealth::Starting
    } else {
        ServiceHealth::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Duration = Duration::from_secs(30);

    fn status(
        down: Vec<&'static str>,
        unchecked: Vec<&'static str>,
        awaiting_first_probe: bool,
    ) -> state::CriticalEndpointStatus {
        state::CriticalEndpointStatus {
            down,
            unchecked,
            awaiting_first_probe,
        }
    }

    #[test]
    fn unprobed_endpoints_are_starting_within_their_window() {
        assert!(matches!(
            endpoints_health(&status(vec![], vec![], true), WINDOW),
            ServiceHealth::Starting
        ));
        assert!(matches!(
            endpoints_health(&status(vec![], vec![], false), WINDOW),
            ServiceHealth::Healthy
        ));
    }

    #[test]
    fn an_endpoint_never_checked_after_its_window_is_unhealthy_with_its_cause() {
        let ServiceHealth::Unhealthy(text) =
            endpoints_health(&status(vec![], vec!["rpc"], true), WINDOW)
        else {
            panic!("an endpoint never checked after its window is unhealthy");
        };
        assert_eq!(
            text.id,
            ids::SERVICES_HEALTH_CONNECTIVITY_CRITICAL_UNCHECKED.as_str()
        );
        let rendered = text.render_source_plain();
        assert!(
            rendered.contains("rpc") && rendered.contains("30"),
            "{rendered}"
        );
    }

    #[test]
    fn a_confirmed_down_endpoint_is_unhealthy() {
        let ServiceHealth::Unhealthy(text) =
            endpoints_health(&status(vec!["rpc"], vec![], false), WINDOW)
        else {
            panic!("a confirmed-down endpoint is unhealthy");
        };
        assert_eq!(
            text.id,
            ids::SERVICES_HEALTH_CONNECTIVITY_CRITICAL_UNHEALTHY.as_str()
        );
    }
}
