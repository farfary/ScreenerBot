//! Wallet migration — upgrades wallet data from legacy formats.

use crate::logger::{self, LogTag};

use super::super::error::Error;
use super::super::types::{WalletRole, WalletType};
use super::db_not_initialized;
use super::WALLETS_DB;
use crate::chains::solana::accounts::address_from_encrypted_key;

/// Migrate existing wallet from config.toml to wallets database
pub(super) async fn migrate_from_config() -> Result<(), Error> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or_else(db_not_initialized)?;

    // Check if we already have wallets
    let (total, _) = db.get_wallet_counts()?;
    if total > 0 {
        logger::debug(
            LogTag::Wallet,
            &format!("Skipping migration - {total} wallets already exist"),
        );
        return Ok(());
    }

    // Check if config has encrypted wallet
    let (encrypted, nonce) =
        crate::config::with_config(|cfg| (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone()));

    if encrypted.is_empty() || nonce.is_empty() {
        logger::debug(LogTag::Wallet, "No wallet in config.toml to migrate");
        return Ok(());
    }

    // Decrypt to get address (the key material itself is re-stored encrypted, unchanged)
    let address = address_from_encrypted_key(&encrypted, &nonce).map_err(|reason| {
        Error::InvalidPrivateKey {
            reason: reason.to_string(),
        }
    })?;

    // Insert as main wallet
    db.insert_wallet(
        "Main Wallet",
        &address,
        &encrypted,
        &nonce,
        WalletRole::Main,
        WalletType::Migrated,
        Some("Migrated from config.toml"),
    )?;

    logger::info(
        LogTag::Wallet,
        &format!("Migrated wallet from config.toml: {address}"),
    );

    Ok(())
}
