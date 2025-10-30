/// Database module for persistent price history storage
///
/// This module provides SQLite-based storage for price history data,
/// enabling price history to survive service restarts and providing
/// full historical data access beyond the in-memory cache limits.
use super::types::{PriceResult, PRICE_HISTORY_MAX_ENTRIES};

use crate::logger::{self, LogTag};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex; // Changed to std::sync::Mutex for spawn_blocking compatibility
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

/// Database file path
const POOLS_DB_PATH: &str = "data/pools.db";

/// Maximum age for price history entries (7 days)
const MAX_PRICE_HISTORY_AGE_DAYS: i64 = 7;

/// Batch size for database operations
const DB_BATCH_SIZE: usize = 100;

/// Database write interval (seconds)
const DB_WRITE_INTERVAL_SECONDS: u64 = 10;

/// Maximum allowable gap between price updates (1 minute in seconds)
const MAX_PRICE_GAP_SECONDS: i64 = 60;

// =============================================================================
// DATABASE STRUCTURES
// =============================================================================

/// Database representation of a price result for storage
#[derive(Debug, Clone)]
pub struct DbPriceResult {
    pub id: Option<i64>,
    pub mint: String,
    pub pool_address: String,
    pub price_usd: f64,
    pub price_sol: f64,
    pub confidence: f32,
    pub slot: u64,
    pub timestamp_unix: i64,
    pub sol_reserves: f64,
    pub token_reserves: f64,
    pub source_pool: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl DbPriceResult {
    /// Create from PriceResult
    pub fn from_price_result(price: &PriceResult) -> Self {
        let timestamp_unix = Self::approximate_unix_timestamp(price.timestamp);

        Self {
            id: None,
            mint: price.mint.clone(),
            pool_address: price.pool_address.clone(),
            price_usd: price.price_usd,
            price_sol: price.price_sol,
            confidence: price.confidence,
            slot: price.slot,
            timestamp_unix,
            sol_reserves: price.sol_reserves,
            token_reserves: price.token_reserves,
            source_pool: price.source_pool.clone(),
            created_at: Utc::now(),
        }
    }

    /// Convert to PriceResult
    pub fn to_price_result(&self) -> PriceResult {
        PriceResult {
            mint: self.mint.clone(),
            price_usd: self.price_usd,
            price_sol: self.price_sol,
            confidence: self.confidence,
            source_pool: self.source_pool.clone(),
            pool_address: self.pool_address.clone(),
            slot: self.slot,
            timestamp: Self::instant_from_unix_timestamp(self.timestamp_unix),
            sol_reserves: self.sol_reserves,
            token_reserves: self.token_reserves,
        }
    }

    /// Create from database row
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let created_at_str: String = row.get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    "created_at".to_string(),
                    rusqlite::types::Type::Text,
                )
            })?
            .with_timezone(&Utc);

        Ok(Self {
            id: Some(row.get("id")?),
            mint: row.get("mint")?,
            pool_address: row.get("pool_address")?,
            price_usd: row.get("price_usd")?,
            price_sol: row.get("price_sol")?,
            confidence: row.get("confidence")?,
            slot: row.get("slot")?,
            timestamp_unix: row.get("timestamp_unix")?,
            sol_reserves: row.get("sol_reserves")?,
            token_reserves: row.get("token_reserves")?,
            source_pool: row.get("source_pool")?,
            created_at,
        })
    }

    /// Convert an Instant to an approximate unix timestamp (seconds precision)
    fn approximate_unix_timestamp(instant: std::time::Instant) -> i64 {
        let now_system = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        match now_system.checked_sub(instant.elapsed()) {
            Some(ts) => ts.as_secs() as i64,
            None => 0,
        }
    }

    /// Recreate an Instant from a unix timestamp (seconds precision)
    fn instant_from_unix_timestamp(timestamp_unix: i64) -> std::time::Instant {
        if timestamp_unix <= 0 {
            return std::time::Instant::now();
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let diff = if timestamp_unix >= now {
            0
        } else {
            (now - timestamp_unix) as u64
        };
        let duration = Duration::from_secs(diff);

        std::time::Instant::now()
            .checked_sub(duration)
            .unwrap_or_else(std::time::Instant::now)
    }
}

