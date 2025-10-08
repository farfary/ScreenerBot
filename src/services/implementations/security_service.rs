use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct SecurityService;

#[async_trait]
impl Service for SecurityService {
    fn name(&self) -> &'static str {
        "security"
    }

    fn priority(&self) -> i32 {
        70
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Initialize security analyzer (not async)
        crate::tokens::security::initialize_security_analyzer()
            .map_err(|e| format!("Failed to initialize security analyzer: {}", e))?;
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = tokio::spawn(monitor.instrument(async move {
            crate::tokens::security::start_security_monitoring(shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::global::SECURITY_ANALYZER_READY.load(std::sync::atomic::Ordering::Relaxed) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }
}
