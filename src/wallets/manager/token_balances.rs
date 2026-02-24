use std::collections::HashMap;
use std::str::FromStr;

use super::super::types::TokenBalance;
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};

/// Update token balances for a wallet by fetching from RPC
///
/// Fetches all token accounts for the wallet and caches them in the database.
/// Returns the number of tokens updated.
pub async fn update_wallet_balances(wallet_id: i64) -> Result<usize, String> {
    // Get wallet address
    let wallet = super::get_wallet(wallet_id).await?.ok_or("Wallet not found")?;

    let wallet_pubkey = solana_sdk::pubkey::Pubkey::from_str(&wallet.address)
        .map_err(|e| format!("Invalid wallet address: {e}"))?;

    let rpc_client = get_rpc_client();
    let token_accounts = rpc_client
        .get_all_token_accounts(&wallet_pubkey)
        .await
        .map_err(|e| format!("Failed to fetch token accounts: {e}"))?;

    // Convert to TokenBalance structs
    let now = chrono::Utc::now();
    let balances: Vec<TokenBalance> = token_accounts
        .iter()
        .filter(|acc| !acc.is_nft) // Exclude NFTs
        .map(|acc| {
            let ui_amount = acc.balance as f64 / 10f64.powi(acc.decimals as i32);
            TokenBalance {
                wallet_id,
                mint: acc.mint.clone(),
                balance: acc.balance,
                ui_amount,
                decimals: acc.decimals,
                symbol: None, // Will be populated by token service if available
                name: None,
                is_token_2022: acc.is_token_2022,
                updated_at: now,
            }
        })
        .collect();

    let count = balances.len();

    // Bulk update in database
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.update_balances_bulk(wallet_id, &balances)?;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Updated {} token balances for wallet {} ({})",
            count, wallet.name, wallet.address
        ),
    );

    Ok(count)
}

/// Update token balances for all active wallets
///
/// Returns a map of wallet_id -> number of tokens updated
pub async fn update_all_wallet_balances() -> Result<HashMap<i64, usize>, String> {
    let wallets = super::list_active_wallets().await?;
    let mut results = HashMap::new();

    for wallet in wallets {
        match update_wallet_balances(wallet.id).await {
            Ok(count) => {
                results.insert(wallet.id, count);
            }
            Err(e) => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Failed to update balances for wallet {} ({}): {}",
                        wallet.name, wallet.address, e
                    ),
                );
            }
        }
    }

    Ok(results)
}

/// Get cached token balances for a wallet
pub async fn get_token_balances(wallet_id: i64) -> Result<Vec<TokenBalance>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.get_token_balances(wallet_id)
}

/// Get cached token balances for all wallets
pub async fn get_all_token_balances() -> Result<HashMap<i64, Vec<TokenBalance>>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.get_all_token_balances()
}

/// Clear cached token balances for a wallet
pub async fn clear_token_balances(wallet_id: i64) -> Result<u64, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.clear_token_balances(wallet_id)
}

/// Upsert a single token balance (for incremental updates)
pub async fn upsert_token_balance(
    wallet_id: i64,
    mint: &str,
    balance: u64,
    ui_amount: f64,
    decimals: u8,
    symbol: Option<&str>,
    name: Option<&str>,
    is_token_2022: bool,
) -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.upsert_token_balance(
        wallet_id,
        mint,
        balance,
        ui_amount,
        decimals,
        symbol,
        name,
        is_token_2022,
    )
}
