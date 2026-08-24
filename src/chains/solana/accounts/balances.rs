//! Solana wallet balance reads: Pubkey parsing, RPC SOL/token-account fetches,
//! and mapping the raw RPC wire shape into the chain-neutral `TokenBalance`.
//! Callers in `crate::wallets` own persistence, filtering and aggregation
//! policy; this module owns everything that touches an RPC client or a
//! `Pubkey` to produce that data.

use std::str::FromStr;

use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::{Error, Result};
use crate::wallets::TokenBalance;

/// Fetch a wallet's SOL balance (in SOL, not lamports). Returns 0.0 on any
/// RPC error, matching the caller's historical fallback behavior.
pub async fn fetch_wallet_sol_balance(address: &str) -> f64 {
    get_rpc_client()
        .get_sol_balance(address)
        .await
        .unwrap_or_default()
}

/// Fetch and normalize all non-NFT token balances for a wallet address.
/// `wallet_id` is stamped onto each `TokenBalance` for the caller's use
/// (database keys, cross-referencing) but is not itself chain data.
pub async fn fetch_wallet_token_balances(
    wallet_id: i64,
    address: &str,
) -> Result<Vec<TokenBalance>> {
    let wallet_pubkey = Pubkey::from_str(address).map_err(|_| Error::InvalidAddress {
        kind: "wallet",
        value: address.to_owned(),
    })?;

    let token_accounts = get_rpc_client()
        .get_all_token_accounts(&wallet_pubkey)
        .await
        .map_err(|e| Error::Rpc {
            operation: "get_all_token_accounts",
            detail: e.to_string(),
        })?;

    let now = chrono::Utc::now();
    Ok(token_accounts
        .iter()
        .filter(|acc| !acc.is_nft)
        .map(|acc| TokenBalance {
            wallet_id,
            mint: acc.mint.clone(),
            balance: acc.balance,
            ui_amount: acc.balance as f64 / 10f64.powi(acc.decimals as i32),
            decimals: acc.decimals,
            symbol: None,
            name: None,
            is_token_2022: acc.is_token_2022,
            updated_at: now,
        })
        .collect())
}
