use crate::logger::{self, LogTag};
use crate::services::{log_service_notice, Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct WebserverService;

#[async_trait]
impl Service for WebserverService {
    fn name(&self) -> &'static str {
        "webserver"
    }

    fn priority(&self) -> i32 {
        30
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Webserver MUST have no dependencies so it can start during pre-initialization
        // (before credentials validation when INITIALIZATION_COMPLETE is false)
        vec![]
    }

    fn is_enabled(&self) -> bool {
        // Webserver is ALWAYS enabled (even before initialization)
        // This allows users to access the initialization dialog
        true
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Enable demo mode if --dashboard-demo flag is present
        if crate::arguments::is_dashboard_demo_enabled() {
            crate::webserver::demo::enable_demo_mode();
            logger::info(
                LogTag::Webserver,
                "Dashboard demo mode enabled for screenshots",
            );
        }
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        logger::debug(
            LogTag::System,
            "[PRE-FLIGHT] Webserver service start() called",
        );

        // Get CLI overrides from arguments module
        let port_override = crate::arguments::get_port_override();
        let host_override = crate::arguments::get_host_override();

        logger::debug(
            LogTag::System,
            &format!(
                "[PRE-FLIGHT] CLI overrides: port={:?}, host={:?}",
                port_override, host_override
            ),
        );

        // Check GUI mode state before pre-flight
        let is_gui = crate::global::is_gui_mode();
        logger::debug(
            LogTag::System,
            &format!("[PRE-FLIGHT] GUI mode state: {}", is_gui),
        );

        // Pre-flight check: test port binding BEFORE spawning background task
        // This ensures any binding errors are caught and propagated to ServiceManager,
        // which will stop bot initialization immediately (no silent failures)
        logger::debug(
            LogTag::System,
            "[PRE-FLIGHT] Calling test_port_binding()...",
        );

        let test_result =
            crate::webserver::test_port_binding(port_override, host_override.clone()).await;

        logger::debug(
            LogTag::System,
            &format!(
                "[PRE-FLIGHT] test_port_binding() returned: {:?}",
                test_result
            ),
        );

        if let Err(e) = test_result {
            logger::error(
                LogTag::System,
                &format!(
                    "[PRE-FLIGHT] ❌ FAILED - Webserver pre-flight check failed: {}",
                    e
                ),
            );
            return Err(format!("Failed to bind webserver port: {}", e));
        }

        logger::debug(
            LogTag::System,
            "[PRE-FLIGHT] ✅ PASSED - Pre-flight check succeeded",
        );

        // Pre-flight passed, spawn background task with CLI overrides
        logger::debug(
            LogTag::System,
            "[PRE-FLIGHT] Spawning background webserver task...",
        );

        let handle = tokio::spawn(monitor.instrument(async move {
            logger::debug(
                LogTag::System,
                "[WEBSERVER] Background task started, calling start_server()...",
            );

            if let Err(e) = crate::webserver::start_server(port_override, host_override).await {
                logger::error(
                    LogTag::System,
                    &format!("[WEBSERVER] ❌ start_server() FAILED: {}", e),
                );
            } else {
                logger::debug(
                    LogTag::System,
                    "[WEBSERVER] ✅ start_server() completed successfully",
                );
            }
        }));

        // Brief delay to let server initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Get actual configured host and port (not the defaults)
        let host = crate::global::get_webserver_host();
        let port = crate::global::get_webserver_port();

        log_service_notice(
            self.name(),
            "ready",
            Some(&format!(
                "endpoint=http://{}:{}",
                if host.is_empty() {
                    crate::webserver::DEFAULT_HOST
                } else {
                    &host
                },
                if port == 0 {
                    crate::webserver::DEFAULT_PORT
                } else {
                    port
                }
            )),
            true,
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
