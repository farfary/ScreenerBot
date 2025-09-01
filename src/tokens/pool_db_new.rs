use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use rusqlite::{ params, Connection, Row };
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Database file path
const POOLS_DB_PATH: &str = "data/pools.db";

/// Maximum age for price history entries (24 hours)
const MAX_PRICE_HISTORY_AGE_HOURS: i64 = 24;

/// Maximum gap allowed in price history (10 minutes)
/// If there's a gap longer than this, older entries are considered stale
const MAX_HISTORY_GAP_MINUTES: i64 = 10;

// =============================================================================
// DATABASE STRUCTURES
// =============================================================================

/// Price history entry for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbPriceHistoryEntry {
    pub id: Option<i64>,
    pub token_mint: String,
    pub pool_address: String,
    pub dex_id: String,
    pub pool_type: Option<String>,
    pub price_sol: f64,
    pub price_usd: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub source: String,
    pub timestamp: DateTime<Utc>,
}

impl DbPriceHistoryEntry {
    /// Create from price data
    pub fn new(
        token_mint: &str,
        pool_address: &str,
        dex_id: &str,
        pool_type: Option<String>,
        price_sol: f64,
        price_usd: Option<f64>,
        liquidity_usd: Option<f64>,
        volume_24h: Option<f64>,
        source: &str
    ) -> Self {
        Self {
            id: None,
            token_mint: token_mint.to_string(),
            pool_address: pool_address.to_string(),
            dex_id: dex_id.to_string(),
            pool_type,
            price_sol,
            price_usd,
            liquidity_usd,
            volume_24h,
            source: source.to_string(),
            timestamp: Utc::now(),
        }
    }

    /// Convert to (timestamp, price) tuple for compatibility
    pub fn to_history_tuple(&self) -> (DateTime<Utc>, f64) {
        (self.timestamp, self.price_sol)
    }
}

// =============================================================================
// MAIN POOL DATABASE SERVICE
// =============================================================================

pub struct PoolDbService {
    db_path: String,
}

impl PoolDbService {
    /// Create new pool database service
    pub fn new() -> Self {
        Self {
            db_path: POOLS_DB_PATH.to_string(),
        }
    }

    /// Initialize database and create tables
    pub fn initialize(&self) -> Result<(), String> {
        // Ensure data directory exists
        if let Some(parent) = Path::new(&self.db_path).parent() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        // Create price history table
        conn
            .execute(
                "CREATE TABLE IF NOT EXISTS price_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                dex_id TEXT NOT NULL,
                pool_type TEXT,
                price_sol REAL NOT NULL,
                price_usd REAL,
                liquidity_usd REAL,
                volume_24h REAL,
                source TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
                []
            )
            .map_err(|e| format!("Failed to create price_history table: {}", e))?;

        // Create indexes for efficient queries
        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_price_history_token_timestamp 
             ON price_history(token_mint, timestamp DESC)",
                []
            )
            .map_err(|e| format!("Failed to create token_timestamp index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_price_history_timestamp 
             ON price_history(timestamp DESC)",
                []
            )
            .map_err(|e| format!("Failed to create timestamp index: {}", e))?;

        log(LogTag::Pool, "DB_INIT", "✅ Pool database initialized successfully");
        Ok(())
    }

    // =============================================================================
    // PRICE HISTORY OPERATIONS
    // =============================================================================

