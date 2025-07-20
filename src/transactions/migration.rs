// transactions/migration.rs - Migration utilities for transitioning to modular system
use super::cache::TransactionDatabase;
use super::types::*;
use crate::logger::{ log, LogTag };
use std::error::Error;
use std::fs;
use std::path::Path;

/// Migration helper for transitioning from old JSON cache to SQLite database
pub struct TransactionMigration {
    db: TransactionDatabase,
}

impl TransactionMigration {
    /// Create a new migration helper
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let db = TransactionDatabase::new()?;
        Ok(Self { db })
    }

    /// Migrate from old JSON cache file to new SQLite database
    pub async fn migrate_from_json(
        &self,
        json_file_path: &str
    ) -> Result<MigrationResult, Box<dyn Error>> {
        if !Path::new(json_file_path).exists() {
            return Ok(MigrationResult {
                transactions_migrated: 0,
                errors_encountered: 0,
                success: true,
                message: "No JSON cache file found - starting fresh".to_string(),
            });
        }

        log(LogTag::System, "INFO", &format!("Starting migration from {}", json_file_path));

        // Read and parse old JSON cache
        let json_content = fs::read_to_string(json_file_path)?;
        let old_cache: OldTransactionCache = serde_json
            ::from_str(&json_content)
            .map_err(|e| format!("Failed to parse old cache: {}", e))?;

        let mut migrated_count = 0;
        let mut error_count = 0;

        // Migrate transactions in batches
        let batch_size = 100;
        let total_transactions = old_cache.transactions.len();

        for (i, batch) in old_cache.transactions.chunks(batch_size).enumerate() {
            let batch_transactions: Vec<(String, TransactionResult)> = batch
                .iter()
                .map(|(sig, tx)| (sig.clone(), tx.clone()))
                .collect();

            match self.db.batch_upsert_transactions(&batch_transactions) {
                Ok(_) => {
                    migrated_count += batch_transactions.len();
                    log(
                        LogTag::System,
                        "INFO",
                        &format!(
                            "Migrated batch {} ({}/{} transactions)",
                            i + 1,
                            migrated_count,
                            total_transactions
                        )
                    );
                }
                Err(e) => {
                    error_count += batch_transactions.len();
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to migrate batch {}: {}", i + 1, e)
                    );
                }
            }
        }

        // Create backup of old file
        let backup_path = format!("{}.backup", json_file_path);
        if let Err(e) = fs::copy(json_file_path, &backup_path) {
            log(LogTag::System, "WARNING", &format!("Failed to create backup: {}", e));
        } else {
            log(LogTag::System, "INFO", &format!("Created backup at: {}", backup_path));
        }

        let success = error_count == 0;
        let message = if success {
            format!("Successfully migrated {} transactions", migrated_count)
        } else {
            format!("Migrated {} transactions with {} errors", migrated_count, error_count)
        };

        log(LogTag::System, if success { "SUCCESS" } else { "WARNING" }, &message);

        Ok(MigrationResult {
            transactions_migrated: migrated_count,
            errors_encountered: error_count,
            success,
            message,
        })
    }

    /// Verify migration integrity by comparing counts
    pub async fn verify_migration(
        &self,
        json_file_path: &str
    ) -> Result<MigrationVerification, Box<dyn Error>> {
        let mut verification = MigrationVerification {
            json_transaction_count: 0,
            db_transaction_count: 0,
            integrity_check_passed: false,
            missing_transactions: Vec::new(),
        };

        // Count transactions in old JSON file
        if Path::new(json_file_path).exists() {
            let json_content = fs::read_to_string(json_file_path)?;
            let old_cache: OldTransactionCache = serde_json::from_str(&json_content)?;
            verification.json_transaction_count = old_cache.transactions.len();

            // Check for missing transactions
            for signature in old_cache.transactions.keys() {
                if self.db.get_transaction(signature)?.is_none() {
                    verification.missing_transactions.push(signature.clone());
                }
            }
        }

        // Count transactions in new database
        verification.db_transaction_count = self.db.get_transaction_count()?;

        // Integrity check
        verification.integrity_check_passed =
            verification.missing_transactions.is_empty() &&
            verification.json_transaction_count == verification.db_transaction_count;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Migration verification: JSON={}, DB={}, Missing={}, Integrity={}",
                verification.json_transaction_count,
                verification.db_transaction_count,
                verification.missing_transactions.len(),
                verification.integrity_check_passed
            )
        );

        Ok(verification)
    }

    /// Clean up old files after successful migration
    pub async fn cleanup_old_files(&self, json_file_path: &str) -> Result<(), Box<dyn Error>> {
        // Only cleanup if verification passes
        let verification = self.verify_migration(json_file_path).await?;

        if verification.integrity_check_passed {
            // Move to archive directory instead of deleting
            let archive_dir = "migration_archive";
            fs::create_dir_all(archive_dir)?;

            let file_name = Path::new(json_file_path).file_name().ok_or("Invalid file path")?;

            let archive_path = Path::new(archive_dir).join(file_name);
            fs::rename(json_file_path, &archive_path)?;

            log(
                LogTag::System,
                "SUCCESS",
                &format!("Archived old cache file to: {:?}", archive_path)
            );
        } else {
            log(LogTag::System, "WARNING", "Migration verification failed - keeping old files");
        }

        Ok(())
    }

    /// Export current database to JSON for backup
    pub async fn export_to_json(&self, output_path: &str) -> Result<usize, Box<dyn Error>> {
        let transactions = self.db.get_all_transactions_raw()?;

        let export_cache = OldTransactionCache {
            transactions: transactions.into_iter().collect(),
            last_update: chrono::Utc::now(),
        };

        let json_content = serde_json::to_string_pretty(&export_cache)?;
        fs::write(output_path, json_content)?;

        log(
            LogTag::System,
            "SUCCESS",
            &format!("Exported {} transactions to {}", export_cache.transactions.len(), output_path)
        );

        Ok(export_cache.transactions.len())
    }

    /// Get migration statistics
    pub async fn get_migration_stats(&self) -> Result<MigrationStats, Box<dyn Error>> {
        let db_count = self.db.get_transaction_count()?;
        let db_size = self.get_database_size()?;

        Ok(MigrationStats {
            total_transactions: db_count,
            database_size_bytes: db_size,
            migration_complete: db_count > 0,
            last_migration_time: chrono::Utc::now(), // Could be stored in database
        })
    }

    /// Get database file size
    fn get_database_size(&self) -> Result<u64, Box<dyn Error>> {
        let db_path = "transactions.db";
        if Path::new(db_path).exists() {
            let metadata = fs::metadata(db_path)?;
            Ok(metadata.len())
        } else {
            Ok(0)
        }
    }
}

