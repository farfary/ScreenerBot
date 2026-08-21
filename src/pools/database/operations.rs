//! Core PoolsDatabase struct and operations

use super::super::types::{PriceResult, PRICE_HISTORY_MAX_ENTRIES};
use super::types::{BlacklistedAccountRecord, BlacklistedPoolRecord, DbPriceResult};
use super::writer::run_database_writer;
use crate::logger::{self, LogTag};

use crate::chains::ChainId;
use crate::database;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex; // Changed to std::sync::Mutex for spawn_blocking compatibility
use std::sync::RwLock;
use tokio::sync::mpsc;

/// Maximum age for price history entries (7 days)
const MAX_PRICE_HISTORY_AGE_DAYS: i64 = 7;

/// Maximum allowable gap between price updates (1 minute in seconds)
const MAX_PRICE_GAP_SECONDS: i64 = 60;

const POOLS_SCHEMA_VERSION: i64 = 1;

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| format!("Failed to inspect {table} schema: {e}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to inspect {table} columns: {e}"))?;
    let has_column = columns.filter_map(Result::ok).any(|name| name == column);
    Ok(has_column)
}

/// Compares row counts between a legacy table and its migrated replacement inside
/// the same transaction. Propagates the count query's own failure instead of
/// treating it as zero rows — a `COUNT(*)` failure (locked table, missing table)
/// must never be silently read as "0 rows, counts match" and let the integrity
/// guard pass vacuously.
fn verify_migrated_row_count(
    tx: &rusqlite::Transaction,
    old: &str,
    new: &str,
) -> Result<(), String> {
    let old_count: i64 = tx
        .query_row(&format!("SELECT COUNT(*) FROM {old}"), [], |row| row.get(0))
        .map_err(|e| format!("Failed to read {old} row count for migration validation: {e}"))?;
    let new_count: i64 = tx
        .query_row(&format!("SELECT COUNT(*) FROM {new}"), [], |row| row.get(0))
        .map_err(|e| format!("Failed to validate migrated {new} row count: {e}"))?;
    if old_count != new_count {
        return Err(format!(
            "Pools migration row-count mismatch for {old}: {old_count} != {new_count}"
        ));
    }
    Ok(())
}

