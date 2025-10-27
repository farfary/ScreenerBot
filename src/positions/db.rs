use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
/// Database module for positions management
/// Replaces JSON file-based storage with high-performance SQLite database
///
/// This module provides:
/// - Thread-safe database operations using connection pooling
/// - ACID transactions for data integrity
/// - High-performance batch operations
/// - Comprehensive position state management
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::logger::{self, LogTag};
use crate::positions::types::{EntryRecord, ExitRecord, Position};

// Static flag to track if database has been initialized (to reduce log noise)
static POSITIONS_DB_INITIALIZED: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

// Database schema version
const POSITIONS_SCHEMA_VERSION: u32 = 1;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

// Column list for SELECT queries (DRY principle)
const POSITION_SELECT_COLUMNS: &str = r#"
    id, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
    position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
    entry_transaction_signature, exit_transaction_signature, token_amount,
    effective_entry_price, effective_exit_price, sol_received,
    profit_target_min, profit_target_max, liquidity_tier,
    transaction_entry_verified, transaction_exit_verified,
    entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
    phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
    pnl, pnl_percent, unrealized_pnl, unrealized_pnl_percent,
    remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
    dca_count, average_entry_price, last_dca_time
"#;

const SCHEMA_POSITIONS: &str = r#"
CREATE TABLE IF NOT EXISTS positions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    entry_price REAL NOT NULL,
    entry_time TEXT NOT NULL,
    exit_price REAL,
    exit_time TEXT,
    position_type TEXT NOT NULL, -- 'buy' or 'sell'
    entry_size_sol REAL NOT NULL,  -- Initial SOL spent on first entry
    total_size_sol REAL NOT NULL,  -- Cumulative SOL invested (includes DCA)
    price_highest REAL NOT NULL,
    price_lowest REAL NOT NULL,
    -- Real swap tracking
    entry_transaction_signature TEXT,
    exit_transaction_signature TEXT,
    token_amount INTEGER, -- Initial amount of tokens bought (first entry)
    effective_entry_price REAL, -- Initial entry price (deprecated, use average_entry_price)
    effective_exit_price REAL, -- Final exit price (deprecated, use average_exit_price)
    sol_received REAL, -- Total SOL received after all exits
    -- Smart profit targeting
    profit_target_min REAL, -- Minimum profit target percentage
    profit_target_max REAL, -- Maximum profit target percentage
    liquidity_tier TEXT, -- Liquidity tier for reference
    -- Transaction verification status
    transaction_entry_verified BOOLEAN NOT NULL DEFAULT false,
    transaction_exit_verified BOOLEAN NOT NULL DEFAULT false,
    -- Actual transaction fees (in lamports)
    entry_fee_lamports INTEGER, -- Actual entry transaction fee
    exit_fee_lamports INTEGER, -- Actual exit transaction fee
    -- Current price tracking
    current_price REAL, -- Current market price
    current_price_updated TEXT, -- When current_price was last updated
    -- Phantom detection tracking
    phantom_confirmations INTEGER NOT NULL DEFAULT 0,
    phantom_first_seen TEXT, -- When first confirmed phantom
    synthetic_exit BOOLEAN NOT NULL DEFAULT false,
    closed_reason TEXT, -- Optional reason for closure
    -- Pre-calculated P&L values (updated by positions system)
    pnl REAL, -- Realized P&L in SOL (for closed positions)
    pnl_percent REAL, -- Realized P&L percentage (for closed positions)
    unrealized_pnl REAL, -- Unrealized P&L in SOL (for open positions)
    unrealized_pnl_percent REAL, -- Unrealized P&L percentage (for open positions)
    -- Partial exit tracking
    remaining_token_amount INTEGER, -- Current holdings after partial exits
    total_exited_amount INTEGER NOT NULL DEFAULT 0, -- Cumulative tokens sold
    average_exit_price REAL, -- Weighted average exit price
    partial_exit_count INTEGER NOT NULL DEFAULT 0, -- Number of partial exits
    -- DCA tracking
    dca_count INTEGER NOT NULL DEFAULT 0, -- Number of additional entries (DCA)
    average_entry_price REAL NOT NULL DEFAULT 0, -- Weighted average entry price
    last_dca_time TEXT, -- Last DCA timestamp for cooldown
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_POSITION_STATES: &str = r#"
CREATE TABLE IF NOT EXISTS position_states (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    state TEXT NOT NULL, -- 'Open', 'Closing', 'Closed', 'ExitPending', 'ExitFailed', 'Phantom', 'Reconciling'
    changed_at TEXT NOT NULL DEFAULT (datetime('now')),
    reason TEXT, -- Optional reason for state change
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

const SCHEMA_POSITION_EXITS: &str = r#"
CREATE TABLE IF NOT EXISTS position_exits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    amount INTEGER NOT NULL, -- Tokens sold
    price REAL NOT NULL, -- Exit price per token
    sol_received REAL NOT NULL, -- SOL received
    transaction_signature TEXT NOT NULL,
    is_partial BOOLEAN NOT NULL, -- true if partial, false if full exit
    percentage REAL NOT NULL, -- % of position sold
    fees_lamports INTEGER, -- Transaction fee
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

const SCHEMA_POSITION_ENTRIES: &str = r#"
CREATE TABLE IF NOT EXISTS position_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    amount INTEGER NOT NULL, -- Tokens bought
    price REAL NOT NULL, -- Entry price per token
    sol_spent REAL NOT NULL, -- SOL spent
    transaction_signature TEXT NOT NULL,
    is_dca BOOLEAN NOT NULL, -- true if DCA, false if initial entry
    fees_lamports INTEGER, -- Transaction fee
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

const SCHEMA_POSITION_TRACKING: &str = r#"
CREATE TABLE IF NOT EXISTS position_tracking (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    price REAL NOT NULL,
    price_source TEXT NOT NULL, -- 'pool', 'api', 'cache'
    pool_type TEXT, -- e.g., 'RAYDIUM CPMM'
    pool_address TEXT,
    api_price REAL,
    tracked_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

const SCHEMA_POSITION_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS position_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_TOKEN_SNAPSHOTS: &str = r#"
CREATE TABLE IF NOT EXISTS token_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    position_id INTEGER NOT NULL,
    snapshot_type TEXT NOT NULL, -- 'opening' or 'closing'
    mint TEXT NOT NULL,
    -- DexScreener token data
    symbol TEXT,
    name TEXT,
    price_sol REAL,
    price_usd REAL,
    price_native REAL,
    dex_id TEXT,
    pair_address TEXT,
    pair_url TEXT,
    fdv REAL,
    market_cap REAL,
    pair_created_at INTEGER,
    -- Liquidity data
    liquidity_usd REAL,
    liquidity_base REAL,
    liquidity_quote REAL,
    -- Volume data
    volume_h24 REAL,
    volume_h6 REAL,
    volume_h1 REAL,
    volume_m5 REAL,
    -- Transaction stats
    txns_h24_buys INTEGER,
    txns_h24_sells INTEGER,
    txns_h6_buys INTEGER,
    txns_h6_sells INTEGER,
    txns_h1_buys INTEGER,
    txns_h1_sells INTEGER,
    txns_m5_buys INTEGER,
    txns_m5_sells INTEGER,
    -- Price change data
    price_change_h24 REAL,
    price_change_h6 REAL,
    price_change_h1 REAL,
    price_change_m5 REAL,
    -- Token meta
    token_uri TEXT,
    token_description TEXT,
    token_image TEXT,
    token_website TEXT,
    token_twitter TEXT,
    token_telegram TEXT,
    -- Snapshot metadata
    snapshot_time TEXT NOT NULL DEFAULT (datetime('now')),
    api_fetch_time TEXT NOT NULL DEFAULT (datetime('now')),
    data_freshness_score INTEGER DEFAULT 0, -- 0-100 based on data recency
    FOREIGN KEY (position_id) REFERENCES positions(id) ON DELETE CASCADE
);
"#;

const MIGRATION_ADD_PNL_FIELDS: &str = r#"
-- Add P&L fields to positions table (safe migration - columns are nullable)
ALTER TABLE positions ADD COLUMN pnl REAL;
ALTER TABLE positions ADD COLUMN pnl_percent REAL;
ALTER TABLE positions ADD COLUMN unrealized_pnl REAL;
ALTER TABLE positions ADD COLUMN unrealized_pnl_percent REAL;
"#;

// Performance indexes
const POSITIONS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_positions_mint ON positions(mint);",
    "CREATE INDEX IF NOT EXISTS idx_positions_entry_time ON positions(entry_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_positions_exit_time ON positions(exit_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_positions_mint_exit_time ON positions(mint, exit_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_positions_entry_signature ON positions(entry_transaction_signature);",
    "CREATE INDEX IF NOT EXISTS idx_positions_exit_signature ON positions(exit_transaction_signature);",
    "CREATE INDEX IF NOT EXISTS idx_positions_state ON positions(id, position_type, exit_time);",
    "CREATE INDEX IF NOT EXISTS idx_position_states_position_id ON position_states(position_id, changed_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_states_state ON position_states(state, changed_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_tracking_position_id ON position_tracking(position_id, tracked_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_tracking_price ON position_tracking(price, tracked_at DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_snapshots_position_id ON token_snapshots(position_id, snapshot_type);",
    "CREATE INDEX IF NOT EXISTS idx_token_snapshots_mint ON token_snapshots(mint, snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_snapshots_type ON token_snapshots(snapshot_type, snapshot_time DESC);",
    // New indexes for partial exit and DCA tracking
    "CREATE INDEX IF NOT EXISTS idx_position_exits_position_id ON position_exits(position_id, timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_exits_timestamp ON position_exits(timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_entries_position_id ON position_entries(position_id, timestamp DESC);",
    "CREATE INDEX IF NOT EXISTS idx_position_entries_timestamp ON position_entries(timestamp DESC);",
];

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Position state enum with comprehensive lifecycle tracking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PositionState {
    Open,        // No exit transaction, actively trading
    Closing,     // Exit transaction submitted but not yet verified
    Closed,      // Exit transaction verified and exit_price set
    ExitPending, // Exit transaction in verification queue (similar to Closing but more explicit)
    ExitFailed,  // Exit transaction failed and needs retry
    Phantom,     // Position exists but wallet has zero tokens (needs reconciliation)
    Reconciling, // Auto-healing in progress for phantom positions
}

