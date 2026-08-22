//! Wallet bulk operations — batch import, export, and multi-wallet management.

use std::collections::HashSet;

use super::super::bulk::{
    BulkImportResult, ImportOptions, ImportRowResult, ParsedWalletRow, WalletExportRow,
};
use super::super::types::{CreateWalletRequest, ImportWalletRequest, Wallet, WalletRole};
use crate::chains::solana::accounts::address_from_private_key;
use crate::logger::{self, LogTag};

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
    let existing_addresses = match get_existing_wallet_addresses().await {
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
        let address = match address_from_private_key(&row.private_key) {
            Ok(addr) => addr,
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
        let import_result = super::import_wallet(ImportWalletRequest {
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
            if !super::has_main_wallet().await {
                if let Err(e) = super::set_main_wallet(id).await {
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
    let db_guard = super::WALLETS_DB.read().await;
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
        let private_key =
            match crate::chains::solana::accounts::export_private_key(&encrypted, &nonce) {
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

/// Get existing addresses as HashSet (for duplicate checking)
pub async fn get_existing_wallet_addresses() -> Result<HashSet<String>, String> {
    let db_guard = super::WALLETS_DB.read().await;
    let db = db_guard.as_ref().ok_or("Wallet database not initialized")?;

    let wallets = db.list_wallets(true)?; // Include inactive
    Ok(wallets.into_iter().map(|w| w.address).collect())
}

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

        match super::create_wallet(CreateWalletRequest {
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
