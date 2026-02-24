//! Tokens service — orchestrates token discovery, metadata updates, and stale token cleanup.

use crate::errors::ServiceError;
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

    async fn initialize(&mut self) -> crate::Result<()> {
        let mut service = crate::tokens::service::TokensServiceNew::default();
        service.initialize().await?;
        self.orchestrator = Some(service);
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let mut handles = Vec::new();
        let orchestrator = self.orchestrator.as_mut().ok_or_else(|| {
            crate::Error::Service(ServiceError::Start {
                service: "tokens".to_owned(),
                message: "Tokens orchestrator not initialized".to_owned(),
            })
        })?;
        let mut orch_handles = orchestrator.start(shutdown, monitor).await?;
        handles.append(&mut orch_handles);
        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Health check: verify orchestrator exists AND tokens system is ready (update loops started)
        if self.orchestrator.is_some()
            && crate::global::TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        {
            ServiceHealth::Healthy
        } else if self.orchestrator.is_some() {
            ServiceHealth::Starting
        } else {
            ServiceHealth::Starting
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
