//! Main wallet operations (fast path)
//!
//! Never decrypts. `crate::chains::solana::accounts::signing` owns the main
//! wallet's decrypted-keypair cache; this module only caches the record.

use super::super::types::Wallet;
use super::cache::{refresh_main_wallet_cache, MAIN_WALLET_CACHE};

/// Get the main wallet's address (cached for performance)
pub async fn get_main_address() -> Result<String, String> {
    // Fast path: check cache first
    {
        let cache = MAIN_WALLET_CACHE.read().await;
        if let Some(wallet) = cache.as_ref() {
            return Ok(wallet.address.clone());
        }
    }

    // Cache miss - refresh and retry
    refresh_main_wallet_cache().await?;

    let cache = MAIN_WALLET_CACHE.read().await;
    cache
        .as_ref()
        .map(|w| w.address.clone())
        .ok_or_else(|| "No main wallet configured".to_owned())
}

/// Get the main wallet info
pub async fn get_main_wallet() -> Result<Option<Wallet>, String> {
    let mut wallet = { MAIN_WALLET_CACHE.read().await.clone() };

    // Derive "last used" from the most recent recorded transaction (canonical
    // source of wallet activity) rather than the rarely-written stored column.
    if let Some(wallet) = wallet.as_mut() {
        super::access::enrich_last_used(wallet).await;
    }

    Ok(wallet)
}

/// Check if a main wallet is configured
pub async fn has_main_wallet() -> bool {
    MAIN_WALLET_CACHE.read().await.is_some()
}

/// The main wallet's ID plus its encrypted key material (ciphertext, nonce).
/// Chain-neutral — only `crate::chains::solana::accounts` decrypts this.
pub(crate) async fn get_main_wallet_encrypted_key() -> Result<Option<(i64, String, String)>, String>
{
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let Some(wallet) = db.get_main_wallet()? else {
        return Ok(None);
    };
    let Some((ciphertext, nonce)) = db.get_main_wallet_encrypted_key()? else {
        return Ok(None);
    };

    Ok(Some((wallet.id, ciphertext, nonce)))
}
