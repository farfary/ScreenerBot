//! Wallet Manager
//!
//! Core wallet management functionality with caching and thread-safe operations.

use once_cell::sync::Lazy;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::crypto::{
    decrypt_to_keypair, generate_and_encrypt_keypair, import_and_encrypt, keypair_to_address,
};
use super::database::WalletsDatabase;
use super::types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, UpdateWalletRequest, Wallet,
    WalletRole, WalletType, WalletWithKey, WalletsSummary,
};
use crate::logger::{self, LogTag};

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global wallet database instance
static WALLETS_DB: Lazy<Arc<RwLock<Option<WalletsDatabase>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Cached main wallet keypair for fast access
static MAIN_WALLET_CACHE: Lazy<Arc<RwLock<Option<CachedMainWallet>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Cached main wallet data
struct CachedMainWallet {
    wallet: Wallet,
    keypair: Keypair,
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize the wallet manager
///
/// Must be called once at startup before using any wallet functions
pub async fn initialize() -> Result<(), String> {
    let db = WalletsDatabase::new()?;

    {
        let mut guard = WALLETS_DB.write().await;
        *guard = Some(db);
    }

    // Try to migrate from config.toml if no wallets exist
    migrate_from_config().await?;

    // Cache main wallet
    refresh_main_wallet_cache().await?;

    logger::info(LogTag::Wallet, "Wallet manager initialized");
    Ok(())
}

/// Check if wallet manager is initialized
pub async fn is_initialized() -> bool {
    WALLETS_DB.read().await.is_some()
}

/// Migrate existing wallet from config.toml to wallets database
async fn migrate_from_config() -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    // Check if we already have wallets
    let (total, _) = db.get_wallet_counts()?;
    if total > 0 {
        logger::debug(
            LogTag::Wallet,
            &format!("Skipping migration - {} wallets already exist", total),
        );
        return Ok(());
    }

    // Check if config has encrypted wallet
    let (encrypted, nonce) = crate::config::with_config(|cfg| {
        (cfg.wallet_encrypted.clone(), cfg.wallet_nonce.clone())
    });

    if encrypted.is_empty() || nonce.is_empty() {
        logger::debug(LogTag::Wallet, "No wallet in config.toml to migrate");
        return Ok(());
    }

    // Decrypt to get address
    let keypair = decrypt_to_keypair(&encrypted, &nonce)
        .map_err(|e| format!("Failed to decrypt config wallet: {}", e))?;
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
        &format!("Migrated wallet from config.toml: {}", address),
    );

    Ok(())
}

/// Refresh the cached main wallet
async fn refresh_main_wallet_cache() -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

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

// =============================================================================
// MAIN WALLET OPERATIONS (FAST PATH)
// =============================================================================

/// Get the main wallet's keypair (cached for performance)
pub async fn get_main_keypair() -> Result<Keypair, String> {
    // Fast path: check cache first
    {
        let cache = MAIN_WALLET_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            // Clone the keypair bytes and reconstruct
            let bytes = cached.keypair.to_bytes();
            return Keypair::from_bytes(&bytes)
                .map_err(|e| format!("Failed to clone keypair: {}", e));
        }
    }

    // Cache miss - refresh and retry
    refresh_main_wallet_cache().await?;

    let cache = MAIN_WALLET_CACHE.read().await;
    if let Some(cached) = cache.as_ref() {
        let bytes = cached.keypair.to_bytes();
        return Keypair::from_bytes(&bytes)
            .map_err(|e| format!("Failed to clone keypair: {}", e));
    }

    Err("No main wallet configured".to_string())
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
        .ok_or_else(|| "No main wallet configured".to_string())
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

// =============================================================================
// WALLET CRUD OPERATIONS
// =============================================================================

/// Create a new wallet
pub async fn create_wallet(request: CreateWalletRequest) -> Result<Wallet, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    // Generate new keypair
    let (keypair, encrypted) = generate_and_encrypt_keypair()?;
    let address = keypair_to_address(&keypair);

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
        refresh_main_wallet_cache().await?;
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
        &format!(
            "Created new wallet: {} ({})",
            request.name,
            address
        ),
    );

    // Return the wallet - re-acquire lock
    let db_guard = WALLETS_DB.read().await;
    db_guard
        .as_ref()
        .ok_or("Database unavailable")?
        .get_wallet_by_address(&address)?
        .ok_or_else(|| "Failed to retrieve created wallet".to_string())
}

/// Import an existing wallet
pub async fn import_wallet(request: ImportWalletRequest) -> Result<Wallet, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    // Parse and encrypt the private key
    let (keypair, encrypted) = import_and_encrypt(&request.private_key)?;
    let address = keypair_to_address(&keypair);

    // Check if wallet already exists
    if db.wallet_exists(&address)? {
        return Err(format!("Wallet {} already exists", address));
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
        refresh_main_wallet_cache().await?;
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

    WALLETS_DB
        .read()
        .await
        .as_ref()
        .ok_or("Database unavailable")?
        .get_wallet_by_address(&address)?
        .ok_or_else(|| "Failed to retrieve imported wallet".to_string())
}

