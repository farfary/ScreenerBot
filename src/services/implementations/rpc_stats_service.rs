use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

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
        log(LogTag::System, "INFO", "Initializing RPC stats service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting RPC stats auto-save...");

        let handle = tokio::spawn(async move {
            crate::rpc::start_rpc_stats_auto_save_service(shutdown).await;
        });

        log(LogTag::System, "SUCCESS", "✅ RPC stats service started");

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