/// Legacy transaction cache structure for migration
#[derive(serde::Deserialize, serde::Serialize)]
struct OldTransactionCache {
    transactions: std::collections::HashMap<String, TransactionResult>,
    last_update: chrono::DateTime<chrono::Utc>,
}

/// Migration result information
#[derive(Debug)]
pub struct MigrationResult {
    pub transactions_migrated: usize,
    pub errors_encountered: usize,
    pub success: bool,
    pub message: String,
}

/// Migration verification results
#[derive(Debug)]
pub struct MigrationVerification {
    pub json_transaction_count: usize,
    pub db_transaction_count: usize,
    pub integrity_check_passed: bool,
    pub missing_transactions: Vec<String>,
}

/// Migration statistics
#[derive(Debug)]
pub struct MigrationStats {
    pub total_transactions: usize,
    pub database_size_bytes: u64,
    pub migration_complete: bool,
    pub last_migration_time: chrono::DateTime<chrono::Utc>,
}

/// Run complete migration process
pub async fn run_complete_migration(
    json_file_path: &str
) -> Result<MigrationResult, Box<dyn Error>> {
    let migration = TransactionMigration::new()?;

    log(LogTag::System, "INFO", "Starting complete migration process...");

    // Step 1: Migrate data
    let result = migration.migrate_from_json(json_file_path).await?;

    if result.success {
        // Step 2: Verify migration
        let verification = migration.verify_migration(json_file_path).await?;

        if verification.integrity_check_passed {
            log(
                LogTag::System,
                "SUCCESS",
                "Migration completed successfully with integrity check passed"
            );

            // Step 3: Cleanup (optional)
            // migration.cleanup_old_files(json_file_path).await?;
        } else {
            log(
                LogTag::System,
                "WARNING",
                &format!(
                    "Migration completed but integrity check failed. Missing {} transactions",
                    verification.missing_transactions.len()
                )
            );
        }
    }

    Ok(result)
}
