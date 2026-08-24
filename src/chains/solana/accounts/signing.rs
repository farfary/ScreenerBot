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
use crate::chains::solana::{Error, Result};

use super::keypair::decrypt_to_keypair;

struct CachedMainKeypair {
    wallet_id: i64,
    keypair: Keypair,
}

static MAIN_KEYPAIR_CACHE: LazyLock<RwLock<Option<CachedMainKeypair>>> =
    LazyLock::new(|| RwLock::new(None));

fn clone_keypair(keypair: &Keypair) -> Result<Keypair> {
    Keypair::from_bytes(&keypair.to_bytes()).map_err(|e| Error::InvalidKeypair {
        detail: format!("failed to clone keypair: {e}"),
    })
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
pub async fn main_keypair() -> Result<Keypair> {
    {
        let cache = MAIN_KEYPAIR_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            return clone_keypair(&cached.keypair);
        }
    }

    let (wallet_id, ciphertext, nonce) = crate::wallets::get_main_wallet_encrypted_key()
        .await
        .map_err(|e| Error::KeypairUnavailable {
            detail: e.to_string(),
        })?
        .ok_or_else(|| Error::KeypairUnavailable {
            detail: "no main wallet configured".to_owned(),
        })?;

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
pub async fn keypair_for_wallet(wallet_id: i64) -> Result<Keypair> {
    let (ciphertext, nonce) = crate::wallets::get_wallet_encrypted_key(wallet_id)
        .await
        .map_err(|e| Error::KeypairUnavailable {
            detail: e.to_string(),
        })?;
    decrypt_to_keypair(&ciphertext, &nonce)
}

/// Sign an arbitrary text message with the main wallet. The one place the
/// bot signs something that is not a transaction: proving wallet ownership
/// to screenerbot.io during account sign-in.
pub async fn sign_message_with_main_wallet(message: &str) -> Result<String> {
    let keypair = main_keypair().await?;
    Ok(keypair.sign_message(message.as_bytes()).to_string())
}

/// Sign a per-address message with every active wallet's key, for the
/// referral activation announce: proves ownership of every held wallet
/// without ever handing the keys themselves to the caller. A wallet whose
/// key fails to decrypt is skipped (and logged), not fatal to the batch.
pub async fn sign_message_for_active_wallets(
    message_for: impl Fn(&str) -> String,
) -> Result<Vec<(String, String)>> {
    let wallets =
        crate::wallets::list_active_wallets()
            .await
            .map_err(|e| Error::KeypairUnavailable {
                detail: e.to_string(),
            })?;
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
pub async fn configured_keypair_async() -> Result<Keypair> {
    if crate::wallets::is_initialized().await {
        return main_keypair().await;
    }
    configured_keypair_from_legacy_config()
}

/// Async, key-free form of [`configured_address`]. Once the multi-wallet
/// database is initialized this never touches `main_keypair()` or
/// `MAIN_KEYPAIR_CACHE` — it reads `crate::wallets::get_main_address()`,
/// which resolves the cached wallet record only. Any read-only caller that
/// wants an address (dashboard, wallet-monitor snapshots/metrics) should
/// prefer this over `configured_pubkey`/`configured_keypair`, which force a
/// decrypt.
pub async fn configured_address_async() -> Result<String> {
    if crate::wallets::is_initialized().await {
        return crate::wallets::get_main_address()
            .await
            .map_err(|e| Error::KeypairUnavailable {
                detail: e.to_string(),
            });
    }
    configured_keypair_from_legacy_config().map(|kp| kp.pubkey().to_string())
}

/// The configured trading wallet's keypair — the multi-wallet database's
/// main wallet once it's initialized, else the legacy single-wallet keypair
/// straight from `config.toml`. Callable from sync or async context.
///
/// On a multi-thread Tokio runtime this drives the async lookup via
/// `tokio::task::block_in_place`, which tells the runtime this worker thread
/// is about to block so it can move the thread's other queued tasks
/// elsewhere before `futures::executor::block_on` parks it — the safe way to
/// wait synchronously without starving the runtime.
///
/// `block_in_place` panics on a current-thread runtime (there is no other
/// worker to move work to), and blocking there with a bare `block_on` risks
/// a real deadlock if the awaited path ever needs a task that only the
/// current thread could poll. Rather than gamble on that never happening,
/// a current-thread caller gets a clear error telling it to use
/// `configured_keypair_async()` instead — never a hang.
pub fn configured_keypair() -> Result<Keypair> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => tokio::task::block_in_place(|| {
                futures::executor::block_on(configured_keypair_async())
            }),
            _ => Err(Error::KeypairUnavailable {
                detail: "configured_keypair() cannot run synchronously on a current-thread \
                    Tokio runtime (would risk a deadlock) - call configured_keypair_async() \
                    instead"
                    .to_owned(),
            }),
        },
        Err(_) => configured_keypair_from_legacy_config(),
    }
}

/// Legacy fallback: decrypt the single wallet stored directly in
/// `config.toml`, used only before the multi-wallet database initializes.
fn configured_keypair_from_legacy_config() -> Result<Keypair> {
    crate::config::with_config(|cfg| {
        if cfg.wallet_encrypted.is_empty() || cfg.wallet_nonce.is_empty() {
            return Err(Error::KeypairUnavailable {
                detail: "wallet not configured - encrypted private key is missing".to_owned(),
            });
        }

        let encrypted = crate::secure_storage::EncryptedData {
            ciphertext: cfg.wallet_encrypted.clone(),
            nonce: cfg.wallet_nonce.clone(),
        };

        let private_key = crate::secure_storage::decrypt_private_key(&encrypted).map_err(|e| {
            Error::KeypairUnavailable {
                detail: format!("failed to decrypt wallet: {e}"),
            }
        })?;

        super::keypair::parse_private_key(&private_key)
    })
}

/// The configured trading wallet's public key, parsed from
/// [`configured_address`] — never decrypts a keypair merely to derive a
/// public key that is already recoverable from the stored address string.
pub fn configured_pubkey() -> Result<Pubkey> {
    use std::str::FromStr;

    let address = configured_address()?;
    Pubkey::from_str(&address).map_err(|_| Error::InvalidAddress {
        kind: "wallet",
        value: address,
    })
}

/// The configured trading wallet's address as a base58 string — the only
/// one of this trio that shared code outside `crate::chains::solana` may
/// call (`crate::config::get_wallet_pubkey_string`).
///
/// Key-free once the multi-wallet database is initialized (see
/// `configured_address_async`); only the legacy pre-multi-wallet fallback
/// decrypts. Same runtime-flavor bridging as `configured_keypair`.
pub fn configured_address() -> Result<String> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => tokio::task::block_in_place(|| {
                futures::executor::block_on(configured_address_async())
            }),
            _ => Err(Error::KeypairUnavailable {
                detail: "configured_address() cannot run synchronously on a current-thread \
                    Tokio runtime (would risk a deadlock) - call configured_address_async() \
                    instead"
                    .to_owned(),
            }),
        },
        Err(_) => configured_keypair_from_legacy_config().map(|kp| kp.pubkey().to_string()),
    }
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
