use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

pub struct PoolsService;

#[async_trait]
impl Service for PoolsService {
    fn name(&self) -> &'static str {
        "pool_helpers"
    }

    fn priority(&self) -> i32 {
        35 // After pool sub-services (31-34)
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Depends on all pool sub-services (starts helper tasks after main tasks running)
        vec!["pool_discovery", "pool_fetcher", "pool_calculator", "pool_analyzer"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Initializing pool components...");

        // Initialize all pool components (database, cache, RPC, components)
        crate::pools
            ::initialize_pool_components().await
            .map_err(|e| format!("Failed to initialize pool components: {:?}", e))?;

        log(LogTag::PoolService, "SUCCESS", "✅ Pool components initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::PoolService, "INFO", "Starting pool helper tasks...");

        // Start helper background tasks (health monitor, database cleanup, gap cleanup)
        // Note: Main pool tasks (discovery, fetcher, calculator, analyzer) are started by separate services
        crate::pools::start_helper_tasks(shutdown).await;

        log(LogTag::PoolService, "SUCCESS", "✅ Pool helper tasks started");

        // Helper tasks are fire-and-forget (no handles returned)
        Ok(vec![])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Stopping pool service...");

        // Stop pool service gracefully
        crate::pools
            ::stop_pool_service(5).await
            .map_err(|e| format!("Failed to stop pool service: {:?}", e))?;

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::is_pool_service_running() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("Pool service not running".to_string())
        }
    }
}
