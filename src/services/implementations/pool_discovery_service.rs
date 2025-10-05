use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

pub struct PoolDiscoveryService;

#[async_trait]
impl Service for PoolDiscoveryService {
    fn name(&self) -> &'static str {
        "pool_discovery"
    }

    fn priority(&self) -> i32 {
        31
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Initializing pool discovery service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::PoolService, "INFO", "Starting pool discovery service...");

        // Get the PoolDiscovery component from global state
        let discovery = crate::pools
            ::get_pool_discovery()
            .ok_or("PoolDiscovery component not initialized".to_string())?;

        // Spawn discovery task
        let handle = tokio::spawn(async move {
            discovery.start_discovery_task(shutdown).await;
        });

        log(LogTag::PoolService, "SUCCESS", "✅ Pool discovery service started");

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Pool discovery service stopping (via shutdown signal)");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::get_pool_discovery().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("PoolDiscovery component not available".to_string())
        }
    }
}
