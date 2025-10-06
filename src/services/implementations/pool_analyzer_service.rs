use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

pub struct PoolAnalyzerService;

#[async_trait]
impl Service for PoolAnalyzerService {
    fn name(&self) -> &'static str {
        "pool_analyzer"
    }

    fn priority(&self) -> i32 {
        34
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["pool_helpers", "pool_fetcher"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Initializing pool analyzer service...");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::PoolService, "INFO", "Starting pool analyzer service...");

        // Get the PoolAnalyzer component from global state
        let analyzer = crate::pools
            ::get_pool_analyzer()
            .ok_or("PoolAnalyzer component not initialized".to_string())?;

        // Spawn analyzer task
        let handle = tokio::spawn(
            monitor.instrument(async move {
                analyzer.start_analyzer_task(shutdown).await;
            })
        );

        log(LogTag::PoolService, "SUCCESS", "✅ Pool analyzer service started (instrumented)");

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(LogTag::PoolService, "INFO", "Pool analyzer service stopping (via shutdown signal)");
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::get_pool_analyzer().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("PoolAnalyzer component not available".to_string())
        }
    }
}
