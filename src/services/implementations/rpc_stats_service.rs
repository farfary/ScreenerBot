use crate::services::{Service, ServiceHealth, ServiceMetrics};
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

    async fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = tokio::spawn(monitor.instrument(async move {
            crate::rpc::start_rpc_stats_auto_save_service(shutdown).await;
        }));

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
