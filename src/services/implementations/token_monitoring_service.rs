use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct TokenMonitoringService;

#[async_trait]
impl Service for TokenMonitoringService {
    fn name(&self) -> &'static str {
        "token_monitoring"
    }

    fn priority(&self) -> i32 {
        42
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Start token monitoring task
        let handle = crate::tokens::monitor::start_token_monitoring(shutdown, monitor)
            .await
            .map_err(|e| format!("Failed to start token monitoring: {}", e))?;

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Token monitoring is healthy if tokens system is ready
        if crate::global::TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded("Tokens system not yet ready".to_string())
        }
    }
}
