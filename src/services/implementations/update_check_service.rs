//! Update Check Service
//!
//! Periodically checks for application updates from the screenerbot.io API.
//! Runs in the background and notifies users when updates are available.

use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct UpdateCheckService;

#[async_trait]
impl Service for UpdateCheckService {
    fn name(&self) -> &'static str {
        "update_check"
    }

    fn priority(&self) -> i32 {
        // Low priority - runs after all core services are started
        10
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // No dependencies - runs independently
        vec![]
    }

    fn is_enabled(&self) -> bool {
        // Only run when fully initialized
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = crate::version::start_update_check_service(shutdown, monitor);
        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        // Update check is healthy if we can determine update availability
        // (either true or false - we just need to have checked)
        ServiceHealth::Healthy
    }
}
