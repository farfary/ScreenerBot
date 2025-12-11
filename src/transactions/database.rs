// Database operations and persistence for the transactions module
//
// This module provides high-performance SQLite-based caching and persistence
// for transaction data, replacing the previous JSON file-based approach.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::logger::{self, LogTag};
use crate::transactions::{types::*, utils::*};

// =============================================================================
// LIST/FILTER TYPES FOR UI
// =============================================================================

/// Cursor for pagination (timestamp desc, signature desc)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCursor {
    pub timestamp: String, // RFC3339 format
    pub signature: String,
}

/// Filters for listing transactions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionListFilters {
    /// Transaction types to include: ["buy", "sell", "swap", "transfer", "ata", "failed", "unknown"]
    #[serde(default)]
    pub types: Vec<String>,

    /// Filter by token mint (partial match)
    pub mint: Option<String>,

    /// Only confirmed/finalized transactions
    pub only_confirmed: Option<bool>,

    /// Filter by direction: "Incoming", "Outgoing", "Internal", "Unknown"
    pub direction: Option<String>,

    /// Filter by status: "Pending", "Confirmed", "Finalized", "Failed"
    pub status: Option<String>,

    /// Filter by signature (partial match)
    pub signature: Option<String>,

    /// Time range (RFC3339)
    pub time_from: Option<DateTime<Utc>>,
    pub time_to: Option<DateTime<Utc>>,

    /// Filter by router (partial match)
    pub router: Option<String>,

    /// SOL delta range
    pub min_sol: Option<f64>,
    pub max_sol: Option<f64>,
}

/// Lightweight transaction row for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionListRow {
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub slot: Option<u64>,
    pub status: String,
    pub success: bool,
    pub direction: Option<String>,
    pub transaction_type: Option<String>,
    pub token_mint: Option<String>,
    pub token_symbol: Option<String>,
    pub router: Option<String>,
    pub sol_delta: f64,
    pub fee_sol: f64,
    pub fee_lamports: Option<u64>,
    pub ata_rents: f64,
    pub instructions_count: usize,
}

/// Result of list_transactions query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionListResult {
    pub items: Vec<TransactionListRow>,
    pub next_cursor: Option<TransactionCursor>,
    pub total_estimate: Option<u64>,
}

// =============================================================================
// DATABASE SCHEMA AND CONSTANTS
// =============================================================================

/// Database schema version for migration management
const DATABASE_SCHEMA_VERSION: u32 = 4;

