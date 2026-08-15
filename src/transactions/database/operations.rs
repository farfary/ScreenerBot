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
        let db = Self::create_database(database_path, is_first_init, true).await?;

        DATABASE_INITIALIZED.store(true, Ordering::Relaxed);

        Ok(db)
    }

    async fn create_database(
        database_path: PathBuf,
        log_details: bool,
        record_current_wallet: bool,
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
    pub(crate) async fn new_with_path<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let database_path = path.as_ref().to_path_buf();

        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {e}"))?;
        }

        // Explicit-path databases are used for isolated tests and offline
        // migration validation. A fresh composite-schema database has no need to
        // resolve process-global wallet configuration merely to write metadata.
        Self::create_database(database_path, true, false).await
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

        // Apply lightweight migrations for existing databases (fee_sol/sol_delta
        // columns, bootstrap_state row). Must run before the composite-key migration
        // below: that migration copies every column of `processed_transactions`, and
        // depends on fee_sol/sol_delta already existing on the pre-v5 table.
        self.apply_migrations(&mut conn)?;

        // §7.1: rebuild the signature-keyed tables onto a composite
        // (signature, wallet_address) primary key so a second subject's perspective
        // on a signature no longer silently replaces the first. This drops and
        // recreates whichever of the five tables is still in the pre-v5 shape, so it
        // must run after table creation above (a table has to exist to migrate) and
        // before index creation below (rebuilding a table drops its indexes with it).
        self.migrate_signature_wallet_tables(&mut conn)?;

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

    // =========================================================================
    // §7.1 MIGRATION: composite (signature, wallet_address) primary key
    // =========================================================================

    /// Rebuild `raw_transactions`, `processed_transactions`, `known_signatures`,
    /// `pending_transactions` and `deferred_retries` onto a composite
    /// `(signature, wallet_address)` primary key.
    ///
    /// Before this migration all five declared `signature TEXT PRIMARY KEY`, so one
    /// signature could hold exactly one subject's perspective. That is invisible while
    /// the own wallet is the only subject, and wrong the moment a watched wallet is
    /// recorded too: our wallet and a target appear in the same transaction whenever
    /// the target sends to us, or we and the target trade the same pool in one bundle,
    /// and the later write would silently replace the earlier row.
    ///
    /// Gated on the stored schema version (fast path: a no-op read once already at
    /// v5+) and, per table, on the table's actual on-disk shape (defensive: a fresh
    /// install's tables are already composite from `CREATE TABLE IF NOT EXISTS`, and a
    /// crash mid-migration leaves only the untouched tables needing another pass), so
    /// this is safe to call on every boot.
    fn migrate_signature_wallet_tables(&self, conn: &mut Connection) -> Result<(), String> {
        let stored_version = Self::read_schema_version(conn)?;
        if stored_version.unwrap_or(0) >= 5 {
            return Ok(());
        }

        // Fresh databases are created directly in the composite shape. Do not make
        // their explicit-path/test constructor depend on process-global wallet
        // configuration merely to discover there is nothing to migrate.
        let signature_tables = [
            "raw_transactions",
            "processed_transactions",
            "known_signatures",
            "pending_transactions",
            "deferred_retries",
        ];
        if signature_tables
            .iter()
            .map(|table| Self::has_composite_signature_wallet_key(conn, table))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(|composite| composite)
        {
            return Ok(());
        }

        let own_wallet_address = crate::utils::get_wallet_address().map_err(|e| {
            format!("Failed to resolve own wallet address for schema migration: {e}")
        })?;

        logger::info(
            LogTag::Transactions,
            "Migrating transactions schema to composite (signature, wallet_address) keys (v5)...",
        );

        // SQLite's own recommended procedure for a table rebuild that other tables
        // reference by foreign key: disable enforcement for the duration (it cannot be
        // toggled inside a transaction, so this happens before BEGIN) so an orphaned
        // processed_transactions row -- possible today, see `IntegrityReport` -- cannot
        // abort the whole migration; it is simply carried over as still-orphaned.
        conn.pragma_update(None, "foreign_keys", 0)
            .map_err(|e| format!("Failed to disable foreign_keys for migration: {e}"))?;

        let migration_result = (|| -> Result<(), String> {
            let tx = conn
                .transaction()
                .map_err(|e| format!("Failed to begin v5 schema migration: {e}"))?;

            Self::rebuild_raw_transactions(&tx, &own_wallet_address)?;
            Self::rebuild_processed_transactions(&tx, &own_wallet_address)?;
            Self::rebuild_known_signatures(&tx, &own_wallet_address)?;
            Self::rebuild_pending_transactions(&tx, &own_wallet_address)?;
            Self::rebuild_deferred_retries(&tx, &own_wallet_address)?;

            tx.commit()
                .map_err(|e| format!("Failed to commit v5 schema migration: {e}"))
        })();

        conn.pragma_update(None, "foreign_keys", 1)
            .map_err(|e| format!("Failed to re-enable foreign_keys after migration: {e}"))?;

        migration_result?;

        logger::info(
            LogTag::Transactions,
            "Transactions schema migration to v5 complete",
        );

        Ok(())
    }

    /// The stored `schema_version`, or `None` when `db_metadata` has no row for it yet
    /// (a database that has never finished `initialize_schema`, including a brand new
    /// install).
    fn read_schema_version(conn: &Connection) -> Result<Option<u32>, String> {
        let raw: Option<String> = conn
            .query_row(
                "SELECT value FROM db_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to read schema_version: {e}"))?;

        raw.map(|v| {
            v.parse::<u32>()
                .map_err(|e| format!("Invalid stored schema_version '{v}': {e}"))
        })
        .transpose()
    }

    /// True when `table`'s primary key already spans more than one column. SQLite
    /// reports each PK column's 1-based position via `PRAGMA table_info`'s `pk` field,
    /// so counting columns with `pk > 0` tells composite apart from single-column.
    /// Also `false` when the table does not exist -- callers only reach this after the
    /// `CREATE TABLE IF NOT EXISTS` pass, so that case does not arise in practice, but
    /// treating it as "not yet composite" rather than erroring keeps the check total.
    fn has_composite_signature_wallet_key(conn: &Connection, table: &str) -> Result<bool, String> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(|e| format!("Failed to inspect {table} schema: {e}"))?;

        let pk_columns = stmt
            .query_map([], |row| row.get::<_, i64>(5))
            .map_err(|e| format!("Failed to read {table} schema: {e}"))?
            .filter_map(|r| r.ok())
            .filter(|&pk| pk > 0)
            .count();

        Ok(pk_columns >= 2)
    }

    /// Rebuild `raw_transactions` in place: create the v5 shape, copy every row
    /// (backfilling a NULL/empty `wallet_address` with `own_wallet_address` rather
    /// than dropping the row), drop the old table, rename the new one into place.
    fn rebuild_raw_transactions(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "raw_transactions")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE raw_transactions__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                slot INTEGER,
                block_time INTEGER,
                timestamp TEXT NOT NULL,
                status TEXT NOT NULL,
                success BOOLEAN NOT NULL DEFAULT false,
                error_message TEXT,
                fee_lamports INTEGER,
                compute_units_consumed INTEGER,
                instructions_count INTEGER NOT NULL DEFAULT 0,
                accounts_count INTEGER NOT NULL DEFAULT 0,
                raw_transaction_data TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create raw_transactions__v5: {e}"))?;

        tx.execute(
            "INSERT INTO raw_transactions__v5
                (signature, wallet_address, slot, block_time, timestamp, status, success,
                 error_message, fee_lamports, compute_units_consumed, instructions_count,
                 accounts_count, raw_transaction_data, created_at, updated_at)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), slot, block_time,
                    timestamp, status, success, error_message, fee_lamports,
                    compute_units_consumed, instructions_count, accounts_count,
                    raw_transaction_data, created_at, updated_at
             FROM raw_transactions",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy raw_transactions rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE raw_transactions", [])
            .map_err(|e| format!("Failed to drop old raw_transactions: {e}"))?;
        tx.execute(
            "ALTER TABLE raw_transactions__v5 RENAME TO raw_transactions",
            [],
        )
        .map_err(|e| format!("Failed to rename raw_transactions__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `processed_transactions` in place. Same shape as
    /// `rebuild_raw_transactions`; its foreign key is updated to the composite parent
    /// key in the same change.
    fn rebuild_processed_transactions(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "processed_transactions")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE processed_transactions__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                direction TEXT NOT NULL,
                sol_balance_change TEXT,
                token_balance_changes TEXT,
                token_swap_info TEXT,
                swap_pnl_info TEXT,
                ata_operations TEXT,
                token_transfers TEXT,
                instruction_info TEXT,
                analysis_duration_ms INTEGER,
                cached_analysis TEXT,
                analysis_version INTEGER NOT NULL DEFAULT 2,
                fee_sol REAL NOT NULL DEFAULT 0,
                sol_delta REAL,
                processed_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address),
                FOREIGN KEY (signature, wallet_address)
                    REFERENCES raw_transactions(signature, wallet_address) ON DELETE CASCADE
            )",
            [],
        )
        .map_err(|e| format!("Failed to create processed_transactions__v5: {e}"))?;

        tx.execute(
            "INSERT INTO processed_transactions__v5
                (signature, wallet_address, transaction_type, direction, sol_balance_change,
                 token_balance_changes, token_swap_info, swap_pnl_info, ata_operations,
                 token_transfers, instruction_info, analysis_duration_ms, cached_analysis,
                 analysis_version, fee_sol, sol_delta, processed_at, updated_at)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), transaction_type,
                    direction, sol_balance_change, token_balance_changes, token_swap_info,
                    swap_pnl_info, ata_operations, token_transfers, instruction_info,
                    analysis_duration_ms, cached_analysis, analysis_version, fee_sol,
                    sol_delta, processed_at, updated_at
             FROM processed_transactions",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy processed_transactions rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE processed_transactions", [])
            .map_err(|e| format!("Failed to drop old processed_transactions: {e}"))?;
        tx.execute(
            "ALTER TABLE processed_transactions__v5 RENAME TO processed_transactions",
            [],
        )
        .map_err(|e| format!("Failed to rename processed_transactions__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `known_signatures` in place (no foreign key, no extra columns).
    fn rebuild_known_signatures(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "known_signatures")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE known_signatures__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'known',
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create known_signatures__v5: {e}"))?;

        tx.execute(
            "INSERT INTO known_signatures__v5 (signature, wallet_address, status, added_at)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), status, added_at
             FROM known_signatures",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy known_signatures rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE known_signatures", [])
            .map_err(|e| format!("Failed to drop old known_signatures: {e}"))?;
        tx.execute(
            "ALTER TABLE known_signatures__v5 RENAME TO known_signatures",
            [],
        )
        .map_err(|e| format!("Failed to rename known_signatures__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `pending_transactions` in place (no foreign key, no extra columns).
    fn rebuild_pending_transactions(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "pending_transactions")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE pending_transactions__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_checked_at TEXT,
                check_count INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create pending_transactions__v5: {e}"))?;

        tx.execute(
            "INSERT INTO pending_transactions__v5
                (signature, wallet_address, added_at, last_checked_at, check_count)
             SELECT signature, COALESCE(NULLIF(wallet_address, ''), ?1), added_at,
                    last_checked_at, check_count
             FROM pending_transactions",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy pending_transactions rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE pending_transactions", [])
            .map_err(|e| format!("Failed to drop old pending_transactions: {e}"))?;
        tx.execute(
            "ALTER TABLE pending_transactions__v5 RENAME TO pending_transactions",
            [],
        )
        .map_err(|e| format!("Failed to rename pending_transactions__v5: {e}"))?;

        Ok(())
    }

    /// Rebuild `deferred_retries` in place. Unlike the other four tables this one has
    /// no `wallet_address` column at all pre-v5, so every existing row is attributed to
    /// the own wallet outright rather than backfilled from a NULL/empty value.
    fn rebuild_deferred_retries(
        tx: &rusqlite::Transaction,
        own_wallet_address: &str,
    ) -> Result<(), String> {
        if Self::has_composite_signature_wallet_key(tx, "deferred_retries")? {
            return Ok(());
        }

        tx.execute(
            "CREATE TABLE deferred_retries__v5 (
                signature TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                next_retry_at TEXT NOT NULL,
                remaining_attempts INTEGER NOT NULL DEFAULT 3,
                current_delay_secs INTEGER NOT NULL DEFAULT 60,
                last_error TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (signature, wallet_address)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create deferred_retries__v5: {e}"))?;

        tx.execute(
            "INSERT INTO deferred_retries__v5
                (signature, wallet_address, next_retry_at, remaining_attempts,
                 current_delay_secs, last_error, created_at, updated_at)
             SELECT signature, ?1, next_retry_at, remaining_attempts, current_delay_secs,
                    last_error, created_at, updated_at
             FROM deferred_retries",
            params![own_wallet_address],
        )
        .map_err(|e| format!("Failed to copy deferred_retries rows into v5 shape: {e}"))?;

        tx.execute("DROP TABLE deferred_retries", [])
            .map_err(|e| format!("Failed to drop old deferred_retries: {e}"))?;
        tx.execute(
            "ALTER TABLE deferred_retries__v5 RENAME TO deferred_retries",
            [],
        )
        .map_err(|e| format!("Failed to rename deferred_retries__v5: {e}"))?;

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
