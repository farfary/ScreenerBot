use crate::arguments::is_debug_pool_service_enabled;
use crate::logger::{log, LogTag};
use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolsService;

#[async_trait]
impl Service for PoolsService {
    fn name(&self) -> &'static str {
        "pool_helpers"
    }

    fn priority(&self) -> i32 {
        30 // Before pool sub-services (31-34) - must initialize components first
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Only depends on transactions (components need to be initialized before sub-services start)
        vec!["transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "INFO",
                "Initializing pool components...",
            );
        }

        // Initialize all pool components (database, cache, RPC, components)
        crate::pools::initialize_pool_components()
            .await
            .map_err(|e| format!("Failed to initialize pool components: {:?}", e))?;

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "SUCCESS",
                "✅ Pool components initialized",
            );
        }
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        if is_debug_pool_service_enabled() {
            log(LogTag::PoolService, "INFO", "Starting pool helper tasks...");
        }

        // Start helper background tasks (health monitor, database cleanup, gap cleanup)
        // Note: Main pool tasks (discovery, fetcher, calculator, analyzer) are started by separate services
        let handles = crate::pools::start_helper_tasks(shutdown, monitor).await;

        if is_debug_pool_service_enabled() {
            log(
                LogTag::PoolService,
                "SUCCESS",
                &format!(
                    "✅ Pool helper tasks started ({} instrumented handles)",
                    handles.len()
                ),
            );
        }

        // Return handles so ServiceManager can wait for graceful shutdown
        Ok(handles)
    }

    async fn stop(&mut self) -> Result<(), String> {
        if is_debug_pool_service_enabled() {
            log(LogTag::PoolService, "INFO", "Stopping pool service...");
        }

        // Stop pool service gracefully
        crate::pools::stop_pool_service(5)
            .await
            .map_err(|e| format!("Failed to stop pool service: {:?}", e))?;

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::is_pool_service_running() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("Pool service not running".to_string())
        }
    }
}