    /// Store price history entry
    pub fn store_price_entry(&self, entry: &DbPriceHistoryEntry) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        conn
            .execute(
                "INSERT INTO price_history (
                token_mint, pool_address, dex_id, pool_type, price_sol, 
                price_usd, liquidity_usd, volume_24h, source, timestamp
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    entry.token_mint,
                    entry.pool_address,
                    entry.dex_id,
                    entry.pool_type,
                    entry.price_sol,
                    entry.price_usd,
                    entry.liquidity_usd,
                    entry.volume_24h,
                    entry.source,
                    entry.timestamp.to_rfc3339()
                ]
            )
            .map_err(|e| format!("Failed to store price entry: {}", e))?;

        Ok(())
    }

    /// Store multiple price entries in a transaction (for batch loading)
    pub fn store_price_entries_batch(&self, entries: &[DbPriceHistoryEntry]) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }

        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        for entry in entries {
            tx
                .execute(
                    "INSERT INTO price_history (
                    token_mint, pool_address, dex_id, pool_type, price_sol, 
                    price_usd, liquidity_usd, volume_24h, source, timestamp
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        entry.token_mint,
                        entry.pool_address,
                        entry.dex_id,
                        entry.pool_type,
                        entry.price_sol,
                        entry.price_usd,
                        entry.liquidity_usd,
                        entry.volume_24h,
                        entry.source,
                        entry.timestamp.to_rfc3339()
                    ]
                )
                .map_err(|e| format!("Failed to store price entry in batch: {}", e))?;
        }

        tx.commit().map_err(|e| format!("Failed to commit batch transaction: {}", e))?;

        Ok(())
    }

    /// Get price history for a token with gap detection
    /// Returns only continuous price history (removes stale data if gaps found)
    pub fn get_price_history_with_gap_detection(
        &self,
        token_mint: &str
    ) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let mut stmt = conn
            .prepare(
                "SELECT timestamp, price_sol FROM price_history 
                 WHERE token_mint = ?1 
                 AND timestamp > datetime('now', '-24 hours')
                 ORDER BY timestamp DESC"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let entry_iter = stmt
            .query_map([token_mint], |row| {
                let timestamp_str: String = row.get(0)?;
                let price_sol: f64 = row.get(1)?;

                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            0,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok((timestamp, price_sol))
            })
            .map_err(|e| format!("Failed to query price history: {}", e))?;

        let mut entries: Vec<(DateTime<Utc>, f64)> = Vec::new();
        for entry_result in entry_iter {
            let entry = entry_result.map_err(|e| format!("Failed to parse price entry: {}", e))?;
            entries.push(entry);
        }

        // Entries are in DESC order (newest first), reverse for gap detection
        entries.reverse();

        // Apply gap detection - remove entries older than significant gaps
        let filtered_entries = self.filter_entries_by_gaps(entries);

        // Return in DESC order (newest first) for consistency
        let mut result = filtered_entries;
        result.reverse();

        Ok(result)
    }

    /// Get detailed price history for a token (with metadata)
    pub fn get_detailed_price_history(
        &self,
        token_mint: &str
    ) -> Result<Vec<DbPriceHistoryEntry>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let mut stmt = conn
            .prepare(
                "SELECT token_mint, pool_address, dex_id, pool_type, price_sol, 
                        price_usd, liquidity_usd, volume_24h, source, timestamp
                 FROM price_history 
                 WHERE token_mint = ?1 
                 AND timestamp > datetime('now', '-24 hours')
                 ORDER BY timestamp DESC"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let entry_iter = stmt
            .query_map([token_mint], |row| {
                let timestamp_str: String = row.get(9)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            9,
                            "timestamp".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok(DbPriceHistoryEntry {
                    id: None,
                    token_mint: row.get(0)?,
                    pool_address: row.get(1)?,
                    dex_id: row.get(2)?,
                    pool_type: row.get(3)?,
                    price_sol: row.get(4)?,
                    price_usd: row.get(5)?,
                    liquidity_usd: row.get(6)?,
                    volume_24h: row.get(7)?,
                    source: row.get(8)?,
                    timestamp,
                })
            })
            .map_err(|e| format!("Failed to query detailed price history: {}", e))?;

        let mut entries = Vec::new();
        for entry_result in entry_iter {
            let entry = entry_result.map_err(|e|
                format!("Failed to parse detailed price entry: {}", e)
            )?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get list of tokens that have price history
    pub fn get_tokens_with_history(&self) -> Result<Vec<String>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT token_mint FROM price_history 
                 WHERE timestamp > datetime('now', '-24 hours')"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let token_iter = stmt
            .query_map([], |row| { Ok(row.get::<_, String>(0)?) })
            .map_err(|e| format!("Failed to query tokens with history: {}", e))?;

        let mut tokens = Vec::new();
        for token_result in token_iter {
            let token = token_result.map_err(|e| format!("Failed to parse token: {}", e))?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    // =============================================================================
    // GAP DETECTION AND CLEANUP
    // =============================================================================

    /// Filter price entries by gaps - removes entries older than significant gaps
    fn filter_entries_by_gaps(
        &self,
        entries: Vec<(DateTime<Utc>, f64)>
    ) -> Vec<(DateTime<Utc>, f64)> {
        if entries.len() <= 1 {
            return entries;
        }

        let max_gap_duration = chrono::Duration::minutes(MAX_HISTORY_GAP_MINUTES);
        let mut filtered = Vec::new();

        // Find the first significant gap from the end (most recent)
        let mut gap_found_at = None;

        for i in (1..entries.len()).rev() {
            let current_time = entries[i].0;
            let previous_time = entries[i - 1].0;
            let gap = current_time - previous_time;

            if gap > max_gap_duration {
                gap_found_at = Some(i);
                log(
                    LogTag::Pool,
                    "GAP_DETECTED",
                    &format!(
                        "Found {:.1} minute gap in price history, keeping only entries after gap",
                        gap.num_minutes() as f64
                    )
                );
                break;
            }
        }

        // If gap found, keep only entries after the gap
        if let Some(gap_index) = gap_found_at {
            filtered.extend_from_slice(&entries[gap_index..]);
        } else {
            // No significant gaps, keep all entries
            filtered = entries;
        }

        filtered
    }

    /// Clean up old price history entries
    pub fn cleanup_old_entries(&self) -> Result<usize, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let deleted_count = conn
            .execute(
                "DELETE FROM price_history 
                 WHERE timestamp <= datetime('now', '-24 hours')",
                []
            )
            .map_err(|e| format!("Failed to cleanup old entries: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::Pool,
                "CLEANUP",
                &format!("🧹 Cleaned up {} old price history entries", deleted_count)
            );
        }

        Ok(deleted_count)
    }

    /// Get database statistics
    pub fn get_statistics(&self) -> Result<(usize, usize, usize), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let total_entries: usize = conn
            .query_row("SELECT COUNT(*) FROM price_history", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count total entries: {}", e))?;

        let recent_entries: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM price_history WHERE timestamp > datetime('now', '-24 hours')",
                [],
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to count recent entries: {}", e))?;

        let unique_tokens: usize = conn
            .query_row(
                "SELECT COUNT(DISTINCT token_mint) FROM price_history WHERE timestamp > datetime('now', '-24 hours')",
                [],
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to count unique tokens: {}", e))?;

        Ok((total_entries, recent_entries, unique_tokens))
    }

    /// Vacuum database to optimize performance
    pub fn vacuum(&self) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        conn.execute("VACUUM", []).map_err(|e| format!("Failed to vacuum database: {}", e))?;

        log(LogTag::Pool, "VACUUM", "🔧 Database vacuum completed");
        Ok(())
    }
}

// =============================================================================
// GLOBAL DATABASE SERVICE
// =============================================================================

static mut GLOBAL_POOL_DB_SERVICE: Option<PoolDbService> = None;
static POOL_DB_INIT: std::sync::Once = std::sync::Once::new();

/// Initialize global pool database service (idempotent)
pub fn init_pool_db_service() -> Result<&'static PoolDbService, String> {
    POOL_DB_INIT.call_once(|| {
        let service = PoolDbService::new();
        if let Err(e) = service.initialize() {
            log(
                LogTag::Pool,
                "DB_INIT_ERROR",
                &format!("Failed to initialize pool database: {}", e)
            );
            return;
        }
        unsafe {
            GLOBAL_POOL_DB_SERVICE = Some(service);
        }
    });

    unsafe {
        GLOBAL_POOL_DB_SERVICE.as_ref().ok_or_else(||
            "Pool database service not initialized".to_string()
        )
    }
}

