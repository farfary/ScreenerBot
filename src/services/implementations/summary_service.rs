use crate::arguments::is_summary_enabled;
use crate::services::{ log_service_notice, Service, ServiceHealth, ServiceMetrics };
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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

    fn is_enabled(&self) -> bool {
        // Only enable if --summary flag is present
        is_summary_enabled()
    }

    async fn initialize(&mut self) -> Result<(), String> {
        if is_summary_enabled() {
            log_service_notice(self.name(), "init", Some("interval_secs=15"), true);
        }
        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        if !is_summary_enabled() {
            // Return empty handle if not enabled
            let handle = tokio::spawn(
                monitor.instrument(async move {
                    shutdown.notified().await;
                })
            );
            return Ok(vec![handle]);
        }

        let handle = tokio::spawn(
            monitor.instrument(async move {
                crate::summary::summary_loop(shutdown).await;
            })
        );

        log_service_notice(self.name(), "loop_started", None, true);

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        if is_summary_enabled() {
            log_service_notice(self.name(), "stopped", None, true);
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
