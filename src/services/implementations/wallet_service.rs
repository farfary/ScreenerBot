use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };

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
        log(LogTag::System, "INFO", "Initializing wallet monitoring...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting wallet monitoring...");

        let handle = crate::wallet::start_wallet_monitoring_service(shutdown).await;

        log(LogTag::System, "SUCCESS", "✅ Wallet service started");

        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
