//! Wallet access helpers — record lookup and encrypted-credential resolution.
//!
//! Never decrypts. Returns ciphertext/nonce (chain-neutral) so only
//! `crate::chains::solana::accounts` ever holds a decrypted `Keypair`.

use super::super::error::Error;
use super::super::types::Wallet;
use super::db_not_initialized;

/// Get a wallet by ID
pub async fn get_wallet(wallet_id: i64) -> Result<Option<Wallet>, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or_else(db_not_initialized)?;

    db.get_wallet(wallet_id)
}

/// Get a wallet by address
pub async fn get_wallet_by_address(address: &str) -> Result<Option<Wallet>, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or_else(db_not_initialized)?;

    db.get_wallet_by_address(address)
}

/// Get a wallet's encrypted key material (ciphertext, nonce) by ID.
/// Chain-neutral — only `crate::chains::solana::accounts` decrypts this.
pub(crate) async fn get_wallet_encrypted_key(wallet_id: i64) -> Result<(String, String), Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or_else(db_not_initialized)?;

    db.get_wallet_encrypted_key(wallet_id)?
        .ok_or(Error::WalletNotFound {
            address: format!("id={wallet_id}"),
        })
}

/// List all wallets
pub async fn list_wallets(include_inactive: bool) -> Result<Vec<Wallet>, Error> {
    let mut wallets = {
        let db_guard = super::WALLETS_DB.read().await;
        let db = db_guard.as_ref().ok_or_else(db_not_initialized)?;
        db.list_wallets(include_inactive)?
    };

    for wallet in &mut wallets {
        enrich_last_used(wallet).await;
    }

    Ok(wallets)
}

/// Fill a wallet's `last_used_at` from its most recent recorded transaction.
///
/// The transactions DB is the canonical source of a wallet's activity ("last
/// time any transaction happened"), which is broader and more reliable than the
/// stored column. The column is left untouched as a fallback when the
/// transactions DB is unavailable or the wallet has no recorded transactions.
pub async fn enrich_last_used(wallet: &mut Wallet) {
    if let Some(db) = crate::transactions::database::get_transaction_database().await {
        if let Ok(Some(ts)) = db.get_latest_transaction_time(&wallet.address).await {
            wallet.last_used_at = Some(ts);
        }
    }
}

/// List active wallets (usable for operations)
pub async fn list_active_wallets() -> Result<Vec<Wallet>, Error> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or_else(db_not_initialized)?;

    db.list_active_wallets()
}
