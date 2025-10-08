use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Start entry monitor (instrumented)
        let shutdown_entry = shutdown.clone();
        let monitor_entry = monitor.clone();
        let entry_handle = tokio::spawn(
            monitor_entry.instrument(async move {
                crate::trader::monitor_new_entries(shutdown_entry).await;
            })
        );

        // Start positions monitor (instrumented)
        let shutdown_positions = shutdown.clone();
        let positions_handle = tokio::spawn(
            monitor.instrument(async move {
                crate::trader::monitor_open_positions(shutdown_positions).await;
            })
        );

        Ok(vec![entry_handle, positions_handle])
    }

    async fn health(&self) -> ServiceHealth {
        ServiceHealth::Healthy
    }
}
