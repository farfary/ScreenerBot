// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet balance queries — the listed wallets' SOL balances, read live over RPC.

use crate::chains::solana::accounts::fetch_wallet_sol_balances;

use super::super::types::Wallet;

/// SOL balance of each wallet, aligned with `wallets`, read in batched account
/// calls; `None` where the balance could not be read.
pub async fn get_wallet_sol_balances(wallets: &[Wallet]) -> Vec<Option<f64>> {
    let addresses: Vec<String> = wallets
        .iter()
        .map(|wallet| wallet.address.clone())
        .collect();
    fetch_wallet_sol_balances(&addresses).await
}
