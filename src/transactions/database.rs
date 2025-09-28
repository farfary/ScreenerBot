// Database operations and persistence for the transactions module
//
// This module provides high-performance SQLite-based caching and persistence
// for transaction data, replacing the previous JSON file-based approach.

use chrono::{ DateTime, Utc };
use once_cell::sync::Lazy;
use r2d2::{ Pool, PooledConnection };
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{ params, Connection, OptionalExtension, Result as SqliteResult };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::path::{ Path, PathBuf };
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::global::is_debug_transactions_enabled;
use crate::logger::{ log, LogTag };
use crate::transactions::{ types::*, utils::* };

// =============================================================================
// DATABASE SCHEMA AND CONSTANTS
// =============================================================================

/// Database schema version for migration management
const DATABASE_SCHEMA_VERSION: u32 = 3;

/// Static flag to track if database has been initialized (to reduce log noise)
static DATABASE_INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

/// Raw transactions table schema - stores blockchain data
const SCHEMA_RAW_TRANSACTIONS: &str =
    r#"
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
const SCHEMA_PROCESSED_TRANSACTIONS: &str =
    r#"
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
const SCHEMA_KNOWN_SIGNATURES: &str =
    r#"
CREATE TABLE IF NOT EXISTS known_signatures (
    signature TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'known',
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

/// Deferred retries tracking table
const SCHEMA_DEFERRED_RETRIES: &str =
    r#"
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
const SCHEMA_PENDING_TRANSACTIONS: &str =
    r#"
CREATE TABLE IF NOT EXISTS pending_transactions (
    signature TEXT PRIMARY KEY,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_checked_at TEXT,
    check_count INTEGER NOT NULL DEFAULT 0
);
"#;

/// Database metadata table
const SCHEMA_METADATA: &str =
    r#"
CREATE TABLE IF NOT EXISTS db_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
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
            std::fs
                ::create_dir_all(&data_dir)
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
                &format!("Initializing TransactionDatabase at: {}", database_path_str)
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
            log(LogTag::Transactions, "INIT", "TransactionDatabase initialization complete");
        }

        Ok(db)
    }

    #[cfg(test)]
    pub(crate) async fn new_with_path<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let database_path = path.as_ref().to_path_buf();

        if let Some(parent) = database_path.parent() {
            std::fs
                ::create_dir_all(parent)
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
        ];

        for table_sql in &tables {
            conn.execute(table_sql, []).map_err(|e| format!("Failed to create table: {}", e))?;
        }

        // Create all indexes
        for index_sql in INDEXES {
            conn.execute(index_sql, []).map_err(|e| format!("Failed to create index: {}", e))?;
        }

        // Apply lightweight migrations for existing databases
        self.apply_migrations(&conn)?;

        // Set or update schema version
        conn
            .execute(
                "INSERT OR REPLACE INTO db_metadata (key, value) VALUES (?1, ?2)",
                params!["schema_version", self.schema_version.to_string()]
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
            conn
                .execute(
                    "ALTER TABLE processed_transactions ADD COLUMN fee_sol REAL NOT NULL DEFAULT 0",
                    []
                )
                .map_err(|e| format!("Failed to add fee_sol column: {}", e))?;
        }
        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool.get().map_err(|e| format!("Failed to get database connection from pool: {}", e))
    }

    /// Health check - verify database connectivity and basic operations
    pub async fn health_check(&self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Test basic query
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table'", [], |row|
                row.get(0)
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
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to check known signature: {}", e))?;

        Ok(exists)
    }

    /// Add signature to known signatures
    pub async fn add_known_signature(&self, signature: &str) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
            .execute(
                "INSERT OR IGNORE INTO known_signatures (signature) VALUES (?1)",
                params![signature]
            )
            .map_err(|e| format!("Failed to add known signature: {}", e))?;

        Ok(())
    }

    /// Get count of known signatures
    pub async fn get_known_signatures_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM known_signatures", [], |row| { row.get(0) })
            .map_err(|e| format!("Failed to get known signatures count: {}", e))?;

        Ok(count as u64)
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
        pending: &HashMap<String, DateTime<Utc>>
    ) -> Result<(), String> {
        let conn = self.get_connection()?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        for (signature, timestamp) in pending {
            tx
                .execute(
                    "INSERT OR REPLACE INTO pending_transactions (signature, added_at) VALUES (?1, ?2)",
                    params![signature, timestamp.to_rfc3339()]
                )
                .map_err(|e| format!("Failed to save pending transaction: {}", e))?;
        }

        tx.commit().map_err(|e| format!("Failed to commit pending transactions: {}", e))?;

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
                            rusqlite::types::Type::Text
                        )
                    })?
                    .with_timezone(&Utc);
                Ok((signature, timestamp))
            })
            .map_err(|e| format!("Failed to query pending transactions: {}", e))?;

        let mut result = HashMap::new();
        for row in rows {
            let (signature, timestamp) = row.map_err(|e|
                format!("Failed to parse pending transaction row: {}", e)
            )?;
            result.insert(signature, timestamp);
        }

        Ok(result)
    }

    /// Remove pending transaction
    pub async fn remove_pending_transaction(&self, signature: &str) -> Result<bool, String> {
        let conn = self.get_connection()?;

        let affected = conn
            .execute("DELETE FROM pending_transactions WHERE signature = ?1", params![signature])
            .map_err(|e| format!("Failed to remove pending transaction: {}", e))?;

        Ok(affected > 0)
    }

    /// Get count of pending transactions
    pub async fn get_pending_transactions_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pending_transactions", [], |row| { row.get(0) })
            .map_err(|e| format!("Failed to get pending transactions count: {}", e))?;

        Ok(count as u64)
    }
}

