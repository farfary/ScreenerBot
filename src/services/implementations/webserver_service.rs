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
            logger::info(LogTag::Webserver, "📸 Dashboard demo mode enabled for screenshots");
        }
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = tokio::spawn(monitor.instrument(async move {
            if let Err(e) = crate::webserver::start_server().await {
                logger::error(LogTag::System, &format!("Webserver failed to start: {}", e));
            }
        }));

        // Brief delay to let server initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        log_service_notice(
            self.name(),
            "ready",
            Some(&format!(
                "endpoint=http://{}:{}",
                crate::webserver::DEFAULT_HOST,
                crate::webserver::DEFAULT_PORT
            )),
            true,
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
