//! Wallet CRUD — create, read, update, and delete wallet operations.

use super::super::crypto::{generate_wallet_material, import_wallet_material};
use super::super::types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, UpdateWalletRequest, Wallet,
    WalletRole, WalletType,
};
use crate::logger::{self, LogTag};

/// Create a new wallet
pub async fn create_wallet(request: CreateWalletRequest) -> Result<Wallet, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    // Generate new wallet material (address + encrypted key) without ever
    // holding the intermediate keypair here.
    let (address, encrypted) = generate_wallet_material()?;

    // Determine role
    let role = if request.set_as_main {
        WalletRole::Main
    } else {
        WalletRole::Secondary
    };

    // If setting as main, we need to handle the transaction properly
    if request.set_as_main {
        // First insert as secondary
        let id = db.insert_wallet(
            &request.name,
            &address,
            &encrypted.ciphertext,
            &encrypted.nonce,
            WalletRole::Secondary,
            WalletType::Generated,
            request.notes.as_deref(),
        )?;

        // Then set as main (handles unsetting previous main)
        db.set_main_wallet(id)?;

        // Refresh cache
        drop(db_guard);
        super::cache::refresh_main_wallet_cache().await?;
        crate::chains::solana::accounts::invalidate_main_wallet_cache().await;
    } else {
        db.insert_wallet(
            &request.name,
            &address,
            &encrypted.ciphertext,
            &encrypted.nonce,
            role,
            WalletType::Generated,
            request.notes.as_deref(),
        )?;
    }

    logger::info(
        LogTag::Wallet,
        &format!("Created new wallet: {} ({})", request.name, address),
    );

    // Return the wallet - re-acquire lock
    let db_guard = super::WALLETS_DB.read().await;
    db_guard
        .as_ref()
        .ok_or("Database unavailable")?
        .get_wallet_by_address(&address)?
        .ok_or_else(|| "Failed to retrieve created wallet".to_owned())
}

/// Import an existing wallet
pub async fn import_wallet(request: ImportWalletRequest) -> Result<Wallet, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    // Parse and encrypt the private key
    let (address, encrypted) = import_wallet_material(&request.private_key)?;

    // Check if wallet already exists
    if db.wallet_exists(&address)? {
        return Err(format!("Wallet {address} already exists"));
    }

    // Determine role
    let role = if request.set_as_main {
        WalletRole::Main
    } else {
        WalletRole::Secondary
    };

    // If setting as main, handle properly
    if request.set_as_main {
        let id = db.insert_wallet(
            &request.name,
            &address,
            &encrypted.ciphertext,
            &encrypted.nonce,
            WalletRole::Secondary,
            WalletType::Imported,
            request.notes.as_deref(),
        )?;

        db.set_main_wallet(id)?;

        drop(db_guard);
        super::cache::refresh_main_wallet_cache().await?;
        crate::chains::solana::accounts::invalidate_main_wallet_cache().await;
    } else {
        db.insert_wallet(
            &request.name,
            &address,
            &encrypted.ciphertext,
            &encrypted.nonce,
            role,
            WalletType::Imported,
            request.notes.as_deref(),
        )?;
    }

    logger::info(
        LogTag::Wallet,
        &format!("Imported wallet: {} ({})", request.name, address),
    );

    super::WALLETS_DB
        .read()
        .await
        .as_ref()
        .ok_or("Database unavailable")?
        .get_wallet_by_address(&address)?
        .ok_or_else(|| "Failed to retrieve imported wallet".to_owned())
}

/// Export a wallet's private key
pub async fn export_wallet(wallet_id: i64) -> Result<ExportWalletResponse, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let wallet = db.get_wallet(wallet_id)?.ok_or("Wallet not found")?;

    let (encrypted, nonce) = db
        .get_wallet_encrypted_key(wallet_id)?
        .ok_or("Wallet encrypted key not found")?;

    let private_key = super::super::crypto::export_private_key(&encrypted, &nonce)?;

    logger::warning(
        LogTag::Wallet,
        &format!("Wallet exported: {} ({})", wallet.name, wallet.address),
    );

    Ok(ExportWalletResponse {
        address: wallet.address,
        private_key,
        warning: "NEVER share this private key. Anyone with access can steal your funds."
            .to_string(),
    })
}

/// Update wallet metadata
pub async fn update_wallet(wallet_id: i64, request: UpdateWalletRequest) -> Result<Wallet, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.update_wallet(
        wallet_id,
        request.name.as_deref(),
        request.notes.as_deref(),
        request.role.clone(),
    )?;

    // If role changed to main, refresh cache
    if request.role == Some(WalletRole::Main) {
        drop(db_guard);
        super::cache::refresh_main_wallet_cache().await?;
        crate::chains::solana::accounts::invalidate_main_wallet_cache().await;
    }

    super::WALLETS_DB
        .read()
        .await
        .as_ref()
        .ok_or("Database unavailable")?
        .get_wallet(wallet_id)?
        .ok_or_else(|| "Wallet not found after update".to_owned())
}

/// Set a wallet as the main wallet
pub async fn set_main_wallet(wallet_id: i64) -> Result<Wallet, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.set_main_wallet(wallet_id)?;

    let wallet = db.get_wallet(wallet_id)?.ok_or("Wallet not found")?;

    drop(db_guard);
    super::cache::refresh_main_wallet_cache().await?;
    crate::chains::solana::accounts::invalidate_main_wallet_cache().await;

    logger::info(
        LogTag::Wallet,
        &format!("Set main wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(wallet)
}

/// Archive a wallet (soft delete)
pub async fn archive_wallet(wallet_id: i64) -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let wallet = db.get_wallet(wallet_id)?.ok_or("Wallet not found")?;

    db.archive_wallet(wallet_id)?;

    logger::info(
        LogTag::Wallet,
        &format!("Archived wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(())
}

/// Restore an archived wallet
pub async fn restore_wallet(wallet_id: i64) -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let wallet = db.get_wallet(wallet_id)?.ok_or("Wallet not found")?;

    db.restore_wallet(wallet_id)?;

    logger::info(
        LogTag::Wallet,
        &format!("Restored wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(())
}

/// Permanently delete a wallet
pub async fn delete_wallet(wallet_id: i64) -> Result<(), String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let wallet = db.get_wallet(wallet_id)?.ok_or("Wallet not found")?;

    db.delete_wallet(wallet_id)?;

    logger::warning(
        LogTag::Wallet,
        &format!("Deleted wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(())
}
