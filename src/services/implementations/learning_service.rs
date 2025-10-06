use crate::logger::{log, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct LearningService;

#[async_trait]
impl Service for LearningService {
    fn name(&self) -> &'static str {
        "learning"
    }

    fn priority(&self) -> i32 {
        130
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing learning system...");

        // Initialize learning system
        crate::learner::initialize_learning_system()
            .await
            .map_err(|e| format!("Failed to initialize learning system: {}", e))?;

        log(LogTag::System, "SUCCESS", "Learning system initialized");
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
            "Learning service started (instrumented)",
        );

        // Learning system doesn't spawn background tasks currently
        let handle = tokio::spawn(monitor.instrument(async move {
            shutdown.notified().await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
