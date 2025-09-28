use crate::global::ATA_FAILED_CACHE;
/// Background ATA Cleanup Service
///
/// This service runs independently from the trading logic and periodically
/// scans for empty Associated Token Accounts (ATAs) to close and reclaim rent.
/// This prevents blocking the main trading flow while ensuring optimal rent utilization.
use crate::logger::{ log, LogTag };
use crate::utils::get_wallet_address;

use chrono;
use serde::{ Deserialize, Serialize };
use std::collections::HashSet;
use std::path::Path;
use std::sync::{ Arc, LazyLock, Mutex };
use tokio::sync::Notify;
use tokio::time::{ interval, sleep, Duration, MissedTickBehavior };

/// Configuration constants for ATA cleanup service
const ATA_CLEANUP_INTERVAL_MINUTES: u64 = 5; // Check every 5 minutes
const ATA_CLEANUP_STARTUP_DELAY_SECONDS: u64 = 30; // Wait 30 seconds before first cleanup
const ATA_FAILED_CACHE_FILE: &str = ATA_FAILED_CACHE; // Cache file for failed ATA closures

/// Global statistics for ATA cleanup operations
static ATA_STATS: LazyLock<Mutex<AtaCleanupStats>> = LazyLock::new(|| {
    Mutex::new(AtaCleanupStats {
        total_closed: 0,
        total_rent_reclaimed: 0.0,
        failed_attempts: 0,
        last_cleanup_time: None,
    })
});

/// Cache for failed ATA closure attempts to avoid retrying them
static FAILED_ATA_CACHE: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(||
    Mutex::new(HashSet::new())
);

/// Statistics structure for ATA cleanup operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaCleanupStats {
    pub total_closed: u32,
    pub total_rent_reclaimed: f64,
    pub failed_attempts: u32,
    pub last_cleanup_time: Option<String>,
}

/// Cache structure for failed ATA closure attempts
#[derive(Debug, Serialize, Deserialize)]
struct FailedAtaCache {
    failed_atas: HashSet<String>,
}

