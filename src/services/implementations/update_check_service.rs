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

    async fn initialize(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = crate::version::start_update_check_service(shutdown, monitor);
        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        let state = crate::version::get_update_state().await;
        if let Some(error) = state.check_error {
            ServiceHealth::Degraded(error)
        } else if state.last_check.is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }
}