fn migrate_schema(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS price_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL,
            price_usd REAL NOT NULL, price_sol REAL NOT NULL, confidence REAL NOT NULL, slot INTEGER NOT NULL,
            timestamp_unix INTEGER NOT NULL, sol_reserves REAL NOT NULL, token_reserves REAL NOT NULL,
            source_pool TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(mint, pool_address, timestamp_unix)
        );
        CREATE TABLE IF NOT EXISTS blacklist_accounts (
            account_pubkey TEXT PRIMARY KEY, reason TEXT NOT NULL, source TEXT, pool_id TEXT, token_mint TEXT,
            error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS blacklist_pools (
            pool_id TEXT PRIMARY KEY, reason TEXT NOT NULL, token_mint TEXT, program_id TEXT,
            error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| format!("Failed to create legacy pools tables for migration: {e}"))?;

    // `user_version` gates the structural column check below: once it records
    // the current schema version, every later boot skips straight past this
    // block instead of re-running `PRAGMA table_info` on three tables. It is
    // only a fast-path — the structural check (not the version number alone)
    // is still what decides whether a real migration is needed the first time,
    // so a DB whose version was never bumped is migrated correctly regardless.
    let schema_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("Failed to read pools schema version: {e}"))?;

    if schema_version < POOLS_SCHEMA_VERSION {
        let price_history_has_chain = table_has_column(conn, "price_history", "chain_id")?;
        let accounts_has_chain = table_has_column(conn, "blacklist_accounts", "chain_id")?;
        let pools_has_chain = table_has_column(conn, "blacklist_pools", "chain_id")?;

        if !price_history_has_chain || !accounts_has_chain || !pools_has_chain {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| format!("Failed to start pools schema migration: {e}"))?;

            tx.execute_batch(
                "CREATE TABLE price_history_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                price_usd REAL NOT NULL,
                price_sol REAL NOT NULL,
                confidence REAL NOT NULL,
                slot INTEGER NOT NULL,
                timestamp_unix INTEGER NOT NULL,
                sol_reserves REAL NOT NULL,
                token_reserves REAL NOT NULL,
                source_pool TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(chain_id, mint, pool_address, timestamp_unix)
            );
            CREATE TABLE blacklist_accounts_new (
                chain_id TEXT NOT NULL,
                account_pubkey TEXT NOT NULL,
                reason TEXT NOT NULL,
                source TEXT,
                pool_id TEXT,
                token_mint TEXT,
                error_count INTEGER DEFAULT 1,
                first_failed_at INTEGER NOT NULL,
                last_failed_at INTEGER NOT NULL,
                added_at INTEGER NOT NULL,
                PRIMARY KEY(chain_id, account_pubkey)
            );
            CREATE TABLE blacklist_pools_new (
                chain_id TEXT NOT NULL,
                pool_id TEXT NOT NULL,
                reason TEXT NOT NULL,
                token_mint TEXT,
                program_id TEXT,
                error_count INTEGER DEFAULT 1,
                first_failed_at INTEGER NOT NULL,
                last_failed_at INTEGER NOT NULL,
                added_at INTEGER NOT NULL,
                PRIMARY KEY(chain_id, pool_id)
            );",
            )
            .map_err(|e| format!("Failed to create chain-aware pools tables: {e}"))?;

            if price_history_has_chain {
            tx.execute("INSERT INTO price_history_new SELECT * FROM price_history", [])
        } else {
            tx.execute(
                "INSERT INTO price_history_new (id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, source_pool, created_at)
                 SELECT id, 'solana', mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, source_pool, created_at FROM price_history",
                [],
            )
        }
        .map_err(|e| format!("Failed to migrate price history: {e}"))?;
            if accounts_has_chain {
            tx.execute("INSERT INTO blacklist_accounts_new SELECT * FROM blacklist_accounts", [])
        } else {
            tx.execute(
                "INSERT INTO blacklist_accounts_new (chain_id, account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at)
                 SELECT 'solana', account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at FROM blacklist_accounts",
                [],
            )
        }
        .map_err(|e| format!("Failed to migrate account blacklist: {e}"))?;
            if pools_has_chain {
            tx.execute("INSERT INTO blacklist_pools_new SELECT * FROM blacklist_pools", [])
        } else {
            tx.execute(
                "INSERT INTO blacklist_pools_new (chain_id, pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at)
                 SELECT 'solana', pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at FROM blacklist_pools",
                [],
            )
        }
        .map_err(|e| format!("Failed to migrate pool blacklist: {e}"))?;

            for (old, new) in [
                ("price_history", "price_history_new"),
                ("blacklist_accounts", "blacklist_accounts_new"),
                ("blacklist_pools", "blacklist_pools_new"),
            ] {
                verify_migrated_row_count(&tx, old, new)?;
            }

            tx.execute_batch(
                "DROP TABLE IF EXISTS price_history;
             DROP TABLE IF EXISTS blacklist_accounts;
             DROP TABLE IF EXISTS blacklist_pools;
             ALTER TABLE price_history_new RENAME TO price_history;
             ALTER TABLE blacklist_accounts_new RENAME TO blacklist_accounts;
             ALTER TABLE blacklist_pools_new RENAME TO blacklist_pools;",
            )
            .map_err(|e| format!("Failed to replace legacy pools tables: {e}"))?;
            tx.commit()
                .map_err(|e| format!("Failed to commit pools schema migration: {e}"))?;
        }
    }

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_price_history_chain_mint_timestamp ON price_history(chain_id, mint, timestamp_unix DESC);
         CREATE INDEX IF NOT EXISTS idx_price_history_chain_pool_timestamp ON price_history(chain_id, pool_address, timestamp_unix DESC);
         CREATE INDEX IF NOT EXISTS idx_price_history_created_at ON price_history(created_at);
         CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_chain_pool ON blacklist_accounts(chain_id, pool_id);
         CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_chain_token ON blacklist_accounts(chain_id, token_mint);
         CREATE INDEX IF NOT EXISTS idx_blacklist_pools_chain_token ON blacklist_pools(chain_id, token_mint);",
    )
    .map_err(|e| format!("Failed to create chain-aware pools indexes: {e}"))?;
    let foreign_key_violation: Option<String> = conn
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()
        .map_err(|e| format!("Failed to validate pools foreign keys: {e}"))?;
    if let Some(table) = foreign_key_violation {
        return Err(format!(
            "Pools migration foreign-key validation failed for {table}"
        ));
    }
    conn.pragma_update(None, "user_version", POOLS_SCHEMA_VERSION)
        .map_err(|e| format!("Failed to record pools schema version: {e}"))?;
    Ok(())
}

// =============================================================================
// POOLS DATABASE
// =============================================================================