/// Export a wallet's private key
pub async fn export_wallet(wallet_id: i64) -> Result<ExportWalletResponse, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallet = db
        .get_wallet(wallet_id)?
        .ok_or("Wallet not found")?;

    let (encrypted, nonce) = db
        .get_wallet_encrypted_key(wallet_id)?
        .ok_or("Wallet encrypted key not found")?;

    let private_key = super::crypto::export_private_key(&encrypted, &nonce)?;

    logger::warning(
        LogTag::Wallet,
        &format!("Wallet exported: {} ({})", wallet.name, wallet.address),
    );

    Ok(ExportWalletResponse {
        address: wallet.address,
        private_key,
        warning: "NEVER share this private key. Anyone with access can steal your funds.".to_string(),
    })
}

/// Get a wallet by ID
pub async fn get_wallet(wallet_id: i64) -> Result<Option<Wallet>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.get_wallet(wallet_id)
}

/// Get a wallet by address
pub async fn get_wallet_by_address(address: &str) -> Result<Option<Wallet>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.get_wallet_by_address(address)
}

/// Get a wallet's keypair by ID
pub async fn get_wallet_keypair(wallet_id: i64) -> Result<Keypair, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let (encrypted, nonce) = db
        .get_wallet_encrypted_key(wallet_id)?
        .ok_or("Wallet not found")?;

    decrypt_to_keypair(&encrypted, &nonce)
}

/// List all wallets
pub async fn list_wallets(include_inactive: bool) -> Result<Vec<Wallet>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.list_wallets(include_inactive)
}

/// List active wallets (usable for operations)
pub async fn list_active_wallets() -> Result<Vec<Wallet>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.list_active_wallets()
}

/// Update wallet metadata
pub async fn update_wallet(wallet_id: i64, request: UpdateWalletRequest) -> Result<Wallet, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.update_wallet(
        wallet_id,
        request.name.as_deref(),
        request.notes.as_deref(),
        request.role.clone(),
    )?;

    // If role changed to main, refresh cache
    if request.role == Some(WalletRole::Main) {
        drop(db_guard);
        refresh_main_wallet_cache().await?;
    }

    WALLETS_DB
        .read()
        .await
        .as_ref()
        .ok_or("Database unavailable")?
        .get_wallet(wallet_id)?
        .ok_or_else(|| "Wallet not found after update".to_string())
}

/// Set a wallet as the main wallet
pub async fn set_main_wallet(wallet_id: i64) -> Result<Wallet, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.set_main_wallet(wallet_id)?;

    let wallet = db
        .get_wallet(wallet_id)?
        .ok_or("Wallet not found")?;

    drop(db_guard);
    refresh_main_wallet_cache().await?;

    logger::info(
        LogTag::Wallet,
        &format!("Set main wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(wallet)
}

/// Archive a wallet (soft delete)
pub async fn archive_wallet(wallet_id: i64) -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallet = db
        .get_wallet(wallet_id)?
        .ok_or("Wallet not found")?;

    db.archive_wallet(wallet_id)?;

    logger::info(
        LogTag::Wallet,
        &format!("Archived wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(())
}

/// Restore an archived wallet
pub async fn restore_wallet(wallet_id: i64) -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallet = db
        .get_wallet(wallet_id)?
        .ok_or("Wallet not found")?;

    db.restore_wallet(wallet_id)?;

    logger::info(
        LogTag::Wallet,
        &format!("Restored wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(())
}

/// Permanently delete a wallet
pub async fn delete_wallet(wallet_id: i64) -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallet = db
        .get_wallet(wallet_id)?
        .ok_or("Wallet not found")?;

    db.delete_wallet(wallet_id)?;

    logger::warning(
        LogTag::Wallet,
        &format!("Deleted wallet: {} ({})", wallet.name, wallet.address),
    );

    Ok(())
}

// =============================================================================
// TOOLS INTEGRATION
// =============================================================================

/// Get all wallets with their keypairs for volume aggregator/tools
pub async fn get_wallets_with_keys() -> Result<Vec<WalletWithKey>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallets = db.list_active_wallets()?;
    let mut result = Vec::with_capacity(wallets.len());

    for wallet in wallets {
        let (encrypted, nonce) = match db.get_wallet_encrypted_key(wallet.id)? {
            Some(data) => data,
            None => continue,
        };

        let keypair = match decrypt_to_keypair(&encrypted, &nonce) {
            Ok(kp) => kp,
            Err(_) => continue,
        };

        result.push(WalletWithKey { wallet, keypair });
    }

    Ok(result)
}

/// Update last used timestamp for a wallet
pub async fn update_last_used(wallet_id: i64) -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.update_last_used(wallet_id)
}

// =============================================================================
// SUMMARY / STATS
// =============================================================================

/// Get wallets summary for dashboard
pub async fn get_wallets_summary() -> Result<WalletsSummary, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let (total, active) = db.get_wallet_counts()?;

    let main_wallet = db.get_main_wallet()?;

    Ok(WalletsSummary {
        total_count: total,
        active_count: active,
        main_wallet: main_wallet.as_ref().map(|w| w.address.clone()),
        main_wallet_name: main_wallet.as_ref().map(|w| w.name.clone()),
        total_sol: 0.0, // Will be updated by balance fetching
    })
}
