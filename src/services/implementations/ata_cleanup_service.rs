use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = tokio::spawn(
            monitor.instrument(async move {
                crate::ata_cleanup::start_ata_cleanup_service(shutdown).await;
            })
        );

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
