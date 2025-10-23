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
        }
    });

    // Wait for both tasks
    let _ = tokio::try_join!(entry_task, exit_task);

    log(LogTag::Trader, "INFO", "Auto trading monitors stopped");
    Ok(())
}
