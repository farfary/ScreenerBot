use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

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
        vec![]
    }

    fn is_enabled(&self) -> bool {
        crate::config::with_config(|cfg| cfg.webserver.enabled)
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing webserver...");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "🌐 Starting webserver dashboard...");

        // Get webserver config from main config system
        let webserver_config = crate::config::with_config(|cfg| cfg.webserver.clone());

        let host = webserver_config.host.clone();
        let port = webserver_config.port;

        let handle = tokio::spawn(
            monitor.instrument(async move {
                if let Err(e) = crate::webserver::start_server(webserver_config).await {
                    log(LogTag::System, "ERROR", &format!("Webserver failed to start: {}", e));
                }
            })
        );

        // Brief delay to let server initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        log(
            LogTag::System,
            "SUCCESS",
            &format!("✅ Webserver started (instrumented) on http://{}:{}", host, port)
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
