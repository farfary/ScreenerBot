// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The ATA cleanup store's schema: the failed-close cache and the step that retires the tables of removed features.

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

/// Drops the tables of the removed tools area and of the manual cleanup sessions.
/// Children come before the parent their foreign key names.
const DROP_RETIRED_TABLES: &str = r#"
DROP TABLE IF EXISTS ata_closures;
DROP TABLE IF EXISTS ata_sessions;
DROP TABLE IF EXISTS mw_wallet_ops;
DROP TABLE IF EXISTS mw_sessions;
DROP TABLE IF EXISTS tool_favorites;
DROP TABLE IF EXISTS watched_tokens;
DROP TABLE IF EXISTS schema_version;
"#;

/// The ATA cleanup store's schema. Version 2 drops the tools tables and version 3
/// the cleanup session tables; no older core works without them, so each drop
/// makes its own version the oldest that reads the store.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: 3,
    min_reader_version: 3,
    baseline: 1,
    renames: None,
    step: create_schema,
    legacy_stamp: None,
};

fn create_schema(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    ddl::run_batch(tx, "create ata_failed_cache table", SCHEMA_ATA_FAILED_CACHE)?;
    // A store written by an older core holds the retired tables, and a core from
    // before the store ledger re-creates them whenever it opens the store, so every
    // open checks for them rather than only the upgrade that retired them.
    ddl::run_batch(tx, "drop retired tables", DROP_RETIRED_TABLES)
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
