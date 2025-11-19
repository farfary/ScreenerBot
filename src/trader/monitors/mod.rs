//! Monitoring loops for automated trading
//!
//! This module contains orchestration-only code:
//! - Entry monitor: Loops through available tokens, calls evaluators, executes trades
//! - Exit monitor: Loops through open positions, calls evaluators, executes trades
//!
//! All business logic (safety checks, strategy evaluation, exit conditions) is in evaluators module.

mod entry;
mod exit;

pub use entry::monitor_entries;
pub use exit::monitor_positions;

use crate::events::{record_trader_event, Severity};
use crate::logger::{self, LogTag};
use serde_json::json;

/// Start the auto trading monitors
pub async fn start_automated_trading(
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    logger::info(LogTag::Trader, "Starting automated trading monitors...");

    // Record auto trading start event
    record_trader_event(
        "auto_trading_started",
        Severity::Info,
        None,
        None,
        json!({
            "system": "auto_trading",
            "message": "Automated trading monitors starting up",
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

    logger::info(LogTag::Trader, "Automated trading monitors stopped");

    // Record auto trading stop event
    record_trader_event(
        "auto_trading_stopped",
        Severity::Info,
        None,
        None,
        json!({
            "system": "auto_trading",
            "message": "Automated trading monitors stopped",
        }),
    )
    .await;

    Ok(())
}
