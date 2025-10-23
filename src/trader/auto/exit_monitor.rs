//! Position monitoring and exit strategy application

use crate::logger::{log, LogTag};
use crate::trader::config;
use tokio::time::{sleep, Duration};

/// Constants for position monitoring
const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

/// Monitor open positions for exit opportunities
pub async fn monitor_positions(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Starting position monitor");

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            log(LogTag::Trader, "INFO", "Position monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            log(
                LogTag::Trader,
                "INFO",
                "Position monitor paused - trader disabled",
            );
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // TODO: Implement position monitoring when positions module is ready
        // For now, just sleep and loop
        
        // Wait for next cycle or shutdown
        tokio::select! {
            _ = sleep(Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log(LogTag::Trader, "INFO", "Position monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}
