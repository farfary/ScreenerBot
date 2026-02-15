//! ATA cleanup and wallet generator handlers

use axum::response::Response;
use axum::Json;
use solana_sdk::signer::Signer;

use crate::ata_cleanup::{
    clear_failed_ata_cache, get_ata_cleanup_statistics, get_failed_ata_count,
    trigger_immediate_ata_cleanup,
};
use crate::logger::{self, LogTag};
use crate::utils::{get_all_token_accounts, get_wallet_address};
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// Wallet Cleanup Handlers
// =============================================================================

/// Scan wallet for empty ATAs without closing them
pub async fn scan_atas() -> Response {
    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to get wallet address: {}", e),
            );
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "WALLET_ERROR",
                "Failed to get wallet",
                Some(&e.to_string()),
            );
        }
    };

    // Get all token accounts
    let all_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::error(
                LogTag::Wallet,
                &format!("Failed to get token accounts: {}", e),
            );
            return error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "SCAN_ERROR",
                "Failed to scan accounts",
                Some(&e.to_string()),
            );
        }
    };

    // Separate empty and non-empty
    let empty_accounts: Vec<_> = all_accounts.iter().filter(|acc| acc.balance == 0).collect();
    let non_empty_count = all_accounts.len() - empty_accounts.len();
    let failed_count = get_failed_ata_count();

    // Estimate rent reclaimable (approximately 0.00203928 SOL per ATA)
    const ATA_RENT_LAMPORTS: u64 = 2_039_280;
    let reclaimable_sol =
        (empty_accounts.len() as f64 * ATA_RENT_LAMPORTS as f64) / 1_000_000_000.0;

    // Build empty ATA info list
    let empty_atas: Vec<EmptyAtaInfo> = empty_accounts
        .iter()
        .map(|acc| EmptyAtaInfo {
            mint: acc.mint.clone(),
            ata_address: acc.account.clone(),
            rent_lamports: ATA_RENT_LAMPORTS,
        })
        .collect();

    logger::info(
        LogTag::Wallet,
        &format!(
            "ATA scan complete: {} total, {} empty (reclaimable: {:.6} SOL), {} non-empty",
            all_accounts.len(),
            empty_accounts.len(),
            reclaimable_sol,
            non_empty_count
        ),
    );

    success_response(AtaScanResponse {
        total_atas: all_accounts.len(),
        empty_count: empty_accounts.len(),
        non_empty_count,
        failed_count,
        reclaimable_sol,
        empty_atas,
    })
}

/// Get ATA cleanup statistics
pub async fn get_ata_stats() -> Response {
    let stats = get_ata_cleanup_statistics();
    let cached_failures = get_failed_ata_count();

    success_response(AtaStatsResponse {
        total_closed: stats.total_closed,
        total_rent_reclaimed: stats.total_rent_reclaimed,
        failed_attempts: stats.failed_attempts,
        cached_failures,
        last_cleanup_time: stats.last_cleanup_time,
    })
}

/// Execute ATA cleanup (close empty ATAs)
pub async fn cleanup_atas() -> Response {
    logger::info(LogTag::Wallet, "Manual ATA cleanup requested via API");

    match trigger_immediate_ata_cleanup().await {
        Ok((closed_count, signatures)) => {
            // Get updated stats for rent reclaimed
            let stats = get_ata_cleanup_statistics();

            success_response(AtaCleanupResponse {
                closed_count,
                failed_count: stats.failed_attempts,
                rent_reclaimed: stats.total_rent_reclaimed,
                signatures,
            })
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("ATA cleanup failed: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CLEANUP_ERROR",
                "Cleanup failed",
                Some(&e.to_string()),
            )
        }
    }
}

/// Clear the failed ATA cache to retry previously failed closures
pub async fn clear_ata_cache() -> Response {
    match clear_failed_ata_cache().await {
        Ok(()) => {
            logger::info(LogTag::Wallet, "Failed ATA cache cleared via API");
            success_response(serde_json::json!({
                "message": "Failed ATA cache cleared - previously failed ATAs will be retried"
            }))
        }
        Err(e) => {
            logger::error(LogTag::Wallet, &format!("Failed to clear ATA cache: {}", e));
            error_response(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                "Failed to clear cache",
                Some(&e.to_string()),
            )
        }
    }
}

// =============================================================================
// Wallet Generator Handlers
// =============================================================================

/// Generate a single new Solana keypair
pub async fn generate_keypair() -> Response {
    use solana_sdk::signature::Keypair;

    let keypair = Keypair::new();
    let pubkey = keypair.pubkey().to_string();
    let secret = bs58::encode(keypair.to_bytes()).into_string();

    logger::info(
        LogTag::Wallet,
        &format!("Generated new keypair via API: {}", pubkey),
    );

    success_response(KeypairResponse { pubkey, secret })
}

/// Generate multiple new Solana keypairs
pub async fn generate_keypairs(Json(request): Json<GenerateKeypairsRequest>) -> Response {
    use solana_sdk::signature::Keypair;

    // Limit to reasonable number
    let count = request.count.min(10);

    let keypairs: Vec<KeypairResponse> = (0..count)
        .map(|_| {
            let keypair = Keypair::new();
            KeypairResponse {
                pubkey: keypair.pubkey().to_string(),
                secret: bs58::encode(keypair.to_bytes()).into_string(),
            }
        })
        .collect();

    logger::info(
        LogTag::Wallet,
        &format!("Generated {} new keypairs via API", keypairs.len()),
    );

    success_response(keypairs)
}
