use crate::logger::{self, LogTag};
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
        102
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["pool_helpers", "pool_fetcher", "filtering"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        logger::info(
            LogTag::PoolService,
            &"Initializing pool calculator service...".to_string(),
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        logger::info(
            LogTag::PoolService,
            &"Starting pool calculator service...".to_string(),
        );

        // Get the PriceCalculator component from global state
        let calculator = crate::pools::get_price_calculator()
            .ok_or("PriceCalculator component not initialized".to_string())?;

        // Spawn calculator task
        let handle = tokio::spawn(monitor.instrument(async move {
            calculator.start_calculator_task(shutdown).await;
        }));

        logger::info(
            LogTag::PoolService,
            &"✅ Pool calculator service started (instrumented)".to_string(),
        );

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        logger::info(
            LogTag::PoolService,
            &"Pool calculator service stopping (via shutdown signal)".to_string(),
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