impl std::fmt::Display for PositionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionState::Open => write!(f, "Open"),
            PositionState::Closing => write!(f, "Closing"),
            PositionState::Closed => write!(f, "Closed"),
            PositionState::ExitPending => write!(f, "ExitPending"),
            PositionState::ExitFailed => write!(f, "ExitFailed"),
            PositionState::Phantom => write!(f, "Phantom"),
            PositionState::Reconciling => write!(f, "Reconciling"),
        }
    }
}

impl std::str::FromStr for PositionState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(PositionState::Open),
            "Closing" => Ok(PositionState::Closing),
            "Closed" => Ok(PositionState::Closed),
            "ExitPending" => Ok(PositionState::ExitPending),
            "ExitFailed" => Ok(PositionState::ExitFailed),
            "Phantom" => Ok(PositionState::Phantom),
            "Reconciling" => Ok(PositionState::Reconciling),
            _ => Err(format!("Unknown position state: {}", s)),
        }
    }
}

/// Position state history record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionStateHistory {
    pub position_id: i64,
    pub state: PositionState,
    pub changed_at: DateTime<Utc>,
    pub reason: Option<String>,
}

/// Token snapshot captured at position opening or closing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSnapshot {
    pub id: Option<i64>,
    pub position_id: i64,
    pub snapshot_type: String, // "opening" or "closing"
    pub mint: String,
    // DexScreener data
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_native: Option<f64>,
    pub dex_id: Option<String>,
    pub pair_address: Option<String>,
    pub pair_url: Option<String>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub pair_created_at: Option<i64>,
    // Liquidity data
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,
    // Volume data
    pub volume_h24: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h1: Option<f64>,
    pub volume_m5: Option<f64>,
    // Transaction stats
    pub txns_h24_buys: Option<i64>,
    pub txns_h24_sells: Option<i64>,
    pub txns_h6_buys: Option<i64>,
    pub txns_h6_sells: Option<i64>,
    pub txns_h1_buys: Option<i64>,
    pub txns_h1_sells: Option<i64>,
    pub txns_m5_buys: Option<i64>,
    pub txns_m5_sells: Option<i64>,
    // Price change data
    pub price_change_h24: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_m5: Option<f64>,
    // Token meta
    pub token_uri: Option<String>,
    pub token_description: Option<String>,
    pub token_image: Option<String>,
    pub token_website: Option<String>,
    pub token_twitter: Option<String>,
    pub token_telegram: Option<String>,
    // Snapshot metadata
    pub snapshot_time: DateTime<Utc>,
    pub api_fetch_time: DateTime<Utc>,
    pub data_freshness_score: i32,
}

/// Position tracking record for price updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionTracking {
    pub position_id: i64,
    pub price: f64,
    pub price_source: String,      // "pool", "api", "cache"
    pub pool_type: Option<String>, // e.g., "RAYDIUM CPMM"
    pub pool_address: Option<String>,
    pub api_price: Option<f64>,
    pub tracked_at: DateTime<Utc>,
}

/// Statistics about positions database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionsDatabaseStats {
    pub total_positions: u64,
    pub open_positions: u64,
    pub closed_positions: u64,
    pub phantom_positions: u64,
    pub total_state_history: u64,
    pub total_tracking_records: u64,
    pub database_size_bytes: u64,
    pub schema_version: u32,
}

// =============================================================================
// POSITIONS DATABASE MANAGER
// =============================================================================

/// High-performance, thread-safe database manager for positions
/// Replaces JSON file-based storage with SQLite database
#[derive(Clone)]
pub struct PositionsDatabase {
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
}

impl PositionsDatabase {
    /// Create new PositionsDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        // Database should be at data/positions.db
        let data_dir = std::path::PathBuf::from("data");

        // Ensure data directory exists
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("positions.db");
        let database_path_str = database_path.to_string_lossy().to_string();

        // Only log detailed initialization on first database creation
        let is_first_init = !POSITIONS_DB_INITIALIZED.load(Ordering::Relaxed);
        if is_first_init {
            logger::info(
                LogTag::Positions,
                &format!("Initializing positions database at: {}", database_path_str),
            );
        }

        // Configure connection manager
        let manager = SqliteConnectionManager::file(&database_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(5) // Reduce pool size to avoid timeouts
            .min_idle(Some(1)) // Keep at least 1 connection ready
            .build(manager)
            .map_err(|e| format!("Failed to create positions connection pool: {}", e))?;

        let mut db = PositionsDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: POSITIONS_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema(is_first_init).await?;

        if is_first_init {
            logger::info(
                LogTag::Positions,
                "Positions database initialized successfully",
            );
            POSITIONS_DB_INITIALIZED.store(true, Ordering::Relaxed);
        }

        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self, log_initialization: bool) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Configure database settings
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
        // Additional performance-related PRAGMAs
        let _ = conn.pragma_update(None, "cache_size", 10000);
        let _ = conn.pragma_update(None, "temp_store", "memory");
        let _ = conn.busy_timeout(std::time::Duration::from_millis(30_000));

        // Create all tables
        conn.execute(SCHEMA_POSITIONS, [])
            .map_err(|e| format!("Failed to create positions table: {}", e))?;

        conn.execute(SCHEMA_POSITION_STATES, [])
            .map_err(|e| format!("Failed to create position_states table: {}", e))?;

        conn.execute(SCHEMA_POSITION_EXITS, [])
            .map_err(|e| format!("Failed to create position_exits table: {}", e))?;

        conn.execute(SCHEMA_POSITION_ENTRIES, [])
            .map_err(|e| format!("Failed to create position_entries table: {}", e))?;

        conn.execute(SCHEMA_POSITION_TRACKING, [])
            .map_err(|e| format!("Failed to create position_tracking table: {}", e))?;

        conn.execute(SCHEMA_POSITION_METADATA, [])
            .map_err(|e| format!("Failed to create position_metadata table: {}", e))?;

        conn.execute(SCHEMA_TOKEN_SNAPSHOTS, [])
            .map_err(|e| format!("Failed to create token_snapshots table: {}", e))?;

