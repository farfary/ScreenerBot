use crate::logger::{log, LogTag};
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
        vec!["filtering"]
    }

    fn is_enabled(&self) -> bool {
        crate::config::with_config(|cfg| cfg.webserver.enabled)
    }

    async fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Get webserver config from main config system
        let webserver_config = crate::config::with_config(|cfg| cfg.webserver.clone());

        let host = webserver_config.host.clone();
        let port = webserver_config.port;

        let handle = tokio::spawn(monitor.instrument(async move {
            if let Err(e) = crate::webserver::start_server(webserver_config).await {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Webserver failed to start: {}", e),
                );
            }
        }));

        // Brief delay to let server initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        log_service_notice(
            self.name(),
            "ready",
            Some(&format!("endpoint=http://{}:{}", host, port)),
            true,
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
