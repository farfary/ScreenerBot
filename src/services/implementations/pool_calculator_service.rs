//! Pool calculator service — computes token prices from pool reserves.

use crate::errors::ServiceError;
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
        vec!["pools", "pool_fetcher", "filtering"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::info(
            LogTag::PoolService,
            &"Initializing pool calculator service...".to_owned(),
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::info(
            LogTag::PoolService,
            &"Starting pool calculator service...".to_owned(),
        );

        // Get the PriceCalculator component from global state
        let calculator = crate::pools::get_price_calculator().ok_or_else(|| {
            crate::Error::Service(ServiceError::Start {
                service: self.name().to_string(),
                message: "PriceCalculator component not initialized".to_owned(),
            })
        })?;

        // Spawn calculator task
        let handle = tokio::spawn(monitor.instrument(async move {
            calculator.start_calculator_task(shutdown).await;
        }));

        logger::info(
            LogTag::PoolService,
            &"Pool calculator service started (instrumented)".to_owned(),
        );

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(
            LogTag::PoolService,
            &"Pool calculator service stopping (via shutdown signal)".to_owned(),
        );
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::get_price_calculator().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("PriceCalculator component not available".to_owned())
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();

        // Get metrics from the component if available
        if let Some(calculator) = crate::pools::get_price_calculator() {
            let (operations, errors, prices_calculated) = calculator.get_metrics();
            metrics.operations_total = operations;
            metrics.errors_total = errors;
            metrics
                .custom_metrics
                .insert("prices_calculated".to_owned(), prices_calculated as f64);
            if operations > 0 {
                metrics.custom_metrics.insert(
                    "success_rate".to_owned(),
                    (prices_calculated as f64 / operations as f64) * 100.0,
                );
            }
        }

        metrics
    }
}
