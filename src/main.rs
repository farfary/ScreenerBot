use screenerbot::{ monitor::monitor, trader::trader };
use screenerbot::logger::{ log, LogTag };

use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    log(LogTag::System, "INFO", "Starting ScreenerBot background tasks");
    let shutdown = Arc::new(Notify::new());
    let shutdown_monitor = shutdown.clone();
    let shutdown_trader = shutdown.clone();

    let monitor_handle = tokio::spawn(async move {
        log(LogTag::Monitor, "INFO", "Monitor task started");
        monitor(shutdown_monitor).await;
        log(LogTag::Monitor, "INFO", "Monitor task ended");
    });
    let trader_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "Trader task started");
        trader(shutdown_trader).await;
        log(LogTag::Trader, "INFO", "Trader task ended");
    });

    log(LogTag::System, "INFO", "Waiting for Ctrl+C to shutdown");
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    log(LogTag::System, "INFO", "Shutdown signal received, notifying tasks");
    shutdown.notify_waiters();

    // Wait for background tasks to finish
    let _ = tokio::try_join!(monitor_handle, trader_handle);
    log(LogTag::System, "INFO", "All background tasks finished. Exiting.");
}
