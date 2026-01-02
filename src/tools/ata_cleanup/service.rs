//! ATA cleanup background service
//!
//! Runs periodically to scan and close empty ATAs to reclaim rent

use std::sync::Arc;

use tokio::sync::Notify;
use tokio::time::{interval, sleep, Duration, MissedTickBehavior};

use crate::logger::{self, LogTag};
use crate::utils::get_wallet_address;

use super::operations::{cleanup_empty_atas, update_stats};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Check for empty ATAs every 5 minutes
const ATA_CLEANUP_INTERVAL_MINUTES: u64 = 5;

/// Wait 30 seconds after startup before first cleanup
const ATA_CLEANUP_STARTUP_DELAY_SECONDS: u64 = 30;

// =============================================================================
// SERVICE
// =============================================================================

/// Start the background ATA cleanup service
///
/// This service runs independently from trading logic and periodically
/// scans for empty ATAs to close and reclaim rent.
pub async fn start_ata_cleanup_service(shutdown_notify: Arc<Notify>) {
    logger::debug(LogTag::Wallet, "Starting background ATA cleanup service...");

    // Wait before starting to allow system initialization
    sleep(Duration::from_secs(ATA_CLEANUP_STARTUP_DELAY_SECONDS)).await;

    // Create interval timer for periodic cleanup
    let mut cleanup_timer = interval(Duration::from_secs(ATA_CLEANUP_INTERVAL_MINUTES * 60));
    cleanup_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

    logger::debug(
        LogTag::Wallet,
        &format!(
            "ATA cleanup service started - will check every {} minutes",
            ATA_CLEANUP_INTERVAL_MINUTES
        ),
    );

    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_notify.notified() => {
                logger::info(LogTag::Wallet, "ATA cleanup service shutting down...");
                break;
            }

            // Periodic cleanup timer
            _ = cleanup_timer.tick() => {
                match perform_scheduled_cleanup().await {
                    Ok((closed_count, signatures)) => {
                        if closed_count > 0 {
                            logger::info(
                                LogTag::Wallet,
                                &format!(
                                    "Cleanup cycle completed: {} ATAs closed, {} signatures",
                                    closed_count,
                                    signatures.len()
                                ),
                            );
                        }
                    }
                    Err(e) => {
                        logger::error(
                            LogTag::Wallet,
                            &format!("ATA cleanup service error: {}", e),
                        );
                        // Sleep before continuing on error to avoid rapid failures
                        sleep(Duration::from_secs(30)).await;
                    }
                }
            }
        }
    }

    logger::info(LogTag::Wallet, "ATA cleanup service stopped");
}

/// Perform a scheduled cleanup cycle
async fn perform_scheduled_cleanup() -> Result<(u32, Vec<String>), String> {
    logger::debug(LogTag::Wallet, "Starting periodic ATA cleanup check...");

    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;

    let result = cleanup_empty_atas(&wallet_address).await?;

    // Update global stats
    update_stats(&result);

    if result.closed_count == 0 && result.failed_count == 0 {
        logger::debug(
            LogTag::Wallet,
            "No empty ATAs found - wallet is already optimized",
        );
    }

    logger::debug(LogTag::Wallet, "Periodic ATA cleanup check completed");

    Ok((result.closed_count, result.signatures))
}

/// Trigger an immediate ATA cleanup (manual trigger)
pub async fn trigger_immediate_cleanup(
) -> Result<(u32, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    logger::info(LogTag::Wallet, "Manual ATA cleanup triggered...");

    let wallet_address = get_wallet_address()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    let result = cleanup_empty_atas(&wallet_address).await.map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;

    // Update global stats
    update_stats(&result);

    logger::info(
        LogTag::Wallet,
        &format!(
            "Manual ATA cleanup completed: {} closed, {} signatures",
            result.closed_count,
            result.signatures.len()
        ),
    );

    Ok((result.closed_count, result.signatures))
}