/// Static flag to track if database has been initialized (to reduce log noise)
static DATABASE_INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Raw transactions table schema - stores blockchain data
const SCHEMA_RAW_TRANSACTIONS: &str = r#"
CREATE TABLE IF NOT EXISTS raw_transactions (
    signature TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    slot INTEGER,
    block_time INTEGER,
    timestamp TEXT NOT NULL,
    status TEXT NOT NULL, -- 'Pending', 'Confirmed', 'Finalized', 'Failed'
    success BOOLEAN NOT NULL DEFAULT false,
    error_message TEXT,
    fee_lamports INTEGER,
    compute_units_consumed INTEGER,
    instructions_count INTEGER NOT NULL DEFAULT 0,
    accounts_count INTEGER NOT NULL DEFAULT 0,
    raw_transaction_data TEXT, -- JSON blob of raw Solana transaction data
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Processed transactions table schema - stores analysis results
const SCHEMA_PROCESSED_TRANSACTIONS: &str = r#"
CREATE TABLE IF NOT EXISTS processed_transactions (
    signature TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    transaction_type TEXT NOT NULL, -- Serialized TransactionType enum
    direction TEXT NOT NULL, -- 'Incoming', 'Outgoing', 'Internal', 'Unknown'
    
    -- Balance change data (calculated fresh, not cached)
    sol_balance_change TEXT, -- JSON blob of SolBalanceChange
    token_balance_changes TEXT, -- JSON array of TokenBalanceChange
    
    -- Swap analysis data (calculated fresh, not cached)
    token_swap_info TEXT, -- JSON blob of TokenSwapInfo
    swap_pnl_info TEXT, -- JSON blob of SwapPnLInfo
    
    -- ATA operations data (calculated fresh, not cached)
    ata_operations TEXT, -- JSON array of AtaOperation
    
    -- Token transfers data (calculated fresh, not cached)
    token_transfers TEXT, -- JSON array of TokenTransfer
    
    -- Instruction analysis data (calculated fresh, not cached)
    instruction_info TEXT, -- JSON array of InstructionInfo
    
    -- Analysis metadata
    analysis_duration_ms INTEGER,
    cached_analysis TEXT, -- JSON blob of CachedAnalysis
    analysis_version INTEGER NOT NULL DEFAULT 2,
    -- Commonly queried scalar fields
    fee_sol REAL NOT NULL DEFAULT 0,
    sol_delta REAL,
    
    -- Processing timestamps
    processed_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (signature) REFERENCES raw_transactions(signature) ON DELETE CASCADE
);
"#;

/// Known signatures tracking table
const SCHEMA_KNOWN_SIGNATURES: &str = r#"
CREATE TABLE IF NOT EXISTS known_signatures (
    signature TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'known',
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Deferred retries tracking table
const SCHEMA_DEFERRED_RETRIES: &str = r#"
CREATE TABLE IF NOT EXISTS deferred_retries (
    signature TEXT PRIMARY KEY,
    next_retry_at TEXT NOT NULL,
    remaining_attempts INTEGER NOT NULL DEFAULT 3,
    current_delay_secs INTEGER NOT NULL DEFAULT 60,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Pending transactions tracking table
const SCHEMA_PENDING_TRANSACTIONS: &str = r#"
CREATE TABLE IF NOT EXISTS pending_transactions (
    signature TEXT PRIMARY KEY,
    wallet_address TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_checked_at TEXT,
    check_count INTEGER NOT NULL DEFAULT 0
);
"#;

/// Database metadata table
const SCHEMA_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS db_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Bootstrap state table to persist resume cursor and completion flag across restarts
const SCHEMA_BOOTSTRAP_STATE: &str = r#"
CREATE TABLE IF NOT EXISTS bootstrap_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    backfill_before_cursor TEXT,
    full_history_completed INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Performance indexes for efficient queries
const INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_wallet ON raw_transactions(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_timestamp ON raw_transactions(timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_status ON raw_transactions(status);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_slot ON raw_transactions(slot DESC);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_success ON raw_transactions(success);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_wallet ON processed_transactions(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_type ON processed_transactions(transaction_type);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_direction ON processed_transactions(direction);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_analysis_version ON processed_transactions(analysis_version);",
    "CREATE INDEX IF NOT EXISTS idx_deferred_retries_next_retry ON deferred_retries(next_retry_at);",
    "CREATE INDEX IF NOT EXISTS idx_known_signatures_wallet ON known_signatures(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_known_signatures_added_at ON known_signatures(added_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_pending_transactions_wallet ON pending_transactions(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_pending_transactions_added_at ON pending_transactions(added_at DESC);",
];

// =============================================================================
// DATABASE STATISTICS AND REPORTING
// =============================================================================

/// Statistics about database operations and contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_raw_transactions: u64,
    pub total_processed_transactions: u64,
    pub total_known_signatures: u64,
    pub total_deferred_retries: u64,
    pub total_pending_transactions: u64,
    pub database_size_bytes: u64,
    pub schema_version: u32,
    pub last_updated: DateTime<Utc>,
}

/// Database integrity check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub raw_transactions_count: u64,
    pub processed_transactions_count: u64,
    pub orphaned_processed_transactions: u64,
    pub missing_processed_transactions: u64,
    pub schema_version_correct: bool,
    pub foreign_key_violations: u64,
    pub index_integrity_ok: bool,
    pub pending_transactions_count: u64,
}

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
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
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
                &format!("Initializing TransactionDatabase at: {}", database_path_str),
            );
        }

        let manager = SqliteConnectionManager::file(&database_path).with_init(|c| {
            c.pragma_update(None, "journal_mode", &"WAL")?;
            c.pragma_update(None, "synchronous", &"NORMAL")?;
            c.pragma_update(None, "cache_size", &10000)?;
            c.pragma_update(None, "temp_store", &"MEMORY")?;
            c.pragma_update(None, "mmap_size", &268_435_456)?;
            c.pragma_update(None, "foreign_keys", &1)?;
            Ok(())
        });

        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

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
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        Self::create_database(database_path, true).await
    }

    /// Initialize database schema and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let mut conn = self
            .get_connection()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

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
                .map_err(|e| format!("Failed to create table: {}", e))?;
        }

        // Create all indexes
        for index_sql in INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create index: {}", e))?;
        }

        // Apply lightweight migrations for existing databases
        self.apply_migrations(&mut conn)?;

        // Set or update schema version
        conn.execute(
            "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
            params!["schema_version", self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set schema version: {}", e))?;

        // Store current wallet address in metadata
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
            params!["current_wallet", wallet_address],
        )
        .map_err(|e| format!("Failed to set current_wallet in metadata: {}", e))?;

        Ok(())
    }

    /// Apply schema migrations when upgrading versions
    fn apply_migrations(&self, conn: &mut Connection) -> Result<(), String> {
        // Ensure processed_transactions has fee_sol column for MCP tools compatibility
        let mut has_fee_sol = false;
        let mut has_sol_delta = false;
        let mut stmt = conn
            .prepare("PRAGMA table_info(processed_transactions)")
            .map_err(|e| format!("Failed to inspect processed_transactions schema: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })
            .map_err(|e| format!("Failed to read processed_transactions schema: {}", e))?;
        for r in rows {
            let name = r.map_err(|e| format!("Failed to parse schema row: {}", e))?;
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
            .map_err(|e| format!("Failed to add fee_sol column: {}", e))?;
        }

        if !has_sol_delta {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN sol_delta REAL",
                [],
            )
            .map_err(|e| format!("Failed to add sol_delta column: {}", e))?;

            self.backfill_processed_sol_delta(conn)?;
        }

        // Ensure bootstrap_state table exists (idempotent)
        conn.execute(SCHEMA_BOOTSTRAP_STATE, [])
            .map_err(|e| format!("Failed to ensure bootstrap_state table: {}", e))?;

        // Ensure the single row exists
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (id, full_history_completed) VALUES (1, 0)",
            [],
        )
        .map_err(|e| format!("Failed to initialize bootstrap_state row: {}", e))?;
        Ok(())
    }

    fn backfill_processed_sol_delta(&self, conn: &mut Connection) -> Result<(), String> {
        const BATCH_SIZE: i64 = 1000;
        let mut total_updated = 0usize;

        // Get wallet address for filtering (this is a migration function, so it operates on current wallet data only)
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address for sol_delta backfill: {}", e))?;

        loop {
            let mut stmt = conn
                .prepare(
                    "SELECT signature, sol_balance_change FROM processed_transactions WHERE wallet_address = ?1 AND sol_delta IS NULL LIMIT ?2",
                )
                .map_err(|e| format!("Failed to prepare sol_delta backfill query: {}", e))?;

            let rows = stmt
                .query_map(params![wallet_address, BATCH_SIZE], |row| {
                    let signature: String = row.get(0)?;
                    let change_json: Option<String> = row.get(1)?;
                    Ok((signature, change_json))
                })
                .map_err(|e| format!("Failed to iterate sol_delta backfill rows: {}", e))?;

            let mut batch: Vec<(String, Option<String>)> = Vec::new();
            for row in rows {
                let (signature, change_json) =
                    row.map_err(|e| format!("Failed to read sol_delta row: {}", e))?;
                batch.push((signature, change_json));
            }

            if batch.is_empty() {
                break;
            }

            drop(stmt);

            let tx = conn
                .transaction()
                .map_err(|e| format!("Failed to start sol_delta backfill transaction: {}", e))?;

            for (signature, change_json) in batch.into_iter() {
                let delta = Self::compute_sol_delta_from_json(change_json.as_deref());
                tx.execute(
                    "UPDATE processed_transactions SET sol_delta = ?1 WHERE signature = ?2 AND wallet_address = ?3",
                    params![delta, signature, wallet_address],
                )
                .map_err(|e| format!("Failed to update sol_delta: {}", e))?;
                total_updated += 1;
            }

            tx.commit()
                .map_err(|e| format!("Failed to commit sol_delta backfill: {}", e))?;
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
                    &format!("Failed to parse sol_balance_change payload: {}", err),
                );
                0.0
            }
        }
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get database connection from pool: {}", e))
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
            .map_err(|e| format!("Database health check failed: {}", e))?;

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
            .map_err(|e| format!("Failed to check known signature: {}", e))?;

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
        .map_err(|e| format!("Failed to add known signature: {}", e))?;

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
            .map_err(|e| format!("Failed to get known signatures count: {}", e))?;

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
            .map_err(|e| format!("Failed to get newest known signature: {}", e))?;

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
            .map_err(|e| format!("Failed to get oldest known signature: {}", e))?;

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
            .map_err(|e| format!("Failed to cleanup old known signatures: {}", e))?;

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
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        for (signature, timestamp) in pending {
            tx.execute(
                "INSERT OR REPLACE INTO pending_transactions (signature, wallet_address, added_at) VALUES (?1, ?2, ?3)",
                params![signature, wallet_address, timestamp.to_rfc3339()],
            )
            .map_err(|e| format!("Failed to save pending transaction: {}", e))?;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit pending transactions: {}", e))?;

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
            .map_err(|e| format!("Failed to prepare pending transactions query: {}", e))?;

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
            .map_err(|e| format!("Failed to query pending transactions: {}", e))?;

        let mut result = HashMap::new();
        for row in rows {
            let (signature, timestamp) =
                row.map_err(|e| format!("Failed to parse pending transaction row: {}", e))?;
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
            .map_err(|e| format!("Failed to remove pending transaction: {}", e))?;

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
            .map_err(|e| format!("Failed to get pending transactions count: {}", e))?;

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
            Err(e) => Err(format!("Failed to read cached raw transaction: {}", e)),
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
            .map_err(|e| format!("Failed to store raw transaction: {}", e))?;

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
            .map_err(|e| format!("Failed to store processed transaction: {}", e))?;

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
            .map_err(|e| format!("Failed to update transaction status: {}", e))?;

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
                let sol_delta: f64 = row.get::<_, Option<f64>>(24)?.unwrap_or(0.0);

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

                let fee_sol: f64 = row.get::<_, Option<f64>>(23)?.unwrap_or(0.0);

                Ok(Transaction {
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
                    instructions_count: row.get(9).unwrap_or(0),
                    accounts_count: row.get(10).unwrap_or(0),
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
                    // These require deeper parsing from raw_transaction_data
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
                })
            },
        );

        match result {
            Ok(transaction) => Ok(Some(transaction)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get transaction: {}", e)),
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
            .map_err(|e| format!("Failed to get successful transactions count: {}", e))?;

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
            .map_err(|e| format!("Failed to get failed transactions count: {}", e))?;

        Ok(count as u64)
    }
}

