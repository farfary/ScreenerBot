//! Pool discovery service — discovers new liquidity pools from on-chain transactions.

use crate::errors::ServiceError;
use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolDiscoveryService;

#[async_trait]
impl Service for PoolDiscoveryService {
    fn name(&self) -> &'static str {
        "pool_discovery"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions", "pools", "filtering"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::debug(
            LogTag::PoolService,
            "Initializing pool discovery service...",
        );
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::debug(LogTag::PoolService, "Starting pool discovery service...");

        // Get the PoolDiscovery component from global state
        let discovery =
            crate::chains::solana::pools::service::get_pool_discovery().ok_or_else(|| {
                crate::Error::Service(ServiceError::Start {
                    service: self.name().to_string(),
                    message: "PoolDiscovery component not initialized".to_owned(),
                })
            })?;

        // Spawn discovery task (instrumented) - component tracks its own metrics
        let handle = tokio::spawn(monitor.instrument(async move {
            discovery.start_discovery_task(shutdown).await;
        }));

        logger::info(
            LogTag::PoolService,
            "Pool discovery service started (instrumented)",
        );

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::debug(
            LogTag::PoolService,
            "Pool discovery service stopping (via shutdown signal)",
        );
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::chains::solana::pools::service::get_pool_discovery().is_some() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("PoolDiscovery component not available".to_owned())
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();

        // Get metrics from the component if available
        if let Some(discovery) = crate::chains::solana::pools::service::get_pool_discovery() {
            let (operations, errors, pools_discovered) = discovery.get_metrics();
            metrics.operations_total = operations;
            metrics.errors_total = errors;
            metrics
                .custom_metrics
                .insert("pools_discovered".to_owned(), pools_discovered as f64);
            if operations > 0 {
                metrics.custom_metrics.insert(
                    "avg_pools_per_cycle".to_owned(),
                    pools_discovered as f64 / operations as f64,
                );
            }
        }

        metrics
    }
}
