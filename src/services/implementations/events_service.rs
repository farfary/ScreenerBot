use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

pub struct EventsService;

#[async_trait]
impl Service for EventsService {
    fn name(&self) -> &'static str {
        "events"
    }

    fn priority(&self) -> i32 {
        10 // Start early, stop late
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![] // No dependencies
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing events system...");

        // Initialize events database and system
        crate::events
            ::init().await
            .map_err(|e| { format!("Failed to initialize events system: {}", e) })?;

        log(LogTag::System, "SUCCESS", "Events system initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting events service (instrumented)...");

        // Events system doesn't spawn background tasks currently
        // Just wait for shutdown signal
        let handle = tokio::spawn(
            monitor.instrument(async move {
                shutdown.notified().await;
            })
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        // TODO: Add actual health check if needed
        ServiceHealth::Healthy
    }
}
