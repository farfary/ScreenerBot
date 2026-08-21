//! Wallet-credential resolution and the one cache that may ever hold a
//! decrypted Solana keypair in memory.
//!
//! Shared code (`crate::wallets`, `crate::config::wallet`, `crate::tools`,
//! the webserver) never sees a `Keypair`: it passes a `wallet_id` (or relies
//! on "the main wallet") to the functions here, which resolve the wallet's
//! encrypted credential via `crate::wallets` (ciphertext + nonce only, never
//! Solana-typed), decrypt, use, and drop. The one exception is the main
//! wallet, cached here — not in `crate::wallets` — because it is read on
//! every trade; the cache is invalidated whenever wallet manager mutates
//! wallet records (see `invalidate_main_wallet_cache`).

use std::sync::LazyLock;
use tokio::sync::RwLock;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::solana_sdk::signature::{Keypair, Signer};

use super::keypair::decrypt_to_keypair;

struct CachedMainKeypair {
    wallet_id: i64,
    keypair: Keypair,
}

static MAIN_KEYPAIR_CACHE: LazyLock<RwLock<Option<CachedMainKeypair>>> =
    LazyLock::new(|| RwLock::new(None));

fn clone_keypair(keypair: &Keypair) -> Result<Keypair, String> {
    Keypair::from_bytes(&keypair.to_bytes()).map_err(|e| format!("Failed to clone keypair: {e}"))
}

/// Drop the cached main-wallet keypair. Called by `crate::wallets::manager`
/// after any mutation that could change which wallet is main or its key
/// material (create/import/set-main). The next `main_keypair()` call
/// re-decrypts from the freshly written record.
pub async fn invalidate_main_wallet_cache() {
    *MAIN_KEYPAIR_CACHE.write().await = None;
}

/// The main wallet's keypair, decrypted on first use and cached until the
/// next mutation. Every caller gets its own clone; nothing outside this
/// module ever holds the cached original.
pub async fn main_keypair() -> Result<Keypair, String> {
    {
        let cache = MAIN_KEYPAIR_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            return clone_keypair(&cached.keypair);
        }
    }

    let (wallet_id, ciphertext, nonce) = crate::wallets::get_main_wallet_encrypted_key()
        .await?
        .ok_or_else(|| "No main wallet configured".to_owned())?;

    let keypair = decrypt_to_keypair(&ciphertext, &nonce)?;
    let clone = clone_keypair(&keypair)?;

    *MAIN_KEYPAIR_CACHE.write().await = Some(CachedMainKeypair { wallet_id, keypair });
    Ok(clone)
}

/// The main wallet's database ID, if a keypair is currently cached — cheap
/// identity check that never touches key material.
pub async fn cached_main_wallet_id() -> Option<i64> {
    MAIN_KEYPAIR_CACHE
        .read()
        .await
        .as_ref()
        .map(|c| c.wallet_id)
}

/// Decrypt a specific wallet's keypair by its database ID. Not cached — used
/// by one-shot multi-wallet tooling, never on the per-trade hot path.
pub async fn keypair_for_wallet(wallet_id: i64) -> Result<Keypair, String> {
    let (ciphertext, nonce) = crate::wallets::get_wallet_encrypted_key(wallet_id).await?;
    decrypt_to_keypair(&ciphertext, &nonce)
}

/// Sign an arbitrary text message with the main wallet. The one place the
/// bot signs something that is not a transaction: proving wallet ownership
/// to screenerbot.io during account sign-in.
pub async fn sign_message_with_main_wallet(message: &str) -> Result<String, String> {
    let keypair = main_keypair().await?;
    Ok(keypair.sign_message(message.as_bytes()).to_string())
}

/// Sign a per-address message with every active wallet's key, for the
/// referral activation announce: proves ownership of every held wallet
/// without ever handing the keys themselves to the caller. A wallet whose
/// key fails to decrypt is skipped (and logged), not fatal to the batch.
pub async fn sign_message_for_active_wallets(
    message_for: impl Fn(&str) -> String,
) -> Result<Vec<(String, String)>, String> {
    let wallets = crate::wallets::list_active_wallets().await?;
    let mut signed = Vec::with_capacity(wallets.len());

    for wallet in wallets {
        match keypair_for_wallet(wallet.id).await {
            Ok(keypair) => {
                let message = message_for(&wallet.address);
                signed.push((
                    wallet.address,
                    keypair.sign_message(message.as_bytes()).to_string(),
                ));
            }
            Err(err) => {
                crate::logger::warning(
                    crate::logger::LogTag::Wallet,
                    &format!(
                        "Skipping wallet id={} ({}): {err}",
                        wallet.id, wallet.address
                    ),
                );
            }
        }
    }

    Ok(signed)
}

// =============================================================================
// CONFIGURED WALLET (sync bridge for early-startup / non-async callers)
// =============================================================================

/// Async form of [`configured_keypair`], for callers already running on an
/// executor. Prefer this over the sync bridge when the caller is `async fn`.
pub async fn configured_keypair_async() -> Result<Keypair, String> {
    if crate::wallets::is_initialized().await {
        return main_keypair().await;
    }
    configured_keypair_from_legacy_config()
}

/// The configured trading wallet's keypair — the multi-wallet database's
/// main wallet once it's initialized, else the legacy single-wallet keypair
/// straight from `config.toml`. Callable from sync or async context: when an
/// executor is running, this drives the async lookup with a bare
/// `futures::executor::block_on` rather than Tokio's `block_in_place` (which
/// requires the multi-thread runtime and panics under `#[tokio::test]`'s
/// current-thread flavor, or any future current-thread executor). Safe
/// because the awaited work here is only short-lived lock acquisition
/// (`RwLock`/`Mutex`), never I/O or timers that depend on being polled by
/// the enclosing runtime's reactor.
pub fn configured_keypair() -> Result<Keypair, String> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return futures::executor::block_on(configured_keypair_async());
    }

    configured_keypair_from_legacy_config()
}

/// Legacy fallback: decrypt the single wallet stored directly in
/// `config.toml`, used only before the multi-wallet database initializes.
fn configured_keypair_from_legacy_config() -> Result<Keypair, String> {
    crate::config::with_config(|cfg| {
        if cfg.wallet_encrypted.is_empty() || cfg.wallet_nonce.is_empty() {
            return Err("Wallet not configured - encrypted private key is missing".to_owned());
        }

        let encrypted = crate::secure_storage::EncryptedData {
            ciphertext: cfg.wallet_encrypted.clone(),
            nonce: cfg.wallet_nonce.clone(),
        };

        let private_key = crate::secure_storage::decrypt_private_key(&encrypted)
            .map_err(|e| format!("Failed to decrypt wallet: {e}"))?;

        super::keypair::parse_private_key(&private_key)
    })
}

/// The configured trading wallet's public key.
pub fn configured_pubkey() -> Result<Pubkey, String> {
    configured_keypair().map(|kp| kp.pubkey())
}

/// The configured trading wallet's address as a base58 string — the only
/// one of this trio that shared code outside `crate::chains::solana` may
/// call (`crate::config::get_wallet_pubkey_string`).
pub fn configured_address() -> Result<String, String> {
    configured_pubkey().map(|pk| pk.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_keypair_round_trips_the_public_key() {
        let original = Keypair::new();
        let cloned = clone_keypair(&original).unwrap();
        assert_eq!(original.pubkey(), cloned.pubkey());
    }
}