#[derive(Debug, Clone)]
pub struct BlacklistedAccountRecord {
    pub account_pubkey: String,
    pub reason: String,
    pub source: Option<String>,
    pub pool_id: Option<String>,
    pub token_mint: Option<String>,
    pub error_count: i64,
    pub first_failed_at: i64,
    pub last_failed_at: i64,
    pub added_at: i64,
}

#[derive(Debug, Clone)]
pub struct BlacklistedPoolRecord {
    pub pool_id: String,
    pub reason: String,
    pub token_mint: Option<String>,
    pub program_id: Option<String>,
    pub error_count: i64,
    pub first_failed_at: i64,
    pub last_failed_at: i64,
    pub added_at: i64,
}

// =============================================================================
// POOLS DATABASE
// =============================================================================

/// SQLite-based price history storage
#[derive(Debug)]
pub struct PoolsDatabase {
    db_path: String,
    connection: Arc<Mutex<Option<Connection>>>,
    write_queue: Option<mpsc::UnboundedSender<PriceResult>>,
    // In-memory blacklists (source of truth for runtime checks)
    blacklisted_accounts: Arc<RwLock<HashSet<String>>>,
    blacklisted_pools: Arc<RwLock<HashSet<String>>>,
}

impl Default for PoolsDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl PoolsDatabase {
    /// Create new pools database instance
    pub fn new() -> Self {
        Self {
            db_path: POOLS_DB_PATH.to_string(),
            connection: Arc::new(Mutex::new(None)),
            write_queue: None,
            blacklisted_accounts: Arc::new(RwLock::new(HashSet::new())),
            blacklisted_pools: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Initialize database and create tables
    pub async fn initialize(&mut self) -> Result<(), String> {
        // Ensure data directory exists
        if let Some(parent) = Path::new(&self.db_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        // Create database connection
        let conn = Connection::open(&self.db_path)
            .map_err(|e| format!("Failed to open pools database: {}", e))?;

        // Create price history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS price_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
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
                UNIQUE(mint, pool_address, timestamp_unix)
            )",
            [],
        )
        .map_err(|e| format!("Failed to create price_history table: {}", e))?;

        // Create indices for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_price_history_mint_timestamp 
             ON price_history(mint, timestamp_unix DESC)",
            [],
        )
        .map_err(|e| format!("Failed to create mint timestamp index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_price_history_pool_timestamp 
             ON price_history(pool_address, timestamp_unix DESC)",
            [],
        )
        .map_err(|e| format!("Failed to create pool timestamp index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_price_history_created_at 
             ON price_history(created_at)",
            [],
        )
        .map_err(|e| format!("Failed to create created_at index: {}", e))?;

        // Create blacklist_accounts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blacklist_accounts (
                account_pubkey TEXT PRIMARY KEY,
                reason TEXT NOT NULL,
                source TEXT,
                pool_id TEXT,
                token_mint TEXT,
                error_count INTEGER DEFAULT 1,
                first_failed_at INTEGER NOT NULL,
                last_failed_at INTEGER NOT NULL,
                added_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create blacklist_accounts table: {}", e))?;

        // Create blacklist_pools table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blacklist_pools (
                pool_id TEXT PRIMARY KEY,
                reason TEXT NOT NULL,
                token_mint TEXT,
                program_id TEXT,
                error_count INTEGER DEFAULT 1,
                first_failed_at INTEGER NOT NULL,
                last_failed_at INTEGER NOT NULL,
                added_at INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create blacklist_pools table: {}", e))?;

        // Create indices for blacklist tables
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_pool 
             ON blacklist_accounts(pool_id)",
            [],
        )
        .map_err(|e| format!("Failed to create blacklist_accounts pool index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_token 
             ON blacklist_accounts(token_mint)",
            [],
        )
        .map_err(|e| format!("Failed to create blacklist_accounts token index: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blacklist_pools_token 
             ON blacklist_pools(token_mint)",
            [],
        )
        .map_err(|e| format!("Failed to create blacklist_pools token index: {}", e))?;

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
        tokio::spawn(async move {
            run_database_writer(rx, db_connection).await;
        });

        logger::info(
            LogTag::PoolService,
            &format!("✅ Pools database initialized: {}", self.db_path),
        );

        // Load blacklists into memory (priority for runtime checks)

        let (account_keys, pool_keys) = {
            let connection_guard = self.connection.lock().unwrap();
            if let Some(ref conn) = *connection_guard {
                // Accounts
                let account_keys =
                    match conn.prepare("SELECT account_pubkey FROM blacklist_accounts") {
                        Ok(mut stmt) => {
                            let rows = stmt.query_map([], |row| row.get::<_, String>(0));
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
                                &format!("Failed to prepare load for blacklist_accounts: {}", e),
                            );
                            Vec::new()
                        }
                    };

                // Pools
                let pool_keys = match conn.prepare("SELECT pool_id FROM blacklist_pools") {
                    Ok(mut stmt) => {
                        let rows = stmt.query_map([], |row| row.get::<_, String>(0));
                        match rows {
                            Ok(iter) => iter.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                            Err(e) => {
                                logger::warning(
                                    LogTag::PoolService,
                                    &format!("Failed to load blacklist_pools into memory: {}", e),
                                );
                                Vec::new()
                            }
                        }
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolService,
                            &format!("Failed to prepare load for blacklist_pools: {}", e),
                        );
                        Vec::new()
                    }
                };

                (account_keys, pool_keys)
            } else {
                (Vec::new(), Vec::new())
            }
        };

        {
            let mut set = self.blacklisted_accounts.write().unwrap();
            set.clear();
            set.extend(account_keys);
        }

        {
            let mut set = self.blacklisted_pools.write().unwrap();
            set.clear();
            set.extend(pool_keys);
        }

        let acct_count = self.blacklisted_accounts.read().unwrap().len();
        let pool_count = self.blacklisted_pools.read().unwrap().len();
        logger::info(
            LogTag::PoolService,
            &format!(
                "In-memory blacklists loaded: accounts={}, pools={}",
                acct_count, pool_count
            ),
        );

        Ok(())
    }

    /// Queue a price for batched database storage
    pub async fn queue_price_for_storage(&self, price: PriceResult) -> Result<(), String> {
        if let Some(ref sender) = self.write_queue {
            sender
                .send(price)
                .map_err(|e| format!("Failed to queue price for storage: {}", e))?;
        }
        Ok(())
    }

    /// Load recent price history for a token (for cache initialization)
    pub async fn load_recent_price_history(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<PriceResult>, String> {
        let mint_owned = mint.to_string();
        let mint_for_task = mint_owned.clone();
        let conn_arc = self.connection.clone();
        let limit_value = limit as i64;

        let mut results = tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let mut stmt = conn
                .prepare(
                    "SELECT * FROM price_history 
                 WHERE mint = ? 
                 ORDER BY timestamp_unix DESC 
                 LIMIT ?",
                )
                .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

            let rows = stmt
                .query_map(params![mint_for_task, limit_value], |row| {
                    DbPriceResult::from_row(row)
                })
                .map_err(|e| format!("Failed to query price history: {}", e))?;

            let mut collected = Vec::new();
            for row in rows {
                let db_price = row.map_err(|e| format!("Failed to parse price row: {}", e))?;
                collected.push(db_price.to_price_result());
            }

            collected.reverse();
            Ok::<_, String>(collected)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;

        logger::debug(
            LogTag::PoolCache,
            &format!(
                "Loaded {} price history entries for token: {}",
                results.len(),
                mint_owned
            ),
        );

        Ok(results)
    }

    /// Get price history for a token with optional time range
    pub async fn get_price_history(
        &self,
        mint: &str,
        limit: Option<usize>,
        since_timestamp: Option<i64>,
    ) -> Result<Vec<PriceResult>, String> {
        let mint_owned = mint.to_string();
        let conn_arc = self.connection.clone();
        let limit_value = limit.unwrap_or(1000) as i64;

        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let mut results = Vec::new();

            if let Some(since) = since_timestamp {
                let mut stmt = conn
                    .prepare(
                        "SELECT * FROM price_history 
                     WHERE mint = ? AND timestamp_unix >= ? 
                     ORDER BY timestamp_unix DESC 
                     LIMIT ?",
                    )
                    .map_err(|e| format!("Failed to prepare history query: {}", e))?;

                let rows = stmt
                    .query_map(params![mint_owned.as_str(), since, limit_value], |row| {
                        DbPriceResult::from_row(row)
                    })
                    .map_err(|e| format!("Failed to query price history: {}", e))?;

                for row in rows {
                    let db_price = row.map_err(|e| format!("Failed to parse price row: {}", e))?;
                    results.push(db_price.to_price_result());
                }
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT * FROM price_history 
                     WHERE mint = ? 
                     ORDER BY timestamp_unix DESC 
                     LIMIT ?",
                    )
                    .map_err(|e| format!("Failed to prepare history query: {}", e))?;

                let rows = stmt
                    .query_map(params![mint_owned.as_str(), limit_value], |row| {
                        DbPriceResult::from_row(row)
                    })
                    .map_err(|e| format!("Failed to query price history: {}", e))?;

                for row in rows {
                    let db_price = row.map_err(|e| format!("Failed to parse price row: {}", e))?;
                    results.push(db_price.to_price_result());
                }
            }

            results.reverse();
            Ok::<_, String>(results)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }

    /// Cleanup old price history entries
    pub async fn cleanup_old_entries(&self) -> Result<usize, String> {
        let conn_arc = self.connection.clone();
        let deleted = tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let cutoff_timestamp = Utc::now() - chrono::Duration::days(MAX_PRICE_HISTORY_AGE_DAYS);
            let cutoff_unix = cutoff_timestamp.timestamp();

            conn.execute(
                "DELETE FROM price_history WHERE timestamp_unix < ?",
                params![cutoff_unix],
            )
            .map_err(|e| format!("Failed to cleanup old entries: {}", e))
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;

        if deleted > 0 {
            logger::debug(
                LogTag::PoolService,
                &format!("Cleaned up {} old price history entries", deleted),
            );
        }

        Ok(deleted)
    }

    /// Remove price history entries older than the most recent gap for a specific token
    pub async fn cleanup_gapped_data_for_token(&self, mint: &str) -> Result<usize, String> {
        let mint_string = mint.to_string();
        let mint_for_task = mint_string.clone();
        let conn_arc = self.connection.clone();

        let deleted = tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let cutoff = Self::find_continuous_data_start_timestamp(conn, mint_for_task.as_str())?;

            if let Some(cutoff_timestamp) = cutoff {
                let deleted = conn
                    .execute(
                        "DELETE FROM price_history WHERE mint = ? AND timestamp_unix < ?",
                        params![mint_for_task.as_str(), cutoff_timestamp],
                    )
                    .map_err(|e| {
                        format!(
                            "Failed to cleanup gapped data for token {}: {}",
                            mint_for_task, e
                        )
                    })?;

                Ok::<_, String>(deleted)
            } else {
                Ok::<_, String>(0)
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;

        if deleted > 0 {
            logger::debug(
                LogTag::PoolCache,
                &format!(
                    "Removed {} gapped price entries for token: {}",
                    deleted, mint_string
                ),
            );
        }

        Ok(deleted)
    }

    /// Find the timestamp where continuous data starts (no gaps > 1 minute) for a token
    fn find_continuous_data_start_timestamp(
        conn: &Connection,
        mint: &str,
    ) -> Result<Option<i64>, String> {
        // Get all timestamps for the token, ordered by newest first
        let mut stmt = conn
            .prepare(
                "SELECT timestamp_unix FROM price_history 
                 WHERE mint = ? 
                 ORDER BY timestamp_unix DESC",
            )
            .map_err(|e| format!("Failed to prepare gap detection query: {}", e))?;

        let rows = stmt
            .query_map(
                params![mint],
                |row| Ok(row.get::<_, i64>("timestamp_unix")?),
            )
            .map_err(|e| format!("Failed to execute gap detection query: {}", e))?;

        let mut timestamps = Vec::new();
        for row in rows {
            timestamps.push(row.map_err(|e| format!("Failed to parse timestamp: {}", e))?);
        }

        if timestamps.len() <= 1 {
            return Ok(None); // Not enough data to detect gaps
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

        // Get all unique tokens in the database
        let tokens = tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let mut stmt = conn
                .prepare("SELECT DISTINCT mint FROM price_history")
                .map_err(|e| format!("Failed to prepare token list query: {}", e))?;

            let rows = stmt
                .query_map([], |row| Ok(row.get::<_, String>("mint")?))
                .map_err(|e| format!("Failed to execute token list query: {}", e))?;

            let mut tokens = Vec::new();
            for row in rows {
                tokens.push(row.map_err(|e| format!("Failed to parse token mint: {}", e))?);
            }

            Ok::<_, String>(tokens)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))??;

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
                        &format!("Failed to cleanup gapped data for token {}: {}", token, e),
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

// =============================================================================
// BACKGROUND TASKS
// =============================================================================

/// Background task for batched database writes
async fn run_database_writer(
    mut rx: mpsc::UnboundedReceiver<PriceResult>,
    db_connection: Arc<Mutex<Option<Connection>>>,
) {
    let mut write_buffer = Vec::with_capacity(DB_BATCH_SIZE);
    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(DB_WRITE_INTERVAL_SECONDS));

    loop {
        tokio::select! {
            // Collect prices from queue
            price = rx.recv() => {
                match price {
                    Some(price) => {
                        write_buffer.push(price);

                        // Flush if buffer is full
                        if write_buffer.len() >= DB_BATCH_SIZE {
                            flush_write_buffer(&mut write_buffer, &db_connection).await;
                        }
                    }
                    None => {
                        // Channel closed, flush remaining and exit
                        flush_write_buffer(&mut write_buffer, &db_connection).await;
                        break;
                    }
                }
            }

            // Periodic flush
            _ = interval.tick() => {
                if !write_buffer.is_empty() {
                    flush_write_buffer(&mut write_buffer, &db_connection).await;
                }
            }
        }
    }
}

/// Flush the write buffer to database
async fn flush_write_buffer(
    buffer: &mut Vec<PriceResult>,
    db_connection: &Arc<Mutex<Option<Connection>>>,
) {
    if buffer.is_empty() {
        return;
    }

    let entries: Vec<PriceResult> = buffer.drain(..).collect();
    let entries_for_task = entries.clone();
    let conn_arc = db_connection.clone();

    match tokio::task::spawn_blocking(move || {
        let connection_guard = conn_arc
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let conn = match connection_guard.as_ref() {
            Some(conn) => conn,
            None => return Ok::<usize, String>(0),
        };

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start price history transaction: {}", e))?;

        let mut inserted = 0usize;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO price_history 
                     (mint, pool_address, price_usd, price_sol, confidence, slot, 
                      timestamp_unix, sol_reserves, token_reserves, source_pool, created_at) 
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .map_err(|e| format!("Failed to prepare price history insert: {}", e))?;

            for price in &entries_for_task {
                let db_price = DbPriceResult::from_price_result(price);

                inserted += stmt
                    .execute(params![
                        db_price.mint,
                        db_price.pool_address,
                        db_price.price_usd,
                        db_price.price_sol,
                        db_price.confidence,
                        db_price.slot,
                        db_price.timestamp_unix,
                        db_price.sol_reserves,
                        db_price.token_reserves,
                        db_price.source_pool,
                        db_price.created_at.to_rfc3339()
                    ])
                    .map_err(|e| format!("Failed to insert price history entry: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit price history transaction: {}", e))?;

        Ok::<usize, String>(inserted)
    })
    .await
    .map_err(|e| format!("Blocking task failed: {}", e))
    {
        Ok(Ok(inserted)) => {
            if inserted > 0 {
                logger::debug(
                    LogTag::PoolCache,
                    &format!("Stored {} price history entries to database", inserted),
                );
            }
        }
        Ok(Err(err)) => {
            buffer.extend(entries.into_iter());
            logger::error(
                LogTag::PoolCache,
                &format!("Failed to persist price history batch: {}", err),
            );
        }
        Err(join_err) => {
            buffer.extend(entries.into_iter());
            logger::error(
                LogTag::PoolCache,
                &format!("Price history writer task panicked: {}", join_err),
            );
        }
    }
}

// =============================================================================
// BLACKLIST OPERATIONS
// =============================================================================

impl PoolsDatabase {
    /// Add account to blacklist
    pub async fn add_account_to_blacklist(
        &self,
        account_pubkey: &str,
        reason: &str,
        source: Option<&str>,
        pool_id: Option<&str>,
        token_mint: Option<&str>,
    ) -> Result<(), String> {
        let account_key = account_pubkey.to_string();
        let reason_str = reason.to_string();
        let source_str = source.map(|s| s.to_string());
        let pool_id_str = pool_id.map(|s| s.to_string());
        let token_mint_str = token_mint.map(|s| s.to_string());
        // Update memory immediately
        {
            let mut set = self.blacklisted_accounts.write().unwrap();
            set.insert(account_key.clone());
        }

        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            if let Some(ref conn) = *conn_guard {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                // Check if already exists
                let exists: bool = conn
                    .query_row(
                        "SELECT 1 FROM blacklist_accounts WHERE account_pubkey = ?1",
                        params![&account_key],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if exists {
                    // Increment error count and update last_failed_at
                    conn.execute(
                        "UPDATE blacklist_accounts 
                         SET error_count = error_count + 1, last_failed_at = ?1 
                         WHERE account_pubkey = ?2",
                        params![now, &account_key],
                    )
                    .map_err(|e| format!("Failed to update blacklist_accounts: {}", e))?;
                } else {
                    // Insert new entry
                    conn.execute(
                        "INSERT INTO blacklist_accounts 
                         (account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at) 
                         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, ?6)",
                        params![&account_key, &reason_str, source_str.as_deref(), pool_id_str.as_deref(), token_mint_str.as_deref(), now],
                    )
                    .map_err(|e| format!("Failed to insert into blacklist_accounts: {}", e))?;
                }

                Ok(())
            } else {
                Err("Database connection not available".to_string())
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }

    /// Check if account is blacklisted
    pub async fn is_account_blacklisted(&self, account_pubkey: &str) -> Result<bool, String> {
        // Hot path: memory only
        let set = self.blacklisted_accounts.read().unwrap();
        Ok(set.contains(account_pubkey))
    }

    /// Add pool to blacklist
    pub async fn add_pool_to_blacklist(
        &self,
        pool_id: &str,
        reason: &str,
        token_mint: Option<&str>,
        program_id: Option<&str>,
    ) -> Result<(), String> {
        let pool_id_str = pool_id.to_string();
        let reason_str = reason.to_string();
        let token_mint_str = token_mint.map(|s| s.to_string());
        let program_id_str = program_id.map(|s| s.to_string());
        // Update memory immediately
        {
            let mut set = self.blacklisted_pools.write().unwrap();
            set.insert(pool_id_str.clone());
        }

        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            if let Some(ref conn) = *conn_guard {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                // Check if already exists
                let exists: bool = conn
                    .query_row(
                        "SELECT 1 FROM blacklist_pools WHERE pool_id = ?1",
                        params![&pool_id_str],
                        |_| Ok(true),
                    )
                    .unwrap_or(false);

                if exists {
                    // Increment error count and update last_failed_at
                    conn.execute(
                        "UPDATE blacklist_pools 
                         SET error_count = error_count + 1, last_failed_at = ?1 
                         WHERE pool_id = ?2",
                        params![now, &pool_id_str],
                    )
                    .map_err(|e| format!("Failed to update blacklist_pools: {}", e))?;
                } else {
                    // Insert new entry
                    conn.execute(
                        "INSERT INTO blacklist_pools 
                         (pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at) 
                         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5, ?5)",
                        params![&pool_id_str, &reason_str, token_mint_str.as_deref(), program_id_str.as_deref(), now],
                    )
                    .map_err(|e| format!("Failed to insert into blacklist_pools: {}", e))?;
                }

                Ok(())
            } else {
                Err("Database connection not available".to_string())
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }

    /// Check if pool is blacklisted
    pub async fn is_pool_blacklisted(&self, pool_id: &str) -> Result<bool, String> {
        // Hot path: memory only
        let set = self.blacklisted_pools.read().unwrap();
        Ok(set.contains(pool_id))
    }

    /// Remove account from blacklist
    pub async fn remove_account_from_blacklist(&self, account_pubkey: &str) -> Result<(), String> {
        // Update memory immediately
        {
            let mut set = self.blacklisted_accounts.write().unwrap();
            set.remove(account_pubkey);
        }
        // Persist
        let account_key = account_pubkey.to_string();
        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;
            if let Some(ref conn) = *conn_guard {
                conn.execute(
                    "DELETE FROM blacklist_accounts WHERE account_pubkey = ?1",
                    params![&account_key],
                )
                .map_err(|e| format!("Failed to remove from blacklist_accounts: {}", e))?;
                Ok(())
            } else {
                Err("Database connection not available".to_string())
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }

    /// Remove pool from blacklist
    pub async fn remove_pool_from_blacklist(&self, pool_id: &str) -> Result<(), String> {
        // Update memory immediately
        {
            let mut set = self.blacklisted_pools.write().unwrap();
            set.remove(pool_id);
        }
        // Persist
        let pool_key = pool_id.to_string();
        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let conn_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;
            if let Some(ref conn) = *conn_guard {
                conn.execute(
                    "DELETE FROM blacklist_pools WHERE pool_id = ?1",
                    params![&pool_key],
                )
                .map_err(|e| format!("Failed to remove from blacklist_pools: {}", e))?;
                Ok(())
            } else {
                Err("Database connection not available".to_string())
            }
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }

    /// Get blacklist statistics
    pub async fn get_blacklist_stats(&self) -> Result<(usize, usize), String> {
        let accounts = self.blacklisted_accounts.read().unwrap().len();
        let pools = self.blacklisted_pools.read().unwrap().len();
        Ok((accounts, pools))
    }

    pub async fn list_blacklisted_accounts(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BlacklistedAccountRecord>, String> {
        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let mut records = Vec::new();

            if let Some(limit_value) = limit.map(|l| l as i64) {
                let mut stmt = conn
                    .prepare(
                        "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
                         FROM blacklist_accounts \
                         ORDER BY last_failed_at DESC \
                         LIMIT ?",
                    )
                    .map_err(|e| format!("Failed to prepare blacklist_accounts query: {}", e))?;

                let rows = stmt
                    .query_map(params![limit_value], |row| {
                        Ok(BlacklistedAccountRecord {
                            account_pubkey: row.get(0)?,
                            reason: row.get(1)?,
                            source: row.get(2)?,
                            pool_id: row.get(3)?,
                            token_mint: row.get(4)?,
                            error_count: row.get(5)?,
                            first_failed_at: row.get(6)?,
                            last_failed_at: row.get(7)?,
                            added_at: row.get(8)?,
                        })
                    })
                    .map_err(|e| format!("Failed to query blacklist_accounts: {}", e))?;

                for row in rows {
                    records.push(row.map_err(|e| format!("Failed to read blacklist_accounts row: {}", e))?);
                }
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at \
                         FROM blacklist_accounts \
                         ORDER BY last_failed_at DESC",
                    )
                    .map_err(|e| format!("Failed to prepare blacklist_accounts query: {}", e))?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok(BlacklistedAccountRecord {
                            account_pubkey: row.get(0)?,
                            reason: row.get(1)?,
                            source: row.get(2)?,
                            pool_id: row.get(3)?,
                            token_mint: row.get(4)?,
                            error_count: row.get(5)?,
                            first_failed_at: row.get(6)?,
                            last_failed_at: row.get(7)?,
                            added_at: row.get(8)?,
                        })
                    })
                    .map_err(|e| format!("Failed to query blacklist_accounts: {}", e))?;

                for row in rows {
                    records.push(row.map_err(|e| format!("Failed to read blacklist_accounts row: {}", e))?);
                }
            }

            Ok::<_, String>(records)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }

    pub async fn list_blacklisted_pools(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<BlacklistedPoolRecord>, String> {
        let conn_arc = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let connection_guard = conn_arc
                .lock()
                .map_err(|e| format!("Failed to lock connection: {}", e))?;

            let conn = connection_guard
                .as_ref()
                .ok_or_else(|| "Database not initialized".to_string())?;

            let mut records = Vec::new();

            if let Some(limit_value) = limit.map(|l| l as i64) {
                let mut stmt = conn
                    .prepare(
                        "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
                         FROM blacklist_pools \
                         ORDER BY last_failed_at DESC \
                         LIMIT ?",
                    )
                    .map_err(|e| format!("Failed to prepare blacklist_pools query: {}", e))?;

                let rows = stmt
                    .query_map(params![limit_value], |row| {
                        Ok(BlacklistedPoolRecord {
                            pool_id: row.get(0)?,
                            reason: row.get(1)?,
                            token_mint: row.get(2)?,
                            program_id: row.get(3)?,
                            error_count: row.get(4)?,
                            first_failed_at: row.get(5)?,
                            last_failed_at: row.get(6)?,
                            added_at: row.get(7)?,
                        })
                    })
                    .map_err(|e| format!("Failed to query blacklist_pools: {}", e))?;

                for row in rows {
                    records.push(row.map_err(|e| format!("Failed to read blacklist_pools row: {}", e))?);
                }
            } else {
                let mut stmt = conn
                    .prepare(
                        "SELECT pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at \
                         FROM blacklist_pools \
                         ORDER BY last_failed_at DESC",
                    )
                    .map_err(|e| format!("Failed to prepare blacklist_pools query: {}", e))?;

                let rows = stmt
                    .query_map([], |row| {
                        Ok(BlacklistedPoolRecord {
                            pool_id: row.get(0)?,
                            reason: row.get(1)?,
                            token_mint: row.get(2)?,
                            program_id: row.get(3)?,
                            error_count: row.get(4)?,
                            first_failed_at: row.get(5)?,
                            last_failed_at: row.get(6)?,
                            added_at: row.get(7)?,
                        })
                    })
                    .map_err(|e| format!("Failed to query blacklist_pools: {}", e))?;

                for row in rows {
                    records.push(row.map_err(|e| format!("Failed to read blacklist_pools row: {}", e))?);
                }
            }

            Ok::<_, String>(records)
        })
        .await
        .map_err(|e| format!("Blocking task failed: {}", e))?
    }
}

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

/// Global database instance
static mut GLOBAL_POOLS_DB: Option<PoolsDatabase> = None;

/// Initialize the global pools database
pub async fn initialize_database() -> Result<(), String> {
    unsafe {
        let mut db = PoolsDatabase::new();
        db.initialize().await?;
        GLOBAL_POOLS_DB = Some(db);
    }
    Ok(())
}

/// Queue a price for storage in the global database
pub async fn queue_price_for_storage(price: PriceResult) -> Result<(), String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.queue_price_for_storage(price).await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Load recent price history for cache initialization
pub async fn load_historical_data_for_token(mint: &str) -> Result<Vec<PriceResult>, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.load_recent_price_history(mint, PRICE_HISTORY_MAX_ENTRIES)
                .await
        } else {
            Ok(Vec::new()) // Return empty if DB not available
        }
    }
}

