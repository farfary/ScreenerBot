use crate::logger::{log, LogTag};
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
        log(
            LogTag::System,
            "INFO",
            "Initializing token monitoring service...",
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        log(
            LogTag::System,
            "INFO",
            "Starting token monitoring service...",
        );

        // Start token monitoring task
        let handle = crate::tokens::monitor::start_token_monitoring(shutdown, monitor)
            .await
            .map_err(|e| format!("Failed to start token monitoring: {}", e))?;

        log(
            LogTag::System,
            "SUCCESS",
            "✅ Token monitoring service started (instrumented)",
        );

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(
            LogTag::System,
            "INFO",
            "Token monitoring service stopping (via shutdown signal)",
        );
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
