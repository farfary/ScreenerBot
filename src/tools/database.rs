//! Tools Database Module
//!
//! SQLite database for persistent storage of tool operations:
//! - Volume Aggregator sessions and swaps
//! - ATA cleanup sessions and closures
//! - Failed ATA cache

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::logger::{self, LogTag};
use crate::paths::get_tools_db_path;

use super::types::{DelayConfig, DistributionStrategy, SizingConfig, ToolStatus, WalletMode};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Schema version for migrations
const TOOLS_SCHEMA_VERSION: u32 = 1;

/// Connection pool configuration
const POOL_MAX_SIZE: u32 = 10;
const POOL_MIN_IDLE: u32 = 2;
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

/// Volume Aggregator sessions table
const SCHEMA_VA_SESSIONS: &str = r#"
CREATE TABLE IF NOT EXISTS va_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    token_mint TEXT NOT NULL,
    target_volume_sol REAL NOT NULL,
    actual_volume_sol REAL NOT NULL DEFAULT 0,
    
    -- Delay configuration
    delay_type TEXT NOT NULL DEFAULT 'fixed',
    delay_ms INTEGER NOT NULL DEFAULT 1000,
    delay_max_ms INTEGER,
    
    -- Sizing configuration
    sizing_type TEXT NOT NULL DEFAULT 'fixed',
    amount_sol REAL NOT NULL DEFAULT 0.01,
    amount_max_sol REAL,
    
    -- Strategy configuration
    strategy TEXT NOT NULL DEFAULT 'round_robin',
    wallet_mode TEXT NOT NULL DEFAULT 'auto_select',
    wallet_addresses TEXT,
    
    -- Status tracking
    status TEXT NOT NULL DEFAULT 'ready',
    started_at TEXT,
    ended_at TEXT,
    error_message TEXT,
    
    -- Metrics
    successful_buys INTEGER NOT NULL DEFAULT 0,
    successful_sells INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_va_sessions_session_id ON va_sessions(session_id);
CREATE INDEX IF NOT EXISTS idx_va_sessions_token_mint ON va_sessions(token_mint);
CREATE INDEX IF NOT EXISTS idx_va_sessions_status ON va_sessions(status);
CREATE INDEX IF NOT EXISTS idx_va_sessions_created ON va_sessions(created_at);
"#;

