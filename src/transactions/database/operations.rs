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

use crate::logger::{self, LogTag};
use crate::transactions::{types::*, utils::*};
use crate::{chains::ChainId, database};

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
    pub(super) chain: ChainId,
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
    pub async fn new(chain: ChainId) -> Result<Self, String> {
        let database_path = crate::paths::get_transactions_db_path();
        let is_first_init = !DATABASE_INITIALIZED.load(Ordering::Relaxed);
        let db = Self::create_database(database_path, is_first_init, true, chain).await?;

        DATABASE_INITIALIZED.store(true, Ordering::Relaxed);

        Ok(db)
    }

    async fn create_database(
        database_path: PathBuf,
        log_details: bool,
        record_current_wallet: bool,
        chain: ChainId,
    ) -> Result<Self, String> {
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
            chain,
        };

        db.initialize_schema(record_current_wallet).await?;

        if log_details {
            logger::info(
                LogTag::Transactions,
                "TransactionDatabase initialization complete",
            );
        }

        Ok(db)
    }

    #[cfg(test)]
    pub(crate) async fn new_with_path<P: AsRef<Path>>(
        path: P,
        chain: ChainId,
    ) -> Result<Self, String> {
        let database_path = path.as_ref().to_path_buf();

        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {e}"))?;
        }

        // Explicit-path databases are used for isolated tests and offline
        // migration validation. A fresh composite-schema database has no need to
        // resolve process-global wallet configuration merely to write metadata.
        Self::create_database(database_path, true, false, chain).await
    }

    /// Initialize database schema and indexes
    async fn initialize_schema(&mut self, record_current_wallet: bool) -> Result<(), String> {
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
            SCHEMA_SUBJECT_ASSET_DELTAS,
        ];

        for table_sql in &tables {
            conn.execute(table_sql, [])
                .map_err(|e| format!("Failed to create table: {e}"))?;
        }

        // Apply the legacy processed-transaction column migrations before the v5
        // rebuild below: that rebuild copies every column of
        // `processed_transactions`, and therefore needs fee_sol/sol_delta to exist
        // on the pre-v5 shape. This deliberately does not query chain-aware state;
        // legacy bootstrap_state has no chain_id until the v7 rebuild.
        let needs_sol_delta_backfill = self.apply_pre_chain_migrations(&mut conn)?;

        // §7.1: rebuild the signature-keyed tables onto a composite
        // (signature, wallet_address) primary key so a second subject's perspective
        // on a signature no longer silently replaces the first. This drops and
        // recreates whichever of the five tables is still in the pre-v5 shape, so it
        // must run after table creation above (a table has to exist to migrate) and
        // before index creation below (rebuilding a table drops its indexes with it).
        self.migrate_signature_wallet_tables(&mut conn)?;
        self.migrate_chain_identity_tables(&mut conn)?;

        // Chain-aware bootstrap state and queries are valid only after every legacy
        // transaction table, including bootstrap_state, has been rebuilt to v7.
        self.initialize_chain_bootstrap_state(&mut conn)?;
        if needs_sol_delta_backfill {
            self.backfill_processed_sol_delta(&mut conn)?;
        }

        // Create all indexes (fresh for anything just rebuilt above)
        for index_sql in INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create index: {e}"))?;
        }

        // Set or update schema version
        conn.execute(
            "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
            params!["schema_version", self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set schema version: {e}"))?;

        if record_current_wallet {
            let wallet_address = crate::utils::get_wallet_address()
                .map_err(|e| format!("Failed to get wallet address: {e}"))?;
            conn.execute(
                "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
                params!["current_wallet", wallet_address],
            )
            .map_err(|e| format!("Failed to set current_wallet in metadata: {e}"))?;
        }

        Ok(())
    }

    /// Get database connection from pool
    pub(super) fn get_connection(
        &self,
    ) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get database connection from pool: {e}"))
    }

    pub(super) fn require_subject_chain(&self, subject: &Subject) -> Result<&'static str, String> {
        if subject.chain() != self.chain {
            return Err(format!(
                "Transaction subject chain {} does not match database chain {}",
                subject.chain(),
                self.chain
            ));
        }
        Ok(self.chain.as_str())
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
        let chain_id = self.require_subject_chain(&subject)?;

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM known_signatures WHERE chain_id = ?1 AND signature = ?2 AND wallet_address = ?3)",
                params![chain_id, signature, wallet_address],
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
        let chain_id = self.require_subject_chain(&subject)?;

        conn.execute(
            "INSERT OR IGNORE INTO known_signatures (chain_id, signature, wallet_address) VALUES (?1, ?2, ?3)",
            params![chain_id, signature, wallet_address],
        )
        .map_err(|e| format!("Failed to add known signature: {e}"))?;

        Ok(())
    }

    /// Get count of known signatures, for the given subject
    pub async fn get_known_signatures_count(&self, subject: Subject) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = subject.address();
        let chain_id = self.require_subject_chain(&subject)?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2",
                params![chain_id, wallet_address],
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
        let chain_id = self.require_subject_chain(&subject)?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2 ORDER BY added_at DESC LIMIT 1",
                params![chain_id, wallet_address],
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
        let chain_id = self.require_subject_chain(&subject)?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures WHERE chain_id = ?1 AND wallet_address = ?2 ORDER BY added_at ASC LIMIT 1",
                params![chain_id, wallet_address],
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
                "DELETE FROM known_signatures WHERE chain_id = ?1 AND added_at < datetime('now', '-' || ?2 || ' days')",
                params![self.chain.as_str(), days]
            )
            .map_err(|e| format!("Failed to cleanup old known signatures: {e}"))?;

        Ok(affected)
    }
}
