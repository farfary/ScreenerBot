//! ATA cleanup operations
//!
//! Core functions for scanning and closing empty Associated Token Accounts

use std::collections::HashSet;

use chrono::Utc;

use crate::logger::{self, LogTag};
use crate::tools::database::{
    get_failed_atas_for_wallet, is_ata_failed, remove_failed_ata, upsert_failed_ata,
};
use crate::utils::get_wallet_address;

use super::types::{AtaCleanupResult, AtaCleanupStats, AtaInfo};

// =============================================================================
// SCAN OPERATIONS
// =============================================================================

/// Scan wallet for all ATAs
///
/// Returns a list of all Associated Token Accounts for the wallet
pub async fn scan_wallet_atas(wallet_address: &str) -> Result<Vec<AtaInfo>, String> {
    let token_accounts = crate::utils::get_all_token_accounts(wallet_address)
        .await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    let atas: Vec<AtaInfo> = token_accounts.iter().map(AtaInfo::from).collect();

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Scanned {} ATAs for wallet {}",
            atas.len(),
            &wallet_address[..8]
        ),
    );

    Ok(atas)
}

/// Scan wallet for empty ATAs only
///
/// Returns a list of empty ATAs that can be closed to reclaim rent
pub async fn scan_empty_atas(wallet_address: &str) -> Result<Vec<AtaInfo>, String> {
    let all_atas = scan_wallet_atas(wallet_address).await?;
    let empty_atas: Vec<AtaInfo> = all_atas.into_iter().filter(|ata| ata.is_empty()).collect();

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Found {} empty ATAs for wallet {}",
            empty_atas.len(),
            &wallet_address[..8]
        ),
    );

    Ok(empty_atas)
}

/// Scan empty ATAs excluding failed ones from cache
pub async fn scan_closeable_atas(wallet_address: &str) -> Result<Vec<AtaInfo>, String> {
    let empty_atas = scan_empty_atas(wallet_address).await?;

    // Get failed ATAs from database
    let failed_atas: HashSet<String> = get_failed_atas_for_wallet(wallet_address)
        .unwrap_or_default()
        .into_iter()
        .map(|row| row.ata_address)
        .collect();

    let closeable: Vec<AtaInfo> = empty_atas
        .into_iter()
        .filter(|ata| !failed_atas.contains(&ata.ata_address))
        .collect();

    if !failed_atas.is_empty() {
        logger::debug(
            LogTag::Wallet,
            &format!(
                "Filtered {} failed ATAs, {} closeable remaining",
                failed_atas.len(),
                closeable.len()
            ),
        );
    }

    Ok(closeable)
}

// =============================================================================
// CLOSE OPERATIONS
// =============================================================================

/// Close a single ATA
///
/// Returns the transaction signature if successful
pub async fn close_ata(wallet_address: &str, ata: &AtaInfo) -> Result<String, String> {
    logger::debug(
        LogTag::Wallet,
        &format!(
            "Closing ATA {} for mint {}",
            &ata.ata_address[..8],
            &ata.mint[..8]
        ),
    );

    match crate::utils::close_single_ata(wallet_address, &ata.mint).await {
        Ok(signature) => {
            // Remove from failed cache if it was there
            let _ = remove_failed_ata(&ata.ata_address);

            logger::info(
                LogTag::Wallet,
                &format!(
                    "Closed ATA {} for mint {} - Tx: {}",
                    &ata.ata_address[..8],
                    &ata.mint[..8],
                    signature
                ),
            );
            Ok(signature)
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Add to failed cache
            if let Err(cache_err) =
                upsert_failed_ata(&ata.ata_address, Some(&ata.mint), wallet_address, &error_msg, false)
            {
                logger::error(
                    LogTag::Wallet,
                    &format!("Failed to cache failed ATA: {}", cache_err),
                );
            }

            logger::error(
                LogTag::Wallet,
                &format!(
                    "Failed to close ATA {} for mint {}: {}",
                    &ata.ata_address[..8],
                    &ata.mint[..8],
                    error_msg
                ),
            );
            Err(error_msg)
        }
    }
}

