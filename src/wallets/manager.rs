//! Wallet Manager
//!
//! Core wallet management functionality with caching and thread-safe operations.

use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

use super::database::WalletsDatabase;
use super::error::Error;
use crate::errors::InternalError;
use crate::{
    chains::active_chain,
    logger::{self, LogTag},
};

/// The wallet database has not been initialized yet — an invariant violation,
/// not a not-found: `initialize()` must run once at startup before any other
/// wallet function is called.
pub(super) fn db_not_initialized() -> Error {
    Error::Internal(InternalError::InvariantViolation {
        message: "wallet database not initialized".to_owned(),
    })
}

mod cache;
use cache::refresh_main_wallet_cache;

mod main_wallet;
pub(crate) use main_wallet::get_main_wallet_encrypted_key;
pub use main_wallet::{get_main_address, get_main_wallet, has_main_wallet};

mod balance_queries;
pub use balance_queries::{get_all_wallet_balances, get_wallets_with_token};

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
// WALLET CRUD OPERATIONS
// =============================================================================

mod crud;
pub use crud::{
    archive_wallet, create_wallet, delete_wallet, export_wallet, import_wallet, restore_wallet,
    set_main_wallet, update_wallet,
};

mod access;
pub(crate) use access::get_wallet_encrypted_key;
pub use access::{get_wallet, get_wallet_by_address, list_active_wallets, list_wallets};

// =============================================================================
// BULK IMPORT/EXPORT
// =============================================================================

mod bulk_ops;
pub use bulk_ops::{
    bulk_import_wallets, create_wallets_batch, export_wallets, get_existing_wallet_addresses,
};

// =============================================================================
// TOKEN BALANCE OPERATIONS
// =============================================================================

mod balance_ops;
pub use balance_ops::{
    clear_token_balances, get_all_token_balances, get_token_balances, update_all_wallet_balances,
    update_wallet_balances, upsert_token_balance,
};

// =============================================================================
// TOOLS & SUMMARY
// =============================================================================

mod tools;
pub use tools::get_wallets_summary;
