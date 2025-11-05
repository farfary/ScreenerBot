use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Centralized tokens service that delegates all token background logic
/// to the tokens module orchestrator (new architecture).
pub struct TokensService {
    orchestrator: Option<crate::tokens::service::TokensServiceNew>,
}

impl Default for TokensService {
    fn default() -> Self {
        Self { orchestrator: None }
    }
}

#[async_trait]
impl Service for TokensService {
    fn name(&self) -> &'static str {
        "tokens"
    }

    fn priority(&self) -> i32 {
        40 // Before webserver and trader; after core infra
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["events", "transactions", "pools"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        let mut service = crate::tokens::service::TokensServiceNew::default();
        service.initialize().await?;
        self.orchestrator = Some(service);
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let mut handles = Vec::new();
        if let Some(orchestrator) = &mut self.orchestrator {
            let mut orch_handles = orchestrator.start(shutdown, monitor).await?;
            handles.append(&mut orch_handles);
        } else {
            return Err("Tokens orchestrator not initialized".into());
        }
        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Basic health: tokens system considered healthy if orchestrator exists
        if self.orchestrator.is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
