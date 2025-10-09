use crate::services::{Service, ServiceHealth, ServiceMetrics};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

pub struct TokenMonitoringService;

#[async_trait]
impl Service for TokenMonitoringService {
    fn name(&self) -> &'static str {
        "token_monitoring"
    }

    fn priority(&self) -> i32 {
        25 // Start before webserver (30) to pre-warm cache
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["transactions"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        use crate::logger::{log, LogTag};
        use crate::tokens::prewarm_summary_cache;

        let start = std::time::Instant::now();
        match prewarm_summary_cache().await {
            Ok(count) => {
                log(
                    LogTag::Monitor,
                    "CACHE_PREWARM",
                    &format!(
                        "✅ Pre-warmed cache with {} tokens in {:?}",
                        count,
                        start.elapsed()
                    ),
                );
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "WARN",
                    &format!("Cache pre-warm failed (will rely on monitor): {}", e),
                );
            }
        }

        Ok(())
    }

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // Start token monitoring task
        let handle = crate::tokens::monitor::start_token_monitoring(shutdown, monitor)
            .await
            .map_err(|e| format!("Failed to start token monitoring: {}", e))?;

        Ok(vec![handle])
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    async fn health(&self) -> ServiceHealth {
        // Token monitoring is healthy if tokens system is ready
        if crate::global::TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Degraded("Tokens system not yet ready".to_string())
        }
    }
}
