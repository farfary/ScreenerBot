//! Wallet service — monitors wallet balances, token holdings, and flow tracking.

use crate::errors::ServiceError;
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

    fn is_enabled(&self) -> bool {
        crate::global::is_initialization_complete()
    }

    // Opens the wallet-monitor database (and flips it readable-ready) before
    // the service is allowed to start, so the webserver never has to guess
    // whether "no snapshot yet" is startup or a real failure — see
    // `crate::webserver::snapshot::collectors::collect_wallet_snapshot`.
    // `initialize_wallet_database()` is idempotent, so this is the one path
    // even though `start_wallet_monitoring_service` also calls it as a
    // belt-and-suspenders guard for the monitoring loop itself.
    async fn initialize(&mut self) -> crate::Result<()> {
        crate::wallet::initialize_wallet_database()
            .await
            .map_err(|e| {
                crate::Error::Service(ServiceError::Initialize {
                    service: self.name().to_owned(),
                    message: e.to_string(),
                })
            })
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = crate::wallet::start_wallet_monitoring_service(shutdown, monitor).await;

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::wallet::is_wallet_database_ready() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        let mut metrics = ServiceMetrics::default();

        let (operations, errors, snapshots_taken, flow_syncs) =
            crate::wallet::get_wallet_service_metrics();
        metrics.operations_total = operations;
        metrics.errors_total = errors;
        metrics
            .custom_metrics
            .insert("snapshots_taken".to_owned(), snapshots_taken as f64);
        metrics
            .custom_metrics
            .insert("flow_syncs".to_owned(), flow_syncs as f64);

        metrics
    }
}
