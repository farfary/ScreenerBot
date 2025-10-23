//! Automated trading system using strategies
//!
//! This module handles strategy-based trading including:
//! 1. Entry monitoring based on strategies
//! 2. Exit decisions based on strategies and exit rules
//! 3. DCA implementation

mod dca;
mod entry_monitor;
mod exit_monitor;
mod strategy_manager;

pub use dca::process_dca_opportunities;
pub use entry_monitor::monitor_entries;
pub use exit_monitor::monitor_positions;
pub use strategy_manager::StrategyManager;

use crate::logger::{log, LogTag};

/// Start the auto trading monitors
pub async fn start_auto_trading(
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Starting auto trading monitors...");

    // Record auto trading start event
    crate::events::record_safe(crate::events::Event::new(
        crate::events::EventCategory::Trader,
        Some("auto_trading_started".to_string()),
        crate::events::Severity::Info,
        None,
        None,
        serde_json::json!({
            "system": "auto_trading",
            "message": "Auto trading monitors starting up",
        }),
    ))
    .await;

    // Clone shutdown receiver for multiple tasks
    let entry_shutdown = shutdown.clone();
    let exit_shutdown = shutdown.clone();

    // Spawn entry monitor
    let entry_task = tokio::spawn(async move {
        if let Err(e) = monitor_entries(entry_shutdown).await {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Entry monitor error: {}", e),
            );
            
            // Record entry monitor error
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("entry_monitor_error".to_string()),
                crate::events::Severity::Error,
                None,
                None,
                serde_json::json!({
                    "monitor": "entry",
                    "error": e.to_string(),
                }),
            ))
            .await;
        }
    });

    // Spawn exit monitor
    let exit_task = tokio::spawn(async move {
        if let Err(e) = monitor_positions(exit_shutdown).await {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Exit monitor error: {}", e),
            );
            
            // Record exit monitor error
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("exit_monitor_error".to_string()),
                crate::events::Severity::Error,
                None,
                None,
                serde_json::json!({
                    "monitor": "exit",
                    "error": e.to_string(),
                }),
            ))
            .await;
        }
    });

    // Wait for both tasks
    let _ = tokio::try_join!(entry_task, exit_task);

    log(LogTag::Trader, "INFO", "Auto trading monitors stopped");
    
    // Record auto trading stop event
    crate::events::record_safe(crate::events::Event::new(
        crate::events::EventCategory::Trader,
        Some("auto_trading_stopped".to_string()),
        crate::events::Severity::Info,
        None,
        None,
        serde_json::json!({
            "system": "auto_trading",
            "message": "Auto trading monitors stopped",
        }),
    ))
    .await;
    
    Ok(())
}