/// Get extended price history from database
pub async fn get_extended_price_history(
    mint: &str,
    limit: Option<usize>,
    since_timestamp: Option<i64>,
) -> Result<Vec<PriceResult>, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.get_price_history(mint, limit, since_timestamp).await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Cleanup old database entries
pub async fn cleanup_old_entries() -> Result<usize, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.cleanup_old_entries().await
        } else {
            Ok(0)
        }
    }
}

/// Cleanup gapped data for a specific token
pub async fn cleanup_gapped_data_for_token(mint: &str) -> Result<usize, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.cleanup_gapped_data_for_token(mint).await
        } else {
            Ok(0)
        }
    }
}

/// Cleanup gapped data for all tokens
pub async fn cleanup_all_gapped_data() -> Result<usize, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.cleanup_all_gapped_data().await
        } else {
            Ok(0)
        }
    }
}

/// Add account to blacklist (global helper)
pub async fn add_account_to_blacklist(
    account_pubkey: &str,
    reason: &str,
    source: Option<&str>,
    pool_id: Option<&str>,
    token_mint: Option<&str>,
) -> Result<(), String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.add_account_to_blacklist(account_pubkey, reason, source, pool_id, token_mint)
                .await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Check if account is blacklisted (global helper)
pub async fn is_account_blacklisted(account_pubkey: &str) -> Result<bool, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.is_account_blacklisted(account_pubkey).await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Add pool to blacklist (global helper)
pub async fn add_pool_to_blacklist(
    pool_id: &str,
    reason: &str,
    token_mint: Option<&str>,
    program_id: Option<&str>,
) -> Result<(), String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.add_pool_to_blacklist(pool_id, reason, token_mint, program_id)
                .await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Check if pool is blacklisted (global helper)
pub async fn is_pool_blacklisted(pool_id: &str) -> Result<bool, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.is_pool_blacklisted(pool_id).await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Get blacklist statistics (global helper)
pub async fn get_blacklist_stats() -> Result<(usize, usize), String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.get_blacklist_stats().await
        } else {
            Ok((0, 0))
        }
    }
}

pub async fn list_blacklisted_accounts(
    limit: Option<usize>,
) -> Result<Vec<BlacklistedAccountRecord>, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.list_blacklisted_accounts(limit).await
        } else {
            Ok(Vec::new())
        }
    }
}

pub async fn list_blacklisted_pools(
    limit: Option<usize>,
) -> Result<Vec<BlacklistedPoolRecord>, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.list_blacklisted_pools(limit).await
        } else {
            Ok(Vec::new())
        }
    }
}
