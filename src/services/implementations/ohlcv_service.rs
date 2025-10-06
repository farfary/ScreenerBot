use crate::logger::{ log, LogTag };
use crate::services::{ Service, ServiceHealth };
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

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

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor
    ) -> Result<Vec<JoinHandle<()>>, String> {
        log(LogTag::System, "INFO", "Starting OHLCV monitoring...");

        // Get cloned service for background monitoring
        let service = crate::tokens::ohlcvs
            ::get_ohlcv_service_clone().await
            .map_err(|e| format!("Failed to get OHLCV service: {}", e))?;

        // Start background monitoring
        service.start_monitoring(shutdown.clone()).await;

        // Auto-populate watch list with open positions for immediate data collection
        let service_clone = service.clone();
        tokio::spawn(async move {
            // Wait a moment for other services to be ready
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            log(LogTag::Ohlcv, "AUTO_POPULATE", "Adding open positions to OHLCV watch list...");

            // Add all open positions to watch list
            let open_positions = crate::positions::get_open_positions().await;
            for position in &open_positions {
                service_clone.add_to_watch_list(&position.mint, true).await;
            }
            log(
                LogTag::Ohlcv,
                "AUTO_POPULATE_DONE",
                &format!("✅ Added {} open positions to OHLCV watch list", open_positions.len())
            );
        });

        // Create task handle for lifecycle tracking
        let monitor_handle = tokio::spawn(
            monitor.instrument(async move {
                log(LogTag::Ohlcv, "TASK_START", "🚀 OHLCV monitoring task started (instrumented)");
                shutdown.notified().await;
                log(LogTag::Ohlcv, "TASK_END", "✅ OHLCV monitoring task ended");
            })
        );

        log(LogTag::System, "SUCCESS", "✅ OHLCV monitoring started (instrumented)");

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
