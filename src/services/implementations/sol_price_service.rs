use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

pub struct SolPriceService;

#[async_trait]
impl Service for SolPriceService {
    fn name(&self) -> &'static str {
        "sol_price"
    }

    fn priority(&self) -> i32 {
        120
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec![]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing SOL price service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting SOL price tracking...");

        let handle = crate::sol_price
            ::start_sol_price_service(shutdown.clone()).await
            .map_err(|e| format!("Failed to start SOL price service: {}", e))?;

        log(LogTag::System, "SUCCESS", "✅ SOL price service started (1 handle)");

        // Return price_task handle so ServiceManager can wait for graceful shutdown
        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