/// Volume Aggregator swaps table
const SCHEMA_VA_SWAPS: &str = r#"
CREATE TABLE IF NOT EXISTS va_swaps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    tx_index INTEGER NOT NULL,
    wallet_address TEXT NOT NULL,
    
    -- Transaction details
    is_buy INTEGER NOT NULL,
    amount_sol REAL NOT NULL,
    token_amount REAL,
    signature TEXT,
    
    -- Status
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    executed_at TEXT,
    
    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (session_id) REFERENCES va_sessions(session_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_va_swaps_session_id ON va_swaps(session_id);
CREATE INDEX IF NOT EXISTS idx_va_swaps_wallet ON va_swaps(wallet_address);
CREATE INDEX IF NOT EXISTS idx_va_swaps_status ON va_swaps(status);
CREATE INDEX IF NOT EXISTS idx_va_swaps_executed ON va_swaps(executed_at);
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

// =============================================================================
// CONNECTION POOL
// =============================================================================

/// Global connection pool for tools database
static DB_POOL: Lazy<Pool<SqliteConnectionManager>> = Lazy::new(|| {
    let db_path = get_tools_db_path();

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let manager = SqliteConnectionManager::file(&db_path);
    Pool::builder()
        .max_size(POOL_MAX_SIZE)
        .min_idle(Some(POOL_MIN_IDLE))
        .connection_timeout(std::time::Duration::from_millis(CONNECTION_TIMEOUT_MS))
        .build(manager)
        .expect("Failed to create tools database pool")
});

/// Get a connection from the pool
pub fn get_connection() -> Result<PooledConnection<SqliteConnectionManager>, String> {
    DB_POOL
        .get()
        .map_err(|e| format!("Failed to get tools database connection: {}", e))
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

    // Enable WAL mode for better concurrency
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA cache_size = 10000;
        PRAGMA temp_store = memory;
        PRAGMA busy_timeout = 30000;
        PRAGMA foreign_keys = ON;
    ",
    )
    .map_err(|e| format!("Failed to set pragmas: {}", e))?;

    // Create version table first
    conn.execute_batch(SCHEMA_VERSION_TABLE)
        .map_err(|e| format!("Failed to create version table: {}", e))?;

    // Check current schema version
    let current_version: Option<u32> = conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to check schema version: {}", e))?;

    if current_version.is_none() || current_version.unwrap() < TOOLS_SCHEMA_VERSION {
        // Create all tables
        conn.execute_batch(SCHEMA_VA_SESSIONS)
            .map_err(|e| format!("Failed to create va_sessions table: {}", e))?;

        conn.execute_batch(SCHEMA_VA_SWAPS)
            .map_err(|e| format!("Failed to create va_swaps table: {}", e))?;

        conn.execute_batch(SCHEMA_ATA_SESSIONS)
            .map_err(|e| format!("Failed to create ata_sessions table: {}", e))?;

        conn.execute_batch(SCHEMA_ATA_CLOSURES)
            .map_err(|e| format!("Failed to create ata_closures table: {}", e))?;

        conn.execute_batch(SCHEMA_ATA_FAILED_CACHE)
            .map_err(|e| format!("Failed to create ata_failed_cache table: {}", e))?;

        conn.execute_batch(SCHEMA_TOOL_FAVORITES)
            .map_err(|e| format!("Failed to create tool_favorites table: {}", e))?;

        // Update version
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![TOOLS_SCHEMA_VERSION, Utc::now().to_rfc3339()],
        )
        .map_err(|e| format!("Failed to update schema version: {}", e))?;

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

// =============================================================================
// VOLUME AGGREGATOR SESSION OPERATIONS
// =============================================================================

/// Insert a new VA session
pub fn insert_va_session(
    session_id: &str,
    token_mint: &str,
    target_volume_sol: f64,
    delay_config: &DelayConfig,
    sizing_config: &SizingConfig,
    strategy: &DistributionStrategy,
    wallet_mode: &WalletMode,
    wallet_addresses: Option<&[String]>,
) -> Result<i64, String> {
    let conn = get_connection()?;

    let (delay_type, delay_ms, delay_max_ms) = delay_config.to_db_values();
    let (sizing_type, amount_sol, amount_max_sol) = sizing_config.to_db_values();
    let strategy_str = strategy.to_db_value();
    let wallet_mode_str = wallet_mode.to_db_value();
    let wallet_addresses_json = wallet_addresses.map(|addrs| serde_json::to_string(addrs).ok()).flatten();

    conn.execute(
        r#"
        INSERT INTO va_sessions (
            session_id, token_mint, target_volume_sol,
            delay_type, delay_ms, delay_max_ms,
            sizing_type, amount_sol, amount_max_sol,
            strategy, wallet_mode, wallet_addresses,
            status
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            session_id,
            token_mint,
            target_volume_sol,
            delay_type,
            delay_ms,
            delay_max_ms,
            sizing_type,
            amount_sol,
            amount_max_sol,
            strategy_str,
            wallet_mode_str,
            wallet_addresses_json,
            ToolStatus::Ready.to_string(),
        ],
    )
    .map_err(|e| format!("Failed to insert VA session: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// Update VA session status
pub fn update_va_session_status(
    session_id: &str,
    status: &ToolStatus,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    let started_at = if *status == ToolStatus::Running {
        Some(now.clone())
    } else {
        None
    };

    let ended_at = if matches!(status, ToolStatus::Completed | ToolStatus::Failed | ToolStatus::Aborted) {
        Some(now.clone())
    } else {
        None
    };

    conn.execute(
        r#"
        UPDATE va_sessions 
        SET status = ?1, 
            error_message = ?2,
            started_at = COALESCE(?3, started_at),
            ended_at = COALESCE(?4, ended_at),
            updated_at = ?5
        WHERE session_id = ?6
        "#,
        params![
            status.to_string(),
            error_message,
            started_at,
            ended_at,
            now,
            session_id,
        ],
    )
    .map_err(|e| format!("Failed to update VA session status: {}", e))?;

    Ok(())
}

/// Update VA session metrics
pub fn update_va_session_metrics(
    session_id: &str,
    actual_volume_sol: f64,
    successful_buys: i32,
    successful_sells: i32,
    failed_count: i32,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE va_sessions 
        SET actual_volume_sol = ?1,
            successful_buys = ?2,
            successful_sells = ?3,
            failed_count = ?4,
            updated_at = ?5
        WHERE session_id = ?6
        "#,
        params![
            actual_volume_sol,
            successful_buys,
            successful_sells,
            failed_count,
            now,
            session_id,
        ],
    )
    .map_err(|e| format!("Failed to update VA session metrics: {}", e))?;

    Ok(())
}

/// Get VA session by session_id
pub fn get_va_session(session_id: &str) -> Result<Option<VaSessionRow>, String> {
    let conn = get_connection()?;

    conn.query_row(
        r#"
        SELECT id, session_id, token_mint, target_volume_sol, actual_volume_sol,
               delay_type, delay_ms, delay_max_ms,
               sizing_type, amount_sol, amount_max_sol,
               strategy, wallet_mode, wallet_addresses,
               status, started_at, ended_at, error_message,
               successful_buys, successful_sells, failed_count,
               created_at, updated_at
        FROM va_sessions WHERE session_id = ?1
        "#,
        params![session_id],
        |row| Ok(VaSessionRow::from_row(row)),
    )
    .optional()
    .map_err(|e| format!("Failed to get VA session: {}", e))?
    .transpose()
}

/// Get recent VA sessions
pub fn get_recent_va_sessions(limit: i32) -> Result<Vec<VaSessionRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, session_id, token_mint, target_volume_sol, actual_volume_sol,
                   delay_type, delay_ms, delay_max_ms,
                   sizing_type, amount_sol, amount_max_sol,
                   strategy, wallet_mode, wallet_addresses,
                   status, started_at, ended_at, error_message,
                   successful_buys, successful_sells, failed_count,
                   created_at, updated_at
            FROM va_sessions 
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![limit], |row| Ok(VaSessionRow::from_row(row)))
        .map_err(|e| format!("Failed to query sessions: {}", e))?;

    let mut sessions = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(session)) => sessions.push(session),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(format!("Failed to read row: {}", e)),
        }
    }

    Ok(sessions)
}

