// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet cache — in-memory main-wallet record for fast access without a
//! database query. Holds only the chain-neutral `Wallet` record; the
//! decrypted keypair cache lives in `crate::chains::solana::accounts::signing`,
//! which never needs invalidating because the trading wallet is fixed for the
//! life of the process.

use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

use super::super::error::Error;
use super::super::types::Wallet;

/// Cached main wallet record
pub(super) static MAIN_WALLET_CACHE: LazyLock<Arc<RwLock<Option<Wallet>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Refresh the cached main wallet record
pub(super) async fn refresh_main_wallet_cache() -> Result<(), Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

    let main_wallet = db.get_main_wallet()?;

    let mut cache = MAIN_WALLET_CACHE.write().await;
    *cache = main_wallet;

    Ok(())
}