// =============================================================================
// IMPLEMENTATION - DATABASE STATISTICS AND MAINTENANCE
// =============================================================================

impl TransactionDatabase {
    /// Get comprehensive database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let processed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let known_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let retries_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deferred_retries", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);

        Ok(DatabaseStats {
            total_raw_transactions: raw_count as u64,
            total_processed_transactions: processed_count as u64,
            total_known_signatures: known_count as u64,
            total_deferred_retries: retries_count as u64,
            total_pending_transactions: pending_count as u64,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
            last_updated: Utc::now(),
        })
    }

    /// Perform database maintenance (vacuum, analyze, cleanup)
    pub async fn perform_maintenance(&self) -> Result<(), String> {
        let conn = self.get_connection()?;

        logger::info(LogTag::Transactions, "Starting database maintenance");

        // Vacuum to reclaim space
        conn.execute("VACUUM", [])
            .map_err(|e| format!("Failed to vacuum database: {}", e))?;

        // Analyze for query optimization
        conn.execute("ANALYZE", [])
            .map_err(|e| format!("Failed to analyze database: {}", e))?;

        // Cleanup old pending transactions (older than 1 day)
        let cleaned_pending = conn
            .execute(
                "DELETE FROM pending_transactions WHERE added_at < datetime('now', '-1 day')",
                [],
            )
            .map_err(|e| format!("Failed to cleanup old pending transactions: {}", e))?;

        // Cleanup old deferred retries (older than 1 day with 0 attempts)
        let cleaned_retries = conn
            .execute(
                "DELETE FROM deferred_retries WHERE remaining_attempts = 0 AND created_at < datetime('now', '-1 day')",
                []
            )
            .map_err(|e| format!("Failed to cleanup old deferred retries: {}", e))?;

        logger::info(
            LogTag::Transactions,
            &format!(
                "Database maintenance complete: cleaned {} pending, {} retries",
                cleaned_pending, cleaned_retries
            ),
        );

        Ok(())
    }

    /// Get integrity report
    pub async fn get_integrity_report(&self) -> Result<IntegrityReport, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let raw_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let processed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE wallet_address = ?1 AND signature NOT IN (SELECT signature FROM raw_transactions WHERE wallet_address = ?1)",
                params![wallet_address],
                |row| row.get(0)
            )
            .unwrap_or(0);

        let missing: i64 = raw_count - processed_count;

        let pending_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pending_transactions WHERE wallet_address = ?1",
                params![wallet_address],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Check schema version
        let schema_version_correct = conn
            .query_row(
                "SELECT value FROM db_metadata WHERE key = 'schema_version'",
                [],
                |row| {
                    let version_str: String = row.get(0)?;
                    Ok(version_str == self.schema_version.to_string())
                },
            )
            .unwrap_or(false);

        Ok(IntegrityReport {
            raw_transactions_count: raw_count as u64,
            processed_transactions_count: processed_count as u64,
            orphaned_processed_transactions: orphaned as u64,
            missing_processed_transactions: missing.max(0) as u64,
            schema_version_correct,
            foreign_key_violations: 0, // Would require FK check
            index_integrity_ok: true,  // Would require index check
            pending_transactions_count: pending_count as u64,
        })
    }
}

