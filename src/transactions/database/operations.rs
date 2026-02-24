// Core database operations and implementation

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use crate::database;
use crate::logger::{self, LogTag};
use crate::transactions::{types::*, utils::*};

use super::schema::*;

// =============================================================================
// TRANSACTION DATABASE MANAGER
// =============================================================================

/// High-performance, thread-safe database manager for transactions
///
/// Features:
/// - Connection pooling for concurrent access
/// - Separation of raw blockchain data from analysis results
/// - ACID transactions for data integrity
/// - High-performance batch operations
/// - Comprehensive indexing for fast queries
/// - Built-in health checks and integrity validation
pub struct TransactionDatabase {
    pub(super) pool: Pool<SqliteConnectionManager>,
    pub(super) database_path: String,
    pub(super) schema_version: u32,
}

/// Minimal row for wallet flow cache export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowExportRow {
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub sol_delta: f64,
}

impl TransactionDatabase {
    /// Create new TransactionDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        let database_path = crate::paths::get_transactions_db_path();
        let is_first_init = !DATABASE_INITIALIZED.load(Ordering::Relaxed);
        let db = Self::create_database(database_path, is_first_init).await?;

        DATABASE_INITIALIZED.store(true, Ordering::Relaxed);

        Ok(db)
    }

    async fn create_database(database_path: PathBuf, log_details: bool) -> Result<Self, String> {
        let database_path_str = database_path.to_string_lossy().to_string();

        if log_details {
            logger::info(
                LogTag::Transactions,
                &format!("Initializing TransactionDatabase at: {database_path_str}"),
            );
        }

        let manager = SqliteConnectionManager::file(&database_path)
            .with_init(|c| database::configure_connection(c, database::TRANSACTIONS_DB));

        let pool = Pool::builder()
            .max_size(5)
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {e}"))?;

        let mut db = Self {
            pool,
            database_path: database_path_str,
            schema_version: DATABASE_SCHEMA_VERSION,
        };

        db.initialize_schema().await?;

        if log_details {
            logger::info(
                LogTag::Transactions,
                "TransactionDatabase initialization complete",
            );
        }

        Ok(db)
    }

    #[cfg(test)]
    pub(crate) async fn new_with_path<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let database_path = path.as_ref().to_path_buf();

        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {e}"))?;
        }

        Self::create_database(database_path, true).await
    }

    /// Initialize database schema and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let mut conn = self
            .get_connection()
            .map_err(|e| format!("Failed to get database connection: {e}"))?;

        // Create all tables
        let tables = [
            SCHEMA_RAW_TRANSACTIONS,
            SCHEMA_PROCESSED_TRANSACTIONS,
            SCHEMA_KNOWN_SIGNATURES,
            SCHEMA_DEFERRED_RETRIES,
            SCHEMA_PENDING_TRANSACTIONS,
            SCHEMA_METADATA,
            SCHEMA_BOOTSTRAP_STATE,
        ];

        for table_sql in &tables {
            conn.execute(table_sql, [])
                .map_err(|e| format!("Failed to create table: {e}"))?;
        }

        // Create all indexes
        for index_sql in INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create index: {e}"))?;
        }

        // Apply lightweight migrations for existing databases
        self.apply_migrations(&mut conn)?;

        // Set or update schema version
        conn.execute(
            "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
            params!["schema_version", self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set schema version: {e}"))?;

        // Store current wallet address in metadata
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
            params!["current_wallet", wallet_address],
        )
        .map_err(|e| format!("Failed to set current_wallet in metadata: {e}"))?;

        Ok(())
    }

    /// Apply schema migrations when upgrading versions
    fn apply_migrations(&self, conn: &mut Connection) -> Result<(), String> {
        // Ensure processed_transactions has fee_sol column for MCP tools compatibility
        let mut has_fee_sol = false;
        let mut has_sol_delta = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(processed_transactions)")
            .map_err(|e| format!("Failed to inspect processed_transactions schema: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| format!("Failed to read processed_transactions schema: {e}"))?;
        for r in rows {
            let name = r.map_err(|e| format!("Failed to parse schema row: {e}"))?;
            if name.eq_ignore_ascii_case("fee_sol") {
                has_fee_sol = true;
            } else if name.eq_ignore_ascii_case("sol_delta") {
                has_sol_delta = true;
            }
        }
        drop(stmt);
        if !has_fee_sol {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN fee_sol REAL NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|e| format!("Failed to add fee_sol column: {e}"))?;
        }

        if !has_sol_delta {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN sol_delta REAL",
                [],
            )
            .map_err(|e| format!("Failed to add sol_delta column: {e}"))?;

            self.backfill_processed_sol_delta(conn)?;
        }

        // Ensure bootstrap_state table exists (idempotent)
        conn.execute(SCHEMA_BOOTSTRAP_STATE, [])
            .map_err(|e| format!("Failed to ensure bootstrap_state table: {e}"))?;

        // Ensure the single row exists
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (id, full_history_completed) VALUES (1, 0)",
            [],
        )
        .map_err(|e| format!("Failed to initialize bootstrap_state row: {e}"))?;
        Ok(())
    }

    fn backfill_processed_sol_delta(&self, conn: &mut Connection) -> Result<(), String> {
        const BATCH_SIZE: i64 = 1000;
        let mut total_updated = 0usize;

        // Get wallet address for filtering (this is a migration function, so it operates on current wallet data only)
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address for sol_delta backfill: {e}"))?;

        loop {
            let mut stmt = conn
                .prepare(
                    "SELECT signature, sol_balance_change FROM processed_transactions WHERE wallet_address = ?1 AND sol_delta IS NULL LIMIT ?2",
                )
                .map_err(|e| format!("Failed to prepare sol_delta backfill query: {e}"))?;

            let rows = stmt
                .query_map(params![wallet_address, BATCH_SIZE], |row| {
                    let signature: String = row.get(0)?;
                    let change_json: Option<String> = row.get(1)?;
                    Ok((signature, change_json))
                })
                .map_err(|e| format!("Failed to iterate sol_delta backfill rows: {e}"))?;

            let mut batch: Vec<(String, Option<String>)> = Vec::new();
            for row in rows {
                let (signature, change_json) =
                    row.map_err(|e| format!("Failed to read sol_delta row: {e}"))?;
                batch.push((signature, change_json));
            }

            if batch.is_empty() {
                break;
            }

            drop(stmt);

            let tx = conn
                .transaction()
                .map_err(|e| format!("Failed to start sol_delta backfill transaction: {e}"))?;

            for (signature, change_json) in batch.into_iter() {
                let delta = Self::compute_sol_delta_from_json(change_json.as_deref());
                tx.execute(
                    "UPDATE processed_transactions SET sol_delta = ?1 WHERE signature = ?2 AND wallet_address = ?3",
                    params![delta, signature, wallet_address],
                )
                .map_err(|e| format!("Failed to update sol_delta: {e}"))?;
                total_updated += 1;
            }

            tx.commit()
                .map_err(|e| format!("Failed to commit sol_delta backfill: {e}"))?;
        }

        if total_updated > 0 {
            logger::info(
                LogTag::Transactions,
                &format!(
                    "Backfilled sol_delta for {} processed transactions",
                    total_updated
                ),
            );
        }

        Ok(())
    }

    fn compute_sol_delta_from_json(payload: Option<&str>) -> f64 {
        let Some(raw) = payload else {
            return 0.0;
        };

        if raw.trim().is_empty() {
            return 0.0;
        }

        match serde_json::from_str::<Vec<SolBalanceChange>>(raw) {
            Ok(changes) => changes.iter().map(|change| change.change).sum(),
            Err(err) => {
                logger::info(
                    LogTag::Transactions,
                    &format!("Failed to parse sol_balance_change payload: {err}"),
                );
                0.0
            }
        }
    }

    /// Get database connection from pool
    pub(super) fn get_connection(
        &self,
    ) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get database connection from pool: {e}"))
    }

    /// Health check - verify database connectivity and basic operations
    pub async fn health_check(&self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Test basic query
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Database health check failed: {e}"))?;

        if count < 5 {
            return Err("Database schema incomplete".to_string());
        }

        Ok(())
    }
}

