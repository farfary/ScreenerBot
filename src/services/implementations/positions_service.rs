use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

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
        log(LogTag::System, "INFO", "Initializing positions system...");

        // Positions system initialization happens in start
        log(LogTag::System, "SUCCESS", "Positions system initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting positions manager service...");

        let handle = crate::positions
            ::start_positions_manager_service(shutdown.clone()).await
            .map_err(|e| format!("Failed to start positions service: {}", e))?;

        log(LogTag::System, "SUCCESS", "✅ Positions service started (1 handle)");

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