// =============================================================================
// IMPLEMENTATION - BOOTSTRAP STATE AND RECONCILIATION
// =============================================================================

/// Bootstrap state structure for resuming backfill across restarts
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BootstrapState {
    pub backfill_before_cursor: Option<String>,
    pub full_history_completed: bool,
}

impl TransactionDatabase {
    /// Get the current bootstrap state
    pub async fn get_bootstrap_state(&self) -> Result<BootstrapState, String> {
        let conn = self.get_connection()?;
        let mut state = BootstrapState::default();

        let result = conn
            .query_row(
                "SELECT backfill_before_cursor, full_history_completed FROM bootstrap_state WHERE id = 1",
                [],
                |row| {
                    let cursor: Option<String> = row.get(0)?;
                    let completed_i: i64 = row.get(1)?;
                    Ok((cursor, completed_i))
                }
            )
            .optional()
            .map_err(|e| format!("Failed to load bootstrap_state: {}", e))?;

        if let Some((cursor, completed_i)) = result {
            state.backfill_before_cursor = cursor;
            state.full_history_completed = completed_i != 0;
        }

        Ok(state)
    }

    /// Update the backfill cursor (the `before` parameter for next page)
    pub async fn set_backfill_cursor(&self, cursor: Option<&str>) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR IGNORE INTO bootstrap_state (id, full_history_completed) VALUES (1, 0)",
            [],
        )
        .map_err(|e| format!("Failed to ensure bootstrap_state row: {}", e))?;

        conn
            .execute(
                "UPDATE bootstrap_state SET backfill_before_cursor = ?1, updated_at = datetime('now') WHERE id = 1",
                params![cursor]
            )
            .map_err(|e| format!("Failed to update backfill cursor: {}", e))?;
        Ok(())
    }

    /// Clear the backfill cursor
    pub async fn clear_backfill_cursor(&self) -> Result<(), String> {
        self.set_backfill_cursor(None).await
    }

    /// Mark the full history as completed
    pub async fn mark_full_history_completed(&self) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn
            .execute(
                "UPDATE bootstrap_state SET full_history_completed = 1, updated_at = datetime('now') WHERE id = 1",
                []
            )
            .map_err(|e| format!("Failed to mark full history completed: {}", e))?;
        Ok(())
    }

    /// Reconcile known_signatures with already processed transactions
    /// Ensures no processed transaction is missing from known_signatures
    pub async fn reconcile_known_with_processed(&self) -> Result<usize, String> {
        let conn = self.get_connection()?;
        let affected = conn
            .execute(
                "INSERT OR IGNORE INTO known_signatures(signature) SELECT signature FROM processed_transactions",
                []
            )
            .map_err(|e| format!("Failed to reconcile known signatures: {}", e))?;
        Ok(affected as usize)
    }

    // =============================================================================
    // LIST AND FILTER OPERATIONS FOR UI
    // =============================================================================

    /// List transactions with filtering and cursor-based pagination
    /// Returns lightweight rows suitable for UI list views
    pub async fn list_transactions(
        &self,
        filters: &TransactionListFilters,
        cursor: Option<&TransactionCursor>,
        limit: usize,
    ) -> Result<TransactionListResult, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Limit page size to max 200 for performance
        let effective_limit = limit.min(200);

        // Build SQL query with filters
        let mut query = String::from(
            "SELECT 
                r.signature, r.timestamp, r.slot, r.status, r.success, 
                r.fee_lamports, r.instructions_count,
                p.transaction_type, p.direction, p.token_swap_info, 
                p.token_transfers, p.ata_operations,
                p.fee_sol, p.sol_delta
            FROM raw_transactions r
            LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1
            WHERE r.wallet_address = ?1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(wallet_address));

        // Apply cursor for pagination (timestamp desc, signature desc)
        if let Some(cursor) = cursor {
            query.push_str(&format!(
                " AND (r.timestamp < ?{} OR (r.timestamp = ?{} AND r.signature < ?{}))",
                params_vec.len() + 1,
                params_vec.len() + 1,
                params_vec.len() + 2
            ));
            params_vec.push(Box::new(cursor.timestamp.clone()));
            params_vec.push(Box::new(cursor.signature.clone()));
        }

        // Apply time range filters
        if let Some(ref from) = filters.time_from {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(ref to) = filters.time_to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to.to_rfc3339()));
        }

        // Apply status filter
        if let Some(ref status) = filters.status {
            if let Some(normalized) = canonical_status(status) {
                query.push_str(&format!(" AND r.status = ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(normalized));
            }
        }

        // Apply success filter
        if filters.only_confirmed.unwrap_or(false) {
            query.push_str(" AND r.status IN ('Confirmed', 'Finalized')");
        }

        // Apply signature filter
        if let Some(ref signature) = filters.signature {
            let trimmed = signature.trim();
            if !trimmed.is_empty() {
                query.push_str(&format!(" AND r.signature LIKE ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(format!("%{}%", trimmed)));
            }
        }

        // Fetch 3x limit to allow Rust-side filtering
        let fetch_limit = effective_limit * 3;
        query.push_str(&format!(
            " ORDER BY r.timestamp DESC, r.signature DESC LIMIT {}",
            fetch_limit
        ));

        // Execute query
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare list query: {}", e))?;

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                let signature: String = row.get(0)?;
                let timestamp = {
                    let timestamp_str: String = row.get(1)?;
                    DateTime::parse_from_rfc3339(&timestamp_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                };

                let slot = row.get::<_, Option<i64>>(2)?.and_then(|raw| {
                    if raw >= 0 {
                        Some(raw as u64)
                    } else {
                        None
                    }
                });
                let status: String = row.get(3)?;
                let success: bool = row.get(4)?;

                let fee_lamports = row.get::<_, Option<i64>>(5)?.and_then(|raw| {
                    if raw >= 0 {
                        Some(raw as u64)
                    } else {
                        None
                    }
                });
                let instructions_count = row.get::<_, Option<i64>>(6)?.unwrap_or(0).max(0) as usize;

                let transaction_type: Option<String> = row.get(7)?;
                let direction: Option<String> = row.get(8)?;
                let token_swap_info_json: Option<String> = row.get(9)?;
                let token_transfers_json: Option<String> = row.get(10)?;
                let ata_operations_json: Option<String> = row.get(11)?;
                let fee_sol = row.get::<_, Option<f64>>(12)?.unwrap_or(0.0);
                let sol_delta = row.get::<_, Option<f64>>(13)?.unwrap_or(0.0);

                let swap_info: Option<TokenSwapInfo> = token_swap_info_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());
                let token_transfers: Option<Vec<TokenTransfer>> = token_transfers_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());
                let ata_operations: Option<Vec<AtaOperation>> = ata_operations_json
                    .as_ref()
                    .and_then(|json| serde_json::from_str(json).ok());

                let ata_rents = ata_operations
                    .as_ref()
                    .map(|ops| ops.iter().map(|op| op.rent_amount).sum())
                    .unwrap_or(0.0);

                let mut token_mint = swap_info
                    .as_ref()
                    .map(|info| info.mint.clone())
                    .filter(|mint| !mint.is_empty());

                if token_mint.is_none() {
                    token_mint = swap_info
                        .as_ref()
                        .map(|info| info.output_mint.clone())
                        .filter(|mint| !mint.is_empty());
                }

                if token_mint.is_none() {
                    if let Some(transfers) = token_transfers.as_ref() {
                        token_mint = transfers.iter().find_map(|transfer| {
                            if transfer.mint.is_empty() {
                                None
                            } else {
                                Some(transfer.mint.clone())
                            }
                        });
                    }
                }

                let token_symbol = swap_info
                    .as_ref()
                    .map(|info| info.symbol.clone())
                    .filter(|symbol| !symbol.is_empty());

                let router = swap_info
                    .as_ref()
                    .map(|info| info.router.clone())
                    .filter(|router| !router.is_empty());

                Ok(TransactionListRow {
                    signature,
                    timestamp,
                    slot,
                    status,
                    success,
                    direction,
                    transaction_type,
                    token_mint,
                    token_symbol,
                    router,
                    sol_delta,
                    fee_sol,
                    fee_lamports,
                    ata_rents,
                    instructions_count,
                })
            })
            .map_err(|e| format!("Failed to execute list query: {}", e))?;

        // Collect and apply Rust-side filters
        let mut results: Vec<TransactionListRow> = Vec::new();

        for row_result in rows {
            let row = row_result.map_err(|e| format!("Failed to parse row: {}", e))?;

            if !Self::row_matches_filters(&row, filters) {
                continue;
            }

            results.push(row);

            // Stop when we have enough results
            if results.len() >= effective_limit {
                break;
            }
        }

        // Determine next cursor
        let next_cursor = if results.len() == effective_limit {
            results.last().map(|row| TransactionCursor {
                timestamp: row.timestamp.to_rfc3339(),
                signature: row.signature.clone(),
            })
        } else {
            None
        };

        Ok(TransactionListResult {
            items: results,
            next_cursor,
            total_estimate: None, // Optional, can be computed with COUNT query
        })
    }

    /// Helper to check if a row matches all filters
    fn row_matches_filters(row: &TransactionListRow, filters: &TransactionListFilters) -> bool {
        // Type filter
        if !filters.types.is_empty() {
            let row_type = row.transaction_type.as_deref().unwrap_or("Unknown");
            let matches_type = filters
                .types
                .iter()
                .any(|t| matches_transaction_type(t, row_type, row.success));
            if !matches_type {
                return false;
            }
        }

        // Mint filter
        if let Some(ref mint) = filters.mint {
            let mint_trimmed = mint.trim();
            if !mint_trimmed.is_empty() {
                if let Some(ref row_mint) = row.token_mint {
                    if !row_mint.contains(mint_trimmed) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // Direction filter
        if let Some(ref dir) = filters.direction {
            if let Some(expected) = canonical_direction(dir) {
                let row_dir = row.direction.as_deref().unwrap_or("Unknown");
                if !row_dir.eq_ignore_ascii_case(&expected) {
                    return false;
                }
            }
        }

        // Status filter (safety check, SQL already applies exact match)
        if let Some(ref status) = filters.status {
            if let Some(expected) = canonical_status(status) {
                if !row.status.eq_ignore_ascii_case(&expected) {
                    return false;
                }
            }
        }

        // Router filter (case-insensitive contains)
        if let Some(ref router) = filters.router {
            let router_trimmed = router.trim();
            if !router_trimmed.is_empty() {
                let needle = router_trimmed.to_ascii_lowercase();
                if let Some(ref row_router) = row.router {
                    if !row_router.to_ascii_lowercase().contains(&needle) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        // SOL delta range filter
        if let Some(min_sol) = filters.min_sol {
            if row.sol_delta < min_sol {
                return false;
            }
        }

        if let Some(max_sol) = filters.max_sol {
            if row.sol_delta > max_sol {
                return false;
            }
        }

        true
    }

    /// Aggregate SOL inflow/outflow metrics within a time window for wallet dashboard usage
    pub async fn aggregate_sol_flows_since(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Check if this is "all time" query (from epoch = no time filter)
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let is_all_time = from == epoch;

        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) > 0 THEN p.sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) < 0 THEN -p.sol_delta ELSE 0 END), 0), \
                COUNT(r.signature) \
             FROM raw_transactions r \
             LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1 \
             WHERE r.wallet_address = ?1 AND r.status IN ('Confirmed', 'Finalized')",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        params_vec.push(Box::new(wallet_address.clone()));

        // Only add timestamp filter if NOT all-time query
        if !is_all_time {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to_ts) = to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|value| value.as_ref()).collect();

        logger::debug(
            LogTag::Transactions,
            &format!(
                "Aggregating SOL flows for wallet {} from {}",
                wallet_address,
                from.to_rfc3339()
            ),
        );

        // Change query to get all rows so we can parse JSON
        let row_query = query.replace(
            "SELECT \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) > 0 THEN p.sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN COALESCE(p.sol_delta, 0) < 0 THEN -p.sol_delta ELSE 0 END), 0), \
                COUNT(r.signature)",
            "SELECT r.signature, r.timestamp, p.sol_balance_change",
        );

        let mut stmt = conn
            .prepare(&row_query)
            .map_err(|e| format!("Failed to prepare flow aggregation query: {}", e))?;

        let mut rows = stmt
            .query(params_refs.as_slice())
            .map_err(|e| format!("Failed to execute flow aggregation query: {}", e))?;

        let mut inflow = 0.0;
        let mut outflow = 0.0;
        let mut count = 0;
        let mut parsed_count = 0;
        let mut no_json_count = 0;
        let mut parse_error_count = 0;
        let mut no_wallet_account_count = 0;

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to read flow row: {}", e))?
        {
            count += 1;
            let signature: String = row.get(0).unwrap_or_default();
            let sol_balance_change_json: Option<String> = row.get(2).ok();

            if let Some(json_str) = sol_balance_change_json {
                // Parse JSON array of balance changes
                match serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    Ok(changes) => {
                        let mut found_wallet = false;
                        let changes_len = changes.len();
                        for change_obj in &changes {
                            if let Some(account) =
                                change_obj.get("account").and_then(|v| v.as_str())
                            {
                                if account == wallet_address {
                                    found_wallet = true;
                                    if let Some(change) =
                                        change_obj.get("change").and_then(|v| v.as_f64())
                                    {
                                        parsed_count += 1;
                                        if count <= 5 {
                                            logger::debug(
                                                LogTag::Transactions,
                                                &format!(
                                                    "TX {}: wallet change={:.6} SOL",
                                                    &signature[..8],
                                                    change
                                                ),
                                            );
                                        }
                                        if change > 0.0 {
                                            inflow += change;
                                        } else if change < 0.0 {
                                            outflow += change.abs();
                                        }
                                    }
                                    break; // Found wallet, no need to check other accounts
                                }
                            }
                        }
                        if !found_wallet {
                            no_wallet_account_count += 1;
                            if no_wallet_account_count <= 3 {
                                logger::debug(
                                    LogTag::Transactions,
                                    &format!(
                                        "TX {}: no wallet account in {} balance changes",
                                        &signature[..8],
                                        changes_len
                                    ),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        parse_error_count += 1;
                        if parse_error_count <= 3 {
                            logger::debug(
                                LogTag::Transactions,
                                &format!("TX {}: JSON parse error: {}", &signature[..8], e),
                            );
                        }
                    }
                }
            } else {
                no_json_count += 1;
            }
        }

        logger::debug(
        LogTag::Transactions,
                &format!(
                    "Aggregated {} txs: parsed={} with wallet, no_json={}, parse_errors={}, no_wallet_account={} | inflow={:.6} SOL, outflow={:.6} SOL, net={:.6} SOL",
                    count,
                    parsed_count,
                    no_json_count,
                    parse_error_count,
                    no_wallet_account_count,
                    inflow,
                    outflow,
                    inflow - outflow
                ),
            );

        Ok((inflow, outflow, count))
    }

    /// Get daily flow aggregation for time-series chart
    pub async fn aggregate_daily_flows(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, f64, f64, usize)>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        // Check if this is "all time" query
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let is_all_time = from == epoch;

        // Query to get daily aggregated flows
        let mut query = String::from(
            "SELECT \
                DATE(r.timestamp) as day, \
                r.signature, \
                p.sol_balance_change \
             FROM raw_transactions r \
             LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1 \
             WHERE r.wallet_address = ?1 AND r.status IN ('Confirmed', 'Finalized')",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![];
        params_vec.push(Box::new(wallet_address.clone()));

        if !is_all_time {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(to_ts) = to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }

        query.push_str(" ORDER BY day ASC, r.timestamp ASC");

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|value| value.as_ref()).collect();

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare daily flows query: {}", e))?;

        let mut rows = stmt
            .query(params_refs.as_slice())
            .map_err(|e| format!("Failed to execute daily flows query: {}", e))?;

        // Group by day manually
        use std::collections::HashMap;
        let mut daily_data: HashMap<String, (f64, f64, usize)> = HashMap::new();

        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to read daily flow row: {}", e))?
        {
            let day: String = row.get(0).unwrap_or_default();
            let sol_balance_change_json: Option<String> = row.get(2).ok();

            if let Some(json_str) = sol_balance_change_json {
                if let Ok(changes) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                    for change_obj in &changes {
                        if let Some(account) = change_obj.get("account").and_then(|v| v.as_str()) {
                            if account == wallet_address {
                                if let Some(change) =
                                    change_obj.get("change").and_then(|v| v.as_f64())
                                {
                                    let entry =
                                        daily_data.entry(day.clone()).or_insert((0.0, 0.0, 0));
                                    if change > 0.0 {
                                        entry.0 += change; // inflow
                                    } else if change < 0.0 {
                                        entry.1 += change.abs(); // outflow
                                    }
                                    entry.2 += 1; // tx count
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Convert to sorted vec
        let mut result: Vec<(String, f64, f64, usize)> = daily_data
            .into_iter()
            .map(|(day, (inflow, outflow, count))| (day, inflow, outflow, count))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(result)
    }

    /// Lightweight export of processed transactions for wallet flow cache
    pub async fn export_processed_for_wallet_flow(
        &self,
        from: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<WalletFlowExportRow>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut stmt = conn
            .prepare(
                "SELECT r.signature, r.timestamp, COALESCE(p.sol_delta, 0) as sol_delta \
                 FROM raw_transactions r \
                 LEFT JOIN processed_transactions p ON r.signature = p.signature AND p.wallet_address = ?1 \
                 WHERE r.wallet_address = ?1 AND r.timestamp >= ?2 AND r.status IN ('Confirmed', 'Finalized') \
                 ORDER BY r.timestamp ASC, r.signature ASC \
                 LIMIT ?3",
            )
            .map_err(|e| format!("Failed to prepare wallet flow export: {}", e))?;

        let mut rows = stmt
            .query(params![
                wallet_address,
                from.to_rfc3339(),
                (limit as i64).max(1)
            ])
            .map_err(|e| format!("Failed to query wallet flow export: {}", e))?;

        let mut results = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("Failed to iterate wallet flow export: {}", e))?
        {
            let signature: String = row
                .get(0)
                .map_err(|e| format!("Failed to read signature: {}", e))?;
            let ts_str: String = row
                .get(1)
                .map_err(|e| format!("Failed to read timestamp: {}", e))?;
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse timestamp: {}", e))?;
            let sol_delta: f64 = row
                .get::<_, Option<f64>>(2)
                .unwrap_or(Some(0.0))
                .unwrap_or(0.0);
            results.push(WalletFlowExportRow {
                signature,
                timestamp,
                sol_delta,
            });
        }

        Ok(results)
    }
    /// Get estimated count of transactions matching filters (optional, for UI)
    pub async fn count_transactions(
        &self,
        filters: &TransactionListFilters,
    ) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address().map_err(|e| e.to_string())?;

        let mut query =
            String::from("SELECT COUNT(*) FROM raw_transactions r WHERE r.wallet_address = ?1");

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params_vec.push(Box::new(wallet_address));

        // Apply coarse filters (can't filter by JSON columns efficiently)
        if let Some(ref from) = filters.time_from {
            query.push_str(&format!(" AND r.timestamp >= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(from.to_rfc3339()));
        }

        if let Some(ref to) = filters.time_to {
            query.push_str(&format!(" AND r.timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to.to_rfc3339()));
        }

        if let Some(ref status) = filters.status {
            if let Some(normalized) = canonical_status(status) {
                query.push_str(&format!(" AND r.status = ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(normalized));
            }
        }

        if filters.only_confirmed.unwrap_or(false) {
            query.push_str(" AND r.status IN ('Confirmed', 'Finalized')");
        }

        if let Some(ref signature) = filters.signature {
            let trimmed = signature.trim();
            if !trimmed.is_empty() {
                query.push_str(&format!(" AND r.signature LIKE ?{}", params_vec.len() + 1));
                params_vec.push(Box::new(format!("%{}%", trimmed)));
            }
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&query, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| format!("Failed to count transactions: {}", e))?;

        Ok(count as u64)
    }
}

fn canonical_status(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lowered = trimmed.to_ascii_lowercase();
    let normalized = match lowered.as_str() {
        "pending" => "Pending",
        "confirmed" => "Confirmed",
        "finalized" => "Finalized",
        "failed" => "Failed",
        _ => return Some(trimmed.to_string()),
    };

    Some(normalized.to_string())
}

fn canonical_direction(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lowered = trimmed.to_ascii_lowercase();
    let normalized = match lowered.as_str() {
        "incoming" => "Incoming",
        "outgoing" => "Outgoing",
        "internal" => "Internal",
        "unknown" => "Unknown",
        _ => return Some(trimmed.to_string()),
    };

    Some(normalized.to_string())
}

fn matches_transaction_type(filter: &str, row_type: &str, success: bool) -> bool {
    let filter_norm = filter.trim().to_ascii_lowercase();
    if filter_norm.is_empty() {
        return false;
    }

    let row_lower = row_type.to_ascii_lowercase();

    match filter_norm.as_str() {
        "buy" => row_lower.contains("swapsoltotoken") || row_lower == "buy",
        "sell" => row_lower.contains("swaptokentosol") || row_lower == "sell",
        "swap" => row_lower.contains("swap") || row_lower == "buy" || row_lower == "sell",
        "transfer" => row_lower.contains("transfer"),
        "ata" => row_lower.contains("ata"),
        "failed" => !success || row_lower.contains("fail"),
        "unknown" => row_lower.contains("unknown"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rusqlite::Connection;
    use serde_json::json;
    use tempfile::tempdir;

    use crate::transactions::types::{
        SolBalanceChange, TransactionDirection, TransactionStatus, TransactionType,
    };

    fn sample_row(
        transaction_type: Option<&str>,
        direction: Option<&str>,
        success: bool,
        router: Option<&str>,
        sol_delta: f64,
    ) -> TransactionListRow {
        TransactionListRow {
            signature: "sig".to_string(),
            timestamp: Utc::now(),
            slot: None,
            status: "Finalized".to_string(),
            success,
            direction: direction.map(|s| s.to_string()),
            transaction_type: transaction_type.map(|s| s.to_string()),
            token_mint: None,
            token_symbol: None,
            router: router.map(|s| s.to_string()),
            sol_delta,
            fee_sol: 0.0,
            fee_lamports: None,
            ata_rents: 0.0,
            instructions_count: 0,
        }
    }

    #[tokio::test]
    async fn upsert_and_fetch_transaction_caches_raw_and_processed() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("transactions.db");
        let db = TransactionDatabase::new_with_path(&db_path)
            .await
            .expect("create database");

        let mut transaction = Transaction::new("test_signature".to_string());
        transaction.slot = Some(12345);
        transaction.block_time = Some(1_700_000_000);
        transaction.timestamp = Utc::now();
        transaction.status = TransactionStatus::Finalized;
        transaction.success = true;
        transaction.fee_lamports = Some(5_000);
        transaction.fee_sol = 0.000005;
        transaction.instructions_count = 2;
        transaction.accounts_count = 3;
        transaction.transaction_type = TransactionType::Transfer;
        transaction.direction = TransactionDirection::Outgoing;
        transaction.sol_balance_change = -0.25;
        transaction.sol_balance_changes = vec![SolBalanceChange {
            account: "wallet".to_string(),
            pre_balance: 1.0,
            post_balance: 0.75,
            change: -0.25,
        }];
        let raw_json = json!({ "signature": transaction.signature });
        let raw_json_string = raw_json.to_string();
        transaction.raw_transaction_data = Some(raw_json);

        db.upsert_full_transaction(&transaction)
            .await
            .expect("upsert transaction");

        let fetched = db
            .get_transaction(&transaction.signature)
            .await
            .expect("fetch transaction")
            .expect("transaction exists");

        assert_eq!(fetched.signature, transaction.signature);
        assert!(fetched.success);
        assert_eq!(fetched.fee_lamports, transaction.fee_lamports);
        assert_eq!(fetched.instructions_count, transaction.instructions_count);

        let conn = Connection::open(&db_path).expect("open sqlite connection");
        let stored_raw: Option<String> = conn
            .query_row(
                "SELECT raw_transaction_data FROM raw_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| row.get(0),
            )
            .expect("query raw data");
        assert_eq!(stored_raw, Some(raw_json_string));

        let stored_fee: f64 = conn
            .query_row(
                "SELECT fee_sol FROM processed_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| row.get(0),
            )
            .expect("query processed fee");
        assert!((stored_fee - transaction.fee_sol).abs() < 1e-12);

        let stored_delta: f64 = conn
            .query_row(
                "SELECT sol_delta FROM processed_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| Ok(row.get::<_, Option<f64>>(0)?.unwrap_or(0.0)),
            )
            .expect("query processed sol_delta");
        assert!((stored_delta - transaction.sol_balance_change).abs() < 1e-9);
    }

    #[test]
    fn type_filters_match_modern_and_legacy_variants() {
        let row_swap = sample_row(
            Some("SwapSolToToken { .. }"),
            Some("Outgoing"),
            true,
            None,
            0.0,
        );
        let row_buy = sample_row(Some("Buy"), Some("Outgoing"), true, None, 0.0);

        let filters = TransactionListFilters {
            types: vec!["buy".to_string()],
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(
            &row_swap, &filters
        ));
        assert!(TransactionDatabase::row_matches_filters(&row_buy, &filters));

        let failed_filters = TransactionListFilters {
            types: vec!["failed".to_string()],
            ..Default::default()
        };

        let failed_row = sample_row(
            Some("SwapTokenToSol { .. }"),
            Some("Outgoing"),
            false,
            None,
            0.0,
        );
        assert!(TransactionDatabase::row_matches_filters(
            &failed_row,
            &failed_filters
        ));
    }

    #[test]
    fn direction_filter_is_case_insensitive() {
        let row = sample_row(Some("Transfer"), Some("Incoming"), true, None, 0.0);

        let filters = TransactionListFilters {
            direction: Some("incoming".to_string()),
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(&row, &filters));
    }

    #[test]
    fn router_filter_handles_case_insensitive_search() {
        let row = sample_row(Some("Swap"), Some("Outgoing"), true, Some("Raydium"), 0.0);

        let filters = TransactionListFilters {
            router: Some("ray".to_string()),
            ..Default::default()
        };

        assert!(TransactionDatabase::row_matches_filters(&row, &filters));
    }
}

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global database instance for cross-module access
static GLOBAL_TRANSACTION_DATABASE: Lazy<Arc<Mutex<Option<Arc<TransactionDatabase>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize global transaction database
pub async fn init_transaction_database() -> Result<Arc<TransactionDatabase>, String> {
    let db = TransactionDatabase::new().await?;
    let db_arc = Arc::new(db);

    let mut global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    *global = Some(Arc::clone(&db_arc));

    logger::info(
        LogTag::Transactions,
        "Global transaction database initialized",
    );
    Ok(db_arc)
}

/// Get global transaction database instance
pub async fn get_transaction_database() -> Option<Arc<TransactionDatabase>> {
    let global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    global.as_ref().map(Arc::clone)
}