// =============================================================================
// IMPLEMENTATION - KNOWN SIGNATURES MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Check if signature is known in database
    pub async fn is_signature_known(&self, signature: &str) -> Result<bool, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM known_signatures WHERE signature = ?1 AND wallet_address = ?2)",
                params![signature, wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check known signature: {e}"))?;

        Ok(exists)
    }

    /// Add signature to known signatures
    pub async fn add_known_signature(&self, signature: &str) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT OR IGNORE INTO known_signatures (signature, wallet_address) VALUES (?1, ?2)",
            params![signature, wallet_address],
        )
        .map_err(|e| format!("Failed to add known signature: {e}"))?;

        Ok(())
    }

    /// Get count of known signatures
    pub async fn get_known_signatures_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get known signatures count: {e}"))?;

        Ok(count as u64)
    }

    /// Get the newest known signature (most recently added)
    pub async fn get_newest_known_signature(&self) -> Result<Option<String>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures WHERE wallet_address = ?1 ORDER BY added_at DESC LIMIT 1",
                params![wallet_address],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to get newest known signature: {e}"))?;

        Ok(result)
    }

    /// Get the oldest known signature for incremental fetching checkpoint
    /// Returns None if no signatures are known yet (first run)
    ///
    /// When fetching backwards from blockchain (newest→oldest), we stop when we hit
    /// the oldest signature we already have, ensuring we only fetch missing history.
    pub async fn get_oldest_known_signature(&self) -> Result<Option<String>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures WHERE wallet_address = ?1 ORDER BY added_at ASC LIMIT 1",
                params![wallet_address],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to get oldest known signature: {e}"))?;

        Ok(result)
    }

    /// Remove old known signatures (cleanup)
    pub async fn cleanup_old_known_signatures(&self, days: i64) -> Result<usize, String> {
        let conn = self.get_connection()?;

        let affected = conn
            .execute(
                "DELETE FROM known_signatures WHERE added_at < datetime('now', '-' || ?1 || ' days')",
                params![days]
            )
            .map_err(|e| format!("Failed to cleanup old known signatures: {e}"))?;

        Ok(affected)
    }
}

