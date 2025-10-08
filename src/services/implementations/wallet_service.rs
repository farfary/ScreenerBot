use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct WalletService;

#[async_trait]
impl Service for WalletService {
    fn name(&self) -> &'static str {
        "wallet"
    }

    fn priority(&self) -> i32 {
        90
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
        let handle = crate::wallet::start_wallet_monitoring_service(shutdown, monitor).await;

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
