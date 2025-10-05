use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

pub struct AtaCleanupService;

#[async_trait]
impl Service for AtaCleanupService {
    fn name(&self) -> &'static str {
        "ata_cleanup"
    }

    fn priority(&self) -> i32 {
        110
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing ATA cleanup service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting ATA cleanup service...");

        let handle = tokio::spawn(async move {
            crate::ata_cleanup::start_ata_cleanup_service(shutdown).await;
        });

        log(LogTag::System, "SUCCESS", "✅ ATA cleanup service started");

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