        // Migrate existing database to add PnL fields if needed
        // Check if migration is needed by attempting to add columns
        match conn.execute_batch(MIGRATION_ADD_PNL_FIELDS) {
            Ok(_) => {
                if log_initialization {
                    crate::logger::info(
                        crate::logger::LogTag::Positions,
                        "✅ PnL columns migration completed successfully",
                    );
                }
            }
            Err(e) => {
                let err_msg = e.to_string().to_lowercase();
                // SQLite returns "duplicate column name" if columns already exist - this is OK
                if err_msg.contains("duplicate column") {
                    if log_initialization {
                        crate::logger::debug(
                            crate::logger::LogTag::Positions,
                            "PnL columns already exist, skipping migration",
                        );
                    }
                } else {
                    // Real error - this is critical and should be logged
                    crate::logger::error(
                        crate::logger::LogTag::Positions,
                        &format!("⚠️ CRITICAL: Failed to migrate PnL columns: {}", e),
                    );
                    return Err(format!("Database migration failed: {}", e));
                }
            }
        }

        // Create all indexes
        for index_sql in POSITIONS_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create positions index: {}", e))?;
        }

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO position_metadata (key, value) VALUES ('schema_version', ?1)",
            params![self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set positions schema version: {}", e))?;

        // Run migrations for existing positions (one-time data migration)
        self.run_data_migrations(&conn, log_initialization)?;

        if log_initialization {
            logger::info(
                LogTag::Positions,
                "Positions database schema initialized with all tables and indexes",
            );
        }

        Ok(())
    }

    /// Run data migrations for existing positions
    fn run_data_migrations(&self, conn: &Connection, log: bool) -> Result<(), String> {
        // Migration: Initialize remaining_token_amount and average_entry_price for existing open positions
        // This is a one-time migration for positions created before partial sell/DCA support
        let migration_result = conn.execute(
            r#"
            UPDATE positions 
            SET 
                remaining_token_amount = token_amount,
                average_entry_price = COALESCE(effective_entry_price, entry_price)
            WHERE remaining_token_amount IS NULL 
              AND token_amount IS NOT NULL
              AND exit_time IS NULL
              AND position_type = 'buy'
            "#,
            [],
        );

        match migration_result {
            Ok(updated_count) => {
                if updated_count > 0 && log {
                    crate::logger::warning(
                        crate::logger::LogTag::Positions,
                        &format!(
                            "Migrated {} existing open positions with partial sell/DCA fields",
                            updated_count
                        ),
                    );
                }
            }
            Err(e) => {
                // Non-fatal: columns might not exist yet (fresh install)
                if log {
                    crate::logger::debug(
                        crate::logger::LogTag::Positions,
                        &format!(
                            "Position migration skipped (expected on fresh install): {}",
                            e
                        ),
                    );
                }
            }
        }

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        let conn = self
            .pool
            .get()
            .map_err(|e| format!("Failed to get positions database connection: {}", e))?;

        // Best-effort per-connection configuration to reduce lock contention
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "cache_size", 10000);
        let _ = conn.pragma_update(None, "temp_store", "memory");
        let _ = conn.busy_timeout(std::time::Duration::from_millis(30_000));

        Ok(conn)
    }

    /// Insert new position and return the assigned ID
    pub async fn insert_position(&self, position: &Position) -> Result<i64, String> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Inserting new position for mint {} with entry price {:.6} SOL",
                position.mint, position.entry_price
            ),
        );

        let conn = self.get_connection()?;

        let position_id = conn
            .query_row(
                r#"
            INSERT INTO positions (
                mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
                position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
                entry_transaction_signature, exit_transaction_signature, token_amount,
                effective_entry_price, effective_exit_price, sol_received,
                profit_target_min, profit_target_max, liquidity_tier,
                transaction_entry_verified, transaction_exit_verified,
                entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
                phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
                pnl, pnl_percent, unrealized_pnl, unrealized_pnl_percent,
                remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
                dca_count, average_entry_price, last_dca_time
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31,
                ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42
            ) RETURNING id
            "#,
                params![
                    position.mint,
                    position.symbol,
                    position.name,
                    position.entry_price,
                    position.entry_time.to_rfc3339(),
                    position.exit_price,
                    position.exit_time.map(|t| t.to_rfc3339()),
                    position.position_type,
                    position.entry_size_sol,
                    position.total_size_sol,
                    position.price_highest,
                    position.price_lowest,
                    position.entry_transaction_signature,
                    position.exit_transaction_signature,
                    position.token_amount.map(|t| t as i64),
                    position.effective_entry_price,
                    position.effective_exit_price,
                    position.sol_received,
                    position.profit_target_min,
                    position.profit_target_max,
                    position.liquidity_tier,
                    position.transaction_entry_verified,
                    position.transaction_exit_verified,
                    position.entry_fee_lamports.map(|f| f as i64),
                    position.exit_fee_lamports.map(|f| f as i64),
                    position.current_price,
                    position.current_price_updated.map(|t| t.to_rfc3339()),
                    position.phantom_confirmations as i64,
                    position.phantom_first_seen.map(|t| t.to_rfc3339()),
                    position.synthetic_exit,
                    position.closed_reason,
                    position.pnl,
                    position.pnl_percent,
                    position.unrealized_pnl,
                    position.unrealized_pnl_percent,
                    position.remaining_token_amount.map(|t| t as i64),
                    position.total_exited_amount as i64,
                    position.average_exit_price,
                    position.partial_exit_count as i64,
                    position.dca_count as i64,
                    position.average_entry_price,
                    position.last_dca_time.map(|t| t.to_rfc3339())
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert position: {}", e))?;

        // Record initial state as Open
        self.record_state_change(position_id, PositionState::Open, Some("Position created"))
            .await?;

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully inserted position ID {} for mint {} with entry signature {}",
                position_id,
                position.mint,
                position
                    .entry_transaction_signature
                    .as_deref()
                    .unwrap_or("None")
            ),
        );

        logger::info(
            LogTag::Positions,
            &format!(
                "Inserted new position ID {} for mint {}",
                position_id, position.mint
            ),
        );

        Ok(position_id)
    }

    /// Update existing position by ID
    pub async fn update_position(&self, position: &Position) -> Result<(), String> {
        let position_id = position
            .id
            .ok_or_else(|| "Cannot update position without ID".to_string())?;

        logger::debug(
            LogTag::Positions,
            &format!(
                "Updating position ID {} for mint {} with current price {:.11} SOL",
                position_id,
                position.mint,
                position.current_price.unwrap_or(0.0)
            ),
        );

        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                r#"
            UPDATE positions SET
                mint = ?2, symbol = ?3, name = ?4, entry_price = ?5, entry_time = ?6,
                exit_price = ?7, exit_time = ?8, position_type = ?9, entry_size_sol = ?10,
                total_size_sol = ?11, price_highest = ?12, price_lowest = ?13,
                entry_transaction_signature = ?14, exit_transaction_signature = ?15,
                token_amount = ?16, effective_entry_price = ?17, effective_exit_price = ?18,
                sol_received = ?19, profit_target_min = ?20, profit_target_max = ?21,
                liquidity_tier = ?22, transaction_entry_verified = ?23, transaction_exit_verified = ?24,
                entry_fee_lamports = ?25, exit_fee_lamports = ?26, current_price = ?27,
                current_price_updated = ?28, phantom_confirmations = ?29, phantom_first_seen = ?30,
                synthetic_exit = ?31, closed_reason = ?32,
                pnl = ?33, pnl_percent = ?34, unrealized_pnl = ?35, unrealized_pnl_percent = ?36,
                remaining_token_amount = ?37, total_exited_amount = ?38, average_exit_price = ?39,
                partial_exit_count = ?40, dca_count = ?41, average_entry_price = ?42, last_dca_time = ?43,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
                params![
                    position_id,
                    position.mint,
                    position.symbol,
                    position.name,
                    position.entry_price,
                    position.entry_time.to_rfc3339(),
                    position.exit_price,
                    position.exit_time.map(|t| t.to_rfc3339()),
                    position.position_type,
                    position.entry_size_sol,
                    position.total_size_sol,
                    position.price_highest,
                    position.price_lowest,
                    position.entry_transaction_signature,
                    position.exit_transaction_signature,
                    position.token_amount.map(|t| t as i64),
                    position.effective_entry_price,
                    position.effective_exit_price,
                    position.sol_received,
                    position.profit_target_min,
                    position.profit_target_max,
                    position.liquidity_tier,
                    position.transaction_entry_verified,
                    position.transaction_exit_verified,
                    position.entry_fee_lamports.map(|f| f as i64),
                    position.exit_fee_lamports.map(|f| f as i64),
                    position.current_price,
                    position.current_price_updated.map(|t| t.to_rfc3339()),
                    position.phantom_confirmations as i64,
                    position.phantom_first_seen.map(|t| t.to_rfc3339()),
                    position.synthetic_exit,
                    position.closed_reason,
                    position.pnl,
                    position.pnl_percent,
                    position.unrealized_pnl,
                    position.unrealized_pnl_percent,
                    position.remaining_token_amount.map(|t| t as i64),
                    position.total_exited_amount as i64,
                    position.average_exit_price,
                    position.partial_exit_count as i64,
                    position.dca_count as i64,
                    position.average_entry_price,
                    position.last_dca_time.map(|t| t.to_rfc3339())
                ]
            )
            .map_err(|e| format!("Failed to update position: {}", e))?;

        if rows_affected == 0 {
            return Err(format!("Position with ID {} not found", position_id));
        }

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully updated position ID {} ({} rows affected)",
                position_id, rows_affected
            ),
        );

        // Force WAL checkpoint to ensure all connections see the update immediately
        // This is critical for preventing race conditions in concurrent read operations
        if let Ok(mut stmt) = conn.prepare("PRAGMA wal_checkpoint(PASSIVE);") {
            let _ = stmt.query([]);
        }

        Ok(())
    }

    /// Update only the price-related fields for a position
    pub async fn update_position_prices(
        &self,
        position_id: i64,
        current_price: Option<f64>,
        current_price_updated: Option<DateTime<Utc>>,
        price_highest: f64,
        price_lowest: f64,
    ) -> Result<(), String> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Updating price fields for position ID {} (price={:?}, high={:.11}, low={:.11})",
                position_id, current_price, price_highest, price_lowest
            ),
        );

        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                r#"
            UPDATE positions SET
                current_price = ?2,
                current_price_updated = ?3,
                price_highest = ?4,
                price_lowest = ?5,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
                params![
                    position_id,
                    current_price,
                    current_price_updated.map(|t| t.to_rfc3339()),
                    price_highest,
                    price_lowest
                ],
            )
            .map_err(|e| format!("Failed to update position prices: {}", e))?;

        if rows_affected == 0 {
            return Err(format!(
                "Position with ID {} not found when updating prices",
                position_id
            ));
        }

        Ok(())
    }

    /// Force database synchronization to ensure all connections see recent writes
    /// This should be called after critical updates to prevent race conditions
    pub async fn force_sync(&self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Force WAL checkpoint to synchronize all connections
        // Use prepare and query since PRAGMA wal_checkpoint returns results
        let mut stmt = conn
            .prepare("PRAGMA wal_checkpoint(FULL);")
            .map_err(|e| format!("Failed to prepare WAL checkpoint: {}", e))?;

        let _result = stmt
            .query([])
            .map_err(|e| format!("Failed to execute WAL checkpoint: {}", e))?;

        Ok(())
    }

    pub fn set_metadata_value(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn.execute(
            "INSERT OR REPLACE INTO position_metadata (key, value, updated_at) VALUES (?1, ?2, datetime('now'))",
            params![key, value],
        )
        .map_err(|e| format!("Failed to persist metadata key {}: {}", key, e))?;

        Ok(())
    }

    pub fn get_metadata_value(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare("SELECT value FROM position_metadata WHERE key = ?1 LIMIT 1")
            .map_err(|e| format!("Failed to prepare metadata fetch for key {}: {}", key, e))?;

        let mut rows = stmt
            .query(params![key])
            .map_err(|e| format!("Failed to query metadata for key {}: {}", key, e))?;

        match rows
            .next()
            .map_err(|e| format!("Failed to iterate metadata for key {}: {}", key, e))?
        {
            Some(row) => {
                let value: String = row.get(0).map_err(|e| {
                    format!("Failed to decode metadata payload for key {}: {}", key, e)
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Get position by ID
    pub async fn get_position_by_id(&self, id: i64) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {} FROM positions WHERE id = ?1",
            POSITION_SELECT_COLUMNS
        );
        let result = conn
            .query_row(&query, params![id], |row| self.row_to_position(row))
            .optional()
            .map_err(|e| format!("Failed to get position by ID: {}", e))?;

        Ok(result)
    }

    /// Get position by mint
    pub async fn get_position_by_mint(&self, mint: &str) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {} FROM positions WHERE mint = ?1 ORDER BY entry_time DESC LIMIT 1",
            POSITION_SELECT_COLUMNS
        );
        let result = conn
            .query_row(&query, params![mint], |row| self.row_to_position(row))
            .optional()
            .map_err(|e| format!("Failed to get position by mint: {}", e))?;

        Ok(result)
    }

    /// Get position by entry transaction signature
    pub async fn get_position_by_entry_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT id, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
                   position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
                   entry_transaction_signature, exit_transaction_signature, token_amount,
                   effective_entry_price, effective_exit_price, sol_received,
                   profit_target_min, profit_target_max, liquidity_tier,
                   transaction_entry_verified, transaction_exit_verified,
                   entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
                   phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
                   remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
                   dca_count, average_entry_price, last_dca_time
            FROM positions WHERE entry_transaction_signature = ?1
            "#,
                params![signature],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(|e| format!("Failed to get position by entry signature: {}", e))?;

        Ok(result)
    }

    /// Get position by exit transaction signature
    pub async fn get_position_by_exit_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Position>, String> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {} FROM positions WHERE exit_transaction_signature = ?1",
            POSITION_SELECT_COLUMNS
        );

        let result = conn
            .query_row(&query, params![signature], |row| self.row_to_position(row))
            .optional()
            .map_err(|e| format!("Failed to get position by exit signature: {}", e))?;

        Ok(result)
    }

    /// Get all positions with optional filtering
    pub async fn get_positions(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let mut query = format!(
            "SELECT {} FROM positions ORDER BY entry_time DESC",
            POSITION_SELECT_COLUMNS
        );

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare positions query: {}", e))?;

        let position_iter = stmt
            .query_map([], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute positions query: {}", e))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {}", e))?);
        }

        Ok(positions)
    }

    /// Get open positions (no exit_time)
    pub async fn get_open_positions(&self) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {} FROM positions WHERE transaction_exit_verified = 0 ORDER BY entry_time DESC",
            POSITION_SELECT_COLUMNS
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare open positions query: {}", e))?;

        let position_iter = stmt
            .query_map([], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute open positions query: {}", e))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {}", e))?);
        }

        Ok(positions)
    }

    /// Get closed positions (have exit_time)
    pub async fn get_closed_positions(&self) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {} FROM positions WHERE transaction_exit_verified = 1 ORDER BY exit_time DESC",
            POSITION_SELECT_COLUMNS
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare closed positions query: {}", e))?;

        let position_iter = stmt
            .query_map([], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute closed positions query: {}", e))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {}", e))?);
        }

        Ok(positions)
    }

    /// Count closed positions since the provided timestamp
    pub async fn count_closed_positions_since(&self, since: DateTime<Utc>) -> Result<i64, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT COUNT(1)
            FROM positions
            WHERE transaction_exit_verified = 1
              AND exit_time IS NOT NULL
              AND datetime(exit_time) >= datetime(?1)
            "#,
            )
            .map_err(|e| format!("Failed to prepare closed position count query: {}", e))?;

        let since_str = since.to_rfc3339();
        let count: i64 = stmt
            .query_row(params![since_str], |row| row.get(0))
            .map_err(|e| format!("Failed to execute closed position count query: {}", e))?;

        Ok(count)
    }

    /// Get recent closed & verified positions for a specific mint (exit verified)
    /// Ordered by most recent exit_time DESC. Used for adaptive re-entry profit capping.
    pub async fn get_recent_closed_positions_for_mint(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {} FROM positions WHERE mint = ?1 AND transaction_exit_verified = 1 AND exit_price IS NOT NULL AND exit_time IS NOT NULL ORDER BY exit_time DESC LIMIT ?2",
            POSITION_SELECT_COLUMNS
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare recent closed positions query: {}", e))?;

        let rows = stmt
            .query_map(params![mint, limit as i64], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute recent closed positions query: {}", e))?;

        let mut positions = Vec::new();
        for row in rows {
            if let Ok(p) = row {
                positions.push(p);
            }
        }
        Ok(positions)
    }

    /// Lightweight variant: only fetch (exit_price, effective_exit_price) for recent verified exits
    /// to reduce row size & parsing overhead for re-entry heuristics.
    pub async fn get_recent_closed_exit_prices_for_mint(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<(Option<f64>, Option<f64>)>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                r#"
            SELECT exit_price, effective_exit_price
            FROM positions
            WHERE mint = ?1 AND transaction_exit_verified = 1
              AND exit_price IS NOT NULL AND exit_time IS NOT NULL
            ORDER BY datetime(exit_time) DESC
            LIMIT ?2
            "#,
            )
            .map_err(|e| format!("Failed to prepare recent closed exit prices query: {}", e))?;

        let mut out: Vec<(Option<f64>, Option<f64>)> = Vec::new();
        let rows = stmt
            .query_map(params![mint, limit as i64], |row| {
                let exit_p: Option<f64> = row.get(0).ok();
                let eff_exit_p: Option<f64> = row.get(1).ok();
                Ok((exit_p, eff_exit_p))
            })
            .map_err(|e| format!("Failed to execute recent closed exit prices query: {}", e))?;
        for r in rows {
            if let Ok(v) = r {
                out.push(v);
            }
        }
        Ok(out)
    }

    /// Get positions by state
    pub async fn get_positions_by_state(
        &self,
        state: &PositionState,
    ) -> Result<Vec<Position>, String> {
        // This requires joining with position_states to get current state
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT p.id, p.mint, p.symbol, p.name, p.entry_price, p.entry_time, p.exit_price, p.exit_time,
                   p.position_type, p.entry_size_sol, p.total_size_sol, p.price_highest, p.price_lowest,
                   p.entry_transaction_signature, p.exit_transaction_signature, p.token_amount,
                   p.effective_entry_price, p.effective_exit_price, p.sol_received,
                   p.profit_target_min, p.profit_target_max, p.liquidity_tier,
                   p.transaction_entry_verified, p.transaction_exit_verified,
                   p.entry_fee_lamports, p.exit_fee_lamports, p.current_price, p.current_price_updated,
                   p.phantom_confirmations, p.phantom_first_seen, p.synthetic_exit, p.closed_reason,
                   p.remaining_token_amount, p.total_exited_amount, p.average_exit_price, p.partial_exit_count,
                   p.dca_count, p.average_entry_price, p.last_dca_time
            FROM positions p
            INNER JOIN (
                SELECT position_id, state, MAX(changed_at) as latest_change
                FROM position_states
                GROUP BY position_id
            ) latest_state ON p.id = latest_state.position_id
            WHERE latest_state.state = ?1
            ORDER BY p.entry_time DESC
            "#
            )
            .map_err(|e| format!("Failed to prepare positions by state query: {}", e))?;

        let position_iter = stmt
            .query_map(params![state.to_string()], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute positions by state query: {}", e))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {}", e))?);
        }

        Ok(positions)
    }

    /// Get positions with unverified transactions
    pub async fn get_unverified_positions(&self) -> Result<Vec<Position>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, mint, symbol, name, entry_price, entry_time, exit_price, exit_time,
                   position_type, entry_size_sol, total_size_sol, price_highest, price_lowest,
                   entry_transaction_signature, exit_transaction_signature, token_amount,
                   effective_entry_price, effective_exit_price, sol_received,
                   profit_target_min, profit_target_max, liquidity_tier,
                   transaction_entry_verified, transaction_exit_verified,
                   entry_fee_lamports, exit_fee_lamports, current_price, current_price_updated,
                   phantom_confirmations, phantom_first_seen, synthetic_exit, closed_reason,
                   remaining_token_amount, total_exited_amount, average_exit_price, partial_exit_count,
                   dca_count, average_entry_price, last_dca_time
            FROM positions 
            WHERE transaction_entry_verified = false 
               OR (exit_transaction_signature IS NOT NULL AND transaction_exit_verified = false)
            ORDER BY entry_time DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare unverified positions query: {}", e))?;

        let position_iter = stmt
            .query_map([], |row| self.row_to_position(row))
            .map_err(|e| format!("Failed to execute unverified positions query: {}", e))?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions
                .push(position_result.map_err(|e| format!("Failed to parse position row: {}", e))?);
        }

        Ok(positions)
    }

    /// Delete position by ID
    pub async fn delete_position(&self, id: i64) -> Result<bool, String> {
        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute("DELETE FROM positions WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete position: {}", e))?;

        Ok(rows_affected > 0)
    }

    /// Delete position by entry signature
    pub async fn delete_position_by_entry_signature(
        &self,
        signature: &str,
    ) -> Result<bool, String> {
        let conn = self.get_connection()?;

        let rows_affected = conn
            .execute(
                "DELETE FROM positions WHERE entry_transaction_signature = ?1",
                params![signature],
            )
            .map_err(|e| format!("Failed to delete position by entry signature: {}", e))?;

        if rows_affected > 0 {
            logger::info(
                LogTag::Positions,
                &format!("Deleted position with entry signature: {}", signature),
            );
        }

        Ok(rows_affected > 0)
    }

    /// Record state change for position
    pub async fn record_state_change(
        &self,
        position_id: i64,
        state: PositionState,
        reason: Option<&str>,
    ) -> Result<(), String> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Recording state change for position ID {}: {} (reason: {})",
                position_id,
                state,
                reason.unwrap_or("None")
            ),
        );

        let conn = self.get_connection()?;

        conn.execute(
            "INSERT INTO position_states (position_id, state, reason) VALUES (?1, ?2, ?3)",
            params![position_id, state.to_string(), reason],
        )
        .map_err(|e| format!("Failed to record state change: {}", e))?;

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully recorded state change for position ID {}: {}",
                position_id, state
            ),
        );

        Ok(())
    }

    /// Get position state history
    pub async fn get_position_state_history(
        &self,
        position_id: i64,
    ) -> Result<Vec<PositionStateHistory>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                "SELECT position_id, state, changed_at, reason FROM position_states WHERE position_id = ?1 ORDER BY changed_at DESC"
            )
            .map_err(|e| format!("Failed to prepare state history query: {}", e))?;

        let history_iter = stmt
            .query_map(params![position_id], |row| {
                let state_str: String = row.get(1)?;
                let state = state_str.parse::<PositionState>().map_err(|e| {
                    rusqlite::Error::InvalidColumnType(
                        1,
                        "Invalid state".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?;

                let changed_at_str: String = row.get(2)?;
                let changed_at = DateTime::parse_from_rfc3339(&changed_at_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid datetime".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(PositionStateHistory {
                    position_id: row.get(0)?,
                    state,
                    changed_at,
                    reason: row.get(3)?,
                })
            })
            .map_err(|e| format!("Failed to execute state history query: {}", e))?;

        let mut history = Vec::new();
        for history_result in history_iter {
            history.push(
                history_result.map_err(|e| format!("Failed to parse state history row: {}", e))?,
            );
        }

        Ok(history)
    }

    /// Record position tracking data
    pub async fn record_position_tracking(
        &self,
        tracking: &PositionTracking,
    ) -> Result<(), String> {
        let conn = self.get_connection()?;

        conn
            .execute(
                r#"
            INSERT INTO position_tracking (position_id, price, price_source, pool_type, pool_address, api_price, tracked_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
                params![
                    tracking.position_id,
                    tracking.price,
                    tracking.price_source,
                    tracking.pool_type,
                    tracking.pool_address,
                    tracking.api_price,
                    tracking.tracked_at.to_rfc3339()
                ]
            )
            .map_err(|e| format!("Failed to record position tracking: {}", e))?;

        Ok(())
    }

    /// Get recent position tracking data
    pub async fn get_recent_position_tracking(
        &self,
        position_id: i64,
        limit: usize,
    ) -> Result<Vec<PositionTracking>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT position_id, price, price_source, pool_type, pool_address, api_price, tracked_at
            FROM position_tracking 
            WHERE position_id = ?1 
            ORDER BY tracked_at DESC 
            LIMIT ?2
            "#,
            )
            .map_err(|e| format!("Failed to prepare tracking query: {}", e))?;

        let tracking_iter = stmt
            .query_map(params![position_id, limit], |row| {
                let tracked_at_str: String = row.get(6)?;
                let tracked_at = DateTime::parse_from_rfc3339(&tracked_at_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            6,
                            "Invalid datetime".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(PositionTracking {
                    position_id: row.get(0)?,
                    price: row.get(1)?,
                    price_source: row.get(2)?,
                    pool_type: row.get(3)?,
                    pool_address: row.get(4)?,
                    api_price: row.get(5)?,
                    tracked_at,
                })
            })
            .map_err(|e| format!("Failed to execute tracking query: {}", e))?;

        let mut tracking_data = Vec::new();
        for tracking_result in tracking_iter {
            tracking_data
                .push(tracking_result.map_err(|e| format!("Failed to parse tracking row: {}", e))?);
        }

        Ok(tracking_data)
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<PositionsDatabaseStats, String> {
        let conn = self.get_connection()?;

        let total_positions: i64 = conn
            .query_row("SELECT COUNT(*) FROM positions", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count total positions: {}", e))?;

        let open_positions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM positions WHERE transaction_exit_verified = 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count open positions: {}", e))?;

        let closed_positions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM positions WHERE transaction_exit_verified = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count closed positions: {}", e))?;

        let phantom_positions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM positions WHERE phantom_confirmations > 0",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count phantom positions: {}", e))?;

        let total_state_history: i64 = conn
            .query_row("SELECT COUNT(*) FROM position_states", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count state history: {}", e))?;

        let total_tracking_records: i64 = conn
            .query_row("SELECT COUNT(*) FROM position_tracking", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count tracking records: {}", e))?;

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(PositionsDatabaseStats {
            total_positions: total_positions as u64,
            open_positions: open_positions as u64,
            closed_positions: closed_positions as u64,
            phantom_positions: phantom_positions as u64,
            total_state_history: total_state_history as u64,
            total_tracking_records: total_tracking_records as u64,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Vacuum database to reclaim space and optimize performance
    pub async fn vacuum_database(&self) -> Result<(), String> {
        logger::info(
            LogTag::Positions,
            "Starting positions database vacuum operation...",
        );

        let conn = self.get_connection()?;
        conn.execute("VACUUM", [])
            .map_err(|e| format!("Failed to vacuum positions database: {}", e))?;

        logger::info(
            LogTag::Positions,
            "Positions database vacuum completed successfully",
        );
        Ok(())
    }

    /// Analyze database for query optimization
    pub async fn analyze_database(&self) -> Result<(), String> {
        logger::info(
            LogTag::Positions,
            "Running positions database analysis for optimization...",
        );

        let conn = self.get_connection()?;
        conn.execute("ANALYZE", [])
            .map_err(|e| format!("Failed to analyze positions database: {}", e))?;

        logger::info(
            LogTag::Positions,
            "Positions database analysis completed successfully",
        );
        Ok(())
    }

    /// Save token snapshot to database
    pub async fn save_token_snapshot(&self, snapshot: &TokenSnapshot) -> Result<i64, String> {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Saving token snapshot for position ID {} (type: {}) with price {:.6} SOL",
                snapshot.position_id,
                snapshot.snapshot_type,
                snapshot.price_sol.unwrap_or(0.0)
            ),
        );

        let conn = self.get_connection()?;

        let result = conn
            .execute(
                r#"
            INSERT INTO token_snapshots (
                position_id, snapshot_type, mint, symbol, name, price_sol, price_usd, price_native,
                dex_id, pair_address, pair_url, fdv, market_cap, pair_created_at,
                liquidity_usd, liquidity_base, liquidity_quote,
                volume_h24, volume_h6, volume_h1, volume_m5,
                txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
                txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
                price_change_h24, price_change_h6, price_change_h1, price_change_m5,
                token_uri, token_description, token_image, token_website, token_twitter, token_telegram,
                snapshot_time, api_fetch_time, data_freshness_score
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21,
                ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40,
                ?41, ?42
            )
            "#,
                params![
                    snapshot.position_id,
                    snapshot.snapshot_type,
                    snapshot.mint,
                    snapshot.symbol,
                    snapshot.name,
                    snapshot.price_sol,
                    snapshot.price_usd,
                    snapshot.price_native,
                    snapshot.dex_id,
                    snapshot.pair_address,
                    snapshot.pair_url,
                    snapshot.fdv,
                    snapshot.market_cap,
                    snapshot.pair_created_at,
                    snapshot.liquidity_usd,
                    snapshot.liquidity_base,
                    snapshot.liquidity_quote,
                    snapshot.volume_h24,
                    snapshot.volume_h6,
                    snapshot.volume_h1,
                    snapshot.volume_m5,
                    snapshot.txns_h24_buys,
                    snapshot.txns_h24_sells,
                    snapshot.txns_h6_buys,
                    snapshot.txns_h6_sells,
                    snapshot.txns_h1_buys,
                    snapshot.txns_h1_sells,
                    snapshot.txns_m5_buys,
                    snapshot.txns_m5_sells,
                    snapshot.price_change_h24,
                    snapshot.price_change_h6,
                    snapshot.price_change_h1,
                    snapshot.price_change_m5,
                    snapshot.token_uri,
                    snapshot.token_description,
                    snapshot.token_image,
                    snapshot.token_website,
                    snapshot.token_twitter,
                    snapshot.token_telegram,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.api_fetch_time.to_rfc3339(),
                    snapshot.data_freshness_score
                ]
            )
            .map_err(|e| format!("Failed to insert token snapshot: {}", e))?;

        let snapshot_id = conn.last_insert_rowid();

        logger::debug(
            LogTag::Positions,
            &format!(
                "Successfully saved token snapshot ID {} for position ID {} (type: {})",
                snapshot_id, snapshot.position_id, snapshot.snapshot_type
            ),
        );

        Ok(snapshot_id)
    }

    /// Get token snapshots for a position
    pub async fn get_token_snapshots(
        &self,
        position_id: i64,
    ) -> Result<Vec<TokenSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, position_id, snapshot_type, mint, symbol, name, price_sol, price_usd, price_native,
                   dex_id, pair_address, pair_url, fdv, market_cap, pair_created_at,
                   liquidity_usd, liquidity_base, liquidity_quote,
                   volume_h24, volume_h6, volume_h1, volume_m5,
                   txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
                   txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
                   price_change_h24, price_change_h6, price_change_h1, price_change_m5,
                   token_uri, token_description, token_image, token_website, token_twitter, token_telegram,
                   snapshot_time, api_fetch_time, data_freshness_score
            FROM token_snapshots 
            WHERE position_id = ?1 
            ORDER BY snapshot_time ASC
            "#
            )
            .map_err(|e| format!("Failed to prepare token snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![position_id], |row| self.row_to_token_snapshot(row))
            .map_err(|e| format!("Failed to execute token snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots.push(
                snapshot_result
                    .map_err(|e| format!("Failed to parse token snapshot row: {}", e))?,
            );
        }

        Ok(snapshots)
    }

    /// Get specific token snapshot by type
    pub async fn get_token_snapshot(
        &self,
        position_id: i64,
        snapshot_type: &str,
    ) -> Result<Option<TokenSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, position_id, snapshot_type, mint, symbol, name, price_sol, price_usd, price_native,
                   dex_id, pair_address, pair_url, fdv, market_cap, pair_created_at,
                   liquidity_usd, liquidity_base, liquidity_quote,
                   volume_h24, volume_h6, volume_h1, volume_m5,
                   txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
                   txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
                   price_change_h24, price_change_h6, price_change_h1, price_change_m5,
                   token_uri, token_description, token_image, token_website, token_twitter, token_telegram,
                   snapshot_time, api_fetch_time, data_freshness_score
            FROM token_snapshots 
            WHERE position_id = ?1 AND snapshot_type = ?2
            ORDER BY snapshot_time DESC
            LIMIT 1
            "#
            )
            .map_err(|e| format!("Failed to prepare token snapshot query: {}", e))?;

        let result = stmt
            .query_row(params![position_id, snapshot_type], |row| {
                self.row_to_token_snapshot(row)
            })
            .optional()
            .map_err(|e| format!("Failed to execute token snapshot query: {}", e))?;

        Ok(result)
    }

    /// Helper function to convert database row to TokenSnapshot struct
    fn row_to_token_snapshot(&self, row: &rusqlite::Row) -> rusqlite::Result<TokenSnapshot> {
        let snapshot_time_str: String = row.get("snapshot_time")?;
        let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "Invalid snapshot_time".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        let api_fetch_time_str: String = row.get("api_fetch_time")?;
        let api_fetch_time = DateTime::parse_from_rfc3339(&api_fetch_time_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "Invalid api_fetch_time".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        Ok(TokenSnapshot {
            id: Some(row.get("id")?),
            position_id: row.get("position_id")?,
            snapshot_type: row.get("snapshot_type")?,
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            price_sol: row.get("price_sol")?,
            price_usd: row.get("price_usd")?,
            price_native: row.get("price_native")?,
            dex_id: row.get("dex_id")?,
            pair_address: row.get("pair_address")?,
            pair_url: row.get("pair_url")?,
            fdv: row.get("fdv")?,
            market_cap: row.get("market_cap")?,
            pair_created_at: row.get("pair_created_at")?,
            liquidity_usd: row.get("liquidity_usd")?,
            liquidity_base: row.get("liquidity_base")?,
            liquidity_quote: row.get("liquidity_quote")?,
            volume_h24: row.get("volume_h24")?,
            volume_h6: row.get("volume_h6")?,
            volume_h1: row.get("volume_h1")?,
            volume_m5: row.get("volume_m5")?,
            txns_h24_buys: row.get("txns_h24_buys")?,
            txns_h24_sells: row.get("txns_h24_sells")?,
            txns_h6_buys: row.get("txns_h6_buys")?,
            txns_h6_sells: row.get("txns_h6_sells")?,
            txns_h1_buys: row.get("txns_h1_buys")?,
            txns_h1_sells: row.get("txns_h1_sells")?,
            txns_m5_buys: row.get("txns_m5_buys")?,
            txns_m5_sells: row.get("txns_m5_sells")?,
            price_change_h24: row.get("price_change_h24")?,
            price_change_h6: row.get("price_change_h6")?,
            price_change_h1: row.get("price_change_h1")?,
            price_change_m5: row.get("price_change_m5")?,
            token_uri: row.get("token_uri")?,
            token_description: row.get("token_description")?,
            token_image: row.get("token_image")?,
            token_website: row.get("token_website")?,
            token_twitter: row.get("token_twitter")?,
            token_telegram: row.get("token_telegram")?,
            snapshot_time,
            api_fetch_time,
            data_freshness_score: row.get("data_freshness_score")?,
        })
    }

    /// Helper function to convert database row to Position struct
    fn row_to_position(&self, row: &rusqlite::Row) -> rusqlite::Result<Position> {
        let entry_time_str: String = row.get("entry_time")?;
        let entry_time = DateTime::parse_from_rfc3339(&entry_time_str)
            .map_err(|e| {
                rusqlite::Error::InvalidColumnType(
                    5,
                    "Invalid entry_time".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        let exit_time = if let Some(exit_time_str) = row.get::<_, Option<String>>("exit_time")? {
            Some(
                DateTime::parse_from_rfc3339(&exit_time_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "Invalid exit_time".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        let current_price_updated =
            if let Some(updated_str) = row.get::<_, Option<String>>("current_price_updated")? {
                Some(
                    DateTime::parse_from_rfc3339(&updated_str)
                        .map_err(|e| {
                            rusqlite::Error::InvalidColumnType(
                                27,
                                "Invalid current_price_updated".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?
                        .with_timezone(&Utc),
                )
            } else {
                None
            };

        let phantom_first_seen =
            if let Some(seen_str) = row.get::<_, Option<String>>("phantom_first_seen")? {
                Some(
                    DateTime::parse_from_rfc3339(&seen_str)
                        .map_err(|e| {
                            rusqlite::Error::InvalidColumnType(
                                29,
                                "Invalid phantom_first_seen".to_string(),
                                rusqlite::types::Type::Text,
                            )
                        })?
                        .with_timezone(&Utc),
                )
            } else {
                None
            };

        let last_dca_time = if let Some(dca_str) = row.get::<_, Option<String>>("last_dca_time")? {
            Some(
                DateTime::parse_from_rfc3339(&dca_str)
                    .map_err(|e| {
                        rusqlite::Error::InvalidColumnType(
                            35,
                            "Invalid last_dca_time".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc),
            )
        } else {
            None
        };

        Ok(Position {
            id: Some(row.get("id")?),
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            entry_price: row.get("entry_price")?,
            entry_time,
            exit_price: row.get("exit_price")?,
            exit_time,
            position_type: row.get("position_type")?,
            entry_size_sol: row.get("entry_size_sol")?,
            total_size_sol: row.get("total_size_sol")?,
            price_highest: row.get("price_highest")?,
            price_lowest: row.get("price_lowest")?,
            entry_transaction_signature: row.get("entry_transaction_signature")?,
            exit_transaction_signature: row.get("exit_transaction_signature")?,
            token_amount: row.get::<_, Option<i64>>("token_amount")?.map(|t| t as u64),
            effective_entry_price: row.get("effective_entry_price")?,
            effective_exit_price: row.get("effective_exit_price")?,
            sol_received: row.get("sol_received")?,
            profit_target_min: row.get("profit_target_min")?,
            profit_target_max: row.get("profit_target_max")?,
            liquidity_tier: row.get("liquidity_tier")?,
            transaction_entry_verified: row.get("transaction_entry_verified")?,
            transaction_exit_verified: row.get("transaction_exit_verified")?,
            entry_fee_lamports: row
                .get::<_, Option<i64>>("entry_fee_lamports")?
                .map(|f| f as u64),
            exit_fee_lamports: row
                .get::<_, Option<i64>>("exit_fee_lamports")?
                .map(|f| f as u64),
            current_price: row.get("current_price")?,
            current_price_updated,
            phantom_remove: false, // This is not persisted
            phantom_confirmations: row.get::<_, i64>("phantom_confirmations")? as u32,
            phantom_first_seen,
            synthetic_exit: row.get("synthetic_exit")?,
            closed_reason: row.get("closed_reason")?,
            // Pre-calculated P&L fields
            pnl: row.get::<_, Option<f64>>("pnl").ok().flatten(),
            pnl_percent: row.get::<_, Option<f64>>("pnl_percent").ok().flatten(),
            unrealized_pnl: row.get::<_, Option<f64>>("unrealized_pnl").ok().flatten(),
            unrealized_pnl_percent: row
                .get::<_, Option<f64>>("unrealized_pnl_percent")
                .ok()
                .flatten(),
            // New fields for partial exit and DCA support
            remaining_token_amount: row
                .get::<_, Option<i64>>("remaining_token_amount")?
                .map(|t| t as u64),
            total_exited_amount: row.get::<_, i64>("total_exited_amount")? as u64,
            average_exit_price: row.get("average_exit_price")?,
            partial_exit_count: row.get::<_, i64>("partial_exit_count")? as u32,
            dca_count: row.get::<_, i64>("dca_count")? as u32,
            average_entry_price: row.get("average_entry_price")?,
            last_dca_time,
        })
    }
}

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global positions database instance
static GLOBAL_POSITIONS_DB: Lazy<Arc<Mutex<Option<PositionsDatabase>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global positions database
pub async fn initialize_positions_database() -> Result<(), String> {
    let mut db_lock = GLOBAL_POSITIONS_DB.lock().await;
    if db_lock.is_some() {
        return Ok(()); // Already initialized
    }

    let db = PositionsDatabase::new().await?;
    *db_lock = Some(db);

    logger::info(
        LogTag::Positions,
        "Global positions database initialized successfully",
    );
    Ok(())
}

/// Get reference to global positions database
pub async fn get_positions_database() -> Result<Arc<Mutex<Option<PositionsDatabase>>>, String> {
    Ok(GLOBAL_POSITIONS_DB.clone())
}

/// Execute operation with global database
pub async fn with_positions_database<F, R>(operation: F) -> Result<R, String>
where
    F: FnOnce(&PositionsDatabase) -> Result<R, String>,
{
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => operation(db),
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Execute async operation with global database
pub async fn with_positions_database_async<F, Fut, R>(operation: F) -> Result<R, String>
where
    F: FnOnce(&PositionsDatabase) -> Fut,
    Fut: std::future::Future<Output = Result<R, String>>,
{
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = operation(db).await;
            result
        }
        None => Err("Positions database not initialized".to_string()),
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR POSITIONS MANAGEMENT
// =============================================================================

/// Load all positions from database
pub async fn load_all_positions() -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_positions(None, None).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Save position to database
pub async fn save_position(position: &Position) -> Result<i64, String> {
    logger::debug(
        LogTag::Positions,
        &format!(
            "Saving position for mint {} (ID: {:?}) with entry price {:.6} SOL",
            position.mint, position.id, position.entry_price
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            if let Some(id) = position.id {
                db.update_position(position).await?;
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Updated existing position ID {} for mint {}",
                        id, position.mint
                    ),
                );
                Ok(id)
            } else {
                let new_id = db.insert_position(position).await?;
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Created new position ID {} for mint {}",
                        new_id, position.mint
                    ),
                );
                Ok(new_id)
            }
        }
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Delete position by ID
pub async fn delete_position_by_id(id: i64) -> Result<bool, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.delete_position(id).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Update position in database
pub async fn update_position(position: &Position) -> Result<(), String> {
    logger::debug(
        LogTag::Positions,
        &format!(
            "Updating position ID {:?} for mint {} with current price {:.11} SOL",
            position.id,
            position.mint,
            position.current_price.unwrap_or(0.0)
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = db.update_position(position).await;
            match &result {
                Ok(_) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Successfully updated position ID {:?} for mint {}",
                        position.id, position.mint
                    ),
                ),
                Err(e) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Failed to update position ID {:?} for mint {}: {}",
                        position.id, position.mint, e
                    ),
                ),
            }
            result
        }
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Update only the price-related fields for a position using the latest in-memory state
pub async fn update_position_price_fields(position: &Position) -> Result<(), String> {
    let position_id = position
        .id
        .ok_or_else(|| "Cannot update price fields without position ID".to_string())?;

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            db.update_position_prices(
                position_id,
                position.current_price,
                position.current_price_updated,
                position.price_highest,
                position.price_lowest,
            )
            .await
        }
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Force database synchronization after critical updates
pub async fn force_database_sync() -> Result<(), String> {
    logger::debug(LogTag::Positions, "Forcing database synchronization...");

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = db.force_sync().await;
            match &result {
                Ok(_) => logger::debug(
                    LogTag::Positions,
                    "Database synchronization completed successfully",
                ),
                Err(e) => logger::debug(
                    LogTag::Positions,
                    &format!("Database synchronization failed: {}", e),
                ),
            }
            result
        }
        None => Err("Positions database not initialized".to_string()),
    }
}

pub async fn set_metadata(key: &str, value: &str) -> Result<(), String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.set_metadata_value(key, value),
        None => Err("Positions database not initialized".to_string()),
    }
}

pub async fn get_metadata(key: &str) -> Result<Option<String>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_metadata_value(key),
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get open positions from database
pub async fn get_open_positions() -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_open_positions().await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get closed positions from database
pub async fn get_closed_positions() -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_closed_positions().await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Count closed positions since the provided UTC timestamp
pub async fn get_closed_positions_count_since(since: DateTime<Utc>) -> Result<i64, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.count_closed_positions_since(since).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get position by mint from database
pub async fn get_position_by_mint(mint: &str) -> Result<Option<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_position_by_mint(mint).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get position by ID from database
pub async fn get_position_by_id(id: i64) -> Result<Option<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_position_by_id(id).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Save token snapshot to database
pub async fn save_token_snapshot(snapshot: &TokenSnapshot) -> Result<i64, String> {
    logger::debug(
        LogTag::Positions,
        &format!(
            "Saving token snapshot for position ID {} (type: {}) with mint {}",
            snapshot.position_id, snapshot.snapshot_type, snapshot.mint
        ),
    );

    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            let result = db.save_token_snapshot(snapshot).await;
            match &result {
                Ok(snapshot_id) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Successfully saved token snapshot ID {} for position ID {} (type: {})",
                        snapshot_id, snapshot.position_id, snapshot.snapshot_type
                    ),
                ),
                Err(e) => logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Failed to save token snapshot for position ID {} (type: {}): {}",
                        snapshot.position_id, snapshot.snapshot_type, e
                    ),
                ),
            }
            result
        }
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get token snapshots for a position
pub async fn get_token_snapshots(position_id: i64) -> Result<Vec<TokenSnapshot>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_snapshots(position_id).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get specific token snapshot by type
pub async fn get_token_snapshot(
    position_id: i64,
    snapshot_type: &str,
) -> Result<Option<TokenSnapshot>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_snapshot(position_id, snapshot_type).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

/// Get recent closed positions for a specific mint
pub async fn get_recent_closed_positions_for_mint(
    mint: &str,
    limit: usize,
) -> Result<Vec<Position>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_recent_closed_positions_for_mint(mint, limit).await,
        None => Err("Positions database not initialized".to_string()),
    }
}

