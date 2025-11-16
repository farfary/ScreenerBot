use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct WalletService;

impl Default for WalletService {
    fn default() -> Self {
        Self
    }
}

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

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
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

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();
        
        let (operations, errors, snapshots_taken, flow_syncs) = crate::wallet::get_wallet_service_metrics();
        metrics.operations_total = operations;
        metrics.errors_total = errors;
        metrics.custom_metrics.insert("snapshots_taken".to_string(), snapshots_taken as f64);
        metrics.custom_metrics.insert("flow_syncs".to_string(), flow_syncs as f64);
        
        metrics
    }
}
