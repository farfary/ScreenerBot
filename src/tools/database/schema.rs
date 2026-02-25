//! Database schema definitions and initialization

use chrono::Utc;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::Duration;

use crate::database;
use crate::logger::{self, LogTag};
use crate::paths::get_tools_db_path;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Schema version for migrations
const TOOLS_SCHEMA_VERSION: u32 = 1;

/// Connection pool configuration
const POOL_MAX_SIZE: u32 = 3;
const POOL_MIN_IDLE: u32 = 1;
const CONNECTION_TIMEOUT_MS: u64 = 30_000;

/// Database initialization flag
static TOOLS_DB_INITIALIZED: AtomicBool = AtomicBool::new(false);

// =============================================================================
// SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
"#;

/// ATA cleanup sessions table
const SCHEMA_ATA_SESSIONS: &str = r#"
CREATE TABLE IF NOT EXISTS ata_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    wallet_address TEXT NOT NULL,
    
    -- Target configuration
    target_count INTEGER,
    
    -- Status
    status TEXT NOT NULL DEFAULT 'ready',
    started_at TEXT,
    ended_at TEXT,
    error_message TEXT,
    
    -- Metrics
    total_atas_found INTEGER NOT NULL DEFAULT 0,
    successful_closures INTEGER NOT NULL DEFAULT 0,
    failed_closures INTEGER NOT NULL DEFAULT 0,
    sol_recovered REAL NOT NULL DEFAULT 0,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ata_sessions_session_id ON ata_sessions(session_id);
CREATE INDEX IF NOT EXISTS idx_ata_sessions_wallet ON ata_sessions(wallet_address);
CREATE INDEX IF NOT EXISTS idx_ata_sessions_status ON ata_sessions(status);
"#;

/// ATA closures table
const SCHEMA_ATA_CLOSURES: &str = r#"
CREATE TABLE IF NOT EXISTS ata_closures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    ata_address TEXT NOT NULL,
    token_mint TEXT NOT NULL,
    
    -- Transaction details
    signature TEXT,
    sol_recovered REAL NOT NULL DEFAULT 0,
    
    -- Status
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    executed_at TEXT,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (session_id) REFERENCES ata_sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ata_closures_session_id ON ata_closures(session_id);
CREATE INDEX IF NOT EXISTS idx_ata_closures_ata_address ON ata_closures(ata_address);
CREATE INDEX IF NOT EXISTS idx_ata_closures_status ON ata_closures(status);
"#;

/// ATA failed cache table (replaces JSON file)
const SCHEMA_ATA_FAILED_CACHE: &str = r#"
CREATE TABLE IF NOT EXISTS ata_failed_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ata_address TEXT NOT NULL UNIQUE,
    token_mint TEXT,
    wallet_address TEXT NOT NULL,
    
    -- Failure tracking
    failure_count INTEGER NOT NULL DEFAULT 1,
    last_error TEXT,
    first_failed_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_failed_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    -- Retry tracking
    next_retry_at TEXT,
    is_permanent_failure INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_ata_failed_ata_address ON ata_failed_cache(ata_address);
CREATE INDEX IF NOT EXISTS idx_ata_failed_wallet ON ata_failed_cache(wallet_address);
CREATE INDEX IF NOT EXISTS idx_ata_failed_permanent ON ata_failed_cache(is_permanent_failure);
CREATE INDEX IF NOT EXISTS idx_ata_failed_next_retry ON ata_failed_cache(next_retry_at);
"#;

/// Multi-wallet sessions table
const SCHEMA_MW_SESSIONS: &str = r#"
CREATE TABLE IF NOT EXISTS mw_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    session_type TEXT NOT NULL,
    token_mint TEXT,
    
    -- Configuration
    total_wallets INTEGER NOT NULL DEFAULT 0,
    target_amount_sol REAL,
    min_amount_sol REAL,
    max_amount_sol REAL,
    delay_ms INTEGER NOT NULL DEFAULT 1000,
    delay_max_ms INTEGER,
    concurrency INTEGER NOT NULL DEFAULT 1,
    sol_buffer REAL NOT NULL DEFAULT 0.015,
    
    -- Status tracking
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT,
    ended_at TEXT,
    error_message TEXT,
    
    -- Metrics
    wallets_funded INTEGER NOT NULL DEFAULT 0,
    successful_ops INTEGER NOT NULL DEFAULT 0,
    failed_ops INTEGER NOT NULL DEFAULT 0,
    total_sol_spent REAL NOT NULL DEFAULT 0,
    total_sol_recovered REAL NOT NULL DEFAULT 0,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_mw_sessions_session_id ON mw_sessions(session_id);
CREATE INDEX IF NOT EXISTS idx_mw_sessions_type ON mw_sessions(session_type);
CREATE INDEX IF NOT EXISTS idx_mw_sessions_status ON mw_sessions(status);
"#;

