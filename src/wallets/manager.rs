//! Wallet Manager
//!
//! Core wallet management functionality with caching and thread-safe operations.

use once_cell::sync::Lazy;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::crypto::{
    decrypt_to_keypair, generate_and_encrypt_keypair, import_and_encrypt, keypair_to_address,
};
use super::database::WalletsDatabase;
use super::types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, TokenBalance,
    UpdateWalletRequest, Wallet, WalletRole, WalletType, WalletWithKey, WalletsSummary,
};
use crate::logger::{self, LogTag};
use crate::rpc::{get_new_rpc_client, RpcClientMethods};

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
            None => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Skipping wallet id={} ({}): encrypted key not found",
                        wallet.id, wallet.address
                    ),
                );
                continue;
            }
        };

        let keypair = match decrypt_to_keypair(&encrypted, &nonce) {
            Ok(kp) => kp,
            Err(e) => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Skipping wallet id={} ({}): decryption failed - {}",
                        wallet.id, wallet.address, e
                    ),
                );
                continue;
            }
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

// =============================================================================
// BULK IMPORT/EXPORT
// =============================================================================

use super::bulk::{
    BulkImportResult, ImportOptions, ImportRowResult, ParsedWalletRow, WalletExportRow,
};
use std::collections::HashSet;

/// Bulk import wallets from parsed rows
///
/// Imports multiple wallets in sequence, tracking success/failure for each.
/// Never logs private keys.
pub async fn bulk_import_wallets(
    rows: Vec<ParsedWalletRow>,
    options: &ImportOptions,
) -> BulkImportResult {
    let mut result = BulkImportResult {
        total_rows: rows.len(),
        success_count: 0,
        failed_count: 0,
        skipped_duplicates: 0,
        rows: Vec::with_capacity(rows.len()),
        imported_wallet_ids: Vec::new(),
    };

    // Get existing addresses to check duplicates
    let existing_addresses = match get_existing_addresses().await {
        Ok(addrs) => addrs,
        Err(e) => {
            // If we can't get existing addresses, fail all imports
            for row in &rows {
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: None,
                    success: false,
                    error: Some(format!("Failed to check existing wallets: {}", e)),
                });
                result.failed_count += 1;
            }
            return result;
        }
    };

    let mut first_imported_id: Option<i64> = None;

    for row in rows {
        // Validate and derive address without logging key
        let keypair = match super::crypto::parse_private_key(&row.private_key) {
            Ok(kp) => kp,
            Err(e) => {
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: None,
                    success: false,
                    error: Some(format!("Invalid private key: {}", e)),
                });
                result.failed_count += 1;
                continue;
            }
        };

        let address = keypair_to_address(&keypair);

        // Check for duplicates
        if existing_addresses.contains(&address) {
            if options.skip_duplicates {
                result.skipped_duplicates += 1;
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: Some(address),
                    success: false,
                    error: Some("Skipped: wallet already exists".to_string()),
                });
                continue;
            } else {
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: Some(address),
                    success: false,
                    error: Some("Wallet already exists".to_string()),
                });
                result.failed_count += 1;
                continue;
            }
        }

        // Import the wallet
        let import_result = import_wallet(ImportWalletRequest {
            name: row.name.clone(),
            private_key: row.private_key.clone(),
            notes: row.notes.clone(),
            set_as_main: false,
        })
        .await;

        match import_result {
            Ok(wallet) => {
                result.success_count += 1;
                result.imported_wallet_ids.push(wallet.id);
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: Some(wallet.address),
                    success: true,
                    error: None,
                });

                if first_imported_id.is_none() {
                    first_imported_id = Some(wallet.id);
                }
            }
            Err(e) => {
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: Some(address),
                    success: false,
                    error: Some(e),
                });
                result.failed_count += 1;
            }
        }
    }

    // Set first as main if requested and no main exists
    if options.set_first_as_main {
        if let Some(id) = first_imported_id {
            if !has_main_wallet().await {
                if let Err(e) = set_main_wallet(id).await {
                    logger::warning(
                        LogTag::Wallet,
                        &format!("Failed to set first imported wallet as main: {}", e),
                    );
                }
            }
        }
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "Bulk import completed: {} success, {} failed, {} skipped",
            result.success_count, result.failed_count, result.skipped_duplicates
        ),
    );

    result
}

