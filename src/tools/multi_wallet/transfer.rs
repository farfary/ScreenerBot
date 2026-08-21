//! Bulk transfer orchestration for multi-wallet tooling.
//!
//! Loops over many wallets, calling the single-transfer mechanics in
//! `crate::chains::solana::assets::transfer` for each one and aggregating
//! results. SOL/token transfer and ATA-close construction/submission
//! themselves live there, not here.

use futures::stream::{self, StreamExt};
use tokio::time::{sleep, Duration};

use crate::chains::solana::assets::transfer::{transfer_sol_for_wallet, transfer_sol_from_main};
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::logger::{self, LogTag};
use crate::utils::lamports_to_sol;
use crate::wallets::Wallet;

use super::types::WalletOpResult;

/// Minimum rent-exempt balance for accounts (~0.00089 SOL)
pub const RENT_EXEMPT_MINIMUM: u64 = 890_880;

// =============================================================================
// BULK FUNDING
// =============================================================================

/// Fund multiple wallets from the main wallet.
///
/// # Arguments
/// * `targets` - List of (address, amount_sol) tuples
/// * `concurrency` - Number of concurrent transfers
///
/// # Returns
/// List of operation results
pub async fn fund_wallets(targets: Vec<(String, f64)>, concurrency: usize) -> Vec<WalletOpResult> {
    if targets.is_empty() {
        return Vec::new();
    }

    let concurrency = concurrency.max(1);

    logger::info(
        LogTag::Tools,
        &format!(
            "Funding {} wallets with concurrency {}",
            targets.len(),
            concurrency
        ),
    );

    let results: Vec<WalletOpResult> = stream::iter(targets)
        .map(|(address, amount)| async move {
            match transfer_sol_from_main(&address, amount).await {
                Ok(sig) => WalletOpResult::success(0, address, sig, amount, None),
                Err(e) => WalletOpResult::failure(0, address, e),
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let success_count = results.iter().filter(|r| r.success).count();
    let total_funded: f64 = results
        .iter()
        .filter(|r| r.success)
        .filter_map(|r| r.amount_sol)
        .sum();

    logger::info(
        LogTag::Tools,
        &format!(
            "Funding complete: {}/{} successful, {:.6} SOL transferred",
            success_count,
            results.len(),
            total_funded
        ),
    );

    results
}

// =============================================================================
// SOL COLLECTION
// =============================================================================

/// Collect SOL from multiple wallets to a destination
///
/// # Arguments
/// * `wallets` - List of wallets to collect from
/// * `to_address` - Destination wallet address
/// * `leave_rent` - Whether to leave rent-exempt minimum in source wallets
///
/// # Returns
/// List of operation results
pub async fn collect_sol(
    wallets: Vec<Wallet>,
    to_address: &str,
    leave_rent: bool,
) -> Vec<WalletOpResult> {
    if wallets.is_empty() {
        return Vec::new();
    }

    let rpc_client = get_rpc_client();

    logger::info(
        LogTag::Tools,
        &format!(
            "Collecting SOL from {} wallets to {}",
            wallets.len(),
            &to_address[..8]
        ),
    );

    let mut results = Vec::new();

    for wallet in wallets {
        let wallet_id = wallet.id;
        let wallet_address = wallet.address.clone();

        // Get current balance
        let balance = match rpc_client.get_sol_balance(&wallet_address).await {
            Ok(b) => b,
            Err(e) => {
                results.push(WalletOpResult::failure(
                    wallet_id,
                    wallet_address,
                    format!("Failed to get balance: {e}"),
                ));
                continue;
            }
        };

        // Calculate transfer amount
        let rent_reserve = if leave_rent {
            lamports_to_sol(RENT_EXEMPT_MINIMUM)
        } else {
            0.0
        };

        // Estimate transaction fee (~5000 lamports)
        let tx_fee = 0.000005;
        let transfer_amount = balance - rent_reserve - tx_fee;

        if transfer_amount <= 0.0 {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "Wallet {} has insufficient balance ({:.6} SOL) for collection",
                    &wallet_address[..8],
                    balance
                ),
            );
            continue;
        }

        // Execute transfer
        match transfer_sol_for_wallet(wallet_id, to_address, transfer_amount).await {
            Ok(sig) => {
                results.push(WalletOpResult::success(
                    wallet_id,
                    wallet_address,
                    sig,
                    transfer_amount,
                    None,
                ));
            }
            Err(e) => {
                results.push(WalletOpResult::failure(wallet_id, wallet_address, e));
            }
        }

        // Small delay between transfers to avoid rate limiting
        sleep(Duration::from_millis(100)).await;
    }

    let success_count = results.iter().filter(|r| r.success).count();
    let total_collected: f64 = results
        .iter()
        .filter(|r| r.success)
        .filter_map(|r| r.amount_sol)
        .sum();

    logger::info(
        LogTag::Tools,
        &format!(
            "Collection complete: {}/{} successful, {:.6} SOL collected",
            success_count,
            results.len(),
            total_collected
        ),
    );

    results
}
