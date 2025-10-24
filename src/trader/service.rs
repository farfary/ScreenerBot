//! Trader service implementation

use crate::logger::{self, LogTag};
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
        logger::info(LogTag::Trader, "Starting Trader Service...");

        // Record service start event
        crate::events::record_safe(crate::events::Event::new(
            crate::events::EventCategory::Trader,
            Some("service_start".to_string()),
            crate::events::Severity::Info,
            None,
            None,
            serde_json::json!({
                "action": "startup",
                "message": "Trader service initialization beginning",
            }),
        ))
        .await;

        // Initialize trader system
        super::init_trader_system().await?;

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            logger::info(
                LogTag::Trader,
                "Trader service started but trading is disabled in config",
            );
            
            // Record disabled state event
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("trading_disabled".to_string()),
                crate::events::Severity::Warn,
                None,
                None,
                serde_json::json!({
                    "enabled": false,
                    "message": "Trading is disabled in configuration",
                }),
            ))
            .await;
        } else {
            // Record enabled state event
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("trading_enabled".to_string()),
                crate::events::Severity::Info,
                None,
                None,
                serde_json::json!({
                    "enabled": true,
                    "message": "Trading is enabled and active",
                }),
            ))
            .await;
        }

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        *self.shutdown_tx.write().await = Some(shutdown_tx);

        // Start auto trading monitors
        let _auto_task = tokio::spawn(async move {
            if let Err(e) = auto::start_auto_trading(shutdown_rx).await {
                logger::error(
                    LogTag::Trader,
                    &format!("Auto trading error: {}", e),
                );
                
                // Record auto trading error event
                crate::events::record_safe(crate::events::Event::new(
                    crate::events::EventCategory::Trader,
                    Some("auto_trading_error".to_string()),
                    crate::events::Severity::Error,
                    None,
                    None,
                    serde_json::json!({
                        "error": e.to_string(),
                        "message": "Auto trading encountered an error",
                    }),
                ))
                .await;
            }
        });

        logger::info(LogTag::Trader, "Trader Service started successfully");
        
        // Record successful start event
        crate::events::record_safe(crate::events::Event::new(
            crate::events::EventCategory::Trader,
            Some("service_started".to_string()),
            crate::events::Severity::Info,
            None,
            None,
            serde_json::json!({
                "status": "running",
                "message": "Trader service fully initialized and running",
            }),
        ))
        .await;
        
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        logger::info(LogTag::Trader, "Stopping Trader Service...");

        // Record shutdown event
        crate::events::record_safe(crate::events::Event::new(
            crate::events::EventCategory::Trader,
            Some("service_stop".to_string()),
            crate::events::Severity::Info,
            None,
            None,
            serde_json::json!({
                "action": "shutdown",
                "message": "Trader service shutdown initiated",
            }),
        ))
        .await;

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.write().await.take() {
            let _ = tx.send(true);
        }

        // Wait a moment for graceful shutdown
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        logger::info(LogTag::Trader, "Trader Service stopped");
        
        // Record stopped event
        crate::events::record_safe(crate::events::Event::new(
            crate::events::EventCategory::Trader,
            Some("service_stopped".to_string()),
            crate::events::Severity::Info,
            None,
            None,
            serde_json::json!({
                "status": "stopped",
                "message": "Trader service gracefully stopped",
            }),
        ))
        .await;
        
        Ok(())
    }
}
