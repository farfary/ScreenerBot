use crate::arguments::is_debug_pool_discovery_enabled;
use crate::logger::{log, LogTag};
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
        vec!["transactions", "pool_helpers", "filtering"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                "Initializing pool discovery service...",
            );
        }
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                "Starting pool discovery service...",
            );
        }

        // Get the PoolDiscovery component from global state
        let discovery = crate::pools::get_pool_discovery()
            .ok_or("PoolDiscovery component not initialized".to_string())?;

        // Spawn discovery task (instrumented)
        let handle = tokio::spawn(monitor.instrument(async move {
            discovery.start_discovery_task(shutdown).await;
        }));

        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolService,
                "SUCCESS",
                "✅ Pool discovery service started (instrumented)",
            );
        }

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        if is_debug_pool_discovery_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                "Pool discovery service stopping (via shutdown signal)",
            );
        }
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
