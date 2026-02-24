use solana_sdk::signature::Keypair;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

use super::super::crypto::decrypt_to_keypair;
use super::super::types::Wallet;

/// Cached main wallet keypair for fast access
pub(super) static MAIN_WALLET_CACHE: LazyLock<Arc<RwLock<Option<CachedMainWallet>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Cached main wallet data
pub(super) struct CachedMainWallet {
    pub(super) wallet: Wallet,
    pub(super) keypair: Keypair,
}

/// Refresh the cached main wallet
pub(super) async fn refresh_main_wallet_cache() -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let main_wallet = match db.get_main_wallet()? {
        Some(w) => w,
        None => {
            let mut cache = MAIN_WALLET_CACHE.write().await;
            *cache = None;
            return Ok(());
        }
    };

    let (encrypted, nonce) = db
        .get_main_wallet_encrypted_key()?
        .ok_or("Main wallet encrypted key not found")?;

    let keypair = decrypt_to_keypair(&encrypted, &nonce)?;

    let mut cache = MAIN_WALLET_CACHE.write().await;
    *cache = Some(CachedMainWallet {
        wallet: main_wallet,
        keypair,
    });

    Ok(())
}
