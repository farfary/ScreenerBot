//! Connectivity service — monitors health of all external endpoints (RPC, APIs, internet).

use crate::config::get_config_clone;
use crate::connectivity::checker::ConnectivityChecker;
use crate::connectivity::state;
use crate::events::{record_connectivity_event, Severity};
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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
        "connectivity"
    }

    fn priority(&self) -> i32 {
        5 // Very high priority - starts early, stops late
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![] // No dependencies - foundation service
    }

    fn is_enabled(&self) -> bool {
        // During pre-initialization (no config loaded), connectivity service should not start
        if !crate::global::is_initialization_complete() {
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
                        serde_json::json!({
                            "monitor_count": monitor_count,
                            "message": format!("Connectivity service initialized with {} monitors", monitor_count),
                        }),
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
                    serde_json::json!({
                        "check_interval_secs": check_interval,
                        "message": format!("Connectivity monitoring started (interval={}s)", check_interval),
                    }),
                )
                .await;
            }
        });

        // Move monitors out for the background task
        let checker = self.checker.take().ok_or_else(|| {
            crate::errors::Error::Service(crate::errors::ServiceError::Start {
                service: "connectivity".to_string(),
                message: "Checker already taken".to_string(),
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
                serde_json::json!({
                    "message": "Connectivity monitoring stopped",
                }),
            )
            .await;
        });

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Service is healthy if not initialized yet or if enabled
        if !crate::global::is_initialization_complete() {
            return ServiceHealth::Healthy; // Not started yet, so healthy
        }

        let cfg = get_config_clone();

        if !cfg.connectivity.enabled {
            return ServiceHealth::Healthy;
        }

        if state::are_critical_endpoints_healthy().await {
            ServiceHealth::Healthy
        } else {
            let unhealthy = state::get_unhealthy_critical_endpoints().await;
            ServiceHealth::Unhealthy(format!("Critical endpoints unhealthy: {:?}", unhealthy))
        }
    }
}
