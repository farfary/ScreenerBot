//! Wallet watch service — the observation layer `TransactionsService` and the
//! Telegram alert consumer both depend on: WebSocket + poll fallback + gap-fill
//! detection for the own wallet and every watched target, feeding one shared
//! `WalletActivity` broadcast (see `wallets::watch`).

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::services::{Service, ServiceHealth, ServiceMetrics};

pub struct WalletWatchService;

#[async_trait]
impl Service for WalletWatchService {
    fn name(&self) -> &'static str {
        "wallet_watch"
    }

    fn priority(&self) -> i32 {
        // Immediately after `transactions` (80): decode/persistence requires the
        // global TransactionDatabase initialized by that service. Transactions'
        // consumer loop is spawned before this producer starts, so no broadcast is
        // lost during normal ordered startup.
        81
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["connectivity", "transactions"]
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
        _monitor: tokio_metrics::TaskMonitor,
    ) -> crate::Result<Vec<JoinHandle<()>>> {
        let handle = crate::wallets::watch::start(shutdown).await.map_err(|e| {
            crate::Error::Service(crate::errors::ServiceError::Start {
                service: "wallet_watch".to_owned(),
                message: e,
            })
        })?;

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::wallets::watch::is_healthy() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded("Detection running on polling alone".to_owned())
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
