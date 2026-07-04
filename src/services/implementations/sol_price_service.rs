//! SOL price service — tracks real-time SOL/USD price for portfolio valuation.

use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct SolPriceService;

#[async_trait]
impl Service for SolPriceService {
    fn name(&self) -> &'static str {
        "sol_price"
    }

    fn priority(&self) -> i32 {
        120
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = crate::sol_price::start_sol_price_service(shutdown.clone(), monitor.clone())
            .await
            .map_err(|e| {
                crate::Error::Service(crate::errors::ServiceError::Start {
                    service: "sol_price".to_owned(),
                    message: format!("Failed to start SOL price service: {e}"),
                })
            })?;

        // Also mirror the full SOL/USD reference chart from the data server so the
        // bot always has SOL's own price history (all timeframes) ready for display
        // and USDC->SOL conversion, prepared during runtime (never per request).
        let chart_handle = crate::ohlcvs::sol_usd_chart::start(shutdown.clone(), monitor.clone());

        // Return both handles so ServiceManager can wait for graceful shutdown.
        Ok(vec![handle, chart_handle])
    }

    async fn health(&self) -> ServiceHealth {
        // Check if service is running
        if !crate::sol_price::is_sol_price_service_running() {
            return ServiceHealth::Unhealthy("SOL price service is not running".to_owned());
        }

        // Check if we have valid cached price data
        match crate::sol_price::get_sol_price_info() {
            Some(info) => {
                if info.is_fresh() {
                    ServiceHealth::Healthy
                } else {
                    ServiceHealth::Degraded(format!(
                        "SOL price data is stale ({}s old)",
                        info.age_seconds()
                    ))
                }
            }
            None => ServiceHealth::Degraded("No SOL price data available yet".to_owned()),
        }
    }
}
