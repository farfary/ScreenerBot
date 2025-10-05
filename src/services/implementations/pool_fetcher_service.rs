use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

pub struct PoolFetcherService;

#[async_trait]
impl Service for PoolFetcherService {
    fn name(&self) -> &'static str {
        "pool_fetcher"
    }

    fn priority(&self) -> i32 {
        32
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions", "pool_discovery"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Initializing pool fetcher service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::PoolService, "INFO", "Starting pool fetcher service...");

        // Get the AccountFetcher component from global state
        let fetcher = crate::pools
            ::get_account_fetcher()
            .ok_or("AccountFetcher component not initialized".to_string())?;

        // Spawn fetcher task
        let handle = tokio::spawn(async move {
            fetcher.start_fetcher_task(shutdown).await;
        });

        log(LogTag::PoolService, "SUCCESS", "✅ Pool fetcher service started");

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Pool fetcher service stopping (via shutdown signal)");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::get_account_fetcher().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("AccountFetcher component not available".to_string())
        }
    }
}
