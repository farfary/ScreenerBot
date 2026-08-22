//! ATA (Associated Token Account) cleanup service — reclaims rent from unused token accounts.

use crate::services::{Service, ServiceHealth};
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

    fn is_enabled(&self) -> bool {
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
        let handle = tokio::spawn(monitor.instrument(async move {
            crate::tools::ata_cleanup::start_ata_cleanup_service(shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
