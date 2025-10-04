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
        "pools"
    }

    fn priority(&self) -> i32 {
        60
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["positions", "transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Initializing pool service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::PoolService, "INFO", "Starting pool service...");

        // Start pool service (spawns all background tasks internally)
        crate::pools
            ::start_pool_service().await
            .map_err(|e| format!("Failed to start pool service: {:?}", e))?;

        log(LogTag::PoolService, "SUCCESS", "✅ Pool service started");

        // Pool service manages its own tasks
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
