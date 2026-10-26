// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet migration — imports the config key into an empty vault, and reads the
//! address an encrypted key belongs to without holding the key.

use crate::logger::{self, LogTag};

use super::super::error::Error;
use super::super::types::{WalletRole, WalletType};
use super::WALLETS_DB;
use crate::chains::solana::accounts::address_from_encrypted_key;

/// Migrate existing wallet from config.toml to wallets database
pub(super) async fn migrate_from_config() -> Result<(), Error> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;

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
    let address = encrypted_key_address(&encrypted, &nonce)?;

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

/// The address the encrypted key `ciphertext`/`nonce` belongs to. The key is
/// decrypted inside the chain accounts module and only the address comes back.
pub(in crate::wallets) fn encrypted_key_address(
    ciphertext: &str,
    nonce: &str,
) -> Result<String, Error> {
    address_from_encrypted_key(ciphertext, nonce).map_err(|reason| Error::InvalidPrivateKey {
        reason: reason.to_string(),
    })
}

/// Log a warning when the config key belongs to another wallet than the vault's main
/// wallet. The vault decides the trading wallet once it holds a main, so such a key is
/// inert; a trading wallet changes only through a staged replacement.
pub(super) async fn warn_if_config_key_is_inert() -> Result<(), Error> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or_else(|| Error::NotInitialized { database: "wallet" })?;
    let Some(main) = db.get_main_wallet()? else {
        return Ok(());
    };
    let (encrypted, nonce) =
        crate::config::with_config(|cfg| (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone()));
    if encrypted.is_empty() || nonce.is_empty() {
        return Ok(());
    }
    match encrypted_key_address(&encrypted, &nonce) {
        Ok(address) if address != main.address => logger::warning(
            LogTag::Wallet,
            &format!(
                "The config key belongs to {address}, not to the trading wallet {}; it is ignored. \
                 Replace the trading wallet from the Wallets page to change it",
                main.address
            ),
        ),
        Ok(_) => {}
        Err(error) => logger::warning(
            LogTag::Wallet,
            &format!("The config key could not be read and is ignored: {error}"),
        ),
    }
    Ok(())
}
