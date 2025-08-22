/// Database module for transactions management
/// Replaces JSON file-based caching with high-performance SQLite database
///
/// This module provides:
/// - Thread-safe database operations using connection pooling
/// - Separation of raw blockchain data from calculated analysis
/// - ACID transactions for data integrity
/// - High-performance batch operations
/// - Migration utilities from JSON files

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{ AtomicBool, Ordering };
use tokio::sync::Mutex;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use once_cell::sync::Lazy;
use rusqlite::{ Connection, OptionalExtension, params, Result as SqliteResult };
use r2d2::{ Pool, PooledConnection };
use r2d2_sqlite::SqliteConnectionManager;

use crate::logger::{ log, LogTag };

// Static flag to track if database has been initialized (to reduce log noise)
static DATABASE_INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Database schema version for migration management
const DATABASE_SCHEMA_VERSION: u32 = 1;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

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
    raw_transaction_data TEXT, -- JSON blob of raw Solana transaction data
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_PROCESSED_TRANSACTIONS: &str =
    r#"
CREATE TABLE IF NOT EXISTS processed_transactions (
    signature TEXT PRIMARY KEY,
    transaction_type TEXT NOT NULL, -- Serialized TransactionType enum
    direction TEXT NOT NULL, -- 'Incoming', 'Outgoing', 'Internal'
    fee_sol REAL NOT NULL DEFAULT 0.0,
    sol_balance_change REAL NOT NULL DEFAULT 0.0,
    token_transfers TEXT, -- JSON array of TokenTransfer
    sol_balance_changes TEXT, -- JSON array of SolBalanceChange  
    token_balance_changes TEXT, -- JSON array of TokenBalanceChange
    log_messages TEXT, -- JSON array of log messages
    instructions TEXT, -- JSON array of InstructionInfo
    swap_analysis TEXT, -- JSON blob of SwapAnalysis
    position_impact TEXT, -- JSON blob of PositionImpact
    profit_calculation TEXT, -- JSON blob of ProfitCalculation
    fee_breakdown TEXT, -- JSON blob of FeeBreakdown
    ata_analysis TEXT, -- JSON blob of AtaAnalysis
    token_info TEXT, -- JSON blob of TokenSwapInfo
    calculated_token_price_sol REAL,
    price_source TEXT,
    token_symbol TEXT,
    token_decimals INTEGER,
    cached_analysis TEXT, -- JSON blob of CachedAnalysis
    analysis_version INTEGER NOT NULL DEFAULT 1,
    processed_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (signature) REFERENCES raw_transactions(signature) ON DELETE CASCADE
);
"#;

const SCHEMA_KNOWN_SIGNATURES: &str =
    r#"
CREATE TABLE IF NOT EXISTS known_signatures (
    signature TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'known',
    added_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

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

const SCHEMA_METADATA: &str =
    r#"
CREATE TABLE IF NOT EXISTS db_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Performance indexes
const INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_timestamp ON raw_transactions(timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_status ON raw_transactions(status);",
    "CREATE INDEX IF NOT EXISTS idx_raw_transactions_slot ON raw_transactions(slot DESC);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_type ON processed_transactions(transaction_type);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_direction ON processed_transactions(direction);",
    "CREATE INDEX IF NOT EXISTS idx_processed_transactions_analysis_version ON processed_transactions(analysis_version);",
    "CREATE INDEX IF NOT EXISTS idx_deferred_retries_next_retry ON deferred_retries(next_retry_at);",
    "CREATE INDEX IF NOT EXISTS idx_known_signatures_added_at ON known_signatures(added_at DESC);",
];

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Statistics about database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_raw_transactions: u64,
    pub total_processed_transactions: u64,
    pub total_known_signatures: u64,
    pub total_deferred_retries: u64,
    pub database_size_bytes: u64,
    pub schema_version: u32,
}

/// Migration report for JSON to database migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub total_json_files: usize,
    pub successfully_migrated: usize,
    pub failed_migrations: usize,
    pub duplicate_signatures: usize,
    pub elapsed_seconds: f64,
    pub errors: Vec<String>,
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
}

// =============================================================================
// TRANSACTION DATABASE MANAGER
// =============================================================================

/// High-performance, thread-safe database manager for transactions
/// Replaces JSON file-based caching with SQLite database
pub struct TransactionDatabase {
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
}

impl TransactionDatabase {
    /// Create new TransactionDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        // Database should be at data/transactions.db (not in data/transactions/ subdirectory)
        let data_dir = std::path::PathBuf::from("data");

