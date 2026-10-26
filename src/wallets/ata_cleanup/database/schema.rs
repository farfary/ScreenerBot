// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Database schema definitions and initialization.

use chrono::Utc;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension, Transaction};

use crate::database;
use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::errors::DatabaseError;
use crate::wallets::Error;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Schema version for migrations
const TOOLS_SCHEMA_VERSION: u32 = 1;

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
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Chain scope
    chain_id TEXT NOT NULL DEFAULT 'solana'
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
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Chain scope
    chain_id TEXT NOT NULL DEFAULT 'solana'
);

CREATE INDEX IF NOT EXISTS idx_watched_tokens_mint ON watched_tokens(mint);
CREATE INDEX IF NOT EXISTS idx_watched_tokens_active ON watched_tokens(is_active);
"#;

// =============================================================================
// STORE
// =============================================================================

/// The ATA cleanup store's schema.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: 1,
    min_reader_version: 1,
    baseline: 1,
    renames: None,
    step: create_schema,
    legacy_stamp: None,
};

fn create_schema(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    ddl::run_batch(
        tx,
        "create tools schema_version table",
        SCHEMA_VERSION_TABLE,
    )?;
    let current_version: Option<u32> = tx
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| DatabaseError::classify_sqlite_failure("read tools schema version", e))?;

    if current_version.unwrap_or(0) < TOOLS_SCHEMA_VERSION {
        for (operation, sql) in [
            ("create ata_sessions table", SCHEMA_ATA_SESSIONS),
            ("create ata_closures table", SCHEMA_ATA_CLOSURES),
            ("create ata_failed_cache table", SCHEMA_ATA_FAILED_CACHE),
            ("create tool_favorites table", SCHEMA_TOOL_FAVORITES),
            ("create mw_sessions table", SCHEMA_MW_SESSIONS),
            ("create mw_wallet_ops table", SCHEMA_MW_WALLET_OPS),
            ("create watched_tokens table", SCHEMA_WATCHED_TOKENS),
        ] {
            ddl::run_batch(tx, operation, sql)?;
        }
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![TOOLS_SCHEMA_VERSION, Utc::now().to_rfc3339()],
        )
        .map_err(|e| DatabaseError::classify_sqlite_failure("record tools schema version", e))?;
    }

    // Chain scope: `mw_sessions` and `watched_tokens` gain a chain column, checked
    // whatever the version stamp says: a store from v0.2.13 already records version 1.
    // Historical rows are Solana's by construction, so the migration backfills 'solana'.
    for table in ["mw_sessions", "watched_tokens"] {
        ddl::add_column(tx, table, "chain_id", "TEXT NOT NULL DEFAULT 'solana'")?;
    }
    Ok(())
}

/// Get a connection from the ATA cleanup store's pool.
pub(super) fn get_connection() -> Result<PooledConnection<SqliteConnectionManager>, Error> {
    database::open::store(&database::catalog::ATA_CLEANUP)?
        .get()
        .map_err(|e| DatabaseError::from(e).into())
}

/// Open the ATA cleanup store.
pub fn init_ata_cleanup_db() -> Result<(), Error> {
    database::open::store(&database::catalog::ATA_CLEANUP)?;
    Ok(())
}
