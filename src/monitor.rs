// use crate::global::LIST_MINTS;
// use crate::global::LIST_TOKENS;
use crate::discovery_manager::start_discovery_task;
use crate::token_monitor::start_token_monitoring;
use crate::token_blacklist::cleanup_blacklist;
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };

/// Monitor background task loop - coordinates separate discovery and token monitoring tasks
pub async fn monitor(shutdown: Arc<Notify>) {
    log(LogTag::Monitor, "INFO", "Starting coordinated monitoring system...");

    // Start separate background tasks
    let discovery_shutdown = shutdown.clone();
    let token_monitoring_shutdown = shutdown.clone();
    let cleanup_shutdown = shutdown.clone();

    // Start discovery task (mint finding from DexScreener and RugCheck)
    let discovery_handle = tokio::spawn(async move {
        start_discovery_task(discovery_shutdown).await;
    });

    // Start token monitoring task (periodic database checks with liquidity prioritization)
    let token_monitoring_handle = tokio::spawn(async move {
        start_token_monitoring(token_monitoring_shutdown).await;
    });

    // Start periodic cleanup task
    let cleanup_handle = tokio::spawn(async move {
        let mut cleanup_cycle = 0;
        loop {
            if check_shutdown_or_delay(&cleanup_shutdown, Duration::from_secs(3600)).await {
                // 1 hour
                log(LogTag::Monitor, "INFO", "Cleanup task shutting down...");
                break;
            }

            cleanup_cycle += 1;
            log(LogTag::Monitor, "INFO", &format!("Running cleanup cycle #{}", cleanup_cycle));

            // Cleanup old blacklist entries
            cleanup_blacklist();

            log(LogTag::Monitor, "SUCCESS", &format!("Cleanup cycle #{} completed", cleanup_cycle));
        }
    });

    // Wait for shutdown signal
    loop {
        if check_shutdown_or_delay(&shutdown, Duration::from_secs(5)).await {
            log(LogTag::Monitor, "INFO", "Monitor coordinator shutting down...");
            break;
        }
    }

    // Cancel background tasks
    discovery_handle.abort();
    token_monitoring_handle.abort();
    cleanup_handle.abort();

    log(LogTag::Monitor, "INFO", "All monitoring tasks stopped");
}

// All token discovery and monitoring logic is now handled by separate background tasks:
// - discovery_manager.rs: Mint finding from DexScreener and RugCheck APIs
// - token_monitor.rs: Periodic database checks with liquidity-based prioritization and blacklisting