        // Ensure data directory exists
        if !data_dir.exists() {
            std::fs
                ::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("transactions.db");
        let database_path_str = database_path.to_string_lossy().to_string();

        // Only log detailed initialization on first database creation
        let is_first_init = !DATABASE_INITIALIZED.load(Ordering::Relaxed);
        if is_first_init {
            log(
                LogTag::Transactions,
                "INIT",
                &format!("Initializing TransactionDatabase at: {}", database_path_str)
            );
        }

        // Configure connection manager with basic setup first
        let manager = SqliteConnectionManager::file(&database_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(5) // Reduce pool size to avoid timeouts
            .min_idle(Some(1)) // Keep at least 1 connection ready
            .build(manager)
            .map_err(|e| format!("Failed to create connection pool: {}", e))?;

        let mut db = TransactionDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: DATABASE_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema(is_first_init).await?;

        if is_first_init {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!("TransactionDatabase initialized successfully at: {}", database_path_str)
            );
            DATABASE_INITIALIZED.store(true, Ordering::Relaxed);
        }

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self, log_initialization: bool) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Configure database settings - use pragma_update for setting values
        conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        conn
            .pragma_update(None, "foreign_keys", true)
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        conn
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;

        // Create all tables
        conn
            .execute(SCHEMA_RAW_TRANSACTIONS, [])
            .map_err(|e| format!("Failed to create raw_transactions table: {}", e))?;

        conn
            .execute(SCHEMA_PROCESSED_TRANSACTIONS, [])
            .map_err(|e| format!("Failed to create processed_transactions table: {}", e))?;

        conn
            .execute(SCHEMA_KNOWN_SIGNATURES, [])
            .map_err(|e| format!("Failed to create known_signatures table: {}", e))?;

        conn
            .execute(SCHEMA_DEFERRED_RETRIES, [])
            .map_err(|e| format!("Failed to create deferred_retries table: {}", e))?;

        conn
            .execute(SCHEMA_METADATA, [])
            .map_err(|e| format!("Failed to create db_metadata table: {}", e))?;

        // Create all indexes
        for index_sql in INDEXES {
            conn.execute(index_sql, []).map_err(|e| format!("Failed to create index: {}", e))?;
        }

        // Set schema version
        conn
            .execute(
                "INSERT OR REPLACE INTO db_metadata (key, value) VALUES ('schema_version', ?1)",
                params![DATABASE_SCHEMA_VERSION.to_string()]
            )
            .map_err(|e| format!("Failed to set schema version: {}", e))?;

