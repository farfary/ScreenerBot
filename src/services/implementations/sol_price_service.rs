use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        let handle = crate::sol_price
            ::start_sol_price_service(shutdown.clone(), monitor).await
            .map_err(|e| format!("Failed to start SOL price service: {}", e))?;

        // Return price_task handle so ServiceManager can wait for graceful shutdown
        Ok(vec![handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
