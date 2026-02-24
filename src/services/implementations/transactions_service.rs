//! Transactions service — monitors on-chain transactions for the configured wallet.

use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use solana_sdk::signer::Signer;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct TransactionsService;

#[async_trait]
impl Service for TransactionsService {
    fn name(&self) -> &'static str {
        "transactions"
    }

    fn priority(&self) -> i32 {
        80
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
        // Get wallet pubkey from config
        let wallet_pubkey = crate::config::get_wallet_pubkey().map_err(|e| {
            crate::Error::Service(crate::errors::ServiceError::Start {
                service: "transactions".to_string(),
                message: format!("Failed to load wallet: {e}"),
            })
        })?;

        // Start global transaction service and capture handle (passing monitor)
        let handle =
            crate::transactions::service::start_global_transaction_service(wallet_pubkey, monitor)
                .await
                .map_err(|e| {
                    crate::Error::Service(crate::errors::ServiceError::Start {
                        service: "transactions".to_string(),
                        message: format!("Failed to start transactions service: {e}"),
                    })
                })?;

        // Return service handle so ServiceManager can wait for graceful shutdown
        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        if crate::global::TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Starting
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        // Get global transaction manager
        if let Some(manager_arc) = crate::transactions::get_global_transaction_manager().await {
            // Try to lock and get metrics (non-blocking)
            if let Ok(manager) = manager_arc.try_lock() {
                return manager.metrics();
            }
        }

        // Return default if manager not available or locked
        ServiceMetrics::default()
    }
}
