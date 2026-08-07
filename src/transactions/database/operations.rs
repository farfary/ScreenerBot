//! Transaction database operations — CRUD methods for transaction records.
//
// Core database operations and implementation

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
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
            return Err("Database schema incomplete".to_owned());
        }

        Ok(())
    }
}

// =============================================================================
// IMPLEMENTATION - KNOWN SIGNATURES MANAGEMENT
// =============================================================================

impl TransactionDatabase {
    /// Check if signature is known in database, for the given subject
    pub async fn is_signature_known(
        &self,
        subject: Subject,
        signature: &str,
    ) -> Result<bool, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM known_signatures WHERE signature = ?1 AND wallet_address = ?2)",
                params![signature, wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check known signature: {e}"))?;

        Ok(exists)
    }

    /// Add signature to known signatures, for the given subject
    pub async fn add_known_signature(
        &self,
        subject: Subject,
        signature: &str,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        conn.execute(
            "INSERT OR IGNORE INTO known_signatures (signature, wallet_address) VALUES (?1, ?2)",
            params![signature, wallet_address],
        )
        .map_err(|e| format!("Failed to add known signature: {e}"))?;

        Ok(())
    }

    /// Get count of known signatures, for the given subject
    pub async fn get_known_signatures_count(&self, subject: Subject) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get known signatures count: {e}"))?;

        Ok(count as u64)
    }

    /// Get the newest known signature (most recently added), for the given subject
    pub async fn get_newest_known_signature(
        &self,
        subject: Subject,
    ) -> Result<Option<String>, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

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

    /// Get the oldest known signature for incremental fetching checkpoint, for the given subject
    /// Returns None if no signatures are known yet (first run)
    ///
    /// When fetching backwards from blockchain (newest→oldest), we stop when we hit
    /// the oldest signature we already have, ensuring we only fetch missing history.
    pub async fn get_oldest_known_signature(
        &self,
        subject: Subject,
    ) -> Result<Option<String>, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();

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