/// Get VA session analytics summary
pub fn get_va_sessions_analytics() -> Result<VaAnalyticsSummary, String> {
    let conn = get_connection()?;

    conn.query_row(
        r#"
        SELECT 
            COUNT(*) as total_sessions,
            COALESCE(SUM(actual_volume_sol), 0) as total_volume_sol,
            COALESCE(AVG(
                CASE WHEN (successful_buys + successful_sells + failed_count) > 0 
                THEN CAST(successful_buys + successful_sells AS REAL) / 
                     (successful_buys + successful_sells + failed_count) * 100
                ELSE 0 END
            ), 0) as avg_success_rate,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed_sessions,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed_sessions,
            COUNT(CASE WHEN status = 'aborted' THEN 1 END) as aborted_sessions
        FROM va_sessions
        "#,
        [],
        |row| {
            Ok(VaAnalyticsSummary {
                total_sessions: row.get(0)?,
                total_volume_sol: row.get(1)?,
                avg_success_rate: row.get(2)?,
                completed_sessions: row.get(3)?,
                failed_sessions: row.get(4)?,
                aborted_sessions: row.get(5)?,
            })
        },
    )
    .map_err(|e| format!("Failed to get VA analytics: {}", e))
}

// =============================================================================
// VOLUME AGGREGATOR SWAP OPERATIONS
// =============================================================================

