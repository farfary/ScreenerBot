//! Wallet balance operations — fetch and refresh on-chain token balances via RPC.

use std::collections::HashMap;

use super::super::error::Error;
use super::super::types::TokenBalance;
use crate::chains::solana::accounts::fetch_wallet_token_balances;
use crate::logger::{self, LogTag};

/// Update token balances for a wallet by fetching from RPC
///
/// Fetches all token accounts for the wallet and caches them in the database.
/// Returns the number of tokens updated.
pub async fn update_wallet_balances(wallet_id: i64) -> Result<usize, Error> {
    // Get wallet address
    let wallet = super::get_wallet(wallet_id)
        .await?
        .ok_or(Error::WalletNotFound {
            address: format!("id={wallet_id}"),
        })?;

    let balances = fetch_wallet_token_balances(wallet_id, &wallet.address)
        .await
        .map_err(|detail| Error::BalanceUpdate {
            address: wallet.address.clone(),
            detail: detail.to_string(),
        })?;
    let count = balances.len();

    // Bulk update in database
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

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
pub async fn update_all_wallet_balances() -> Result<HashMap<i64, usize>, Error> {
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
pub async fn get_token_balances(wallet_id: i64) -> Result<Vec<TokenBalance>, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

    db.get_token_balances(wallet_id)
}

/// Get cached token balances for all wallets
pub async fn get_all_token_balances() -> Result<HashMap<i64, Vec<TokenBalance>>, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

    db.get_all_token_balances()
}

/// Clear cached token balances for a wallet
pub async fn clear_token_balances(wallet_id: i64) -> Result<u64, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

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
) -> Result<(), Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

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