// =============================================================================
// IMPLEMENTATION - PENDING TRANSACTIONS MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Save pending transactions to database
    pub async fn save_pending_transactions(
        &self,
        pending: &HashMap<String, DateTime<Utc>>,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        for (signature, timestamp) in pending {
            tx.execute(
                "INSERT OR REPLACE INTO pending_transactions (signature, wallet_address, added_at) VALUES (?1, ?2, ?3)",
                params![signature, wallet_address, timestamp.to_rfc3339()],
            )
            .map_err(|e| format!("Failed to save pending transaction: {e}"))?;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit pending transactions: {e}"))?;

        Ok(())
    }

    /// Load pending transactions from database
    pub async fn get_pending_transactions(&self) -> Result<HashMap<String, DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT signature, added_at FROM pending_transactions WHERE wallet_address = ?1",
            )
            .map_err(|e| format!("Failed to prepare pending transactions query: {e}"))?;

        let rows = stmt
            .query_map(params![wallet_address], |row| {
                let signature: String = row.get(0)?;
                let timestamp_str: String = row.get(1)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);
                Ok((signature, timestamp))
            })
            .map_err(|e| format!("Failed to query pending transactions: {e}"))?;

        let mut result = HashMap::new();
        for row in rows {
            let (signature, timestamp) =
                row.map_err(|e| format!("Failed to parse pending transaction row: {e}"))?;
            result.insert(signature, timestamp);
        }

        Ok(result)
    }

    /// Remove pending transaction
    pub async fn remove_pending_transaction(&self, signature: &str) -> Result<bool, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let affected = conn
            .execute(
                "DELETE FROM pending_transactions WHERE signature = ?1 AND wallet_address = ?2",
                params![signature, wallet_address],
            )
            .map_err(|e| format!("Failed to remove pending transaction: {e}"))?;

        Ok(affected > 0)
    }

    /// Get count of pending transactions
    pub async fn get_pending_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get pending transactions count: {e}"))?;

        Ok(count as u64)
    }
}

