//! Automated trading system using strategies
//!
//! This module handles strategy-based trading including:
//! 1. Entry monitoring based on strategies
//! 2. Exit decisions based on strategies and exit rules
//! 3. DCA implementation

mod dca;
mod dca_evaluation;
mod entry_monitor;
mod exit_monitor;
mod strategy_manager;

pub use dca::process_dca_opportunities;
pub use dca_evaluation::{DcaCalculations, DcaConfigSnapshot, DcaEvaluation};
pub use entry_monitor::monitor_entries;
pub use exit_monitor::monitor_positions;
pub use strategy_manager::StrategyManager;

use crate::events::{record_trader_event, Severity};
use crate::logger::{self, LogTag};
use serde_json::json;

/// Start the auto trading monitors
pub async fn start_auto_trading(
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    logger::info(LogTag::Trader, "Starting auto trading monitors...");

    // Record auto trading start event
    record_trader_event(
        "auto_trading_started",
        Severity::Info,
        None,
        None,
        json!({
            "system": "auto_trading",
            "message": "Auto trading monitors starting up",
        }),
    )
    .await;

    // Clone shutdown receiver for multiple tasks
    let entry_shutdown = shutdown.clone();
    let exit_shutdown = shutdown.clone();

    // Spawn entry monitor
    let entry_task = tokio::spawn(async move {
        if let Err(e) = monitor_entries(entry_shutdown).await {
            logger::error(LogTag::Trader, &format!("Entry monitor error: {}", e));

            // Record entry monitor error
            record_trader_event(
                "entry_monitor_error",
                Severity::Error,
                None,
                None,
                json!({
                    "monitor": "entry",
                    "error": e.to_string(),
                }),
            )
            .await;
        }
    });

    // Spawn exit monitor
    let exit_task = tokio::spawn(async move {
        if let Err(e) = monitor_positions(exit_shutdown).await {
            logger::error(LogTag::Trader, &format!("Exit monitor error: {}", e));

            // Record exit monitor error
            record_trader_event(
                "exit_monitor_error",
                Severity::Error,
                None,
                None,
                json!({
                    "monitor": "exit",
                    "error": e.to_string(),
                }),
            )
            .await;
        }
    });

    // Wait for both tasks
    let _ = tokio::try_join!(entry_task, exit_task);

    logger::info(LogTag::Trader, "Auto trading monitors stopped");

    // Record auto trading stop event
    record_trader_event(
        "auto_trading_stopped",
        Severity::Info,
        None,
        None,
        json!({
            "system": "auto_trading",
            "message": "Auto trading monitors stopped",
        }),
    )
    .await;

    Ok(())
}
