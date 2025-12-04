use crate::config;
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct EventsService;

/// Check if events are enabled in config (safe to call even when config not loaded)
fn is_events_enabled_in_config() -> bool {
    // MUST check initialization first - config may not be loaded yet
    if !crate::global::is_initialization_complete() {
        return false;
    }
    config::with_config(|c| c.events.enabled)
}

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
        // Events system requires both initialization complete AND config enabled
        // MUST check is_initialization_complete() FIRST to avoid panic when config not loaded
        is_events_enabled_in_config()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Check if events are enabled before initializing
        if !is_events_enabled_in_config() {
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
        if !is_events_enabled_in_config() {
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
        if !is_events_enabled_in_config() {
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