/// Get global pool database service
pub fn get_pool_db_service() -> Result<&'static PoolDbService, String> {
    unsafe {
        GLOBAL_POOL_DB_SERVICE.as_ref().ok_or_else(||
            "Pool database service not initialized. Call init_pool_db_service() first.".to_string()
        )
    }
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Store price entry (global function)
pub fn store_price_entry(
    token_mint: &str,
    pool_address: &str,
    dex_id: &str,
    pool_type: Option<String>,
    price_sol: f64,
    price_usd: Option<f64>,
    liquidity_usd: Option<f64>,
    volume_24h: Option<f64>,
    source: &str
) -> Result<(), String> {
    let service = get_pool_db_service()?;
    let entry = DbPriceHistoryEntry::new(
        token_mint,
        pool_address,
        dex_id,
        pool_type,
        price_sol,
        price_usd,
        liquidity_usd,
        volume_24h,
        source
    );
    service.store_price_entry(&entry)
}

/// Get price history for a token with gap detection (global function)
pub fn get_price_history_for_token(token_mint: &str) -> Result<Vec<(DateTime<Utc>, f64)>, String> {
    let service = get_pool_db_service()?;
    service.get_price_history_with_gap_detection(token_mint)
}

/// Get all tokens with price history (global function)
pub fn get_tokens_with_price_history() -> Result<Vec<String>, String> {
    let service = get_pool_db_service()?;
    service.get_tokens_with_history()
}

/// Clean up old entries (global function)
pub fn cleanup_old_price_entries() -> Result<usize, String> {
    let service = get_pool_db_service()?;
    service.cleanup_old_entries()
}

/// Get database statistics (global function)
pub fn get_pool_db_statistics() -> Result<(usize, usize, usize), String> {
    let service = get_pool_db_service()?;
    service.get_statistics()
}