/// Insert a new VA swap
pub fn insert_va_swap(
    session_id: &str,
    tx_index: i32,
    wallet_address: &str,
    is_buy: bool,
    amount_sol: f64,
) -> Result<i64, String> {
    let conn = get_connection()?;

    conn.execute(
        r#"
        INSERT INTO va_swaps (session_id, tx_index, wallet_address, is_buy, amount_sol, status)
        VALUES (?1, ?2, ?3, ?4, ?5, 'pending')
        "#,
        params![session_id, tx_index, wallet_address, is_buy as i32, amount_sol],
    )
    .map_err(|e| format!("Failed to insert VA swap: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// Update VA swap result
pub fn update_va_swap_result(
    id: i64,
    signature: Option<&str>,
    token_amount: Option<f64>,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE va_swaps 
        SET signature = ?1,
            token_amount = ?2,
            status = ?3,
            error_message = ?4,
            executed_at = ?5
        WHERE id = ?6
        "#,
        params![signature, token_amount, status, error_message, now, id],
    )
    .map_err(|e| format!("Failed to update VA swap: {}", e))?;

    Ok(())
}

/// Get swaps for a session
pub fn get_va_swaps(session_id: &str) -> Result<Vec<VaSwapRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, session_id, tx_index, wallet_address,
                   is_buy, amount_sol, token_amount, signature,
                   status, error_message, executed_at, created_at
            FROM va_swaps 
            WHERE session_id = ?1
            ORDER BY tx_index ASC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![session_id], |row| Ok(VaSwapRow::from_row(row)))
        .map_err(|e| format!("Failed to query swaps: {}", e))?;

    let mut swaps = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(swap)) => swaps.push(swap),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(format!("Failed to read row: {}", e)),
        }
    }

    Ok(swaps)
}

// =============================================================================
// ATA FAILED CACHE OPERATIONS
// =============================================================================

/// Add or update failed ATA entry
pub fn upsert_failed_ata(
    ata_address: &str,
    token_mint: Option<&str>,
    wallet_address: &str,
    error: &str,
    is_permanent: bool,
) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO ata_failed_cache (ata_address, token_mint, wallet_address, last_error, is_permanent_failure, last_failed_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(ata_address) DO UPDATE SET
            failure_count = failure_count + 1,
            last_error = ?4,
            is_permanent_failure = ?5,
            last_failed_at = ?6
        "#,
        params![ata_address, token_mint, wallet_address, error, is_permanent as i32, now],
    )
    .map_err(|e| format!("Failed to upsert failed ATA: {}", e))?;

    Ok(())
}

/// Check if ATA is in failed cache
pub fn is_ata_failed(ata_address: &str) -> Result<bool, String> {
    let conn = get_connection()?;

    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM ata_failed_cache WHERE ata_address = ?1",
            params![ata_address],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check ATA: {}", e))?;

    Ok(count > 0)
}

/// Get all failed ATAs for a wallet
pub fn get_failed_atas_for_wallet(wallet_address: &str) -> Result<Vec<FailedAtaRow>, String> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT ata_address, token_mint, wallet_address,
                   failure_count, last_error, first_failed_at, last_failed_at,
                   next_retry_at, is_permanent_failure
            FROM ata_failed_cache 
            WHERE wallet_address = ?1
            ORDER BY last_failed_at DESC
            "#,
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(params![wallet_address], |row| Ok(FailedAtaRow::from_row(row)))
        .map_err(|e| format!("Failed to query failed ATAs: {}", e))?;

    let mut atas = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(ata)) => atas.push(ata),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(format!("Failed to read row: {}", e)),
        }
    }

    Ok(atas)
}

/// Remove ATA from failed cache
pub fn remove_failed_ata(ata_address: &str) -> Result<(), String> {
    let conn = get_connection()?;

    conn.execute(
        "DELETE FROM ata_failed_cache WHERE ata_address = ?1",
        params![ata_address],
    )
    .map_err(|e| format!("Failed to remove failed ATA: {}", e))?;

    Ok(())
}

