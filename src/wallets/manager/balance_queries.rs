//! Wallet balance queries — query stored balance data with filtering and aggregation.

use crate::chains::adapter;
use crate::chains::solana::accounts::{fetch_wallet_sol_balance, fetch_wallet_token_balances};
use crate::chains::solana::constants::RENT_EXEMPT_MINIMUM_LAMPORTS;
use crate::logger::{self, LogTag};

use super::super::types::{SimpleTokenBalance, WalletBalanceSummary, WalletWithTokenBalance};
use super::list_active_wallets;

// =============================================================================
// BALANCE CONSTANTS
// =============================================================================

/// Minimum SOL balance to operate a wallet (for transaction fees)
const MIN_SOL_FOR_OPERATIONS: f64 = 0.005;

fn sol_topup_needed(sol_balance: f64) -> (bool, f64) {
    if sol_balance < MIN_SOL_FOR_OPERATIONS {
        (true, MIN_SOL_FOR_OPERATIONS - sol_balance)
    } else {
        (false, 0.0)
    }
}

fn reclaimable_ata_rent(empty_ata_count: u32) -> f64 {
    let ata_rent_exempt = adapter().raw_to_native(RENT_EXEMPT_MINIMUM_LAMPORTS);
    empty_ata_count as f64 * ata_rent_exempt
}

/// Get all active wallets that hold a specific token
///
/// # Arguments
/// * `token_mint` - Token mint address to search for
/// * `min_balance` - Optional minimum token balance filter (UI amount, not raw)
///
/// # Returns
/// Vector of wallets with their SOL and token balances
pub async fn get_wallets_with_token(
    token_mint: &str,
    min_balance: Option<f64>,
) -> Result<Vec<WalletWithTokenBalance>, String> {
    let wallets = list_active_wallets().await?;
    let min_balance = min_balance.unwrap_or_default();

    let mut results = Vec::new();

    for wallet in wallets {
        let token_balances = match fetch_wallet_token_balances(wallet.id, &wallet.address).await {
            Ok(balances) => balances,
            Err(_) => continue,
        };

        // Find the specific token
        if let Some(token_balance) = token_balances.iter().find(|b| b.mint == token_mint) {
            // Apply minimum balance filter
            if token_balance.ui_amount >= min_balance {
                let sol_balance = fetch_wallet_sol_balance(&wallet.address).await;
                let (needs_sol_topup, topup_amount) = sol_topup_needed(sol_balance);

                results.push(WalletWithTokenBalance {
                    wallet: wallet.clone(),
                    sol_balance,
                    token_balance: token_balance.ui_amount,
                    token_decimals: token_balance.decimals,
                    needs_sol_topup,
                    topup_amount,
                });
            }
        }
    }

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Found {} wallets holding token {} (min_balance: {})",
            results.len(),
            token_mint,
            min_balance
        ),
    );

    Ok(results)
}

/// Get balance summaries for all sub-wallets (non-primary)
///
/// Returns comprehensive balance information for wallet consolidation UI.
/// Excludes the main wallet from results.
///
/// # Returns
/// Vector of wallet balance summaries with SOL, token counts, and reclaimable rent
pub async fn get_all_wallet_balances() -> Result<Vec<WalletBalanceSummary>, String> {
    let wallets = list_active_wallets().await?;

    let mut results = Vec::new();

    for wallet in wallets {
        // Skip main wallet
        if wallet.is_main() {
            continue;
        }

        // Get SOL balance
        let sol_balance = fetch_wallet_sol_balance(&wallet.address).await;

        // Get all token balances for this wallet
        let token_balances = match fetch_wallet_token_balances(wallet.id, &wallet.address).await {
            Ok(balances) => balances,
            Err(e) => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Failed to get token accounts for {} ({}): {}",
                        wallet.name, wallet.address, e
                    ),
                );
                // Still include wallet with 0 token info
                results.push(WalletBalanceSummary {
                    wallet_id: wallet.id,
                    wallet_name: wallet.name.clone(),
                    address: wallet.address.clone(),
                    sol_balance,
                    token_count: 0,
                    tokens: Vec::new(),
                    empty_ata_count: 0,
                    reclaimable_sol: 0.0,
                });
                continue;
            }
        };

        let mut tokens = Vec::new();
        let mut empty_ata_count = 0u32;

        for token_balance in token_balances {
            if token_balance.balance == 0 {
                empty_ata_count += 1;
            } else {
                tokens.push(SimpleTokenBalance {
                    mint: token_balance.mint,
                    symbol: None, // Could be populated from token service if needed
                    balance: token_balance.ui_amount,
                    decimals: token_balance.decimals,
                });
            }
        }

        let token_count = tokens.len() as u32;
        let reclaimable_sol = reclaimable_ata_rent(empty_ata_count);

        results.push(WalletBalanceSummary {
            wallet_id: wallet.id,
            wallet_name: wallet.name.clone(),
            address: wallet.address.clone(),
            sol_balance,
            token_count,
            tokens,
            empty_ata_count,
            reclaimable_sol,
        });
    }

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Retrieved balance summaries for {} sub-wallets",
            results.len()
        ),
    );

    Ok(results)
}