/// Export wallets for backup/transfer
///
/// Returns wallet data including private keys for export to CSV/Excel.
/// WARNING: This exports sensitive data - handle with care!
pub async fn export_wallets(include_inactive: bool) -> Result<Vec<WalletExportRow>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallets = db.list_wallets(include_inactive)?;
    let mut result = Vec::with_capacity(wallets.len());

    for wallet in wallets {
        // Get encrypted key
        let (encrypted, nonce) = match db.get_wallet_encrypted_key(wallet.id)? {
            Some(data) => data,
            None => continue,
        };

        // Decrypt private key
        let private_key = match super::crypto::export_private_key(&encrypted, &nonce) {
            Ok(key) => key,
            Err(_) => continue, // Skip wallets we can't decrypt
        };

        result.push(WalletExportRow {
            name: wallet.name,
            address: wallet.address,
            private_key,
            role: wallet.role.to_string(),
            is_main: wallet.role == WalletRole::Main,
            notes: wallet.notes.unwrap_or_default(),
            created_at: wallet.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        });
    }

    logger::warning(
        LogTag::Wallet,
        &format!(
            "Exported {} wallets - SENSITIVE DATA",
            result.len()
        ),
    );

    Ok(result)
}

/// Get all existing wallet addresses for duplicate checking
async fn get_existing_addresses() -> Result<HashSet<String>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    let wallets = db.list_wallets(true)?; // Include inactive
    Ok(wallets.into_iter().map(|w| w.address).collect())
}

/// Get existing addresses as HashSet (public for validator)
pub async fn get_existing_wallet_addresses() -> Result<HashSet<String>, String> {
    get_existing_addresses().await
}

// =============================================================================
// TOKEN BALANCE OPERATIONS
// =============================================================================

/// Update token balances for a wallet by fetching from RPC
///
/// Fetches all token accounts for the wallet and caches them in the database.
/// Returns the number of tokens updated.
pub async fn update_wallet_balances(wallet_id: i64) -> Result<usize, String> {
    // Get wallet address
    let wallet = get_wallet(wallet_id)
        .await?
        .ok_or("Wallet not found")?;

    let wallet_pubkey = solana_sdk::pubkey::Pubkey::from_str(&wallet.address)
        .map_err(|e| format!("Invalid wallet address: {}", e))?;

    let rpc_client = get_new_rpc_client();
    let token_accounts = rpc_client
        .get_all_token_accounts(&wallet_pubkey)
        .await
        .map_err(|e| format!("Failed to fetch token accounts: {}", e))?;

    // Convert to TokenBalance structs
    let now = chrono::Utc::now();
    let balances: Vec<TokenBalance> = token_accounts
        .iter()
        .filter(|acc| !acc.is_nft) // Exclude NFTs
        .map(|acc| {
            let ui_amount = acc.balance as f64 / 10f64.powi(acc.decimals as i32);
            TokenBalance {
                wallet_id,
                mint: acc.mint.clone(),
                balance: acc.balance,
                ui_amount,
                decimals: acc.decimals,
                symbol: None, // Will be populated by token service if available
                name: None,
                is_token_2022: acc.is_token_2022,
                updated_at: now,
            }
        })
        .collect();

    let count = balances.len();

    // Bulk update in database
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.update_balances_bulk(wallet_id, &balances)?;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Updated {} token balances for wallet {} ({})",
            count, wallet.name, wallet.address
        ),
    );

    Ok(count)
}

/// Update token balances for all active wallets
///
/// Returns a map of wallet_id -> number of tokens updated
pub async fn update_all_wallet_balances() -> Result<HashMap<i64, usize>, String> {
    let wallets = list_active_wallets().await?;
    let mut results = HashMap::new();

    for wallet in wallets {
        match update_wallet_balances(wallet.id).await {
            Ok(count) => {
                results.insert(wallet.id, count);
            }
            Err(e) => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Failed to update balances for wallet {} ({}): {}",
                        wallet.name, wallet.address, e
                    ),
                );
            }
        }
    }

    Ok(results)
}

/// Get cached token balances for a wallet
pub async fn get_token_balances(wallet_id: i64) -> Result<Vec<TokenBalance>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.get_token_balances(wallet_id)
}

/// Get cached token balances for all wallets
pub async fn get_all_token_balances() -> Result<HashMap<i64, Vec<TokenBalance>>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.get_all_token_balances()
}

/// Clear cached token balances for a wallet
pub async fn clear_token_balances(wallet_id: i64) -> Result<u64, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.clear_token_balances(wallet_id)
}

/// Upsert a single token balance (for incremental updates)
pub async fn upsert_token_balance(
    wallet_id: i64,
    mint: &str,
    balance: u64,
    ui_amount: f64,
    decimals: u8,
    symbol: Option<&str>,
    name: Option<&str>,
    is_token_2022: bool,
) -> Result<(), String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard
        .as_ref()
        .ok_or("Wallet database not initialized")?;

    db.upsert_token_balance(
        wallet_id,
        mint,
        balance,
        ui_amount,
        decimals,
        symbol,
        name,
        is_token_2022,
    )
}
