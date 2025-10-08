use crate::services::{ Service, ServiceHealth, ServiceMetrics };
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

    async fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Get wallet pubkey from config
        let wallet_pubkey = crate::config
            ::get_wallet_pubkey()
            .map_err(|e| format!("Failed to load wallet: {}", e))?;

        // Start global transaction service and capture handle (passing monitor)
        let handle = crate::transactions::service
            ::start_global_transaction_service(wallet_pubkey, monitor).await
            .map_err(|e| format!("Failed to start transactions service: {}", e))?;

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
}