/// Clear all non-permanent failed ATAs older than specified days
pub fn cleanup_old_failed_atas(max_age_days: i32) -> Result<i32, String> {
    let conn = get_connection()?;

    let deleted = conn
        .execute(
            r#"
            DELETE FROM ata_failed_cache 
            WHERE is_permanent_failure = 0 
              AND last_failed_at < datetime('now', '-' || ?1 || ' days')
            "#,
            params![max_age_days],
        )
        .map_err(|e| format!("Failed to cleanup old failed ATAs: {}", e))?;

    Ok(deleted as i32)
}

// =============================================================================
// ROW TYPES
// =============================================================================

/// VA session database row
#[derive(Debug, Clone)]
pub struct VaSessionRow {
    pub id: i64,
    pub session_id: String,
    pub token_mint: String,
    pub target_volume_sol: f64,
    pub actual_volume_sol: f64,
    pub delay_type: String,
    pub delay_ms: i64,
    pub delay_max_ms: Option<i64>,
    pub sizing_type: String,
    pub amount_sol: f64,
    pub amount_max_sol: Option<f64>,
    pub strategy: String,
    pub wallet_mode: String,
    pub wallet_addresses: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub error_message: Option<String>,
    pub successful_buys: i32,
    pub successful_sells: i32,
    pub failed_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl VaSessionRow {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            session_id: row.get(1).map_err(|e| e.to_string())?,
            token_mint: row.get(2).map_err(|e| e.to_string())?,
            target_volume_sol: row.get(3).map_err(|e| e.to_string())?,
            actual_volume_sol: row.get(4).map_err(|e| e.to_string())?,
            delay_type: row.get(5).map_err(|e| e.to_string())?,
            delay_ms: row.get(6).map_err(|e| e.to_string())?,
            delay_max_ms: row.get(7).map_err(|e| e.to_string())?,
            sizing_type: row.get(8).map_err(|e| e.to_string())?,
            amount_sol: row.get(9).map_err(|e| e.to_string())?,
            amount_max_sol: row.get(10).map_err(|e| e.to_string())?,
            strategy: row.get(11).map_err(|e| e.to_string())?,
            wallet_mode: row.get(12).map_err(|e| e.to_string())?,
            wallet_addresses: row.get(13).map_err(|e| e.to_string())?,
            status: row.get(14).map_err(|e| e.to_string())?,
            started_at: row.get(15).map_err(|e| e.to_string())?,
            ended_at: row.get(16).map_err(|e| e.to_string())?,
            error_message: row.get(17).map_err(|e| e.to_string())?,
            successful_buys: row.get(18).map_err(|e| e.to_string())?,
            successful_sells: row.get(19).map_err(|e| e.to_string())?,
            failed_count: row.get(20).map_err(|e| e.to_string())?,
            created_at: row.get(21).map_err(|e| e.to_string())?,
            updated_at: row.get(22).map_err(|e| e.to_string())?,
        })
    }

    /// Parse delay config from row data
    pub fn get_delay_config(&self) -> DelayConfig {
        DelayConfig::from_db_values(&self.delay_type, self.delay_ms, self.delay_max_ms)
    }

    /// Parse sizing config from row data
    pub fn get_sizing_config(&self) -> SizingConfig {
        SizingConfig::from_db_values(&self.sizing_type, self.amount_sol, self.amount_max_sol)
    }

    /// Parse strategy from row data
    pub fn get_strategy(&self) -> DistributionStrategy {
        DistributionStrategy::from_db_value(&self.strategy)
    }

    /// Parse wallet mode from row data
    pub fn get_wallet_mode(&self) -> WalletMode {
        WalletMode::from_db_value(&self.wallet_mode)
    }

    /// Parse wallet addresses from JSON
    pub fn get_wallet_addresses(&self) -> Option<Vec<String>> {
        self.wallet_addresses
            .as_ref()
            .and_then(|json| serde_json::from_str(json).ok())
    }

    /// Parse status from row data
    pub fn get_status(&self) -> ToolStatus {
        match self.status.as_str() {
            "ready" => ToolStatus::Ready,
            "running" => ToolStatus::Running,
            "completed" => ToolStatus::Completed,
            "failed" => ToolStatus::Failed,
            "aborted" => ToolStatus::Aborted,
            _ => ToolStatus::Ready,
        }
    }
}

