//! RPC stats service — periodically saves RPC call statistics to database.

use crate::services::{Service, ServiceHealth};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct RpcStatsService;

#[async_trait]
impl Service for RpcStatsService {
    fn name(&self) -> &'static str {
        "rpc_stats"
    }

    fn priority(&self) -> i32 {
        100
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
        let handle = tokio::spawn(monitor.instrument(async move {
            crate::rpc::stats::start_rpc_stats_auto_save_service(shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
