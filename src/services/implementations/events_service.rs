use crate::config;
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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

    fn is_enabled(&self) -> bool {
        // Events system must be explicitly enabled in config
        config::with_config(|c| c.events.enabled) && crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Check if events are enabled before initializing
        if !config::with_config(|c| c.events.enabled) {
            crate::logger::info(
                crate::logger::LogTag::System,
                "Events system disabled in config - skipping initialization",
            );
            return Ok(());
        }

        // Initialize events database and system
        crate::events::init()
            .await
            .map_err(|e| format!("Failed to initialize events system: {}", e))?;
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // If events are disabled, just wait for shutdown
        if !config::with_config(|c| c.events.enabled) {
            let handle = tokio::spawn(monitor.instrument(async move {
                shutdown.notified().await;
            }));
            return Ok(vec![handle]);
        }

        // Start maintenance task
        crate::events::start_maintenance_task().await;

        // Wait for shutdown signal
        let handle = tokio::spawn(monitor.instrument(async move {
            shutdown.notified().await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        // If disabled, report as healthy (not an error)
        if !config::with_config(|c| c.events.enabled) {
            return ServiceHealth::Healthy;
        }

        // Check if events DB is initialized
        if crate::events::EVENTS_DB.get().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("Events database not initialized".to_string())
        }
    }
}