/// Background ATA cleanup service that runs independently
/// This service periodically scans for empty ATAs and closes them to reclaim rent
pub async fn start_ata_cleanup_service(shutdown_notify: Arc<Notify>) {
    log(LogTag::Wallet, "ATA_SERVICE", "Starting background ATA cleanup service...");

    // Load failed ATA cache from disk
    load_failed_ata_cache().await;

    // Wait a bit before starting to allow system initialization
    sleep(Duration::from_secs(ATA_CLEANUP_STARTUP_DELAY_SECONDS)).await;

    // Create interval timer for periodic cleanup with delay behavior
    let mut cleanup_timer = interval(Duration::from_secs(ATA_CLEANUP_INTERVAL_MINUTES * 60));
    cleanup_timer.set_missed_tick_behavior(MissedTickBehavior::Delay);

    log(
        LogTag::Wallet,
        "ATA_SERVICE",
        &format!("ATA cleanup service started - will check every {} minutes", ATA_CLEANUP_INTERVAL_MINUTES)
    );

    loop {
        tokio::select! {
            // Check for shutdown signal
            _ = shutdown_notify.notified() => {
                log(LogTag::Wallet, "ATA_SERVICE", "ATA cleanup service shutting down...");
                // Save failed ATA cache before shutdown
                save_failed_ata_cache().await;
                break;
            }

            // Periodic cleanup timer
            _ = cleanup_timer.tick() => {
                match perform_ata_cleanup().await {
                    Ok((closed_count, signatures)) => {
                        if closed_count > 0 {
                            log(
                                LogTag::Wallet,
                                "ATA_SERVICE",
                                &format!("Cleanup cycle completed: {} ATAs closed, {} signatures",
                                        closed_count, signatures.len())
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Wallet,
                            "ERROR",
                            &format!("ATA cleanup service error: {}", e)
                        );

                        // Sleep a bit before continuing on error to avoid rapid failures
                        sleep(Duration::from_secs(30)).await;
                    }
                }
            }
        }
    }

    log(LogTag::Wallet, "ATA_SERVICE", "ATA cleanup service stopped");
}

/// Performs the actual ATA cleanup operation with failed ATA caching
async fn perform_ata_cleanup() -> Result<
    (u32, Vec<String>),
    Box<dyn std::error::Error + Send + Sync>
> {
    log(LogTag::Wallet, "ATA_SERVICE", "Starting periodic ATA cleanup check...");

    // Get wallet address from config
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to get wallet address for ATA cleanup: {}", e)
            );
            return Err(Box::new(e));
        }
    };

    // Get all token accounts first
    let all_accounts = match crate::utils::get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to get token accounts: {}", e));
            return Err(Box::new(e));
        }
    };

    // Filter out empty accounts, excluding those in failed cache
    let failed_cache = match FAILED_ATA_CACHE.lock() {
        Ok(cache) => cache.clone(),
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to lock failed ATA cache: {}", e));
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Mutex poisoned")));
        }
    };
    let empty_accounts: Vec<_> = all_accounts
        .iter()
        .filter(|acc| acc.balance == 0 && !failed_cache.contains(&acc.mint))
        .collect();

    if empty_accounts.is_empty() {
        log(LogTag::Wallet, "ATA_SERVICE", "No empty ATAs found - wallet is already optimized");
        return Ok((0, Vec::new()));
    }

    log(
        LogTag::Wallet,
        "ATA_SERVICE",
        &format!(
            "Found {} empty ATAs to close (excluding {} cached failures)",
            empty_accounts.len(),
            failed_cache.len()
        )
    );

    // Try to close empty ATAs with individual error handling
    let mut closed_count = 0;
    let mut failed_count = 0;
    let mut total_rent_reclaimed = 0.0;
    let mut signatures = Vec::new();
    let mut newly_failed_atas = Vec::new();

    for account in empty_accounts {
        match crate::utils::close_single_ata(&wallet_address, &account.mint).await {
            Ok(signature) => {
                closed_count += 1;
                // Query actual ATA rent from chain
                let actual_rent = match crate::rpc::get_ata_rent_lamports().await {
                    Ok(rent_lamports) => (rent_lamports as f64) / 1_000_000_000.0,
                    Err(_) => 0.00203928, // Fallback to standard ATA rent
                };
                total_rent_reclaimed += actual_rent;
                log(
                    LogTag::Wallet,
                    "ATA_SERVICE",
                    &format!(
                        "Closed ATA for {} - Tx: {} - Rent reclaimed: {:.8} SOL",
                        &account.mint[..8],
                        signature,
                        actual_rent
                    )
                );
                signatures.push(signature);
            }
            Err(e) => {
                failed_count += 1;
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("Failed to close ATA {}: {} - Adding to cache", &account.mint[..8], e)
                );
                newly_failed_atas.push(account.mint.clone());
            }
        }

        // Add rate limiting to prevent RPC spam
        if closed_count % 5 == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    // Batch update failed cache to minimize lock duration
    if !newly_failed_atas.is_empty() {
        match FAILED_ATA_CACHE.lock() {
            Ok(mut cache) => {
                for failed_ata in newly_failed_atas {
                    cache.insert(failed_ata);
                }
            }
            Err(e) => {
                log(LogTag::Wallet, "ERROR", &format!("Failed to update failed ATA cache: {}", e));
            }
        }
    }

    // Update global statistics with safe mutex handling
    match ATA_STATS.lock() {
        Ok(mut stats) => {
            stats.total_closed += closed_count;
            stats.total_rent_reclaimed += total_rent_reclaimed;
            stats.failed_attempts += failed_count;
            stats.last_cleanup_time = Some(
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
            );
        }
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to update ATA cleanup stats: {}", e));
        }
    }

    // Save failed cache if there were new failures
    if failed_count > 0 {
        save_failed_ata_cache().await;
    }

    // Log summary
    if closed_count > 0 {
        log(
            LogTag::Wallet,
            "ATA_SERVICE",
            &format!(
                "Cleaned up {} ATAs, reclaimed {:.6} SOL in rent. {} failures cached.",
                closed_count,
                total_rent_reclaimed,
                failed_count
            )
        );
    } else {
        log(LogTag::Wallet, "ATA_SERVICE", "No ATAs were successfully closed");
    }

    log(LogTag::Wallet, "ATA_SERVICE", "Periodic ATA cleanup check completed");
    Ok((closed_count, signatures))
}

/// Manually trigger an immediate ATA cleanup (can be called from other parts of the system)
pub async fn trigger_immediate_ata_cleanup() -> Result<
    (u32, Vec<String>),
    Box<dyn std::error::Error + Send + Sync>
> {
    log(LogTag::Wallet, "ATA_SERVICE", "Manual ATA cleanup triggered...");

    // Perform manual cleanup and collect signatures
    let (cleanup_count, actual_signatures) = perform_ata_cleanup().await?;

    // Log completion with actual results (no need to re-access stats)
    log(
        LogTag::Wallet,
        "ATA_SERVICE",
        &format!(
            "Manual ATA cleanup completed: {} closed, {} signatures",
            cleanup_count,
            actual_signatures.len()
        )
    );
    Ok((cleanup_count, actual_signatures))
}

