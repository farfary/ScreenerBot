use crate::logger::{log, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolCalculatorService;

#[async_trait]
impl Service for PoolCalculatorService {
    fn name(&self) -> &'static str {
        "pool_calculator"
    }

    fn priority(&self) -> i32 {
        33
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["pool_helpers", "pool_fetcher"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(
            LogTag::PoolService,
            "INFO",
            "Initializing pool calculator service...",
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        log(
            LogTag::PoolService,
            "INFO",
            "Starting pool calculator service...",
        );

        // Get the PriceCalculator component from global state
        let calculator = crate::pools::get_price_calculator()
            .ok_or("PriceCalculator component not initialized".to_string())?;

        // Spawn calculator task
        let handle = tokio::spawn(monitor.instrument(async move {
            calculator.start_calculator_task(shutdown).await;
        }));

        log(
            LogTag::PoolService,
            "SUCCESS",
            "✅ Pool calculator service started (instrumented)",
        );

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        log(
            LogTag::PoolService,
            "INFO",
            "Pool calculator service stopping (via shutdown signal)",
        );
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::get_price_calculator().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("PriceCalculator component not available".to_string())
        }
    }
}
