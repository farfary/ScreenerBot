/// Database module for persistent price history storage
///
/// This module provides SQLite-based storage for price history data,
/// enabling price history to survive service restarts and providing
/// full historical data access beyond the in-memory cache limits.
use super::types::{PriceResult, PRICE_HISTORY_MAX_ENTRIES};

use crate::logger::{self, LogTag};

use chrono::{DateTime, Utc};
use rusqlite::{params, types::Type, Connection, Row};
use serde_json;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, Mutex};

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
        // Convert Instant to Unix timestamp (approximation)
        let timestamp_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

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
            timestamp: std::time::Instant::now(), // Approximation
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
}

/// Persistent blacklist record for pools with missing or invalid accounts
#[derive(Debug, Clone)]
pub struct PoolBlacklistRecord {
    pub pool_id: String,
    pub token_mint: String,
    pub reason: String,
    pub missing_accounts: Vec<String>,
    pub retry_count: i64,
    pub first_seen: i64,
    pub last_seen: i64,
}

impl PoolBlacklistRecord {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let missing_raw: String = row.get("missing_accounts")?;
        let missing_accounts: Vec<String> = serde_json::from_str(&missing_raw)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(e)))?;

        Ok(Self {
            pool_id: row.get("pool_id")?,
            token_mint: row.get("token_mint")?,
            reason: row.get("reason")?,
            missing_accounts,
            retry_count: row.get("retry_count")?,
            first_seen: row.get("first_seen")?,
            last_seen: row.get("last_seen")?,
        })
    }

    fn missing_accounts_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.missing_accounts)
            .map_err(|e| format!("Failed to serialize missing accounts: {}", e))
    }
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS pool_blacklist (
                pool_id TEXT PRIMARY KEY,
                token_mint TEXT NOT NULL,
                reason TEXT NOT NULL,
                missing_accounts TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                first_seen INTEGER NOT NULL,
                last_seen INTEGER NOT NULL
            )",
            [],
        )
        .map_err(|e| format!("Failed to create pool_blacklist table: {}", e))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pool_blacklist_token 
             ON pool_blacklist(token_mint)",
            [],
        )
        .map_err(|e| format!("Failed to create pool blacklist token index: {}", e))?;

        // Store connection
        let mut connection_guard = self.connection.lock().await;
        *connection_guard = Some(conn);

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

    /// Insert or update a blacklist entry for a pool
    pub async fn upsert_pool_blacklist_entry(
        &self,
        record: &PoolBlacklistRecord,
    ) -> Result<(), String> {
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        let missing_accounts = record.missing_accounts_json()?;

        conn.execute(
            "INSERT INTO pool_blacklist (pool_id, token_mint, reason, missing_accounts, retry_count, first_seen, last_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(pool_id) DO UPDATE SET
                 reason = excluded.reason,
                 missing_accounts = excluded.missing_accounts,
                 last_seen = excluded.last_seen",
            params![
                record.pool_id,
                record.token_mint,
                record.reason,
                missing_accounts,
                record.retry_count,
                record.first_seen,
                record.last_seen,
            ],
        )
        .map_err(|e| format!("Failed to upsert pool blacklist entry {}: {}", record.pool_id, e))?;

        Ok(())
    }

    /// Touch an existing blacklist entry and increment retry count
    pub async fn touch_pool_blacklist_entry(
        &self,
        pool_id: &str,
        missing_accounts: &[String],
        reason: &str,
    ) -> Result<(), String> {
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        let missing_json = serde_json::to_string(missing_accounts)
            .map_err(|e| format!("Failed to serialize missing accounts: {}", e))?;
        let last_seen = Utc::now().timestamp();

        conn.execute(
            "UPDATE pool_blacklist
             SET reason = ?,
                 missing_accounts = ?,
                 retry_count = retry_count + 1,
                 last_seen = ?
             WHERE pool_id = ?",
            params![reason, missing_json, last_seen, pool_id],
        )
        .map_err(|e| format!("Failed to update pool blacklist entry {}: {}", pool_id, e))?;

        Ok(())
    }

    /// Load all blacklist entries from disk
    pub async fn load_pool_blacklist(&self) -> Result<Vec<PoolBlacklistRecord>, String> {
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_id, token_mint, reason, missing_accounts, retry_count, first_seen, last_seen
                 FROM pool_blacklist",
            )
            .map_err(|e| format!("Failed to prepare pool blacklist query: {}", e))?;

        let rows = stmt
            .query_map([], |row| PoolBlacklistRecord::from_row(row))
            .map_err(|e| format!("Failed to query pool blacklist: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to parse pool blacklist row: {}", e))?);
        }

        Ok(results)
    }

    /// Load recent price history for a token (for cache initialization)
    pub async fn load_recent_price_history(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<PriceResult>, String> {
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        let mut stmt = conn
            .prepare(
                "SELECT * FROM price_history 
             WHERE mint = ? 
             ORDER BY timestamp_unix DESC 
             LIMIT ?",
            )
            .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

        let rows = stmt
            .query_map(params![mint, limit as i64], |row| {
                DbPriceResult::from_row(row)
            })
            .map_err(|e| format!("Failed to query price history: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            let db_price = row.map_err(|e| format!("Failed to parse price row: {}", e))?;
            results.push(db_price.to_price_result());
        }

        // Reverse to get chronological order (oldest to newest)
        results.reverse();

        logger::debug(
            LogTag::PoolCache,
            &format!(
                "Loaded {} price history entries for token: {}",
                results.len(),
                mint
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
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        let limit_value = limit.unwrap_or(1000) as i64;

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
                .query_map(params![mint, since, limit_value], |row| {
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
                .query_map(params![mint, limit_value], |row| {
                    DbPriceResult::from_row(row)
                })
                .map_err(|e| format!("Failed to query price history: {}", e))?;

            for row in rows {
                let db_price = row.map_err(|e| format!("Failed to parse price row: {}", e))?;
                results.push(db_price.to_price_result());
            }
        }

        // Reverse to get chronological order (oldest to newest)
        results.reverse();

        Ok(results)
    }

    /// Cleanup old price history entries
    pub async fn cleanup_old_entries(&self) -> Result<usize, String> {
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        let cutoff_timestamp = Utc::now() - chrono::Duration::days(MAX_PRICE_HISTORY_AGE_DAYS);
        let cutoff_unix = cutoff_timestamp.timestamp();

        let deleted = conn
            .execute(
                "DELETE FROM price_history WHERE timestamp_unix < ?",
                params![cutoff_unix],
            )
            .map_err(|e| format!("Failed to cleanup old entries: {}", e))?;

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
        let connection_guard = self.connection.lock().await;
        let conn = connection_guard
            .as_ref()
            .ok_or("Database not initialized")?;

        // Find the most recent timestamp where continuous data starts (no gaps > 1 minute)
        let continuous_start_timestamp = self.find_continuous_data_start_timestamp(conn, mint)?;

        if let Some(cutoff_timestamp) = continuous_start_timestamp {
            let deleted = conn
                .execute(
                    "DELETE FROM price_history WHERE mint = ? AND timestamp_unix < ?",
                    params![mint, cutoff_timestamp],
                )
                .map_err(|e| format!("Failed to cleanup gapped data for token {}: {}", mint, e))?;

            if deleted > 0 {
                logger::debug(
                    LogTag::PoolCache,
                    &format!(
                        "Removed {} gapped price entries for token: {}",
                        deleted, mint
                    ),
                );
            }

            Ok(deleted)
        } else {
            Ok(0) // No gaps found
        }
    }

    /// Find the timestamp where continuous data starts (no gaps > 1 minute) for a token
    fn find_continuous_data_start_timestamp(
        &self,
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
        // Get all unique tokens in the database
        let tokens = {
            let connection_guard = self.connection.lock().await;
            let conn = connection_guard
                .as_ref()
                .ok_or("Database not initialized")?;

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

            tokens
        }; // connection_guard is dropped here

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

    let connection_guard = db_connection.lock().await;
    if let Some(ref conn) = *connection_guard {
        // Begin transaction for atomicity
        if let Ok(tx) = conn.unchecked_transaction() {
            let mut insert_count = 0;

            // Prepare insert statement
            if let Ok(mut stmt) = tx.prepare(
                "INSERT OR REPLACE INTO price_history 
                 (mint, pool_address, price_usd, price_sol, confidence, slot, 
                  timestamp_unix, sol_reserves, token_reserves, source_pool, created_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            ) {
                for price in buffer.iter() {
                    let db_price = DbPriceResult::from_price_result(price);

                    if stmt
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
                        .is_ok()
                    {
                        insert_count += 1;
                    }
                }
            }

            // Commit transaction
            if tx.commit().is_ok() && insert_count > 0 {
                logger::debug(
                    LogTag::PoolCache,
                    &format!("Stored {} price history entries to database", insert_count),
                );
            }
        }
    }

    buffer.clear();
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

/// Persist or update a pool blacklist entry in the global database
pub async fn upsert_pool_blacklist_entry(record: &PoolBlacklistRecord) -> Result<(), String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.upsert_pool_blacklist_entry(record).await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Update retry metadata for a pool blacklist entry
pub async fn touch_pool_blacklist_entry(
    pool_id: &str,
    missing_accounts: &[String],
    reason: &str,
) -> Result<(), String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.touch_pool_blacklist_entry(pool_id, missing_accounts, reason)
                .await
        } else {
            Err("Database not initialized".to_string())
        }
    }
}

/// Load all pool blacklist entries from disk
pub async fn load_pool_blacklist_entries() -> Result<Vec<PoolBlacklistRecord>, String> {
    unsafe {
        if let Some(ref db) = GLOBAL_POOLS_DB {
            db.load_pool_blacklist().await
        } else {
            Ok(Vec::new())
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