/// VA swap database row
#[derive(Debug, Clone)]
pub struct VaSwapRow {
    pub id: i64,
    pub session_id: String,
    pub tx_index: i32,
    pub wallet_address: String,
    pub is_buy: bool,
    pub amount_sol: f64,
    pub token_amount: Option<f64>,
    pub signature: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub executed_at: Option<String>,
    pub created_at: String,
}

impl VaSwapRow {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        let is_buy_int: i32 = row.get(4).map_err(|e| e.to_string())?;
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            session_id: row.get(1).map_err(|e| e.to_string())?,
            tx_index: row.get(2).map_err(|e| e.to_string())?,
            wallet_address: row.get(3).map_err(|e| e.to_string())?,
            is_buy: is_buy_int != 0,
            amount_sol: row.get(5).map_err(|e| e.to_string())?,
            token_amount: row.get(6).map_err(|e| e.to_string())?,
            signature: row.get(7).map_err(|e| e.to_string())?,
            status: row.get(8).map_err(|e| e.to_string())?,
            error_message: row.get(9).map_err(|e| e.to_string())?,
            executed_at: row.get(10).map_err(|e| e.to_string())?,
            created_at: row.get(11).map_err(|e| e.to_string())?,
        })
    }
}

/// VA session analytics summary
#[derive(Debug, Clone)]
pub struct VaAnalyticsSummary {
    pub total_sessions: i64,
    pub total_volume_sol: f64,
    pub avg_success_rate: f64,
    pub completed_sessions: i64,
    pub failed_sessions: i64,
    pub aborted_sessions: i64,
}

/// Failed ATA database row
#[derive(Debug, Clone)]
pub struct FailedAtaRow {
    pub ata_address: String,
    pub token_mint: Option<String>,
    pub wallet_address: String,
    pub failure_count: i32,
    pub last_error: Option<String>,
    pub first_failed_at: String,
    pub last_failed_at: String,
    pub next_retry_at: Option<String>,
    pub is_permanent_failure: bool,
}

impl FailedAtaRow {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        let is_permanent_int: i32 = row.get(8).map_err(|e| e.to_string())?;
        Ok(Self {
            ata_address: row.get(0).map_err(|e| e.to_string())?,
            token_mint: row.get(1).map_err(|e| e.to_string())?,
            wallet_address: row.get(2).map_err(|e| e.to_string())?,
            failure_count: row.get(3).map_err(|e| e.to_string())?,
            last_error: row.get(4).map_err(|e| e.to_string())?,
            first_failed_at: row.get(5).map_err(|e| e.to_string())?,
            last_failed_at: row.get(6).map_err(|e| e.to_string())?,
            next_retry_at: row.get(7).map_err(|e| e.to_string())?,
            is_permanent_failure: is_permanent_int != 0,
        })
    }
}

