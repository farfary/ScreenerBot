use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct BlacklistService;

#[async_trait]
impl Service for BlacklistService {
    fn name(&self) -> &'static str {
        "blacklist"
    }

    fn priority(&self) -> i32 {
        20
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Initialize blacklist system (database and cache)
        if let Err(e) = crate::tokens::blacklist::initialize_blacklist_system() {
            return Err(format!("Failed to initialize blacklist system: {}", e));
        }

        // Initialize system and stable tokens in blacklist
        crate::tokens::blacklist::initialize_system_stable_blacklist();
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Blacklist system doesn't spawn background tasks
        let handle = tokio::spawn(
            monitor.instrument(async move {
                shutdown.notified().await;
            })
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