// =============================================================================
// IMPLEMENTATION - TRANSACTION DATA MANAGEMENT
// =============================================================================

impl TransactionDatabase {
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

        let raw_transaction_json = transaction.raw_transaction_data
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
                    &transaction.signature,
                    status_str,
                    transaction.success
                )
            );
        }

        Ok(())
    }

    /// Store processed transaction analysis snapshot
    pub async fn store_processed_transaction(
        &self,
        transaction: &Transaction
    ) -> Result<(), String> {
        let debug = is_debug_transactions_enabled();
        let conn = self.get_connection()?;

        // Serialize complex fields as JSON strings
        let sol_balance_change_json = serde_json
            ::to_string(&transaction.sol_balance_changes)
            .unwrap_or_else(|_| "[]".to_string());
        let token_balance_changes_json = serde_json
            ::to_string(&transaction.token_balance_changes)
            .unwrap_or_else(|_| "[]".to_string());
        let token_swap_info_json = serde_json
            ::to_string(&transaction.token_swap_info)
            .unwrap_or_else(|_| "null".to_string());
        let swap_pnl_info_json = serde_json
            ::to_string(&transaction.swap_pnl_info)
            .unwrap_or_else(|_| "null".to_string());
        let ata_operations_json = serde_json
            ::to_string(&transaction.ata_operations)
            .unwrap_or_else(|_| "[]".to_string());
        let token_transfers_json = serde_json
            ::to_string(&transaction.token_transfers)
            .unwrap_or_else(|_| "[]".to_string());
        let instruction_info_json = serde_json
            ::to_string(&transaction.instructions)
            .unwrap_or_else(|_| "[]".to_string());
        let cached_analysis_json = serde_json
            ::to_string(&transaction.cached_analysis)
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
                )
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
        error_message: Option<&str>
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
                            rusqlite::types::Type::Text
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
            }
        );

        match result {
            Ok(transaction) => {
                if debug {
                    log(
                        LogTag::Transactions,
                        "DB_HIT",
                        &format!(
                            "Cache hit for {} (status={:?})",
                            signature,
                            transaction.status
                        )
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
                |row| row.get(0)
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
                |row| row.get(0)
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
            .query_row("SELECT COUNT(*) FROM raw_transactions", [], |row| { row.get(0) })
            .unwrap_or(0);

        let processed_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM processed_transactions", [], |row| { row.get(0) })
            .unwrap_or(0);

        let known_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM known_signatures", [], |row| { row.get(0) })
            .unwrap_or(0);

        let retries_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deferred_retries", [], |row| { row.get(0) })
            .unwrap_or(0);

        let pending_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pending_transactions", [], |row| { row.get(0) })
            .unwrap_or(0);

        // Get database file size
        let database_size = std::fs
            ::metadata(&self.database_path)
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

        log(LogTag::Transactions, "INFO", "Starting database maintenance");

        // Vacuum to reclaim space
        conn.execute("VACUUM", []).map_err(|e| format!("Failed to vacuum database: {}", e))?;

        // Analyze for query optimization
        conn.execute("ANALYZE", []).map_err(|e| format!("Failed to analyze database: {}", e))?;

        // Cleanup old pending transactions (older than 1 day)
        let cleaned_pending = conn
            .execute(
                "DELETE FROM pending_transactions WHERE added_at < datetime('now', '-1 day')",
                []
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
                cleaned_pending,
                cleaned_retries
            )
        );

        Ok(())
    }

    /// Get integrity report
    pub async fn get_integrity_report(&self) -> Result<IntegrityReport, String> {
        let conn = self.get_connection()?;

        let raw_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM raw_transactions", [], |row| { row.get(0) })
            .unwrap_or(0);

        let processed_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM processed_transactions", [], |row| { row.get(0) })
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
            .query_row("SELECT COUNT(*) FROM pending_transactions", [], |row| { row.get(0) })
            .unwrap_or(0);

        // Check schema version
        let schema_version_correct = conn
            .query_row("SELECT value FROM db_metadata WHERE key = 'schema_version'", [], |row| {
                let version_str: String = row.get(0)?;
                Ok(version_str == self.schema_version.to_string())
            })
            .unwrap_or(false);

        Ok(IntegrityReport {
            raw_transactions_count: raw_count as u64,
            processed_transactions_count: processed_count as u64,
            orphaned_processed_transactions: orphaned as u64,
            missing_processed_transactions: missing.max(0) as u64,
            schema_version_correct,
            foreign_key_violations: 0, // Would require FK check
            index_integrity_ok: true, // Would require index check
            pending_transactions_count: pending_count as u64,
        })
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
        SolBalanceChange,
        TransactionDirection,
        TransactionStatus,
        TransactionType,
    };

    #[tokio::test]
    async fn upsert_and_fetch_transaction_caches_raw_and_processed() {
        let dir = tempdir().expect("create temp dir");
        let db_path = dir.path().join("transactions.db");
        let db = TransactionDatabase::new_with_path(&db_path).await.expect("create database");

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

        db.upsert_full_transaction(&transaction).await.expect("upsert transaction");

        let fetched = db
            .get_transaction(&transaction.signature).await
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
                |row| row.get(0)
            )
            .expect("query raw data");
        assert_eq!(stored_raw, Some(raw_json_string));

        let stored_fee: f64 = conn
            .query_row(
                "SELECT fee_sol FROM processed_transactions WHERE signature = ?1",
                [transaction.signature.as_str()],
                |row| row.get(0)
            )
            .expect("query processed fee");
        assert!((stored_fee - transaction.fee_sol).abs() < 1e-12);
    }
}

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global database instance for cross-module access
static GLOBAL_TRANSACTION_DATABASE: Lazy<Arc<Mutex<Option<Arc<TransactionDatabase>>>>> = Lazy::new(
    || Arc::new(Mutex::new(None))
);

/// Initialize global transaction database
pub async fn init_transaction_database() -> Result<Arc<TransactionDatabase>, String> {
    let db = TransactionDatabase::new().await?;
    let db_arc = Arc::new(db);

    let mut global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    *global = Some(Arc::clone(&db_arc));

    log(LogTag::Transactions, "INFO", "Global transaction database initialized");
    Ok(db_arc)
}

/// Get global transaction database instance
pub async fn get_transaction_database() -> Option<Arc<TransactionDatabase>> {
    let global = GLOBAL_TRANSACTION_DATABASE.lock().await;
    global.as_ref().map(Arc::clone)
}
