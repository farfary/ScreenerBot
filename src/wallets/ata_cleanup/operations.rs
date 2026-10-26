// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! ATA cleanup operations
//!
//! Core functions for scanning and closing empty Associated Token Accounts

use std::time::Duration;

use chrono::Utc;

use crate::chains::adapter;
use crate::logger::{self, LogTag};
use crate::wallets::Error;

use super::database::{get_excluded_atas_for_wallet, remove_failed_ata, upsert_failed_ata};
use super::types::{AtaCleanupResult, AtaInfo};

// =============================================================================
// SCAN OPERATIONS
// =============================================================================

/// Scan wallet for all ATAs
///
/// Returns a list of all Associated Token Accounts for the wallet
pub async fn scan_wallet_atas(wallet_address: &str) -> Result<Vec<AtaInfo>, Error> {
    let token_accounts = crate::chains::solana::assets::ata::get_all_token_accounts(wallet_address)
        .await
        .map_err(|e| Error::Dependency {
            dependency: "chains",
            detail: e.to_string(),
        })?;

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
pub async fn scan_empty_atas(wallet_address: &str) -> Result<Vec<AtaInfo>, Error> {
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

/// Scan empty ATAs excluding cached failures that are permanent or not yet due for a retry
pub async fn scan_closeable_atas(wallet_address: &str) -> Result<Vec<AtaInfo>, Error> {
    let empty_atas = scan_empty_atas(wallet_address).await?;

    // ATAs the cache parks: permanent failures and transient ones not yet due for a retry
    let failed_atas = get_excluded_atas_for_wallet(wallet_address, Utc::now()).unwrap_or_default();

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
pub async fn close_ata(wallet_address: &str, ata: &AtaInfo) -> Result<String, Error> {
    logger::debug(
        LogTag::Wallet,
        &format!(
            "Closing ATA {} for mint {}",
            &ata.ata_address[..8],
            &ata.mint[..8]
        ),
    );

    match crate::chains::solana::assets::ata::close_single_ata(wallet_address, &ata.mint).await {
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
        Err(crate::Error::Solana(crate::chains::solana::Error::AccountNotFound { .. })) => {
            // The account is already gone: nothing to reclaim and nothing to retry.
            let _ = remove_failed_ata(&ata.ata_address);

            logger::info(
                LogTag::Wallet,
                &format!(
                    "ATA {} for mint {} is already closed",
                    &ata.ata_address[..8],
                    &ata.mint[..8]
                ),
            );
            Err(Error::Dependency {
                dependency: "chains",
                detail: format!("ATA {} no longer exists", ata.ata_address),
            })
        }
        Err(e) => {
            let error_msg = e.to_string();

            // Add to failed cache
            if let Err(cache_err) = upsert_failed_ata(
                &ata.ata_address,
                Some(&ata.mint),
                wallet_address,
                &error_msg,
                false,
            ) {
                logger::error(
                    LogTag::Wallet,
                    &format!("Failed to cache failed ATA: {cache_err}"),
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
            Err(Error::Dependency {
                dependency: "chains",
                detail: error_msg,
            })
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
pub async fn cleanup_empty_atas(wallet_address: &str) -> Result<AtaCleanupResult, Error> {
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
                result.rent_reclaimed += adapter().raw_to_native(ata.lamports);
            }
            Err(_) => {
                result.failed_count += 1;
            }
        }

        // Rate limiting to prevent RPC spam (every 5 closures)
        if (index + 1) % 5 == 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
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
