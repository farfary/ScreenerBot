//! Trader service implementation

use crate::logger::{log, LogTag};
use crate::trader::auto;
use crate::trader::config;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct TraderService {
    shutdown_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<bool>>>>,
}

impl TraderService {
    pub fn new() -> Self {
        Self {
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }
}

impl Default for TraderService {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Implement Service trait when services module exports are fixed
// For now, this is a stub implementation

impl TraderService {
    pub async fn start(&self) -> Result<(), String> {
        log(LogTag::Trader, "INFO", "Starting Trader Service...");

        // Initialize trader system
        super::init_trader_system().await?;

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            log(
                LogTag::Trader,
                "INFO",
                "Trader service started but trading is disabled in config",
            );
        }

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Start auto trading monitors
        let _auto_task = tokio::spawn(async move {
            if let Err(e) = auto::start_auto_trading(shutdown_rx).await {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Auto trading error: {}", e),
                );
            }
        });

        log(LogTag::Trader, "INFO", "Trader Service started successfully");
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        log(LogTag::Trader, "INFO", "Stopping Trader Service...");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(true);
        }

        // Wait a moment for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        log(LogTag::Trader, "INFO", "Trader Service stopped");
        Ok(())
    }
}