/// Multi-wallet individual operations table
const SCHEMA_MW_WALLET_OPS: &str = r#"
CREATE TABLE IF NOT EXISTS mw_wallet_ops (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    wallet_id INTEGER NOT NULL,
    wallet_address TEXT NOT NULL,
    op_index INTEGER NOT NULL,
    
    -- Operation details
    op_type TEXT NOT NULL,
    amount_sol REAL,
    token_amount REAL,
    signature TEXT,
    
    -- Status
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    executed_at TEXT,
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (session_id) REFERENCES mw_sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_mw_wallet_ops_session ON mw_wallet_ops(session_id);
CREATE INDEX IF NOT EXISTS idx_mw_wallet_ops_wallet ON mw_wallet_ops(wallet_id);
"#;

/// Tool favorites table
const SCHEMA_TOOL_FAVORITES: &str = r#"
CREATE TABLE IF NOT EXISTS tool_favorites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Token identification
    mint TEXT NOT NULL,
    symbol TEXT,
    name TEXT,
    logo_url TEXT,
    
    -- Tool context
    tool_type TEXT NOT NULL,
    
    -- Custom configuration (JSON)
    config_json TEXT,
    
    -- User metadata
    label TEXT,
    notes TEXT,
    
    -- Usage tracking
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    UNIQUE(mint, tool_type)
);

CREATE INDEX IF NOT EXISTS idx_tool_favorites_mint ON tool_favorites(mint);
CREATE INDEX IF NOT EXISTS idx_tool_favorites_tool_type ON tool_favorites(tool_type);
CREATE INDEX IF NOT EXISTS idx_tool_favorites_use_count ON tool_favorites(use_count DESC);
"#;

/// Watched tokens table for copy trading / sniper functionality
const SCHEMA_WATCHED_TOKENS: &str = r#"
CREATE TABLE IF NOT EXISTS watched_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    symbol TEXT,
    pool_address TEXT NOT NULL,
    pool_source TEXT NOT NULL,
    pool_dex TEXT,
    pool_pair TEXT,
    pool_liquidity REAL,
    
    -- Watch configuration
    watch_type TEXT NOT NULL,
    trigger_amount_sol REAL,
    action_amount_sol REAL,
    slippage_bps INTEGER DEFAULT 500,
    is_active INTEGER NOT NULL DEFAULT 1,
    
    -- Tracking
    last_checked_at TEXT,
    last_trade_signature TEXT,
    trades_detected INTEGER DEFAULT 0,
    actions_triggered INTEGER DEFAULT 0,
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_watched_tokens_mint ON watched_tokens(mint);
CREATE INDEX IF NOT EXISTS idx_watched_tokens_active ON watched_tokens(is_active);
"#;

// =============================================================================
// CONNECTION POOL
// =============================================================================

/// Global connection pool for tools database
static DB_POOL: LazyLock<Pool<SqliteConnectionManager>> = LazyLock::new(|| {
    let db_path = get_tools_db_path();

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let manager = SqliteConnectionManager::file(&db_path)
        .with_init(|c| database::configure_connection(c, database::TOOLS_DB));
    Pool::builder()
        .max_size(POOL_MAX_SIZE)
        .min_idle(Some(POOL_MIN_IDLE))
        .connection_timeout(Duration::from_millis(CONNECTION_TIMEOUT_MS))
        .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
        .max_lifetime(None) // SQLite: no connection recycling
        .build(manager)
        .expect("Failed to create tools database pool")
});

/// Get a connection from the pool
pub(crate) fn get_connection() -> Result<PooledConnection<SqliteConnectionManager>, String> {
    DB_POOL
        .get()
        .map_err(|e| format!("Failed to get tools database connection: {e}"))
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize the tools database with all schemas
pub fn init_tools_db() -> Result<(), String> {
    if TOOLS_DB_INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let conn = get_connection()?;

    // Create version table first
    conn.execute_batch(SCHEMA_VERSION_TABLE)
        .map_err(|e| format!("Failed to create version table: {e}"))?;

    // Check current schema version
    let current_version: Option<u32> = conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to check schema version: {e}"))?;

    if current_version.unwrap_or(0) < TOOLS_SCHEMA_VERSION {
        // Create all tables
        conn.execute_batch(SCHEMA_ATA_SESSIONS)
            .map_err(|e| format!("Failed to create ata_sessions table: {e}"))?;

        conn.execute_batch(SCHEMA_ATA_CLOSURES)
            .map_err(|e| format!("Failed to create ata_closures table: {e}"))?;

        conn.execute_batch(SCHEMA_ATA_FAILED_CACHE)
            .map_err(|e| format!("Failed to create ata_failed_cache table: {e}"))?;

        conn.execute_batch(SCHEMA_TOOL_FAVORITES)
            .map_err(|e| format!("Failed to create tool_favorites table: {e}"))?;

        conn.execute_batch(SCHEMA_MW_SESSIONS)
            .map_err(|e| format!("Failed to create mw_sessions table: {e}"))?;

        conn.execute_batch(SCHEMA_MW_WALLET_OPS)
            .map_err(|e| format!("Failed to create mw_wallet_ops table: {e}"))?;

        conn.execute_batch(SCHEMA_WATCHED_TOKENS)
            .map_err(|e| format!("Failed to create watched_tokens table: {e}"))?;

        // Update version
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![TOOLS_SCHEMA_VERSION, Utc::now().to_rfc3339()],
        )
        .map_err(|e| format!("Failed to update schema version: {e}"))?;

        logger::info(
            LogTag::System,
            &format!(
                "Tools database initialized at {} (schema v{})",
                get_tools_db_path().display(),
                TOOLS_SCHEMA_VERSION
            ),
        );
    }

    TOOLS_DB_INITIALIZED.store(true, Ordering::SeqCst);
    Ok(())
}
