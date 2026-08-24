//! Pools parent service — initializes pool components and runs helper background tasks.

use crate::logger::{self, LogTag};
use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct PoolsService;

#[async_trait]
impl Service for PoolsService {
    fn name(&self) -> &'static str {
        "pools"
    }

    fn priority(&self) -> i32 {
        30 // Before pool sub-services (31-34) - must initialize components first
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Only depends on transactions (components need to be initialized before sub-services start)
        vec!["transactions"]
    }

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    async fn initialize(&mut self) -> crate::Result<()> {
        logger::info(LogTag::PoolService, "Initializing pool components...");

        // Initialize all pool components (database, cache, RPC, components),
        // selecting the Solana runtime as the concrete implementation.
        // SEAM: crate::pools still constrains this closure to Result<u32,
        // String>; removed when it migrates.
        crate::pools::initialize_pool_components(|| async {
            crate::chains::solana::pools::service::initialize_components()
                .await
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| {
            crate::Error::Service(crate::errors::ServiceError::Initialize {
                service: "pools".to_owned(),
                message: format!("Failed to initialize pool components: {:?}", e),
            })
        })?;

        logger::info(LogTag::PoolService, "Pool components initialized");
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        logger::info(LogTag::PoolService, "Starting pool helper tasks...");

        // Start helper background tasks (health monitor, database cleanup, gap cleanup)
        // Note: Main pool tasks (discovery, fetcher, calculator, analyzer) are started by separate services
        let handles = crate::pools::start_helper_tasks(shutdown, monitor).await;

        logger::info(
            LogTag::PoolService,
            &format!(
                "Pool helper tasks started ({} instrumented handles)",
                handles.len()
            ),
        );

        // Return handles so ServiceManager can wait for graceful shutdown
        Ok(handles)
    }

    async fn stop(&mut self) -> crate::Result<()> {
        logger::info(LogTag::PoolService, "Stopping pool service...");

        // Stop pool service gracefully, releasing the Solana runtime components.
        crate::pools::stop_pool_service(5, crate::chains::solana::pools::service::clear_components)
            .await
            .map_err(|e| {
                crate::Error::Service(crate::errors::ServiceError::Stop {
                    service: "pools".to_owned(),
                    message: format!("Failed to stop pool service: {:?}", e),
                })
            })?;

        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if crate::pools::is_pool_service_running() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Unhealthy("Pool service not running".to_owned())
        }
    }
}
