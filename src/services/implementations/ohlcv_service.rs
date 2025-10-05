use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use crate::services::{ Service, ServiceHealth };
use crate::logger::{ log, LogTag };

/// OHLCV (Open, High, Low, Close, Volume) data collection service
///
/// Manages background monitoring and caching of 1-minute OHLCV data from GeckoTerminal API.
/// Provides efficient data access through SQLite database and in-memory caching.
pub struct OhlcvService;

#[async_trait]
impl Service for OhlcvService {
    fn name(&self) -> &'static str {
        "ohlcv"
    }

    fn priority(&self) -> i32 {
        45
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["tokens"]
    }

    async fn initialize(&mut self) -> Result<(), String> {
        log(LogTag::System, "INFO", "Initializing OHLCV service...");

        // Initialize OHLCV service (creates database, sets up global instance)
        crate::tokens::ohlcvs
            ::init_ohlcv_service().await
            .map_err(|e| format!("Failed to initialize OHLCV service: {}", e))?;

        log(LogTag::System, "SUCCESS", "✅ OHLCV service initialized");
        Ok(())
    }

    async fn start(&mut self, shutdown: Arc<Notify>) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting OHLCV monitoring...");

        // Get cloned service for background monitoring
        let service = crate::tokens::ohlcvs
            ::get_ohlcv_service_clone().await
            .map_err(|e| format!("Failed to get OHLCV service: {}", e))?;

        // Start background monitoring
        service.start_monitoring(shutdown.clone()).await;

        // Create task handle for lifecycle tracking
        let monitor_handle = tokio::spawn(async move {
            log(LogTag::Ohlcv, "TASK_START", "🚀 OHLCV monitoring task started");
            shutdown.notified().await;
            log(LogTag::Ohlcv, "TASK_END", "✅ OHLCV monitoring task ended");
        });

        log(LogTag::System, "SUCCESS", "✅ OHLCV monitoring started");

        Ok(vec![monitor_handle])
    }

    async fn health(&self) -> ServiceHealth {
        // Check if OHLCV service is initialized and operational
        match crate::tokens::ohlcvs::get_ohlcv_service_clone().await {
            Ok(service) => {
                let stats = service.get_stats().await;
                if stats.watched_tokens > 0 || stats.total_api_calls > 0 {
                    ServiceHealth::Healthy
                } else {
                    ServiceHealth::Starting
                }
            }
            Err(_) => ServiceHealth::Unhealthy("OHLCV service not initialized".to_string()),
        }
    }
}
