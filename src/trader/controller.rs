//! Trader controller for starting/stopping trading

use crate::config::update_config_section;
use crate::logger::{self, LogTag};
use crate::trader::error::Error;
use std::time::Duration;

/// Check if the trader is currently running
pub fn is_trader_running() -> bool {
    super::config::is_trader_enabled()
}

/// Start the trader by enabling trader operations
pub async fn start_trader() -> Result<(), Error> {
    if super::config::is_trader_enabled() {
        return Err(Error::AlreadyRunning);
    }

    logger::info(LogTag::Trader, "Enabling trader operations...");

    // Update config to enable trader
    update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: e.to_string(),
    })?;

    logger::info(LogTag::Trader, "Trader operations enabled");
    Ok(())
}

/// Stop the trader gracefully by signaling shutdown and waiting for tasks to complete
pub async fn stop_trader_gracefully() -> Result<(), Error> {
    if !super::config::is_trader_enabled() {
        return Err(Error::AlreadyStopped);
    }

    logger::info(LogTag::Trader, "Disabling trader operations...");

    // Update config to disable trader
    update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: e.to_string(),
    })?;

    // Wait a moment for graceful shutdown
    tokio::time::sleep(Duration::from_secs(2)).await;

    logger::info(LogTag::Trader, "Trader operations disabled");
    Ok(())
}
