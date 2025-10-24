use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolFetcherService;

#[async_trait]
impl Service for PoolFetcherService {
    fn name(&self) -> &'static str {
        "pool_fetcher"
    }

    fn priority(&self) -> i32 {
        101
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![
            "transactions",
            "pool_helpers",
            "pool_discovery",
            "filtering",
        ]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        logger::info(LogTag::PoolService, &"Initializing pool fetcher service...".to_string());
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
    logger::info(LogTag::PoolService, &"Starting pool fetcher service...".to_string());

        // Get the AccountFetcher component from global state
        let fetcher = crate::pools::get_account_fetcher()
            .ok_or("AccountFetcher component not initialized".to_string())?;

        // Spawn fetcher task
        let handle = tokio::spawn(monitor.instrument(async move {
            fetcher.start_fetcher_task(shutdown).await;
        }));

    logger::info(LogTag::PoolService, &"✅ Pool fetcher service started (instrumented)".to_string());

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(
            LogTag::PoolService,
            &"Pool fetcher service stopping (via shutdown signal)".to_string(),
        );
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