/// SQLite-based price history storage
#[derive(Debug)]
pub struct PoolsDatabase {
    pub(super) chain_id: ChainId,
    pub(super) db_path: String,
    pub(super) connection: Arc<Mutex<Option<Connection>>>,
    pub(super) write_queue: Option<mpsc::UnboundedSender<PriceResult>>,
    // In-memory blacklists (source of truth for runtime checks)
    pub(super) blacklisted_accounts: Arc<RwLock<HashSet<String>>>,
    pub(super) blacklisted_pools: Arc<RwLock<HashSet<String>>>,
}

impl Clone for PoolsDatabase {
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id,
            db_path: self.db_path.clone(),
            connection: Arc::clone(&self.connection),
            write_queue: self.write_queue.clone(),
            blacklisted_accounts: Arc::clone(&self.blacklisted_accounts),
            blacklisted_pools: Arc::clone(&self.blacklisted_pools),
        }
    }
}

impl PoolsDatabase {
    /// Create new pools database instance
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            db_path: crate::paths::get_pools_db_path()
                .to_string_lossy()
                .to_string(),
            connection: Arc::new(Mutex::new(None)),
            write_queue: None,
            blacklisted_accounts: Arc::new(RwLock::new(HashSet::new())),
            blacklisted_pools: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Create a clone suitable for async operations
    /// This is the same as clone() but with a more explicit name
    pub fn clone_for_async(&self) -> Self {
        self.clone()
    }

