use crate::logger::{self, LogTag};

use super::WALLETS_DB;
use super::super::crypto::{decrypt_to_keypair, keypair_to_address};
use super::super::types::{WalletRole, WalletType};

/// Migrate existing wallet from config.toml to wallets database
pub(super) async fn migrate_from_config() -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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

    // Decrypt to get address
    let keypair = decrypt_to_keypair(&encrypted, &nonce)
        .map_err(|e| format!("Failed to decrypt config wallet: {e}"))?;
    let address = keypair_to_address(&keypair);

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
