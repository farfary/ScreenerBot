use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PositionsService;

#[async_trait]
impl Service for PositionsService {
    fn name(&self) -> &'static str {
        "positions"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        // Positions system initialization happens in start
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = crate::positions::start_positions_manager_service(shutdown.clone(), monitor)
            .await
            .map_err(|e| format!("Failed to start positions service: {}", e))?;

        // Return verification_worker handle so ServiceManager can wait for graceful shutdown
        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::global::POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }
}