/// Get comprehensive ATA cleanup statistics
pub fn get_ata_cleanup_statistics() -> AtaCleanupStats {
    match ATA_STATS.lock() {
        Ok(stats) => stats.clone(),
        Err(_) => {
            log(LogTag::Wallet, "ERROR", "Failed to lock ATA stats - returning default stats");
            AtaCleanupStats {
                total_closed: 0,
                total_rent_reclaimed: 0.0,
                failed_attempts: 0,
                last_cleanup_time: None,
            }
        }
    }
}

/// Get count of failed ATAs that are cached (won't be retried)
pub fn get_failed_ata_count() -> usize {
    match FAILED_ATA_CACHE.lock() {
        Ok(cache) => cache.len(),
        Err(_) => {
            log(LogTag::Wallet, "ERROR", "Failed to lock failed ATA cache - returning 0");
            0
        }
    }
}

/// Clear the failed ATA cache (force retry of all previously failed ATAs)
pub async fn clear_failed_ata_cache() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match FAILED_ATA_CACHE.lock() {
        Ok(mut cache) => {
            cache.clear();
        }
        Err(e) => {
            let error_msg = format!("Failed to lock failed ATA cache for clearing: {}", e);
            log(LogTag::Wallet, "ERROR", &error_msg);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_msg)));
        }
    }

    save_failed_ata_cache().await;
    log(LogTag::Wallet, "ATA_SERVICE", "Failed ATA cache cleared - will retry all ATAs");
    Ok(())
}

/// Load failed ATA cache from disk
async fn load_failed_ata_cache() {
    if Path::new(ATA_FAILED_CACHE_FILE).exists() {
        match tokio::fs::read_to_string(ATA_FAILED_CACHE_FILE).await {
            Ok(content) =>
                match serde_json::from_str::<FailedAtaCache>(&content) {
                    Ok(cache_data) =>
                        match FAILED_ATA_CACHE.lock() {
                            Ok(mut cache) => {
                                *cache = cache_data.failed_atas;
                                log(
                                    LogTag::Wallet,
                                    "ATA_SERVICE",
                                    &format!("Loaded {} failed ATAs from cache", cache.len())
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Wallet,
                                    "ERROR",
                                    &format!("Failed to lock failed ATA cache during load: {}", e)
                                );
                            }
                        }
                    Err(e) => {
                        log(
                            LogTag::Wallet,
                            "ERROR",
                            &format!("Failed to parse ATA cache file: {}", e)
                        );
                    }
                }
            Err(e) => {
                log(LogTag::Wallet, "ERROR", &format!("Failed to read ATA cache file: {}", e));
            }
        }
    } else {
        log(LogTag::Wallet, "ATA_SERVICE", "No existing failed ATA cache found");
    }
}

/// Save failed ATA cache to disk
async fn save_failed_ata_cache() {
    let cache_data = match FAILED_ATA_CACHE.lock() {
        Ok(cache) =>
            FailedAtaCache {
                failed_atas: cache.clone(),
            },
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to lock failed ATA cache during save: {}", e)
            );
            return;
        }
    };

    match serde_json::to_string_pretty(&cache_data) {
        Ok(json) => {
            if let Err(e) = tokio::fs::write(ATA_FAILED_CACHE_FILE, json).await {
                log(LogTag::Wallet, "ERROR", &format!("Failed to save ATA cache file: {}", e));
            } else {
                log(
                    LogTag::Wallet,
                    "ATA_SERVICE",
                    &format!("Saved {} failed ATAs to cache", cache_data.failed_atas.len())
                );
            }
        }
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to serialize ATA cache: {}", e));
        }
    }
}

/// Get ATA cleanup service status and statistics
pub async fn get_ata_cleanup_stats() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let wallet_address = get_wallet_address().map_err(
        |e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>
    )?;

    // Get current statistics
    let stats = get_ata_cleanup_statistics();
    let failed_count = get_failed_ata_count();

    // Get all token accounts to analyze current state
    match crate::utils::get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => {
            let total_accounts = accounts.len();
            let empty_accounts = accounts
                .iter()
                .filter(|acc| acc.balance == 0)
                .count();
            let non_empty_accounts = total_accounts - empty_accounts;
            let potential_rent = (empty_accounts as f64) * 0.00203928;

            let status = format!(
                "ATA Status - Total: {}, Empty: {}, Active: {}, Failed Cache: {}, Total Closed: {}, Rent Reclaimed: {:.6} SOL, Potential: {:.6} SOL",
                total_accounts,
                empty_accounts,
                non_empty_accounts,
                failed_count,
                stats.total_closed,
                stats.total_rent_reclaimed,
                potential_rent
            );

            log(LogTag::Wallet, "ATA_SERVICE", &status);
            Ok(status)
        }
        Err(e) => {
            let error_msg = format!("Failed to get ATA stats: {}", e);
            log(LogTag::Wallet, "ERROR", &error_msg);
            Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    }
}