        if log_initialization {
            log(
                LogTag::Transactions,
                "SCHEMA",
                &format!("Database schema initialized (version {})", DATABASE_SCHEMA_VERSION)
            );
        }

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool.get().map_err(|e| format!("Failed to get database connection: {}", e))
    }

    /// Check if a signature is already known (cached)
    pub async fn is_signature_known(&self, signature: &str) -> Result<bool, String> {
        let conn = self.get_connection()?;

        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM known_signatures WHERE signature = ?1",
                params![signature],
                |_| Ok(true)
            )
            .optional()
            .map_err(|e| format!("Database error checking signature: {}", e))?
            .unwrap_or(false);

        Ok(exists)
    }

    /// Add signature to known signatures cache
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

    /// Add multiple signatures to known signatures cache (batch operation)
    pub async fn batch_add_known_signatures(&self, signatures: &[String]) -> Result<(), String> {
        let conn = self.get_connection()?;
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        {
            let mut stmt = tx
                .prepare("INSERT OR IGNORE INTO known_signatures (signature) VALUES (?1)")
                .map_err(|e| format!("Failed to prepare statement: {}", e))?;

            for signature in signatures {
                stmt
                    .execute(params![signature])
                    .map_err(|e| format!("Failed to insert signature {}: {}", signature, e))?;
            }
        }

        tx.commit().map_err(|e| format!("Failed to commit batch signature insert: {}", e))?;

        log(
            LogTag::Transactions,
            "BATCH",
            &format!("Added {} signatures to known signatures cache", signatures.len())
        );

        Ok(())
    }

    /// Get count of known signatures
    pub async fn get_known_signatures_count(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM known_signatures", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count known signatures: {}", e))?;

        Ok(count as u64)
    }

    /// Store raw transaction data
    pub async fn store_raw_transaction(
        &self,
        signature: &str,
        slot: Option<u64>,
        block_time: Option<i64>,
        timestamp: &DateTime<Utc>,
        status: &str,
        success: bool,
        error_message: Option<&str>,
        raw_transaction_data: Option<&str>
    ) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
            .execute(
                r#"INSERT OR REPLACE INTO raw_transactions 
               (signature, slot, block_time, timestamp, status, success, error_message, raw_transaction_data, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))"#,
                params![
                    signature,
                    slot.map(|s| s as i64),
                    block_time,
                    timestamp.to_rfc3339(),
                    status,
                    success,
                    error_message,
                    raw_transaction_data
                ]
            )
            .map_err(|e| format!("Failed to store raw transaction: {}", e))?;

        // Also add to known signatures
        self.add_known_signature(signature).await?;

        Ok(())
    }

    /// Get raw transaction data
    pub async fn get_raw_transaction(
        &self,
        signature: &str
    ) -> Result<Option<RawTransactionData>, String> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"SELECT signature, slot, block_time, timestamp, status, success, error_message, raw_transaction_data, created_at, updated_at
               FROM raw_transactions WHERE signature = ?1"#,
                params![signature],
                |row| {
                    Ok(RawTransactionData {
                        signature: row.get(0)?,
                        slot: row.get::<_, Option<i64>>(1)?.map(|s| s as u64),
                        block_time: row.get(2)?,
                        timestamp: row.get(3)?,
                        status: row.get(4)?,
                        success: row.get(5)?,
                        error_message: row.get(6)?,
                        raw_transaction_data: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                }
            )
            .optional()
            .map_err(|e| format!("Failed to get raw transaction: {}", e))?;

        Ok(result)
    }

    /// Store processed transaction analysis
    pub async fn store_processed_transaction(
        &self,
        transaction: &ProcessedTransaction
    ) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
            .execute(
                r#"INSERT OR REPLACE INTO processed_transactions 
               (signature, transaction_type, direction, fee_sol, sol_balance_change, 
                token_transfers, sol_balance_changes, token_balance_changes, log_messages,
                instructions, swap_analysis, position_impact, profit_calculation, fee_breakdown,
                ata_analysis, token_info, calculated_token_price_sol, price_source, token_symbol,
                token_decimals, cached_analysis, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, datetime('now'))"#,
                params![
                    transaction.signature,
                    transaction.swap_type,
                    "Internal", // Default direction
                    0.0, // fee_sol - could be extracted from the transaction if needed
                    0.0, // sol_balance_change - could be extracted if needed
                    None::<String>, // token_transfers JSON
                    None::<String>, // sol_balance_changes JSON
                    None::<String>, // token_balance_changes JSON
                    None::<String>, // log_messages JSON
                    None::<String>, // instructions JSON
                    None::<String>, // swap_analysis JSON
                    None::<String>, // position_impact JSON
                    None::<String>, // profit_calculation JSON
                    None::<String>, // fee_breakdown JSON
                    None::<String>, // ata_analysis JSON
                    None::<String>, // token_info JSON
                    transaction.price_sol,
                    None::<String>, // price_source
                    transaction.token_mint, // Using token_mint as symbol for now
                    None::<i32>, // token_decimals
                    None::<String> // cached_analysis JSON
                ]
            )
            .map_err(|e| format!("Failed to store processed transaction: {}", e))?;

        Ok(())
    }

    /// Get processed transaction analysis
    pub async fn get_processed_transaction(
        &self,
        signature: &str
    ) -> Result<Option<ProcessedTransaction>, String> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"SELECT signature, transaction_type, calculated_token_price_sol, token_symbol, processed_at
               FROM processed_transactions WHERE signature = ?1"#,
                params![signature],
                |row| {
                    Ok(ProcessedTransaction {
                        id: None,
                        signature: row.get(0)?,
                        swap_type: row.get(1)?,
                        token_mint: row.get(3)?,
                        amount_in: None,
                        amount_out: None,
                        price_sol: row.get(2)?,
                        price_usd: None,
                        market_cap: None,
                        liquidity_sol: None,
                        liquidity_usd: None,
                        volume_24h: None,
                        holder_count: None,
                        is_buy: None,
                        wallet_address: None,
                        dex_name: None,
                        pool_address: None,
                        created_at: 0,
                        updated_at: 0,
                    })
                }
            )
            .optional()
            .map_err(|e| format!("Failed to get processed transaction: {}", e))?;

        Ok(result)
    }

    /// Store deferred retry
    pub async fn store_deferred_retry(
        &self,
        signature: &str,
        next_retry_at: &DateTime<Utc>,
        remaining_attempts: i32,
        current_delay_secs: i64,
        last_error: Option<&str>
    ) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
            .execute(
                r#"INSERT OR REPLACE INTO deferred_retries 
               (signature, next_retry_at, remaining_attempts, current_delay_secs, last_error, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))"#,
                params![
                    signature,
                    next_retry_at.to_rfc3339(),
                    remaining_attempts,
                    current_delay_secs,
                    last_error
                ]
            )
            .map_err(|e| format!("Failed to store deferred retry: {}", e))?;

        Ok(())
    }

    /// Get pending deferred retries
    pub async fn get_pending_deferred_retries(&self) -> Result<Vec<DeferredRetryData>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"SELECT signature, next_retry_at, remaining_attempts, current_delay_secs, last_error, created_at, updated_at
               FROM deferred_retries 
               WHERE remaining_attempts > 0 AND next_retry_at <= datetime('now')
               ORDER BY next_retry_at ASC"#
            )
            .map_err(|e| format!("Failed to prepare deferred retries query: {}", e))?;

        let retry_iter = stmt
            .query_map([], |row| {
                Ok(DeferredRetryData {
                    signature: row.get(0)?,
                    next_retry_at: row.get(1)?,
                    remaining_attempts: row.get(2)?,
                    current_delay_secs: row.get(3)?,
                    last_error: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute deferred retries query: {}", e))?;

        let mut retries = Vec::new();
        for retry_result in retry_iter {
            retries.push(
                retry_result.map_err(|e| format!("Failed to parse deferred retry: {}", e))?
            );
        }

        Ok(retries)
    }

    /// Remove deferred retry (when successfully processed)
    pub async fn remove_deferred_retry(&self, signature: &str) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
            .execute("DELETE FROM deferred_retries WHERE signature = ?1", params![signature])
            .map_err(|e| format!("Failed to remove deferred retry: {}", e))?;

        Ok(())
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<DatabaseStats, String> {
        let conn = self.get_connection()?;

        let raw_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM raw_transactions", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count raw transactions: {}", e))?;

        let processed_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM processed_transactions", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count processed transactions: {}", e))?;

        let signatures_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM known_signatures", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count known signatures: {}", e))?;

        let retries_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM deferred_retries", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count deferred retries: {}", e))?;

        // Get database file size
        let database_size = std::fs
            ::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(DatabaseStats {
            total_raw_transactions: raw_count as u64,
            total_processed_transactions: processed_count as u64,
            total_known_signatures: signatures_count as u64,
            total_deferred_retries: retries_count as u64,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get all transaction signatures from the database
    pub async fn get_all_signatures(&self) -> Result<Vec<String>, String> {
        // Use pooled connection directly; ordering by slot DESC if present else by rowid
        let conn = self.pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let mut signatures = Vec::new();
        let mut stmt = conn
            .prepare("SELECT signature FROM raw_transactions ORDER BY slot DESC, timestamp DESC")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map([], |row| { Ok(row.get::<_, String>(0)?) })
            .map_err(|e| format!("Failed to execute query: {}", e))?;

        for row in rows {
            if let Ok(signature) = row {
                signatures.push(signature);
            }
        }

        Ok(signatures)
    }

    /// Vacuum database to reclaim space and optimize performance
    pub async fn vacuum_database(&self) -> Result<(), String> {
        log(LogTag::Transactions, "VACUUM", "Starting database vacuum operation...");

        let conn = self.get_connection()?;
        conn.execute("VACUUM", []).map_err(|e| format!("Failed to vacuum database: {}", e))?;

        log(LogTag::Transactions, "VACUUM", "Database vacuum completed successfully");
        Ok(())
    }

    /// Analyze database for query optimization
    pub async fn analyze_database(&self) -> Result<(), String> {
        log(LogTag::Transactions, "ANALYZE", "Running database analysis for optimization...");

        let conn = self.get_connection()?;
        conn.execute("ANALYZE", []).map_err(|e| format!("Failed to analyze database: {}", e))?;

        log(LogTag::Transactions, "ANALYZE", "Database analysis completed successfully");
        Ok(())
    }
}

// =============================================================================
// DATA STRUCTURES FOR DATABASE RECORDS
// =============================================================================

/// Raw transaction data record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTransactionData {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub timestamp: String,
    pub status: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub raw_transaction_data: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Deferred retry data record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredRetryData {
    pub signature: String,
    pub next_retry_at: String,
    pub remaining_attempts: i32,
    pub current_delay_secs: i64,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Processed transaction data record for database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    pub id: Option<i64>,
    pub signature: String,
    pub swap_type: Option<String>,
    pub token_mint: Option<String>,
    pub amount_in: Option<f64>,
    pub amount_out: Option<f64>,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub market_cap: Option<f64>,
    pub liquidity_sol: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub holder_count: Option<i64>,
    pub is_buy: Option<bool>,
    pub wallet_address: Option<String>,
    pub dex_name: Option<String>,
    pub pool_address: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