// ==================== EXIT/ENTRY HISTORY FUNCTIONS ====================

/// Save an exit record to history
pub async fn save_exit_record(
    position_id: i64,
    timestamp: DateTime<Utc>,
    amount: u64,
    price: f64,
    sol_received: f64,
    transaction_signature: &str,
    is_partial: bool,
    percentage: f64,
    fees_lamports: Option<u64>,
) -> Result<(), String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    conn.execute(
        "INSERT INTO position_exits (position_id, timestamp, amount, price, sol_received, 
         transaction_signature, is_partial, percentage, fees_lamports) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            position_id,
            timestamp.to_rfc3339(),
            amount as i64,
            price,
            sol_received,
            transaction_signature,
            is_partial,
            percentage,
            fees_lamports.map(|f| f as i64),
        ],
    )
    .map_err(|e| format!("Failed to save exit record: {}", e))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "Saved exit record: position={} amount={} partial={} tx={}",
            position_id, amount, is_partial, transaction_signature
        ),
    );

    Ok(())
}

/// Get exit history for a position
pub async fn get_exit_history(position_id: i64) -> Result<Vec<ExitRecord>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, position_id, timestamp, amount, price, sol_received, 
             transaction_signature, is_partial, percentage, fees_lamports 
             FROM position_exits WHERE position_id = ?1 ORDER BY timestamp DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let records = stmt
        .query_map(params![position_id], |row| {
            Ok(ExitRecord {
                id: row.get(0)?,
                position_id: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                amount: row.get::<_, i64>(3)? as u64,
                price: row.get(4)?,
                sol_received: row.get(5)?,
                transaction_signature: row.get(6)?,
                is_partial: row.get(7)?,
                percentage: row.get(8)?,
                fees_lamports: row.get::<_, Option<i64>>(9)?.map(|f| f as u64),
            })
        })
        .map_err(|e| format!("Failed to query exit records: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect exit records: {}", e))?;

    Ok(records)
}

