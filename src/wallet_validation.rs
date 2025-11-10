// Wallet validation module - ensures data integrity across wallet changes
//
// This module provides validation logic to detect wallet changes and
// prevent data mixing from different wallets.

use crate::logger::{self, LogTag};
use crate::paths;
use crate::utils::get_wallet_address;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum WalletValidationResult {
    /// Wallet validation passed - current wallet matches stored data
    Valid,
    /// Wallet changed - data cleanup required
    Mismatch {
        current: String,
        stored: String,
        affected_systems: Vec<String>,
    },
    /// First run - no existing databases
    FirstRun,
}

pub struct WalletValidator;

impl WalletValidator {
    /// Check if wallet changed across all systems
    pub async fn validate_wallet_consistency() -> Result<WalletValidationResult, String> {
        let current_wallet = get_wallet_address()
            .map_err(|e| format!("Failed to get current wallet address: {}", e))?;

        let mut mismatches: Vec<(String, String)> = Vec::new();

        // Check transactions DB
        let transactions_db_path = paths::get_transactions_db_path();
        if transactions_db_path.exists() {
            if let Some(stored_wallet) = Self::get_stored_wallet(
                &transactions_db_path.to_string_lossy(),
                "db_metadata",
            )? {
                if stored_wallet != current_wallet {
                    mismatches.push(("Transactions".to_string(), stored_wallet));
                }
            }
        }

        // Check positions DB
        let positions_db_path = paths::get_positions_db_path();
        if positions_db_path.exists() {
            if let Some(stored_wallet) = Self::get_stored_wallet(
                &positions_db_path.to_string_lossy(),
                "position_metadata",
            )? {
                if stored_wallet != current_wallet {
                    mismatches.push(("Positions".to_string(), stored_wallet));
                }
            }
        }

        // Check wallet DB
        let wallet_db_path = paths::get_wallet_db_path();
        if wallet_db_path.exists() {
            if let Some(stored_wallet) =
                Self::get_stored_wallet(&wallet_db_path.to_string_lossy(), "wallet_metadata")?
            {
                if stored_wallet != current_wallet {
                    mismatches.push(("Wallet History".to_string(), stored_wallet));
                }
            }
        }

        if mismatches.is_empty() {
            if Self::any_database_exists() {
                Ok(WalletValidationResult::Valid)
            } else {
                Ok(WalletValidationResult::FirstRun)
            }
        } else {
            let affected_systems = mismatches.iter().map(|(sys, _)| sys.clone()).collect();
            let stored = mismatches[0].1.clone();

            Ok(WalletValidationResult::Mismatch {
                current: current_wallet,
                stored,
                affected_systems,
            })
        }
    }

    /// Get stored wallet address from database metadata table
    fn get_stored_wallet(db_path: &str, metadata_table: &str) -> Result<Option<String>, String> {
        let conn =
            Connection::open(db_path).map_err(|e| format!("Failed to open {}: {}", db_path, e))?;

        let query = format!(
            "SELECT value FROM {} WHERE key = 'current_wallet'",
            metadata_table
        );

        let wallet: Option<String> = conn
            .query_row(&query, [], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query current_wallet from {}: {}", db_path, e))?;

        Ok(wallet.filter(|w| !w.is_empty()))
    }

    /// Check if any database exists
    fn any_database_exists() -> bool {
        paths::get_transactions_db_path().exists()
            || paths::get_positions_db_path().exists()
            || paths::get_wallet_db_path().exists()
    }

    /// Clean all databases (delete files)
    pub async fn clean_all_databases() -> Result<(), String> {
        // Get all database paths with their WAL and SHM files
        let mut dbs = Vec::new();
        dbs.extend(paths::get_db_with_wal_files(
            paths::get_transactions_db_path(),
        ));
        dbs.extend(paths::get_db_with_wal_files(paths::get_positions_db_path()));
        dbs.extend(paths::get_db_with_wal_files(paths::get_wallet_db_path()));

        let mut deleted_count = 0;
        for db_path in &dbs {
            if db_path.exists() {
                std::fs::remove_file(db_path)
                    .map_err(|e| format!("Failed to remove {}: {}", db_path.display(), e))?;
                logger::info(
                    LogTag::System,
                    &format!("🗑️  Deleted {}", db_path.display()),
                );
                deleted_count += 1;
            }
        }

        if deleted_count > 0 {
            logger::info(
                LogTag::System,
                &format!("✅ Cleaned {} database files", deleted_count),
            );
        }

        Ok(())
    }
}