/// Tool favorite database row
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolFavoriteRow {
    pub id: i64,
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub tool_type: String,
    pub config_json: Option<String>,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub use_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ToolFavoriteRow {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, String> {
        Ok(Self {
            id: row.get(0).map_err(|e| e.to_string())?,
            mint: row.get(1).map_err(|e| e.to_string())?,
            symbol: row.get(2).map_err(|e| e.to_string())?,
            name: row.get(3).map_err(|e| e.to_string())?,
            logo_url: row.get(4).map_err(|e| e.to_string())?,
            tool_type: row.get(5).map_err(|e| e.to_string())?,
            config_json: row.get(6).map_err(|e| e.to_string())?,
            label: row.get(7).map_err(|e| e.to_string())?,
            notes: row.get(8).map_err(|e| e.to_string())?,
            use_count: row.get(9).map_err(|e| e.to_string())?,
            last_used_at: row.get(10).map_err(|e| e.to_string())?,
            created_at: row.get(11).map_err(|e| e.to_string())?,
            updated_at: row.get(12).map_err(|e| e.to_string())?,
        })
    }
}

// =============================================================================
// TOOL FAVORITES OPERATIONS
// =============================================================================

/// Add or update a tool favorite (upsert)
pub fn upsert_tool_favorite(
    mint: &str,
    symbol: Option<&str>,
    name: Option<&str>,
    logo_url: Option<&str>,
    tool_type: &str,
    config_json: Option<&str>,
    label: Option<&str>,
    notes: Option<&str>,
) -> Result<i64, String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO tool_favorites (mint, symbol, name, logo_url, tool_type, config_json, label, notes, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
        ON CONFLICT(mint, tool_type) DO UPDATE SET
            symbol = COALESCE(?2, symbol),
            name = COALESCE(?3, name),
            logo_url = COALESCE(?4, logo_url),
            config_json = COALESCE(?6, config_json),
            label = COALESCE(?7, label),
            notes = COALESCE(?8, notes),
            updated_at = ?9
        "#,
        params![mint, symbol, name, logo_url, tool_type, config_json, label, notes, now],
    )
    .map_err(|e| format!("Failed to upsert tool favorite: {}", e))?;

    // Get the ID (either inserted or existing)
    conn.query_row(
        "SELECT id FROM tool_favorites WHERE mint = ?1 AND tool_type = ?2",
        params![mint, tool_type],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to get favorite ID: {}", e))
}

/// Get all tool favorites, optionally filtered by tool type
pub fn get_tool_favorites(tool_type: Option<&str>) -> Result<Vec<ToolFavoriteRow>, String> {
    let conn = get_connection()?;

    let mut favorites = Vec::new();

    if let Some(tt) = tool_type {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, mint, symbol, name, logo_url, tool_type, config_json, label, notes,
                       use_count, last_used_at, created_at, updated_at
                FROM tool_favorites
                WHERE tool_type = ?1
                ORDER BY use_count DESC, updated_at DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let rows = stmt
            .query_map(params![tt], |row| Ok(ToolFavoriteRow::from_row(row)))
            .map_err(|e| format!("Failed to query favorites: {}", e))?;

        for row in rows {
            match row {
                Ok(Ok(fav)) => favorites.push(fav),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("Failed to read row: {}", e)),
            }
        }
    } else {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, mint, symbol, name, logo_url, tool_type, config_json, label, notes,
                       use_count, last_used_at, created_at, updated_at
                FROM tool_favorites
                ORDER BY use_count DESC, updated_at DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let rows = stmt
            .query_map([], |row| Ok(ToolFavoriteRow::from_row(row)))
            .map_err(|e| format!("Failed to query favorites: {}", e))?;

        for row in rows {
            match row {
                Ok(Ok(fav)) => favorites.push(fav),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("Failed to read row: {}", e)),
            }
        }
    }

    Ok(favorites)
}

/// Remove a tool favorite by ID
pub fn remove_tool_favorite(id: i64) -> Result<bool, String> {
    let conn = get_connection()?;

    let rows = conn
        .execute("DELETE FROM tool_favorites WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to remove tool favorite: {}", e))?;

    Ok(rows > 0)
}

/// Increment use count for a favorite
pub fn increment_tool_favorite_use(id: i64) -> Result<(), String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE tool_favorites SET use_count = use_count + 1, last_used_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| format!("Failed to increment use count: {}", e))?;

    Ok(())
}

/// Update a tool favorite's config/label/notes
pub fn update_tool_favorite(
    id: i64,
    config_json: Option<&str>,
    label: Option<&str>,
    notes: Option<&str>,
) -> Result<bool, String> {
    let conn = get_connection()?;
    let now = Utc::now().to_rfc3339();

    let rows = conn
        .execute(
            r#"
            UPDATE tool_favorites SET
                config_json = COALESCE(?1, config_json),
                label = COALESCE(?2, label),
                notes = COALESCE(?3, notes),
                updated_at = ?4
            WHERE id = ?5
            "#,
            params![config_json, label, notes, now, id],
        )
        .map_err(|e| format!("Failed to update tool favorite: {}", e))?;

    Ok(rows > 0)
}