/// Save an entry record to history
pub async fn save_entry_record(
    position_id: i64,
    timestamp: DateTime<Utc>,
    amount: u64,
    price: f64,
    sol_spent: f64,
    transaction_signature: &str,
    is_dca: bool,
    fees_lamports: Option<u64>,
) -> Result<(), String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    conn.execute(
        "INSERT INTO position_entries (position_id, timestamp, amount, price, sol_spent, 
         transaction_signature, is_dca, fees_lamports) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            position_id,
            timestamp.to_rfc3339(),
            amount as i64,
            price,
            sol_spent,
            transaction_signature,
            is_dca,
            fees_lamports.map(|f| f as i64),
        ],
    )
    .map_err(|e| format!("Failed to save entry record: {}", e))?;

    logger::info(
        LogTag::Positions,
        &format!(
            "Saved entry record: position={} amount={} dca={} tx={}",
            position_id, amount, is_dca, transaction_signature
        ),
    );

    Ok(())
}

/// Get entry history for a position
pub async fn get_entry_history(position_id: i64) -> Result<Vec<EntryRecord>, String> {
    let db_guard = GLOBAL_POSITIONS_DB.lock().await;
    let db = db_guard
        .as_ref()
        .ok_or("Positions database not initialized")?;

    let conn = db
        .pool
        .get()
        .map_err(|e| format!("Failed to get connection: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, position_id, timestamp, amount, price, sol_spent, 
             transaction_signature, is_dca, fees_lamports 
             FROM position_entries WHERE position_id = ?1 ORDER BY timestamp ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let records = stmt
        .query_map(params![position_id], |row| {
            Ok(EntryRecord {
                id: row.get(0)?,
                position_id: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                amount: row.get::<_, i64>(3)? as u64,
                price: row.get(4)?,
                sol_spent: row.get(5)?,
                transaction_signature: row.get(6)?,
                is_dca: row.get(7)?,
                fees_lamports: row.get::<_, Option<i64>>(8)?.map(|f| f as u64),
            })
        })
        .map_err(|e| format!("Failed to query entry records: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect entry records: {}", e))?;

    Ok(records)
}
