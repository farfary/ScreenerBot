use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth, ServiceMetrics };
use crate::logger::{ log, LogTag };
use crate::arguments::is_summary_enabled;

pub struct SummaryService;

#[async_trait]
impl Service for SummaryService {
    fn name(&self) -> &'static str {
        "summary"
    }

    fn priority(&self) -> i32 {
        150 // After Trader (140) - displays trader results
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["positions", "transactions", "tokens", "wallet"]
    }

    fn is_enabled(&self, _config: &crate::configs::Configs) -> bool {
        // Only enable if --summary flag is present
        is_summary_enabled()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        if is_summary_enabled() {
            log(LogTag::System, "INFO", "Initializing summary service...");
            log(LogTag::System, "INFO", "Summary will display positions every 15 seconds");
        }
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        if !is_summary_enabled() {
            // Return empty handle if not enabled
            let handle = tokio::spawn(async move {
                shutdown.notified().await;
            });
            return Ok(vec![handle]);
        }

        log(LogTag::System, "INFO", "Starting summary display...");

        let handle = tokio::spawn(async move {
            crate::summary::summary_loop(shutdown).await;
        });

        log(LogTag::System, "SUCCESS", "✅ Summary service started");

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        if is_summary_enabled() {
            log(LogTag::System, "INFO", "Summary service stopped");
        }
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        if is_summary_enabled() {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Healthy // Healthy even when disabled
        }
    }

    async fn metrics(&self) -> ServiceMetrics {
        ServiceMetrics::default()
    }
}
