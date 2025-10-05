use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::configs::Configs;
use crate::logger::{ log, LogTag };

pub struct TraderService;

#[async_trait]
impl Service for TraderService {
    fn name(&self) -> &'static str {
        "trader"
    }

    fn priority(&self) -> i32 {
        150
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // Depend on actual working services, not empty coordinator services
        vec![
            "positions",
            "pool_discovery",
            "pool_fetcher",
            "pool_calculator",
            "token_discovery",
            "token_monitoring"
        ]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing trader service...");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting trader service...");

        // Start entry monitor
        let shutdown_entry = shutdown.clone();
        let entry_handle = tokio::spawn(async move {
            crate::trader::monitor_new_entries(shutdown_entry).await;
        });

        // Start positions monitor
        let shutdown_positions = shutdown.clone();
        let positions_handle = tokio::spawn(async move {
            crate::trader::monitor_open_positions(shutdown_positions).await;
        });

        log(LogTag::System, "SUCCESS", "✅ Trader service started");

        Ok(vec![entry_handle, positions_handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