    /// Initialize database and create tables
    pub async fn initialize(&mut self) -> Result<(), String> {
        // Create database connection
        let mut conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed to open pools database: {e}"))?;

        // Apply shared SQLite configuration
        database::configure_connection(&conn, database::POOLS_DB)
            .map_err(|e| format!("Failed to configure pools database: {e}"))?;

        migrate_schema(&mut conn)?;

        // Store connection
        {
            let mut connection_guard = self.connection.lock().unwrap();
            *connection_guard = Some(conn);
        }

        // Setup write queue for batched operations
        let (tx, rx) = mpsc::unbounded_channel();
        self.write_queue = Some(tx);

        // Start background writer task
        let db_connection = self.connection.clone();
        let chain_id = self.chain_id;
        tokio::spawn(async move {
            run_database_writer(rx, db_connection, chain_id).await;
        });

        logger::info(
            LogTag::PoolService,
            &format!("Pools database initialized: {}", self.db_path),
        );

        // Load blacklists into memory (priority for runtime checks)

        let (account_keys, pool_keys) = {
            let connection_guard = self.connection.lock().unwrap();
            if let Some(ref conn) = *connection_guard {
                // Accounts
                let account_keys = match conn
                    .prepare("SELECT account_pubkey FROM blacklist_accounts WHERE chain_id = ?")
                {
                    Ok(mut stmt) => {
                        let rows =
                            stmt.query_map([self.chain_id.as_str()], |row| row.get::<_, String>(0));
                        match rows {
                            Ok(iter) => iter.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                            Err(e) => {
                                logger::warning(
                                    LogTag::PoolService,
                                    &format!(
                                        "Failed to load blacklist_accounts into memory: {}",
                                        e
                                    ),
                                );
                                Vec::new()
                            }
                        }
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolService,
                            &format!("Failed to prepare load for blacklist_accounts: {e}"),
                        );
                        Vec::new()
                    }
                };

                // Pools
                let pool_keys =
                    match conn.prepare("SELECT pool_id FROM blacklist_pools WHERE chain_id = ?") {
                        Ok(mut stmt) => {
                            let rows = stmt
                                .query_map([self.chain_id.as_str()], |row| row.get::<_, String>(0));
                            match rows {
                                Ok(iter) => iter.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                                Err(e) => {
                                    logger::warning(
                                        LogTag::PoolService,
                                        &format!("Failed to load blacklist_pools into memory: {e}"),
                                    );
                                    Vec::new()
                                }
                            }
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::PoolService,
                                &format!("Failed to prepare load for blacklist_pools: {e}"),
                            );
                            Vec::new()
                        }
                    };

                (account_keys, pool_keys)
            } else {
                (Vec::new(), Vec::new())
            }
        };

        // Populate memory sets
        {
            let mut accounts = self.blacklisted_accounts.write().unwrap();
            for key in account_keys {
                accounts.insert(key);
            }
        }

        {
            let mut pools = self.blacklisted_pools.write().unwrap();
            for key in pool_keys {
                pools.insert(key);
            }
        }

        Ok(())
    }

    /// Queue a price result for async storage (non-blocking)
    pub async fn queue_price_for_storage(&self, price: PriceResult) -> Result<(), String> {
        if let Some(ref tx) = self.write_queue {
            tx.send(price)
                .map_err(|e| format!("Failed to queue price for storage: {e}"))?;
            Ok(())
        } else {
            Err("Write queue not initialized".to_owned())
        }
    }

    /// Load recent price history for cache initialization
    pub async fn load_recent_price_history(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<PriceResult>, String> {
        let mint_str = mint.to_string();
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_owned())?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
               timestamp_unix, sol_reserves, token_reserves, source_pool, created_at
         FROM price_history
         WHERE chain_id = ? AND mint = ?
         ORDER BY timestamp_unix DESC
         LIMIT ?",
                )
                .map_err(|e| format!("Failed to prepare query: {e}"))?;

            let rows = stmt
                .query_map(params![chain_id, mint_str, limit], |row| DbPriceResult::from_row(row))
                .map_err(|e| format!("Failed to query price history: {e}"))?;

            let mut results = Vec::new();
            for row in rows {
                let db_price = row.map_err(|e| format!("Failed to read row: {e}"))?;
                results.push(db_price.to_price_result());
            }

            // Reverse so oldest comes first
            results.reverse();

            Ok::<_, String>(results)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Get price history with optional filtering
    pub async fn get_price_history(
        &self,
        mint: &str,
        limit: Option<usize>,
        since_timestamp: Option<i64>,
    ) -> Result<Vec<PriceResult>, String> {
        let mint_str = mint.to_string();
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id.as_str().to_owned();
        let limit = limit.unwrap_or(PRICE_HISTORY_MAX_ENTRIES);

        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_owned())?;

            let mut results = Vec::new();

            if let Some(ts) = since_timestamp {
                let query = format!(
                    "SELECT id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
               timestamp_unix, sol_reserves, token_reserves, source_pool, created_at
         FROM price_history
         WHERE chain_id = ? AND mint = ? AND timestamp_unix >= ?
         ORDER BY timestamp_unix DESC
         LIMIT {limit}"
                );

                let mut stmt = conn
                    .prepare(&query)
                    .map_err(|e| format!("Failed to prepare query: {e}"))?;

                let rows = stmt
                    .query_map(params![chain_id, mint_str, ts], |row| DbPriceResult::from_row(row))
                    .map_err(|e| format!("Failed to query price history: {e}"))?;

                for row in rows {
                    let db_price = row.map_err(|e| format!("Failed to read row: {e}"))?;
                    results.push(db_price.to_price_result());
                }
            } else {
                let query = format!(
                    "SELECT id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
               timestamp_unix, sol_reserves, token_reserves, source_pool, created_at
         FROM price_history
         WHERE chain_id = ? AND mint = ?
         ORDER BY timestamp_unix DESC
         LIMIT {limit}"
                );

                let mut stmt = conn
                    .prepare(&query)
                    .map_err(|e| format!("Failed to prepare query: {e}"))?;

                let rows = stmt
                    .query_map(params![chain_id, mint_str], |row| DbPriceResult::from_row(row))
                    .map_err(|e| format!("Failed to query price history: {e}"))?;

                for row in rows {
                    let db_price = row.map_err(|e| format!("Failed to read row: {e}"))?;
                    results.push(db_price.to_price_result());
                }
            }

            // Reverse so oldest comes first
            results.reverse();

            Ok::<_, String>(results)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Cleanup old database entries beyond retention period
    pub async fn cleanup_old_entries(&self) -> Result<usize, String> {
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_owned())?;

            // Calculate cutoff date
            let cutoff_date =
                chrono::Utc::now() - chrono::Duration::days(MAX_PRICE_HISTORY_AGE_DAYS);
            let cutoff_str = cutoff_date.to_rfc3339();

            let deleted = conn
                .execute(
                    "DELETE FROM price_history WHERE chain_id = ? AND created_at < ?",
                    params![chain_id, cutoff_str],
                )
                .map_err(|e| format!("Failed to cleanup old entries: {e}"))?;

            Ok::<_, String>(deleted)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Cleanup gapped data for a specific token
    /// Removes price history entries older than the first significant gap
    pub async fn cleanup_gapped_data_for_token(&self, mint: &str) -> Result<usize, String> {
        // Find the cutoff point (first gap > 1 minute)
        let cutoff_timestamp = match self.find_first_price_gap(mint).await? {
            Some(ts) => ts,
            None => return Ok(0), // No gaps found
        };

        // Delete everything older than the cutoff
        let mint_str = mint.to_string();
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_owned())?;

            let deleted = conn
                .execute(
                    "DELETE FROM price_history WHERE chain_id = ? AND mint = ? AND timestamp_unix <= ?",
                    params![chain_id, mint_str, cutoff_timestamp],
                )
                .map_err(|e| format!("Failed to delete gapped data: {e}"))?;

            Ok::<_, String>(deleted)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))?
    }

    /// Find the first significant gap in price data for a token
    /// Returns the timestamp of the older entry at the gap point
    async fn find_first_price_gap(&self, mint: &str) -> Result<Option<i64>, String> {
        let mint_str = mint.to_string();
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id.as_str().to_owned();

        let timestamps = tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_owned())?;

            let mut stmt = conn
                .prepare(
                    "SELECT timestamp_unix 
           FROM price_history 
           WHERE chain_id = ? AND mint = ?
           ORDER BY timestamp_unix DESC",
                )
                .map_err(|e| format!("Failed to prepare gap query: {e}"))?;

            let rows = stmt
                .query_map(params![chain_id, mint_str], |row| row.get::<_, i64>(0))
                .map_err(|e| format!("Failed to query timestamps: {e}"))?;

            let mut timestamps = Vec::new();
            for row in rows {
                timestamps.push(row.map_err(|e| format!("Failed to read timestamp: {e}"))?);
            }

            Ok::<_, String>(timestamps)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))??;

        if timestamps.len() < 2 {
            return Ok(None); // Not enough data to find a gap
        }

        // Work backwards to find the first gap > 1 minute
        for i in 1..timestamps.len() {
            let current_time = timestamps[i - 1]; // Newer timestamp
            let prev_time = timestamps[i]; // Older timestamp

            let gap = current_time - prev_time;

            if gap > (MAX_PRICE_GAP_SECONDS as i64) {
                // Found a gap - return the older timestamp as cutoff point
                return Ok(Some(prev_time));
            }
        }

        Ok(None) // No significant gaps found
    }

    /// Cleanup gapped data for all tokens
    pub async fn cleanup_all_gapped_data(&self) -> Result<usize, String> {
        let conn_arc = self.connection.clone();
        let chain_id = self.chain_id.as_str().to_owned();

        // Get all unique tokens in the database
        let tokens = tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {e}"))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_owned())?;

            let mut stmt = conn
                .prepare("SELECT DISTINCT mint FROM price_history WHERE chain_id = ?")
                .map_err(|e| format!("Failed to prepare token list query: {e}"))?;

            let rows = stmt
                .query_map([chain_id], |row| Ok(row.get::<_, String>("mint")?))
                .map_err(|e| format!("Failed to execute token list query: {e}"))?;

            let mut tokens = Vec::new();
            for row in rows {
                tokens.push(row.map_err(|e| format!("Failed to parse token mint: {e}"))?);
            }

            Ok::<_, String>(tokens)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {e}"))??;

        // Clean up gapped data for each token
        let mut total_deleted = 0;
        for token in tokens {
            match self.cleanup_gapped_data_for_token(&token).await {
                Ok(deleted) => {
                    total_deleted += deleted;
                }
                Err(e) => {
                    logger::error(
                        LogTag::PoolCache,
                        &format!("Failed to cleanup gapped data for token {token}: {e}"),
                    );
                }
            }
        }

        if total_deleted > 0 {
            logger::debug(
                LogTag::PoolService,
                &format!(
                    "Removed {} total gapped price entries across all tokens",
                    total_deleted
                ),
            );
        }

        Ok(total_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch(
            "CREATE TABLE price_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL,
                price_usd REAL NOT NULL, price_sol REAL NOT NULL, confidence REAL NOT NULL, slot INTEGER NOT NULL,
                timestamp_unix INTEGER NOT NULL, sol_reserves REAL NOT NULL, token_reserves REAL NOT NULL,
                source_pool TEXT, created_at TEXT NOT NULL, UNIQUE(mint, pool_address, timestamp_unix)
            );
            CREATE TABLE blacklist_accounts (
                account_pubkey TEXT PRIMARY KEY, reason TEXT NOT NULL, source TEXT, pool_id TEXT, token_mint TEXT,
                error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
            );
            CREATE TABLE blacklist_pools (
                pool_id TEXT PRIMARY KEY, reason TEXT NOT NULL, token_mint TEXT, program_id TEXT,
                error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
            );
            INSERT INTO price_history VALUES (1, 'mint', 'pool', 1.0, 2.0, 1.0, 7, 10, 3.0, 4.0, NULL, '2026-01-01T00:00:00Z');
            INSERT INTO blacklist_accounts VALUES ('account', 'reason', NULL, 'pool', 'mint', 1, 1, 1, 1);
            INSERT INTO blacklist_pools VALUES ('pool', 'reason', 'mint', NULL, 1, 1, 1, 1);",
        )
        .expect("seed legacy schema");
        conn
    }

    #[test]
    fn legacy_schema_migrates_to_solana_without_losing_rows_and_is_idempotent() {
        let mut conn = legacy_connection();
        migrate_schema(&mut conn).expect("migrate legacy pools database");
        migrate_schema(&mut conn).expect("repeat pools migration");

        for table in ["price_history", "blacklist_accounts", "blacklist_pools"] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count migrated rows");
            assert_eq!(count, 1, "{table} row count");
            let chain: String = conn
                .query_row(
                    &format!("SELECT chain_id FROM {table} LIMIT 1"),
                    [],
                    |row| row.get(0),
                )
                .expect("read migrated chain");
            assert_eq!(chain, ChainId::Solana.as_str());
        }
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            POOLS_SCHEMA_VERSION
        );
        assert!(conn
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .unwrap()
            .is_none());
    }

    #[test]
    fn verify_migrated_row_count_propagates_the_count_querys_own_failure() {
        let mut conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch("CREATE TABLE new_table (id INTEGER PRIMARY KEY);")
            .expect("create replacement table");
        let tx = conn.unchecked_transaction().expect("start transaction");

        // "old_table" was never created, so the COUNT(*) query itself fails.
        // Before the fix, `.unwrap_or(0)` treated that failure as "0 rows",
        // which would have matched the empty new_table and passed vacuously.
        let err = verify_migrated_row_count(&tx, "old_table", "new_table")
            .expect_err("a failed row-count query must propagate, not read as zero");
        assert!(err.contains("old_table"));
    }

    #[test]
    fn verify_migrated_row_count_rejects_a_genuine_mismatch() {
        let mut conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch(
            "CREATE TABLE old_table (id INTEGER PRIMARY KEY);
             CREATE TABLE new_table (id INTEGER PRIMARY KEY);
             INSERT INTO old_table VALUES (1), (2);
             INSERT INTO new_table VALUES (1);",
        )
        .expect("seed mismatched tables");
        let tx = conn.unchecked_transaction().expect("start transaction");

        let err = verify_migrated_row_count(&tx, "old_table", "new_table")
            .expect_err("a real row-count mismatch must be rejected");
        assert!(err.contains("mismatch"));
    }

    #[tokio::test]
    async fn solana_repository_ignores_raw_rows_from_another_chain() {
        let mut conn = legacy_connection();
        migrate_schema(&mut conn).expect("migrate test database");
        conn.execute(
            "INSERT INTO price_history (chain_id, mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, created_at)
             VALUES ('ethereum', 'mint', 'pool', 1.0, 9.0, 1.0, 8, 20, 3.0, 4.0, '2026-01-01T00:00:20Z')",
            [],
        ).expect("insert conceptual foreign-chain row");
        conn.execute(
            "INSERT INTO blacklist_pools (chain_id, pool_id, reason, token_mint, error_count, first_failed_at, last_failed_at, added_at)
             VALUES ('ethereum', 'pool', 'foreign', 'mint', 1, 1, 1, 1)",
            [],
        ).expect("insert conceptual foreign-chain blacklist");

        let db = PoolsDatabase {
            chain_id: ChainId::Solana,
            db_path: ":memory:".to_owned(),
            connection: Arc::new(Mutex::new(Some(conn))),
            write_queue: None,
            blacklisted_accounts: Arc::new(RwLock::new(HashSet::new())),
            blacklisted_pools: Arc::new(RwLock::new(HashSet::new())),
        };
        let history = db
            .get_price_history("mint", None, None)
            .await
            .expect("read solana history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].price_sol, 2.0);
        let pools = db
            .list_blacklisted_pools(None)
            .await
            .expect("read solana blacklist");
        assert_eq!(pools.len(), 1);
        assert!(pools
            .iter()
            .all(|record| record.chain_id == ChainId::Solana));
    }
}
