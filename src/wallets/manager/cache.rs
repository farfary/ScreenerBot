//! Wallet cache — in-memory main-wallet record for fast access without a
//! database query. Holds only the chain-neutral `Wallet` record; the
//! decrypted keypair cache lives in `crate::chains::solana::accounts::signing`
//! and is invalidated separately (see call sites in `crud.rs`).

use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

use super::super::types::Wallet;

/// Cached main wallet record
pub(super) static MAIN_WALLET_CACHE: LazyLock<Arc<RwLock<Option<Wallet>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Refresh the cached main wallet record
pub(super) async fn refresh_main_wallet_cache() -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let main_wallet = db.get_main_wallet()?;

    let mut cache = MAIN_WALLET_CACHE.write().await;
    *cache = main_wallet;

    Ok(())
}
