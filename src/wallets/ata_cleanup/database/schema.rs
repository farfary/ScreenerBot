// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The ATA cleanup store's schema: the cleanup tables and the step that retires the tables of the removed tools area.

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Transaction;

use crate::database;
use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::errors::DatabaseError;
use crate::wallets::Error;

// =============================================================================
// SCHEMA DEFINITIONS
// =============================================================================

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

// =============================================================================
// STORE
// =============================================================================

/// Drops the tables of the removed tools area. Children come before the parent
/// their foreign key names.
const DROP_RETIRED_TABLES: &str = r#"
DROP TABLE IF EXISTS mw_wallet_ops;
DROP TABLE IF EXISTS mw_sessions;
DROP TABLE IF EXISTS tool_favorites;
DROP TABLE IF EXISTS watched_tokens;
DROP TABLE IF EXISTS schema_version;
"#;

/// The ATA cleanup store's schema. Version 2 drops the retired tables, which no
/// version 1 core can work without, so it is the oldest version that reads it.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: 2,
    min_reader_version: 2,
    baseline: 1,
    renames: None,
    step: create_schema,
    legacy_stamp: None,
};

fn create_schema(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    for (operation, sql) in [
        ("create ata_sessions table", SCHEMA_ATA_SESSIONS),
        ("create ata_closures table", SCHEMA_ATA_CLOSURES),
        ("create ata_failed_cache table", SCHEMA_ATA_FAILED_CACHE),
    ] {
        ddl::run_batch(tx, operation, sql)?;
    }
    // A store written by an older core holds the retired tables, and a core from
    // before the store ledger re-creates them whenever it opens the store, so every
    // open checks for them rather than only the upgrade to version 2.
    ddl::run_batch(tx, "drop retired tools tables", DROP_RETIRED_TABLES)
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