// =============================================================================
// IMPLEMENTATION - TRANSACTION DATA MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Load raw transaction JSON blob and deserialize into TransactionDetails (cache-first path)
    pub async fn get_raw_transaction_details(
        &self,
        signature: &str,
    ) -> Result<Option<crate::rpc::TransactionDetails>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let result: rusqlite::Result<Option<String>> = conn
            .query_row(
                "SELECT raw_transaction_data FROM raw_transactions WHERE signature = ?1 AND wallet_address = ?2",
                params![signature, wallet_address],
                |row| row.get(0),
            )
            .optional();

        match result {
            Ok(Some(json_str)) => {
                if json_str.trim().is_empty() {
                    return Ok(None);
                }
                match serde_json::from_str::<crate::rpc::TransactionDetails>(&json_str) {
                    Ok(details) => Ok(Some(details)),
                    Err(e) => Err(format!(
                        "Failed to deserialize cached raw transaction for {}: {}",
                        signature, e
                    )),
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to read cached raw transaction: {e}")),
        }
    }

    /// Store raw transaction data
    pub async fn store_raw_transaction(&self, transaction: &Transaction) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let status_str = match &transaction.status {
            TransactionStatus::Pending => "Pending",
            TransactionStatus::Confirmed => "Confirmed",
            TransactionStatus::Finalized => "Finalized",
            TransactionStatus::Failed(msg) => "Failed",
        };

        let raw_transaction_json = transaction
            .raw_transaction_data
            .as_ref()
            .and_then(|value| serde_json::to_string(value).ok());

        conn
            .execute(
                r#"INSERT OR REPLACE INTO raw_transactions
               (signature, wallet_address, slot, block_time, timestamp, status, success, error_message,
                fee_lamports, compute_units_consumed, instructions_count, accounts_count, raw_transaction_data, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'))"#,
                params![
                    transaction.signature,
                    wallet_address,
                    transaction.slot,
                    transaction.block_time,
                    transaction.timestamp.to_rfc3339(),
                    status_str,
                    transaction.success,
                    transaction.error_message,
                    transaction.fee_lamports,
                    transaction.compute_units_consumed,
                    transaction.instructions_count,
                    transaction.accounts_count,
                    raw_transaction_json
                ]
            )
            .map_err(|e| format!("Failed to store raw transaction: {e}"))?;

        Ok(())
    }

    /// Store processed transaction analysis snapshot
    pub async fn store_processed_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Serialize complex fields as JSON strings
        let sol_balance_change_json = serde_json::to_string(&transaction.sol_balance_changes)
            .unwrap_or_else(|_| "[]".to_string());
        let token_balance_changes_json = serde_json::to_string(&transaction.token_balance_changes)
            .unwrap_or_else(|_| "[]".to_string());
        let token_swap_info_json = serde_json::to_string(&transaction.token_swap_info)
            .unwrap_or_else(|_| "null".to_string());
        let swap_pnl_info_json = serde_json::to_string(&transaction.swap_pnl_info)
            .unwrap_or_else(|_| "null".to_string());
        let ata_operations_json =
            serde_json::to_string(&transaction.ata_operations).unwrap_or_else(|_| "[]".to_string());
        let token_transfers_json = serde_json::to_string(&transaction.token_transfers)
            .unwrap_or_else(|_| "[]".to_string());
        let instruction_info_json =
            serde_json::to_string(&transaction.instructions).unwrap_or_else(|_| "[]".to_string());
        let cached_analysis_json = serde_json::to_string(&transaction.cached_analysis)
            .unwrap_or_else(|_| "null".to_string());

        let tx_type = format!("{:?}", transaction.transaction_type);
        let dir = format!("{:?}", transaction.direction);

        let sol_delta = if !transaction.sol_balance_changes.is_empty() {
            transaction
                .sol_balance_changes
                .iter()
                .map(|change| change.change)
                .sum()
        } else {
            transaction.sol_balance_change
        };

        conn
            .execute(
                r#"INSERT OR REPLACE INTO processed_transactions
                   (signature, wallet_address, transaction_type, direction, sol_balance_change, token_balance_changes,
                    token_swap_info, swap_pnl_info, ata_operations, token_transfers, instruction_info,
                    analysis_duration_ms, cached_analysis, analysis_version, fee_sol, sol_delta, updated_at)
                 VALUES
                   (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, datetime('now'))"#,
                params![
                    transaction.signature,
                    wallet_address,
                    tx_type,
                    dir,
                    sol_balance_change_json,
                    token_balance_changes_json,
                    token_swap_info_json,
                    swap_pnl_info_json,
                    ata_operations_json,
                    token_transfers_json,
                    instruction_info_json,
                    transaction.analysis_duration_ms,
                    cached_analysis_json,
                    ANALYSIS_CACHE_VERSION as i64,
                    transaction.fee_sol,
                    sol_delta
                ]
            )
            .map_err(|e| format!("Failed to store processed transaction: {e}"))?;

        Ok(())
    }

    /// Convenience: upsert both raw and processed snapshots
    pub async fn upsert_full_transaction(&self, transaction: &Transaction) -> Result<(), String> {
        self.store_raw_transaction(transaction).await?;
        self.store_processed_transaction(transaction).await?;
        Ok(())
    }

    /// Update transaction status
    pub async fn update_transaction_status(
        &self,
        signature: &str,
        status: &str,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        conn
            .execute(
                "UPDATE raw_transactions SET status = ?1, success = ?2, error_message = ?3, updated_at = datetime('now') WHERE signature = ?4 AND wallet_address = ?5",
                params![status, success, error_message, signature, wallet_address]
            )
            .map_err(|e| format!("Failed to update transaction status: {e}"))?;

        Ok(())
    }

    /// Get transaction by signature with full analysis data
    pub async fn get_transaction(&self, signature: &str) -> Result<Option<Transaction>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Join raw_transactions with processed_transactions to get full data
        let result = conn.query_row(
            r#"SELECT
                r.signature, r.slot, r.block_time, r.timestamp, r.status, r.success, r.error_message,
                r.fee_lamports, r.compute_units_consumed, r.instructions_count, r.accounts_count,
                r.raw_transaction_data,
                p.transaction_type, p.direction, p.sol_balance_change, p.token_balance_changes,
                p.token_swap_info, p.swap_pnl_info, p.ata_operations, p.token_transfers,
                p.instruction_info, p.analysis_duration_ms, p.cached_analysis, p.fee_sol, p.sol_delta
            FROM raw_transactions r
            LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?2
            WHERE r.signature = ?1 AND r.wallet_address = ?2"#,
            params![signature, wallet_address],
            |row| {
                let timestamp_str: String = row.get(3)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            3,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                let status_str: String = row.get(4)?;
                let status = match status_str.as_str() {
                    "Pending" => TransactionStatus::Pending,
                    "Confirmed" => TransactionStatus::Confirmed,
                    "Finalized" => TransactionStatus::Finalized,
                    s if s.starts_with("Failed") => TransactionStatus::Failed(s.to_string()),
                    _ => TransactionStatus::Pending,
                };

                // Parse raw_transaction_data JSON if present
                let raw_transaction_data: Option<serde_json::Value> = row
                    .get::<_, Option<String>>(11)?
                    .and_then(|json| serde_json::from_str(&json).ok());

                // Parse processed fields from joined data
                let transaction_type_str: Option<String> = row.get(12)?;
                let transaction_type = transaction_type_str
                    .as_ref()
                    .and_then(|s| {
                        // First try parsing as JSON object (for rich variants like SwapSolToToken)
                        serde_json::from_str(s)
                            .ok()
                            // Then try as quoted string (for simple variants like "Sell")
                            .or_else(|| serde_json::from_str(&format!("\"{}\"", s)).ok())
                    })
                    .unwrap_or(TransactionType::Unknown);

                let direction_str: Option<String> = row.get(13)?;
                let direction = match direction_str.as_deref() {
                    Some("Incoming") => TransactionDirection::Incoming,
                    Some("Outgoing") => TransactionDirection::Outgoing,
                    Some("Internal") => TransactionDirection::Internal,
                    _ => TransactionDirection::Unknown,
                };

                let sol_balance_change_json: Option<String> = row.get(14)?;
                let sol_balance_changes: Vec<SolBalanceChange> = sol_balance_change_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                // Use sol_delta from the dedicated column (index 24) for the aggregate change
                let sol_delta: f64 = row.get::<_, Option<f64>>(24)?.unwrap_or_default();

                let token_balance_changes_json: Option<String> = row.get(15)?;
                let token_balance_changes: Vec<TokenBalanceChange> = token_balance_changes_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let token_swap_info_json: Option<String> = row.get(16)?;
                let token_swap_info: Option<TokenSwapInfo> = token_swap_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let swap_pnl_info_json: Option<String> = row.get(17)?;
                let swap_pnl_info: Option<SwapPnLInfo> = swap_pnl_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let ata_operations_json: Option<String> = row.get(18)?;
                let ata_operations: Vec<AtaOperation> = ata_operations_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let token_transfers_json: Option<String> = row.get(19)?;
                let token_transfers: Vec<TokenTransfer> = token_transfers_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let instruction_info_json: Option<String> = row.get(20)?;
                let instruction_info: Vec<InstructionInfo> = instruction_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok())
                    .unwrap_or_default();

                let analysis_duration_ms: Option<u64> = row.get::<_, Option<i64>>(21)?
                    .map(|v| v as u64);

                let cached_analysis_json: Option<String> = row.get(22)?;
                let cached_analysis: Option<CachedAnalysis> = cached_analysis_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let fee_sol: f64 = row.get::<_, Option<f64>>(23)?.unwrap_or_default();

                let mut tx = Transaction {
                    signature: row.get(0)?,
                    slot: row.get(1)?,
                    block_time: row.get(2)?,
                    timestamp,
                    status,
                    transaction_type,
                    direction,
                    success: row.get(5)?,
                    error_message: row.get(6)?,
                    fee_sol,
                    fee_lamports: row.get(7)?,
                    compute_units_consumed: row.get(8)?,
                    instructions_count: row.get(9).unwrap_or_default(),
                    accounts_count: row.get(10).unwrap_or_default(),
                    sol_balance_change: sol_delta,
                    sol_balance_changes,
                    token_transfers,
                    token_balance_changes,
                    token_swap_info,
                    swap_pnl_info,
                    ata_operations,
                    instruction_info,
                    raw_transaction_data,
                    analysis_duration_ms,
                    cached_analysis,
                    last_updated: Utc::now(),
                    // These are populated from raw_transaction_data below
                    wallet_lamport_change: 0,
                    wallet_signed: false,
                    log_messages: Vec::new(),
                    instructions: Vec::new(),
                    position_impact: None,
                    profit_calculation: None,
                    ata_analysis: None,
                    token_info: None,
                    calculated_token_price_sol: None,
                    token_symbol: None,
                    token_decimals: None,
                };

                // Populate log_messages and instructions from raw_transaction_data
                tx.populate_from_raw_data();

                Ok(tx)
            },
        );

        match result {
            Ok(transaction) => Ok(Some(transaction)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get transaction: {e}")),
        }
    }

    /// Get successful transactions count
    pub async fn get_successful_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1 AND success = true AND status != 'Failed'",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get successful transactions count: {e}"))?;

        Ok(count as u64)
    }

    /// Get failed transactions count
    pub async fn get_failed_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1 AND (success = false OR status = 'Failed')",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get failed transactions count: {e}"))?;

        Ok(count as u64)
    }
}
