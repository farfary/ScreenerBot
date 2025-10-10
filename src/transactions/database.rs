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

use crate::global::is_debug_transactions_enabled;
use crate::logger::{log, LogTag};
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

    // Internal fields for filtering (not serialized to API)
    #[serde(skip)]
    pub _token_swap_info_json: Option<String>,
    #[serde(skip)]
    pub _token_transfers_json: Option<String>,
    #[serde(skip)]
    pub _ata_operations_json: Option<String>,
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
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_timestamp ON raw_transactions(timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_status ON raw_transactions(status);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_slot ON raw_transactions(slot DESC);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_success ON raw_transactions(success);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_type ON processed_transactions(transaction_type);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_direction ON processed_transactions(direction);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_analysis_version ON processed_transactions(analysis_version);",
    "CREATE INDEX IF NOT EXISTS idx_deferred_retries_next_retry ON deferred_retries(next_retry_at);",
    "CREATE INDEX IF NOT EXISTS idx_known_signatures_added_at ON known_signatures(added_at DESC);",
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

impl TransactionDatabase {
    /// Create new TransactionDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        let data_dir = PathBuf::from("data");

        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("transactions.db");
        let is_first_init = !DATABASE_INITIALIZED.load(Ordering::Relaxed);
        let db = Self::create_database(database_path, is_first_init).await?;

        DATABASE_INITIALIZED.store(true, Ordering::Relaxed);

        Ok(db)
    }

    async fn create_database(database_path: PathBuf, log_details: bool) -> Result<Self, String> {
        let database_path_str = database_path.to_string_lossy().to_string();

        if log_details {
            log(
                LogTag::Transactions,
                "INIT",
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
            log(
                LogTag::Transactions,
                "INIT",
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
        let conn = self
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
        self.apply_migrations(&conn)?;

        // Set or update schema version
        conn.execute(
            "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
            params!["schema_version", self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set schema version: {}", e))?;

        Ok(())
    }

    /// Apply schema migrations when upgrading versions
    fn apply_migrations(&self, conn: &Connection) -> Result<(), String> {
        // Ensure processed_transactions has fee_sol column for MCP tools compatibility
        let mut has_fee_sol = false;
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
                break;
            }
        }
        if !has_fee_sol {
            conn.execute(
                "ALTER TABLE processed_transactions ADD COLUMN fee_sol REAL NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|e| format!("Failed to add fee_sol column: {}", e))?;
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

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM known_signatures WHERE signature = ?1)",
                params![signature],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check known signature: {}", e))?;

        Ok(exists)
    }

    /// Add signature to known signatures
    pub async fn add_known_signature(&self, signature: &str) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT OR IGNORE INTO known_signatures (signature) VALUES (?1)",
            params![signature],
        )
        .map_err(|e| format!("Failed to add known signature: {}", e))?;

        Ok(())
    }

    /// Get count of known signatures
    pub async fn get_known_signatures_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM known_signatures", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to get known signatures count: {}", e))?;

        Ok(count as u64)
    }

    /// Get the newest known signature (most recently added)
    pub async fn get_newest_known_signature(&self) -> Result<Option<String>, String> {
        let conn = self.get_connection()?;

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures ORDER BY added_at DESC LIMIT 1",
                [],
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

        let result: Option<String> = conn
            .query_row(
                "SELECT signature FROM known_signatures ORDER BY added_at ASC LIMIT 1",
                [],
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

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        for (signature, timestamp) in pending {
            tx.execute(
                "INSERT OR REPLACE INTO pending_transactions (signature, added_at) VALUES (?1, ?2)",
                params![signature, timestamp.to_rfc3339()],
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

        let mut stmt = conn
            .prepare("SELECT signature, added_at FROM pending_transactions")
            .map_err(|e| format!("Failed to prepare pending transactions query: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
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

        let affected = conn
            .execute(
                "DELETE FROM pending_transactions WHERE signature = ?1",
                params![signature],
            )
            .map_err(|e| format!("Failed to remove pending transaction: {}", e))?;

        Ok(affected > 0)
    }

    /// Get count of pending transactions
    pub async fn get_pending_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pending_transactions", [], |row| {
                row.get(0)
            })
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

        let result: rusqlite::Result<Option<String>> = conn
            .query_row(
                "SELECT raw_transaction_data FROM raw_transactions WHERE signature = ?1",
                params![signature],
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
        let debug = is_debug_transactions_enabled();
        let conn = self.get_connection()?;

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
               (signature, slot, block_time, timestamp, status, success, error_message, 
                fee_lamports, compute_units_consumed, instructions_count, accounts_count, raw_transaction_data, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'))"#,
                params![
                    transaction.signature,
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

        if debug {
            log(
                LogTag::Transactions,
                "DB_RAW",
                &format!(
                    "Stored raw {} (status={}, success={})",
                    &transaction.signature, status_str, transaction.success
                ),
            );
        }

        Ok(())
    }

    /// Store processed transaction analysis snapshot
    pub async fn store_processed_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<(), String> {
        let debug = is_debug_transactions_enabled();
        let conn = self.get_connection()?;

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

        conn
            .execute(
                r#"INSERT OR REPLACE INTO processed_transactions
                   (signature, transaction_type, direction, sol_balance_change, token_balance_changes,
                    token_swap_info, swap_pnl_info, ata_operations, token_transfers, instruction_info,
                    analysis_duration_ms, cached_analysis, analysis_version, fee_sol, updated_at)
                 VALUES
                   (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, datetime('now'))"#,
                params![
                    transaction.signature,
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
                    transaction.fee_sol
                ]
            )
            .map_err(|e| format!("Failed to store processed transaction: {}", e))?;

        if debug {
            log(
                LogTag::Transactions,
                "DB_PROCESSED",
                &format!(
                    "Stored processed {} (type={:?}, direction={:?}, fee_sol={:.8})",
                    &transaction.signature,
                    transaction.transaction_type,
                    transaction.direction,
                    transaction.fee_sol
                ),
            );
        }

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

        conn
            .execute(
                "UPDATE raw_transactions SET status = ?1, success = ?2, error_message = ?3, updated_at = datetime('now') WHERE signature = ?4",
                params![status, success, error_message, signature]
            )
            .map_err(|e| format!("Failed to update transaction status: {}", e))?;

        Ok(())
    }

    /// Get transaction by signature
    pub async fn get_transaction(&self, signature: &str) -> Result<Option<Transaction>, String> {
        let debug = is_debug_transactions_enabled();
        let conn = self.get_connection()?;

        let result = conn.query_row(
            r#"SELECT signature, slot, block_time, timestamp, status, success, error_message,
                          fee_lamports, compute_units_consumed, instructions_count, accounts_count
                   FROM raw_transactions WHERE signature = ?1"#,
            params![signature],
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

                Ok(Transaction {
                    signature: row.get(0)?,
                    slot: row.get(1)?,
                    block_time: row.get(2)?,
                    timestamp,
                    status,
                    success: row.get(5)?,
                    error_message: row.get(6)?,
                    fee_lamports: row.get(7)?,
                    compute_units_consumed: row.get(8)?,
                    instructions_count: row.get(9).unwrap_or(0),
                    accounts_count: row.get(10).unwrap_or(0),
                    // Analysis fields are not cached - calculated fresh
                    ..Transaction::new(row.get::<_, String>(0)?)
                })
            },
        );

        match result {
            Ok(transaction) => {
                if debug {
                    log(
                        LogTag::Transactions,
                        "DB_HIT",
                        &format!(
                            "Cache hit for {} (status={:?})",
                            signature, transaction.status
                        ),
                    );
                }
                Ok(Some(transaction))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("Failed to get transaction: {}", e)),
        }
    }

    /// Get successful transactions count
    pub async fn get_successful_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE success = true AND status != 'Failed'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to get successful transactions count: {}", e))?;

        Ok(count as u64)
    }

    /// Get failed transactions count
    pub async fn get_failed_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM raw_transactions WHERE success = false OR status = 'Failed'",
                [],
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

        let raw_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM raw_transactions", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let processed_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM processed_transactions", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let known_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM known_signatures", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let retries_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deferred_retries", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let pending_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pending_transactions", [], |row| {
                row.get(0)
            })
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

        log(
            LogTag::Transactions,
            "INFO",
            "Starting database maintenance",
        );

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

        log(
            LogTag::Transactions,
            "INFO",
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

        let raw_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM raw_transactions", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let processed_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM processed_transactions", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM processed_transactions WHERE signature NOT IN (SELECT signature FROM raw_transactions)",
                [],
                |row| row.get(0)
            )
            .unwrap_or(0);

        let missing: i64 = raw_count - processed_count;

        let pending_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pending_transactions", [], |row| {
                row.get(0)
            })
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

        // Limit page size to max 200 for performance
        let effective_limit = limit.min(200);

        // Build SQL query with filters
        let mut query = String::from(
            "SELECT 
                r.signature, r.timestamp, r.slot, r.status, r.success, 
                r.fee_lamports, r.instructions_count,
                p.transaction_type, p.direction, p.token_swap_info, 
                p.token_transfers, p.sol_balance_change, p.ata_operations,
                p.fee_sol
            FROM raw_transactions r
            LEFT JOIN processed_transactions p ON r.signature = p.signature
            WHERE 1=1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Apply cursor for pagination (timestamp desc, signature desc)
        if let Some(cursor) = cursor {
            query.push_str(" AND (r.timestamp < ?1 OR (r.timestamp = ?1 AND r.signature < ?2))");
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
            query.push_str(&format!(" AND r.status = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(status.clone()));
        }

        // Apply success filter
        if filters.only_confirmed.unwrap_or(false) {
            query.push_str(" AND r.status IN ('Confirmed', 'Finalized')");
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
                let timestamp_str: String = row.get(1)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                let status_str: String = row.get(3)?;
                let success: bool = row.get(4)?;

                // Parse processed data from JSON columns
                let transaction_type_str: Option<String> = row.get(7).ok();
                let direction_str: Option<String> = row.get(8).ok();
                let token_swap_info_json: Option<String> = row.get(9).ok();
                let token_transfers_json: Option<String> = row.get(10).ok();
                let sol_balance_change: Option<f64> = row.get(11).ok();
                let ata_operations_json: Option<String> = row.get(12).ok();
                let fee_sol: Option<f64> = row.get(13).ok();

                Ok(TransactionListRow {
                    signature: row.get(0)?,
                    timestamp,
                    slot: row.get(2).ok(),
                    status: status_str,
                    success,
                    direction: direction_str,
                    transaction_type: transaction_type_str,
                    token_mint: None, // Will be extracted below
                    token_symbol: None,
                    router: None,
                    sol_delta: sol_balance_change.unwrap_or(0.0),
                    fee_sol: fee_sol.unwrap_or(0.0),
                    fee_lamports: row.get(5).ok(),
                    ata_rents: 0.0, // Will be calculated below
                    instructions_count: row.get(6).unwrap_or(0),
                    // Store JSON for Rust-side filtering
                    _token_swap_info_json: token_swap_info_json,
                    _token_transfers_json: token_transfers_json,
                    _ata_operations_json: ata_operations_json,
                })
            })
            .map_err(|e| format!("Failed to execute list query: {}", e))?;

        // Collect and apply Rust-side filters
        let mut results: Vec<TransactionListRow> = Vec::new();

        for row_result in rows {
            let mut row = row_result.map_err(|e| format!("Failed to parse row: {}", e))?;

            // Extract token info from JSON
            if let Some(ref json_str) = row._token_swap_info_json {
                if let Ok(swap_info) = serde_json::from_str::<serde_json::Value>(json_str) {
                    row.token_mint = swap_info
                        .get("output_mint")
                        .or_else(|| swap_info.get("input_mint"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    row.router = swap_info
                        .get("router")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }

            // Calculate ATA rents from JSON
            if let Some(ref json_str) = row._ata_operations_json {
                if let Ok(ops) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                    let total_rent: f64 = ops
                        .iter()
                        .filter_map(|op| op.get("rent_lamports"))
                        .filter_map(|v| v.as_u64())
                        .map(|lamports| lamports as f64 / 1_000_000_000.0)
                        .sum();
                    row.ata_rents = total_rent;
                }
            }

            // Apply Rust-side filters
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
            let matches_type = filters.types.iter().any(|t| match t.as_str() {
                "buy" => row_type.contains("SwapSolToToken"),
                "sell" => row_type.contains("SwapTokenToSol"),
                "swap" => row_type.contains("Swap"),
                "transfer" => row_type.contains("Transfer"),
                "ata" => row_type.contains("AtaOperation") || row_type.contains("CreateAta"),
                "failed" => row_type == "Failed" || !row.success,
                "unknown" => row_type == "Unknown",
                _ => false,
            });
            if !matches_type {
                return false;
            }
        }

        // Mint filter
        if let Some(ref mint) = filters.mint {
            if let Some(ref row_mint) = row.token_mint {
                if !row_mint.contains(mint) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Direction filter
        if let Some(ref dir) = filters.direction {
            if let Some(ref row_dir) = row.direction {
                if !row_dir.eq_ignore_ascii_case(dir) {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Router filter
        if let Some(ref router) = filters.router {
            if let Some(ref row_router) = row.router {
                if !row_router.contains(router) {
                    return false;
                }
            } else {
                return false;
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

    /// Get estimated count of transactions matching filters (optional, for UI)
    pub async fn count_transactions(
        &self,
        filters: &TransactionListFilters,
    ) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let mut query = String::from("SELECT COUNT(*) FROM raw_transactions r WHERE 1=1");

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

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
            query.push_str(&format!(" AND r.status = ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(status.clone()));
        }

        if filters.only_confirmed.unwrap_or(false) {
            query.push_str(" AND r.status IN ('Confirmed', 'Finalized')");
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let count: i64 = conn
            .query_row(&query, params_refs.as_slice(), |row| row.get(0))
            .map_err(|e| format!("Failed to count transactions: {}", e))?;

        Ok(count as u64)
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

    log(
        LogTag::Transactions,
        "INFO",
        "Global transaction database initialized",
    );
    Ok(db_arc)
}

/// Get global transaction database instance
pub async fn get_transaction_database() -> Option<Arc<TransactionDatabase>> {
    let global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    global.as_ref().map(Arc::clone)
}
