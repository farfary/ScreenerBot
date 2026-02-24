//! Wallet Manager
//!
//! Core wallet management functionality with caching and thread-safe operations.

use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;

use super::crypto::{
    decrypt_to_keypair, generate_and_encrypt_keypair, import_and_encrypt, keypair_to_address,
};
use super::database::WalletsDatabase;
use super::types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, SimpleTokenBalance,
    TokenBalance, UpdateWalletRequest, Wallet, WalletBalanceSummary, WalletRole, WalletType,
    WalletWithKey, WalletWithTokenBalance, WalletsSummary,
};
use crate::logger::{self, LogTag};
use crate::rpc::{get_rpc_client, RpcClientMethods};

mod cache;
use cache::refresh_main_wallet_cache;

mod main_wallet;
pub use main_wallet::{get_main_address, get_main_keypair, get_main_wallet, has_main_wallet};

mod balance_constants;

mod token_balance_queries;
pub use token_balance_queries::{get_all_wallet_balances, get_wallets_with_token};

mod migration;

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// Global wallet database instance
static WALLETS_DB: LazyLock<Arc<RwLock<Option<WalletsDatabase>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

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
    migration::migrate_from_config().await?;

    // Cache main wallet
    refresh_main_wallet_cache().await?;

    logger::info(LogTag::Wallet, "Wallet manager initialized");
    Ok(())
}

/// Check if wallet manager is initialized
pub async fn is_initialized() -> bool {
    WALLETS_DB.read().await.is_some()
}

// =============================================================================
// WALLET CRUD OPERATIONS
// =============================================================================

mod crud;
pub use crud::{
    archive_wallet, create_wallet, delete_wallet, export_wallet, import_wallet, restore_wallet,
    set_main_wallet, update_wallet,
};

mod access;
pub use access::{get_wallet, get_wallet_by_address, get_wallet_keypair, list_active_wallets, list_wallets};

// =============================================================================
// TOOLS INTEGRATION
// =============================================================================

/// Get all wallets with their keypairs for tools
pub async fn get_wallets_with_keys() -> Result<Vec<WalletWithKey>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.update_last_used(wallet_id)
}

// =============================================================================
// SUMMARY / STATS
// =============================================================================

/// Get wallets summary for dashboard
pub async fn get_wallets_summary() -> Result<WalletsSummary, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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
                    error: Some(format!("Failed to check existing wallets: {e}")),
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
                    error: Some(format!("Invalid private key: {e}")),
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
                    error: Some("Skipped: wallet already exists".to_owned()),
                });
                continue;
            } else {
                result.rows.push(ImportRowResult {
                    row_num: row.row_num,
                    name: row.name.clone(),
                    address: Some(address),
                    success: false,
                    error: Some("Wallet already exists".to_owned()),
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
                        &format!("Failed to set first imported wallet as main: {e}"),
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
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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
        &format!("Exported {} wallets - SENSITIVE DATA", result.len()),
    );

    Ok(result)
}

/// Get all existing wallet addresses for duplicate checking
async fn get_existing_addresses() -> Result<HashSet<String>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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
    let wallet = get_wallet(wallet_id).await?.ok_or("Wallet not found")?;

    let wallet_pubkey = solana_sdk::pubkey::Pubkey::from_str(&wallet.address)
        .map_err(|e| format!("Invalid wallet address: {e}"))?;

    let rpc_client = get_rpc_client();
    let token_accounts = rpc_client
        .get_all_token_accounts(&wallet_pubkey)
        .await
        .map_err(|e| format!("Failed to fetch token accounts: {e}"))?;

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
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.get_token_balances(wallet_id)
}

/// Get cached token balances for all wallets
pub async fn get_all_token_balances() -> Result<HashMap<i64, Vec<TokenBalance>>, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    db.get_all_token_balances()
}

/// Clear cached token balances for a wallet
pub async fn clear_token_balances(wallet_id: i64) -> Result<u64, String> {
    let db_guard = WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

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

// =============================================================================
// BATCH WALLET OPERATIONS
// =============================================================================

/// Create multiple wallets at once with a name prefix
///
/// # Arguments
/// * `count` - Number of wallets to create
/// * `name_prefix` - Prefix for wallet names (e.g., "Trading" -> "Trading_1", "Trading_2", ...)
/// * `notes` - Optional notes to attach to all created wallets
///
/// # Returns
/// Vector of created wallets on success
pub async fn create_wallets_batch(
    count: u32,
    name_prefix: &str,
    notes: Option<&str>,
) -> Result<Vec<Wallet>, String> {
    if count == 0 {
        return Err("Count must be greater than 0".to_owned());
    }

    if count > 100 {
        return Err("Maximum batch size is 100 wallets".to_owned());
    }

    let mut created_wallets = Vec::with_capacity(count as usize);

    for i in 1..=count {
        let name = format!("{name_prefix}_{i}");

        match create_wallet(CreateWalletRequest {
            name: name.clone(),
            notes: notes.map(|s| s.to_string()),
            set_as_main: false,
        })
        .await
        {
            Ok(wallet) => {
                created_wallets.push(wallet);
            }
            Err(e) => {
                logger::warning(
                    LogTag::Wallet,
                    &format!(
                        "Failed to create wallet {} in batch: {}. Continuing with {} already created.",
                        name, e, created_wallets.len()
                    ),
                );
                // Continue with remaining wallets, don't fail entire batch
            }
        }
    }

    if created_wallets.is_empty() {
        return Err("Failed to create any wallets".to_owned());
    }

    logger::info(
        LogTag::Wallet,
        &format!(
            "Batch created {} wallets with prefix '{}'",
            created_wallets.len(),
            name_prefix
        ),
    );

    Ok(created_wallets)
}