/// Cleanup all empty ATAs for the wallet
///
/// This is the main cleanup function that:
/// 1. Scans for empty ATAs
/// 2. Filters out failed ATAs from cache
/// 3. Closes each empty ATA
/// 4. Returns cleanup statistics
pub async fn cleanup_empty_atas(wallet_address: &str) -> Result<AtaCleanupResult, String> {
    logger::info(LogTag::Wallet, "Starting ATA cleanup...");

    // Get closeable ATAs (empty, not in failed cache)
    let closeable_atas = scan_closeable_atas(wallet_address).await?;

    if closeable_atas.is_empty() {
        logger::info(LogTag::Wallet, "No empty ATAs to close");
        return Ok(AtaCleanupResult::default());
    }

    logger::info(
        LogTag::Wallet,
        &format!("Found {} empty ATAs to close", closeable_atas.len()),
    );

    let mut result = AtaCleanupResult::default();

    for (index, ata) in closeable_atas.iter().enumerate() {
        match close_ata(wallet_address, ata).await {
            Ok(signature) => {
                result.closed_count += 1;
                result.signatures.push(signature);

                // Query actual ATA rent from chain
                let rent = match crate::rpc::get_ata_rent_lamports().await {
                    Ok(lamports) => (lamports as f64) / 1_000_000_000.0,
                    Err(_) => 0.00203928, // Fallback to standard ATA rent
                };
                result.rent_reclaimed += rent;
            }
            Err(_) => {
                result.failed_count += 1;
            }
        }

        // Rate limiting to prevent RPC spam (every 5 closures)
        if (index + 1) % 5 == 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "ATA cleanup complete: {} closed, {} failed, {:.6} SOL reclaimed",
            result.closed_count, result.failed_count, result.rent_reclaimed
        ),
    );

    Ok(result)
}

// =============================================================================
// CACHE OPERATIONS
// =============================================================================

/// Get the count of failed ATAs in cache
pub fn get_failed_ata_count() -> usize {
    match get_wallet_address() {
        Ok(wallet_address) => get_failed_atas_for_wallet(&wallet_address)
            .map(|rows| rows.len())
            .unwrap_or(0),
        Err(_) => 0,
    }
}

/// Check if an ATA is in the failed cache
pub fn is_ata_in_failed_cache(ata_address: &str) -> bool {
    is_ata_failed(ata_address).unwrap_or(false)
}

/// Clear all failed ATAs from cache (force retry)
///
/// This is async for backward compatibility with the old API
pub async fn clear_failed_ata_cache() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let wallet_address = get_wallet_address()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    let failed_atas = get_failed_atas_for_wallet(&wallet_address)
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?;

    let count = failed_atas.len();

    for ata in failed_atas {
        if let Err(e) = remove_failed_ata(&ata.ata_address) {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to remove ATA from cache: {}", e),
            );
        }
    }

    logger::info(
        LogTag::Wallet,
        &format!("Cleared {} failed ATAs from cache - will retry all ATAs", count),
    );

    Ok(())
}

// =============================================================================
// STATS
// =============================================================================

/// Global stats tracking (in-memory for current session)
static STATS: std::sync::LazyLock<std::sync::Mutex<AtaCleanupStats>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(AtaCleanupStats::default()));

/// Update global cleanup stats
pub fn update_stats(result: &AtaCleanupResult) {
    if let Ok(mut stats) = STATS.lock() {
        stats.total_closed += result.closed_count;
        stats.total_rent_reclaimed += result.rent_reclaimed;
        stats.failed_attempts += result.failed_count;
        stats.last_cleanup_time = Some(Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string());
    }
}

/// Get current cleanup statistics
pub fn get_cleanup_stats() -> AtaCleanupStats {
    STATS
        .lock()
        .map(|stats| stats.clone())
        .unwrap_or_default()
}

/// Get comprehensive ATA status for the wallet
pub async fn get_ata_status() -> Result<String, String> {
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;

    let all_atas = scan_wallet_atas(&wallet_address).await?;
    let empty_count = all_atas.iter().filter(|ata| ata.is_empty()).count();
    let non_empty_count = all_atas.len() - empty_count;
    let failed_count = get_failed_ata_count();
    let stats = get_cleanup_stats();
    let potential_rent = (empty_count as f64) * 0.00203928;

    let status = format!(
        "ATA Status - Total: {}, Empty: {}, Active: {}, Failed Cache: {}, Total Closed: {}, Rent Reclaimed: {:.6} SOL, Potential: {:.6} SOL",
        all_atas.len(),
        empty_count,
        non_empty_count,
        failed_count,
        stats.total_closed,
        stats.total_rent_reclaimed,
        potential_rent
    );

    Ok(status)
}
