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
        25 // Start before webserver (30) to pre-warm cache
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Don't block startup - cache pre-warming will happen in background task
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        use crate::logger::{log, LogTag};

        let mut handles = Vec::new();

        // TokenStore is already pre-warmed by TokenStoreService (priority 15)
        // No need for duplicate cache pre-warming here
        log(
            LogTag::Monitor,
            "TOKEN_MONITORING",
            "Token monitoring service started (TokenStore already hydrated by TokenStoreService)",
        );

        // Start token monitoring task
        let monitoring_handle = crate::tokens::monitor::start_token_monitoring(shutdown, monitor)
            .await
            .map_err(|e| format!("Failed to start token monitoring: {}", e))?;
        handles.push(monitoring_handle);

        Ok(handles)
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
