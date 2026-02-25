//! Main wallet operations (fast path)

use solana_sdk::signature::Keypair;

use super::super::types::Wallet;
use super::cache::{refresh_main_wallet_cache, MAIN_WALLET_CACHE};

/// Get the main wallet's keypair (cached for performance)
pub async fn get_main_keypair() -> Result<Keypair, String> {
    // Fast path: check cache first
    {
        let cache = MAIN_WALLET_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            // Clone the keypair bytes and reconstruct
            let bytes = cached.keypair.to_bytes();
            return Keypair::from_bytes(&bytes)
                .map_err(|e| format!("Failed to clone keypair: {e}"));
        }
    }

    // Cache miss - refresh and retry
    refresh_main_wallet_cache().await?;

    let cache = MAIN_WALLET_CACHE.read().await;
    if let Some(cached) = cache.as_ref() {
        let bytes = cached.keypair.to_bytes();
        return Keypair::from_bytes(&bytes).map_err(|e| format!("Failed to clone keypair: {e}"));
    }

    Err("No main wallet configured".to_owned())
}

/// Get the main wallet's address (cached for performance)
pub async fn get_main_address() -> Result<String, String> {
    // Fast path: check cache first
    {
        let cache = MAIN_WALLET_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            return Ok(cached.wallet.address.clone());
        }
    }

    // Cache miss - refresh and retry
    refresh_main_wallet_cache().await?;

    let cache = MAIN_WALLET_CACHE.read().await;
    cache
        .as_ref()
        .map(|c| c.wallet.address.clone())
        .ok_or_else(|| "No main wallet configured".to_owned())
}

/// Get the main wallet info
pub async fn get_main_wallet() -> Result<Option<Wallet>, String> {
    let cache = MAIN_WALLET_CACHE.read().await;
    Ok(cache.as_ref().map(|c| c.wallet.clone()))
}

/// Check if a main wallet is configured
pub async fn has_main_wallet() -> bool {
    MAIN_WALLET_CACHE.read().await.is_some()
}
