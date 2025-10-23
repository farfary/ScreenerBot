//! Trader controller for starting/stopping trading

use crate::config::update_config_section;
use crate::logger::{log, LogTag};
use std::time::Duration;

/// Trader control error types
#[derive(Debug)]
pub enum TraderControlError {
    AlreadyRunning,
    AlreadyStopped,
    ConfigUpdate(String),
}

impl std::fmt::Display for TraderControlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraderControlError::AlreadyRunning => write!(f, "Trader is already running"),
            TraderControlError::AlreadyStopped => write!(f, "Trader is already stopped"),
            TraderControlError::ConfigUpdate(err) => write!(f, "Config update failed: {}", err),
        }
    }
}

impl std::error::Error for TraderControlError {}

/// Check if the trader is currently running
pub fn is_trader_running() -> bool {
    super::config::is_trader_enabled()
}

/// Start the trader by enabling trader operations
pub async fn start_trader() -> Result<(), TraderControlError> {
    if super::config::is_trader_enabled() {
        return Err(TraderControlError::AlreadyRunning);
    }

    log(LogTag::Trader, "INFO", "Enabling trader operations...");

    // Update config to enable trader
    update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
        },
        true,
    )
    .map_err(|e| TraderControlError::ConfigUpdate(e.to_string()))?;

    log(LogTag::Trader, "INFO", "Trader operations enabled");
    Ok(())
}

/// Stop the trader gracefully by signaling shutdown and waiting for tasks to complete
pub async fn stop_trader_gracefully() -> Result<(), TraderControlError> {
    if !super::config::is_trader_enabled() {
        return Err(TraderControlError::AlreadyStopped);
    }

    log(LogTag::Trader, "INFO", "Disabling trader operations...");

    // Update config to disable trader
    update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    )
    .map_err(|e| TraderControlError::ConfigUpdate(e.to_string()))?;

    // Wait a moment for graceful shutdown
    tokio::time::sleep(Duration::from_secs(2)).await;

    log(LogTag::Trader, "INFO", "Trader operations disabled");
    Ok(())
}
