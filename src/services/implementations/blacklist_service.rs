use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

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
        log(LogTag::System, "INFO", "Initializing blacklist system...");

        // Initialize blacklist system (database and cache)
        if let Err(e) = crate::tokens::blacklist::initialize_blacklist_system() {
            return Err(format!("Failed to initialize blacklist system: {}", e));
        }

        // Initialize system and stable tokens in blacklist
        crate::tokens::blacklist::initialize_system_stable_blacklist();

        log(LogTag::System, "SUCCESS", "Blacklist system initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Blacklist service started");

        // Blacklist system doesn't spawn background tasks
        let handle = tokio::spawn(async move {
            shutdown.notified().await;
        });

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
