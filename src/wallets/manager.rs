// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet Manager
//!
//! Core wallet management functionality with caching and thread-safe operations.

use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

use super::database::WalletsDatabase;
use super::error::Error;
use crate::{
    chains::active_chain,
    logger::{self, LogTag},
};

mod cache;
use cache::refresh_main_wallet_cache;

mod main_wallet;
pub(crate) use main_wallet::get_main_wallet_encrypted_key;
pub use main_wallet::{get_main_address, get_main_wallet, has_main_wallet};

mod balance_queries;
pub use balance_queries::get_wallet_sol_balances;

mod migration;

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global wallet database instance
static WALLETS_DB: LazyLock<Arc<RwLock<Option<WalletsDatabase>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize the wallet manager
///
/// Must be called once at startup before using any wallet functions
pub async fn initialize() -> Result<(), Error> {
    let db = WalletsDatabase::new(active_chain())?;

    {
        let mut guard = WALLETS_DB.write().await;
        *guard = Some(db);
    }

    // Try to migrate from config.toml if no wallets exist
    migration::migrate_from_config().await?;

    // Cache main wallet
    refresh_main_wallet_cache().await?;

    logger::info(LogTag::Wallet, "Wallet manager initialized");
    Ok(())
}

/// Check if wallet manager is initialized
pub async fn is_initialized() -> bool {
    WALLETS_DB.read().await.is_some()
}

// =============================================================================
// WALLET ACCESS AND KEY EXPORT
// =============================================================================

mod export;
pub use export::export_wallet;

mod access;
pub(crate) use access::get_wallet_encrypted_key;
pub use access::{get_wallet, list_active_wallets, list_wallets};

// =============================================================================
// TOOLS & SUMMARY
// =============================================================================

mod tools;
pub use tools::get_wallets_summary;
