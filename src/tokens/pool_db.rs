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

/// Maximum age for pool metadata cache (10 minutes as per requirements)
const POOL_METADATA_CACHE_TTL_HOURS: i64 = 24; // Keep for 24 hours, but mark stale after 10 minutes

/// Pool metadata cache staleness threshold (10 minutes)
const POOL_METADATA_STALE_MINUTES: i64 = 10;

// =============================================================================
// DATABASE STRUCTURES
// =============================================================================

/// Comprehensive pool metadata entry for database storage
/// This stores ALL pool information found from APIs for each token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbPoolMetadata {
    pub id: Option<i64>,
    pub token_mint: String,
    pub pool_address: String,
    pub dex_id: String,
    pub chain_id: String,
    pub pool_type: Option<String>,
    pub base_token_address: String,
    pub base_token_symbol: Option<String>,
    pub base_token_name: Option<String>,
    pub quote_token_address: String,
    pub quote_token_symbol: Option<String>,
    pub quote_token_name: Option<String>,
    pub price_native: Option<f64>,
    pub price_usd: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,
    pub volume_24h: Option<f64>,
    pub volume_6h: Option<f64>,
    pub volume_1h: Option<f64>,
    pub txns_24h_buys: Option<i64>,
    pub txns_24h_sells: Option<i64>,
    pub txns_6h_buys: Option<i64>,
    pub txns_6h_sells: Option<i64>,
    pub txns_1h_buys: Option<i64>,
    pub txns_1h_sells: Option<i64>,
    pub price_change_24h: Option<f64>,
    pub price_change_6h: Option<f64>,
    pub price_change_1h: Option<f64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub pair_created_at: Option<DateTime<Utc>>,
    pub labels: Option<String>, // JSON string of labels array
    pub url: Option<String>,
    pub source: String, // "dexscreener", "raydium", "orca", etc.
    pub is_active: bool, // Whether the pool is currently active
    pub first_seen: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub last_verified: DateTime<Utc>, // When we last confirmed this pool exists
}

impl DbPoolMetadata {
    /// Create new pool metadata entry
    pub fn new(
        token_mint: &str,
        pool_address: &str,
        dex_id: &str,
        chain_id: &str,
        source: &str
    ) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            token_mint: token_mint.to_string(),
            pool_address: pool_address.to_string(),
            dex_id: dex_id.to_string(),
            chain_id: chain_id.to_string(),
            pool_type: None,
            base_token_address: token_mint.to_string(),
            base_token_symbol: None,
            base_token_name: None,
            quote_token_address: String::new(),
            quote_token_symbol: None,
            quote_token_name: None,
            price_native: None,
            price_usd: None,
            liquidity_usd: None,
            liquidity_base: None,
            liquidity_quote: None,
            volume_24h: None,
            volume_6h: None,
            volume_1h: None,
            txns_24h_buys: None,
            txns_24h_sells: None,
            txns_6h_buys: None,
            txns_6h_sells: None,
            txns_1h_buys: None,
            txns_1h_sells: None,
            price_change_24h: None,
            price_change_6h: None,
            price_change_1h: None,
            fdv: None,
            market_cap: None,
            pair_created_at: None,
            labels: None,
            url: None,
            source: source.to_string(),
            is_active: true,
            first_seen: now,
            last_updated: now,
            last_verified: now,
        }
    }

    /// Check if the pool metadata is stale (older than 10 minutes)
    pub fn is_stale(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age.num_minutes() > POOL_METADATA_STALE_MINUTES
    }

    /// Check if the pool metadata is expired (older than 24 hours)
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age.num_hours() > POOL_METADATA_CACHE_TTL_HOURS
    }

    /// Update the last_verified timestamp
    pub fn mark_verified(&mut self) {
        self.last_verified = Utc::now();
    }

    /// Update the last_updated timestamp
    pub fn mark_updated(&mut self) {
        self.last_updated = Utc::now();
    }
}

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

        // Create pool metadata table for comprehensive pool caching
        conn
            .execute(
                "CREATE TABLE IF NOT EXISTS pool_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                dex_id TEXT NOT NULL,
                chain_id TEXT NOT NULL,
                pool_type TEXT,
                base_token_address TEXT NOT NULL,
                base_token_symbol TEXT,
                base_token_name TEXT,
                quote_token_address TEXT NOT NULL,
                quote_token_symbol TEXT,
                quote_token_name TEXT,
                price_native REAL,
                price_usd REAL,
                liquidity_usd REAL,
                liquidity_base REAL,
                liquidity_quote REAL,
                volume_24h REAL,
                volume_6h REAL,
                volume_1h REAL,
                txns_24h_buys INTEGER,
                txns_24h_sells INTEGER,
                txns_6h_buys INTEGER,
                txns_6h_sells INTEGER,
                txns_1h_buys INTEGER,
                txns_1h_sells INTEGER,
                price_change_24h REAL,
                price_change_6h REAL,
                price_change_1h REAL,
                fdv REAL,
                market_cap REAL,
                pair_created_at TEXT,
                labels TEXT,
                url TEXT,
                source TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                first_seen TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                last_verified TEXT NOT NULL
            )",
                []
            )
            .map_err(|e| format!("Failed to create pool_metadata table: {}", e))?;

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

        // Create indexes for efficient queries on pool metadata
        conn
            .execute(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_pool_metadata_unique 
             ON pool_metadata(token_mint, pool_address)",
                []
            )
            .map_err(|e| format!("Failed to create unique pool metadata index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_pool_metadata_token_updated 
             ON pool_metadata(token_mint, last_updated DESC)",
                []
            )
            .map_err(|e| format!("Failed to create token_updated index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_pool_metadata_dex_liquidity 
             ON pool_metadata(dex_id, liquidity_usd DESC)",
                []
            )
            .map_err(|e| format!("Failed to create dex_liquidity index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_pool_metadata_active_updated 
             ON pool_metadata(is_active, last_updated DESC)",
                []
            )
            .map_err(|e| format!("Failed to create active_updated index: {}", e))?;

        // Create indexes for efficient queries on price history
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
        let filtered_entries = self.filter_entries_by_gaps(entries, token_mint);

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
    // POOL METADATA OPERATIONS
    // =============================================================================

    /// Store or update pool metadata entry
    pub fn store_or_update_pool_metadata(&self, entry: &DbPoolMetadata) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        // Use INSERT OR REPLACE to handle both new and existing pools
        conn
            .execute(
                "INSERT OR REPLACE INTO pool_metadata (
                token_mint, pool_address, dex_id, chain_id, pool_type,
                base_token_address, base_token_symbol, base_token_name,
                quote_token_address, quote_token_symbol, quote_token_name,
                price_native, price_usd, liquidity_usd, liquidity_base, liquidity_quote,
                volume_24h, volume_6h, volume_1h,
                txns_24h_buys, txns_24h_sells, txns_6h_buys, txns_6h_sells,
                txns_1h_buys, txns_1h_sells,
                price_change_24h, price_change_6h, price_change_1h,
                fdv, market_cap, pair_created_at, labels, url, source,
                is_active, first_seen, last_updated, last_verified
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                     ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                     ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38)",
                params![
                    entry.token_mint,
                    entry.pool_address,
                    entry.dex_id,
                    entry.chain_id,
                    entry.pool_type,
                    entry.base_token_address,
                    entry.base_token_symbol,
                    entry.base_token_name,
                    entry.quote_token_address,
                    entry.quote_token_symbol,
                    entry.quote_token_name,
                    entry.price_native,
                    entry.price_usd,
                    entry.liquidity_usd,
                    entry.liquidity_base,
                    entry.liquidity_quote,
                    entry.volume_24h,
                    entry.volume_6h,
                    entry.volume_1h,
                    entry.txns_24h_buys,
                    entry.txns_24h_sells,
                    entry.txns_6h_buys,
                    entry.txns_6h_sells,
                    entry.txns_1h_buys,
                    entry.txns_1h_sells,
                    entry.price_change_24h,
                    entry.price_change_6h,
                    entry.price_change_1h,
                    entry.fdv,
                    entry.market_cap,
                    entry.pair_created_at.map(|dt| dt.to_rfc3339()),
                    entry.labels,
                    entry.url,
                    entry.source,
                    if entry.is_active {
                        1
                    } else {
                        0
                    },
                    entry.first_seen.to_rfc3339(),
                    entry.last_updated.to_rfc3339(),
                    entry.last_verified.to_rfc3339()
                ]
            )
            .map_err(|e| format!("Failed to store pool metadata: {}", e))?;

        Ok(())
    }

    /// Store multiple pool metadata entries in a transaction (for batch loading)
    pub fn store_pool_metadata_batch(&self, entries: &[DbPoolMetadata]) -> Result<(), String> {
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
                    "INSERT OR REPLACE INTO pool_metadata (
                    token_mint, pool_address, dex_id, chain_id, pool_type,
                    base_token_address, base_token_symbol, base_token_name,
                    quote_token_address, quote_token_symbol, quote_token_name,
                    price_native, price_usd, liquidity_usd, liquidity_base, liquidity_quote,
                    volume_24h, volume_6h, volume_1h,
                    txns_24h_buys, txns_24h_sells, txns_6h_buys, txns_6h_sells,
                    txns_1h_buys, txns_1h_sells,
                    price_change_24h, price_change_6h, price_change_1h,
                    fdv, market_cap, pair_created_at, labels, url, source,
                    is_active, first_seen, last_updated, last_verified
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                         ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30,
                         ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38)",
                    params![
                        entry.token_mint,
                        entry.pool_address,
                        entry.dex_id,
                        entry.chain_id,
                        entry.pool_type,
                        entry.base_token_address,
                        entry.base_token_symbol,
                        entry.base_token_name,
                        entry.quote_token_address,
                        entry.quote_token_symbol,
                        entry.quote_token_name,
                        entry.price_native,
                        entry.price_usd,
                        entry.liquidity_usd,
                        entry.liquidity_base,
                        entry.liquidity_quote,
                        entry.volume_24h,
                        entry.volume_6h,
                        entry.volume_1h,
                        entry.txns_24h_buys,
                        entry.txns_24h_sells,
                        entry.txns_6h_buys,
                        entry.txns_6h_sells,
                        entry.txns_1h_buys,
                        entry.txns_1h_sells,
                        entry.price_change_24h,
                        entry.price_change_6h,
                        entry.price_change_1h,
                        entry.fdv,
                        entry.market_cap,
                        entry.pair_created_at.map(|dt| dt.to_rfc3339()),
                        entry.labels,
                        entry.url,
                        entry.source,
                        if entry.is_active {
                            1
                        } else {
                            0
                        },
                        entry.first_seen.to_rfc3339(),
                        entry.last_updated.to_rfc3339(),
                        entry.last_verified.to_rfc3339()
                    ]
                )
                .map_err(|e| format!("Failed to store pool metadata in batch: {}", e))?;
        }

        tx
            .commit()
            .map_err(|e| format!("Failed to commit pool metadata batch transaction: {}", e))?;

        // Aggregate source information for better logging
        let mut source_counts: std::collections::HashMap<
            String,
            usize
        > = std::collections::HashMap::new();
        for entry in entries {
            *source_counts.entry(entry.source.clone()).or_insert(0) += 1;
        }

        let source_summary: Vec<String> = source_counts
            .into_iter()
            .map(|(source, count)| format!("{}: {}", source, count))
            .collect();

        log(
            LogTag::Pool,
            "BATCH_STORED",
            &format!(
                "✅ Stored {} pool metadata entries [{}]",
                entries.len(),
                source_summary.join(", ")
            )
        );

        Ok(())
    }

    /// Get all pools for a token (including stale ones, ordered by freshness)
    pub fn get_pools_for_token(&self, token_mint: &str) -> Result<Vec<DbPoolMetadata>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let mut stmt = conn
            .prepare(
                "SELECT id, token_mint, pool_address, dex_id, chain_id, pool_type,
                        base_token_address, base_token_symbol, base_token_name,
                        quote_token_address, quote_token_symbol, quote_token_name,
                        price_native, price_usd, liquidity_usd, liquidity_base, liquidity_quote,
                        volume_24h, volume_6h, volume_1h,
                        txns_24h_buys, txns_24h_sells, txns_6h_buys, txns_6h_sells,
                        txns_1h_buys, txns_1h_sells,
                        price_change_24h, price_change_6h, price_change_1h,
                        fdv, market_cap, pair_created_at, labels, url, source,
                        is_active, first_seen, last_updated, last_verified
                 FROM pool_metadata 
                 WHERE token_mint = ?1 AND is_active = 1
                 ORDER BY liquidity_usd DESC NULLS LAST, last_updated DESC"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let pool_iter = stmt
            .query_map([token_mint], |row| {
                let pair_created_at_str: Option<String> = row.get(31)?;
                let pair_created_at = pair_created_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let first_seen_str: String = row.get(36)?;
                let first_seen = DateTime::parse_from_rfc3339(&first_seen_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            36,
                            "first_seen".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                let last_updated_str: String = row.get(37)?;
                let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            37,
                            "last_updated".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                let last_verified_str: String = row.get(38)?;
                let last_verified = DateTime::parse_from_rfc3339(&last_verified_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            38,
                            "last_verified".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok(DbPoolMetadata {
                    id: Some(row.get(0)?),
                    token_mint: row.get(1)?,
                    pool_address: row.get(2)?,
                    dex_id: row.get(3)?,
                    chain_id: row.get(4)?,
                    pool_type: row.get(5)?,
                    base_token_address: row.get(6)?,
                    base_token_symbol: row.get(7)?,
                    base_token_name: row.get(8)?,
                    quote_token_address: row.get(9)?,
                    quote_token_symbol: row.get(10)?,
                    quote_token_name: row.get(11)?,
                    price_native: row.get(12)?,
                    price_usd: row.get(13)?,
                    liquidity_usd: row.get(14)?,
                    liquidity_base: row.get(15)?,
                    liquidity_quote: row.get(16)?,
                    volume_24h: row.get(17)?,
                    volume_6h: row.get(18)?,
                    volume_1h: row.get(19)?,
                    txns_24h_buys: row.get(20)?,
                    txns_24h_sells: row.get(21)?,
                    txns_6h_buys: row.get(22)?,
                    txns_6h_sells: row.get(23)?,
                    txns_1h_buys: row.get(24)?,
                    txns_1h_sells: row.get(25)?,
                    price_change_24h: row.get(26)?,
                    price_change_6h: row.get(27)?,
                    price_change_1h: row.get(28)?,
                    fdv: row.get(29)?,
                    market_cap: row.get(30)?,
                    pair_created_at,
                    labels: row.get(32)?,
                    url: row.get(33)?,
                    source: row.get(34)?,
                    is_active: row.get::<_, i64>(35)? == 1,
                    first_seen,
                    last_updated,
                    last_verified,
                })
            })
            .map_err(|e| format!("Failed to query pools for token: {}", e))?;

        let mut pools = Vec::new();
        for pool_result in pool_iter {
            let pool = pool_result.map_err(|e| format!("Failed to parse pool metadata: {}", e))?;
            pools.push(pool);
        }

        Ok(pools)
    }

    /// Get fresh pools for a token (not stale, within 10 minutes)
    pub fn get_fresh_pools_for_token(
        &self,
        token_mint: &str
    ) -> Result<Vec<DbPoolMetadata>, String> {
        let all_pools = self.get_pools_for_token(token_mint)?;
        let fresh_pools = all_pools
            .into_iter()
            .filter(|pool| !pool.is_stale())
            .collect();
        Ok(fresh_pools)
    }

    /// Get the best pool for a token (highest liquidity, freshest data)
    pub fn get_best_pool_for_token(
        &self,
        token_mint: &str
    ) -> Result<Option<DbPoolMetadata>, String> {
        let pools = self.get_fresh_pools_for_token(token_mint)?;

        // Return the pool with highest liquidity (query already orders by liquidity DESC)
        Ok(pools.into_iter().next())
    }

    /// Get stale pools that need updating (older than 10 minutes)
    pub fn get_stale_pools(&self, limit: usize) -> Result<Vec<DbPoolMetadata>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let stale_cutoff = Utc::now() - chrono::Duration::minutes(POOL_METADATA_STALE_MINUTES);

        let mut stmt = conn
            .prepare(
                "SELECT id, token_mint, pool_address, dex_id, chain_id, pool_type,
                        base_token_address, base_token_symbol, base_token_name,
                        quote_token_address, quote_token_symbol, quote_token_name,
                        price_native, price_usd, liquidity_usd, liquidity_base, liquidity_quote,
                        volume_24h, volume_6h, volume_1h,
                        txns_24h_buys, txns_24h_sells, txns_6h_buys, txns_6h_sells,
                        txns_1h_buys, txns_1h_sells,
                        price_change_24h, price_change_6h, price_change_1h,
                        fdv, market_cap, pair_created_at, labels, url, source,
                        is_active, first_seen, last_updated, last_verified
                 FROM pool_metadata 
                 WHERE is_active = 1 AND last_updated < ?1
                 ORDER BY last_updated ASC
                 LIMIT ?2"
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let pool_iter = stmt
            .query_map([stale_cutoff.to_rfc3339(), limit.to_string()], |row| {
                let pair_created_at_str: Option<String> = row.get(31)?;
                let pair_created_at = pair_created_at_str
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let first_seen_str: String = row.get(36)?;
                let first_seen = DateTime::parse_from_rfc3339(&first_seen_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            36,
                            "first_seen".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                let last_updated_str: String = row.get(37)?;
                let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            37,
                            "last_updated".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                let last_verified_str: String = row.get(38)?;
                let last_verified = DateTime::parse_from_rfc3339(&last_verified_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            38,
                            "last_verified".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok(DbPoolMetadata {
                    id: Some(row.get(0)?),
                    token_mint: row.get(1)?,
                    pool_address: row.get(2)?,
                    dex_id: row.get(3)?,
                    chain_id: row.get(4)?,
                    pool_type: row.get(5)?,
                    base_token_address: row.get(6)?,
                    base_token_symbol: row.get(7)?,
                    base_token_name: row.get(8)?,
                    quote_token_address: row.get(9)?,
                    quote_token_symbol: row.get(10)?,
                    quote_token_name: row.get(11)?,
                    price_native: row.get(12)?,
                    price_usd: row.get(13)?,
                    liquidity_usd: row.get(14)?,
                    liquidity_base: row.get(15)?,
                    liquidity_quote: row.get(16)?,
                    volume_24h: row.get(17)?,
                    volume_6h: row.get(18)?,
                    volume_1h: row.get(19)?,
                    txns_24h_buys: row.get(20)?,
                    txns_24h_sells: row.get(21)?,
                    txns_6h_buys: row.get(22)?,
                    txns_6h_sells: row.get(23)?,
                    txns_1h_buys: row.get(24)?,
                    txns_1h_sells: row.get(25)?,
                    price_change_24h: row.get(26)?,
                    price_change_6h: row.get(27)?,
                    price_change_1h: row.get(28)?,
                    fdv: row.get(29)?,
                    market_cap: row.get(30)?,
                    pair_created_at,
                    labels: row.get(32)?,
                    url: row.get(33)?,
                    source: row.get(34)?,
                    is_active: row.get::<_, i64>(35)? == 1,
                    first_seen,
                    last_updated,
                    last_verified,
                })
            })
            .map_err(|e| format!("Failed to query stale pools: {}", e))?;

        let mut pools = Vec::new();
        for pool_result in pool_iter {
            let pool = pool_result.map_err(|e| format!("Failed to parse pool metadata: {}", e))?;
            pools.push(pool);
        }

        Ok(pools)
    }

    /// Mark a pool as inactive (soft delete)
    pub fn mark_pool_inactive(&self, pool_address: &str) -> Result<(), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        conn
            .execute(
                "UPDATE pool_metadata SET is_active = 0, last_updated = ?1 WHERE pool_address = ?2",
                params![Utc::now().to_rfc3339(), pool_address]
            )
            .map_err(|e| format!("Failed to mark pool inactive: {}", e))?;

        Ok(())
    }

    /// Get pool metadata statistics
    pub fn get_pool_metadata_statistics(&self) -> Result<(usize, usize, usize, usize), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let total_pools: usize = conn
            .query_row("SELECT COUNT(*) FROM pool_metadata", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count total pools: {}", e))?;

        let active_pools: usize = conn
            .query_row("SELECT COUNT(*) FROM pool_metadata WHERE is_active = 1", [], |row|
                row.get(0)
            )
            .map_err(|e| format!("Failed to count active pools: {}", e))?;

        let stale_cutoff = Utc::now() - chrono::Duration::minutes(POOL_METADATA_STALE_MINUTES);
        let fresh_pools: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM pool_metadata WHERE is_active = 1 AND last_updated > ?1",
                [stale_cutoff.to_rfc3339()],
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to count fresh pools: {}", e))?;

        let unique_tokens: usize = conn
            .query_row(
                "SELECT COUNT(DISTINCT token_mint) FROM pool_metadata WHERE is_active = 1",
                [],
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to count unique tokens: {}", e))?;

        Ok((total_pools, active_pools, fresh_pools, unique_tokens))
    }

    /// Clean up expired pool metadata entries
    pub fn cleanup_expired_pool_metadata(&self) -> Result<usize, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open database: {}", e)
        )?;

        let expired_cutoff = Utc::now() - chrono::Duration::hours(POOL_METADATA_CACHE_TTL_HOURS);

        let deleted_count = conn
            .execute("DELETE FROM pool_metadata WHERE last_updated <= ?1", [
                expired_cutoff.to_rfc3339(),
            ])
            .map_err(|e| format!("Failed to cleanup expired pool metadata: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::Pool,
                "CLEANUP",
                &format!("🧹 Cleaned up {} expired pool metadata entries", deleted_count)
            );
        }

        Ok(deleted_count)
    }

    // =============================================================================
    // GAP DETECTION AND CLEANUP
    // =============================================================================

    /// Filter price entries by gaps - removes entries older than significant gaps
    fn filter_entries_by_gaps(
        &self,
        entries: Vec<(DateTime<Utc>, f64)>,
        token_mint: &str
    ) -> Vec<(DateTime<Utc>, f64)> {
        if entries.len() <= 1 {
            return entries;
        }

        let max_gap_duration = chrono::Duration::minutes(MAX_HISTORY_GAP_MINUTES);
        let mut filtered = Vec::new();

        // Find the first significant gap from the end (most recent)
        let mut gap_found_at = None;

        // Iterate backwards through entries (entries are ordered oldest to newest)
        // When iterating backwards: entries[i-1] is NEWER, entries[i] is OLDER
        for i in (1..entries.len()).rev() {
            let older_time = entries[i].0; // Older timestamp
            let newer_time = entries[i - 1].0; // Newer timestamp
            let gap = newer_time - older_time; // Newer - Older = Positive gap duration

            if gap > max_gap_duration {
                gap_found_at = Some(i);
                let token_display = crate::utils::safe_truncate(token_mint, 8);
                log(
                    LogTag::Pool,
                    "GAP_DETECTED",
                    &format!(
                        "Token {} - {:.1} minute gap between {} and {} (keeping {} entries after gap, removing {} older entries)",
                        token_display,
                        gap.num_minutes() as f64,
                        older_time.format("%H:%M:%S"),
                        newer_time.format("%H:%M:%S"),
                        entries.len() - i,
                        i
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

// =============================================================================
// POOL METADATA CONVENIENCE FUNCTIONS
// =============================================================================

/// Store or update pool metadata (global function)
pub fn store_or_update_pool_metadata(pool_metadata: &DbPoolMetadata) -> Result<(), String> {
    let service = get_pool_db_service()?;
    service.store_or_update_pool_metadata(pool_metadata)
}

/// Store multiple pool metadata entries (global function)
pub fn store_pool_metadata_batch(pool_metadata_entries: &[DbPoolMetadata]) -> Result<(), String> {
    let service = get_pool_db_service()?;
    service.store_pool_metadata_batch(pool_metadata_entries)
}

/// Get all pools for a token (global function)
pub fn get_pools_for_token(token_mint: &str) -> Result<Vec<DbPoolMetadata>, String> {
    let service = get_pool_db_service()?;
    service.get_pools_for_token(token_mint)
}

/// Get fresh pools for a token (global function)
pub fn get_fresh_pools_for_token(token_mint: &str) -> Result<Vec<DbPoolMetadata>, String> {
    let service = get_pool_db_service()?;
    service.get_fresh_pools_for_token(token_mint)
}

/// Get the best pool for a token (global function)
pub fn get_best_pool_for_token(token_mint: &str) -> Result<Option<DbPoolMetadata>, String> {
    let service = get_pool_db_service()?;
    service.get_best_pool_for_token(token_mint)
}

/// Get stale pools that need updating (global function)
pub fn get_stale_pools(limit: usize) -> Result<Vec<DbPoolMetadata>, String> {
    let service = get_pool_db_service()?;
    service.get_stale_pools(limit)
}

/// Mark a pool as inactive (global function)
pub fn mark_pool_inactive(pool_address: &str) -> Result<(), String> {
    let service = get_pool_db_service()?;
    service.mark_pool_inactive(pool_address)
}

/// Get pool metadata statistics (global function)
pub fn get_pool_metadata_statistics() -> Result<(usize, usize, usize, usize), String> {
    let service = get_pool_db_service()?;
    service.get_pool_metadata_statistics()
}

/// Clean up expired pool metadata entries (global function)
pub fn cleanup_expired_pool_metadata() -> Result<usize, String> {
    let service = get_pool_db_service()?;
    service.cleanup_expired_pool_metadata()
}

// =============================================================================
// API DATA CONVERSION HELPERS
// =============================================================================

/// Convert DexScreener TokenPair to DbPoolMetadata
pub fn token_pair_to_pool_metadata(
    token_pair: &crate::tokens::dexscreener::TokenPair,
    source: &str
) -> Result<DbPoolMetadata, String> {
    let mut pool_metadata = DbPoolMetadata::new(
        &token_pair.base_token.address,
        &token_pair.pair_address,
        &token_pair.dex_id,
        &token_pair.chain_id,
        source
    );

    // Set token information
    pool_metadata.base_token_symbol = Some(token_pair.base_token.symbol.clone());
    pool_metadata.base_token_name = Some(token_pair.base_token.name.clone());
    pool_metadata.quote_token_address = token_pair.quote_token.address.clone();
    pool_metadata.quote_token_symbol = Some(token_pair.quote_token.symbol.clone());
    pool_metadata.quote_token_name = Some(token_pair.quote_token.name.clone());

    // Set price information
    if let Ok(price_native) = token_pair.price_native.parse::<f64>() {
        pool_metadata.price_native = Some(price_native);
    }

    if let Some(price_usd_str) = &token_pair.price_usd {
        if let Ok(price_usd) = price_usd_str.parse::<f64>() {
            pool_metadata.price_usd = Some(price_usd);
        }
    }

    // Set liquidity information
    if let Some(liquidity) = &token_pair.liquidity {
        pool_metadata.liquidity_usd = Some(liquidity.usd);
        pool_metadata.liquidity_base = Some(liquidity.base);
        pool_metadata.liquidity_quote = Some(liquidity.quote);
    }

    // Set volume information
    pool_metadata.volume_24h = token_pair.volume.h24;
    pool_metadata.volume_6h = token_pair.volume.h6;
    pool_metadata.volume_1h = token_pair.volume.h1;

    // Set transaction information
    pool_metadata.txns_24h_buys = token_pair.txns.h24.as_ref().and_then(|period| period.buys);
    pool_metadata.txns_24h_sells = token_pair.txns.h24.as_ref().and_then(|period| period.sells);
    pool_metadata.txns_6h_buys = token_pair.txns.h6.as_ref().and_then(|period| period.buys);
    pool_metadata.txns_6h_sells = token_pair.txns.h6.as_ref().and_then(|period| period.sells);
    pool_metadata.txns_1h_buys = token_pair.txns.h1.as_ref().and_then(|period| period.buys);
    pool_metadata.txns_1h_sells = token_pair.txns.h1.as_ref().and_then(|period| period.sells);

    // Set price change information
    pool_metadata.price_change_24h = token_pair.price_change.h24;
    pool_metadata.price_change_6h = token_pair.price_change.h6;
    pool_metadata.price_change_1h = token_pair.price_change.h1;

    // Set market data
    pool_metadata.fdv = token_pair.fdv;
    pool_metadata.market_cap = token_pair.market_cap;

    // Set creation date if available
    if let Some(created_timestamp) = token_pair.pair_created_at {
        if let Some(created_dt) = chrono::DateTime::from_timestamp(created_timestamp as i64, 0) {
            pool_metadata.pair_created_at = Some(created_dt.with_timezone(&Utc));
        }
    }

    // Set labels as JSON string
    if let Some(labels) = &token_pair.labels {
        if !labels.is_empty() {
            if let Ok(labels_json) = serde_json::to_string(labels) {
                pool_metadata.labels = Some(labels_json);
            }
        }
    }

    // Set URL
    pool_metadata.url = Some(token_pair.url.clone());

    Ok(pool_metadata)
}

/// Convert multiple TokenPairs to DbPoolMetadata entries
pub fn token_pairs_to_pool_metadata_batch(
    token_pairs: &[crate::tokens::dexscreener::TokenPair],
    source: &str
) -> Vec<DbPoolMetadata> {
    token_pairs
        .iter()
        .filter_map(|pair| {
            match token_pair_to_pool_metadata(pair, source) {
                Ok(metadata) => Some(metadata),
                Err(e) => {
                    log(
                        LogTag::Pool,
                        "CONVERSION_ERROR",
                        &format!("Failed to convert token pair {}: {}", pair.pair_address, e)
                    );
                    None
                }
            }
        })
        .collect()
}

/// Store pools from DexScreener API response (convenience function)
pub fn store_pools_from_dexscreener_response(
    token_pairs: &[crate::tokens::dexscreener::TokenPair]
) -> Result<usize, String> {
    if token_pairs.is_empty() {
        return Ok(0);
    }

    let pool_metadata_entries = token_pairs_to_pool_metadata_batch(token_pairs, "dexscreener");
    let count = pool_metadata_entries.len();

    if count > 0 {
        store_pool_metadata_batch(&pool_metadata_entries)?;
        // Note: store_pool_metadata_batch already logs BATCH_STORED, no need for duplicate API_STORED log
    }

    Ok(count)
}

/// Get pool statistics summary (convenience function)
pub fn get_pool_cache_summary() -> Result<String, String> {
    let (total_pools, active_pools, fresh_pools, unique_tokens) = get_pool_metadata_statistics()?;
    let (total_price_entries, recent_price_entries, unique_price_tokens) =
        get_pool_db_statistics()?;

    let summary = format!(
        "Pool Database Summary:\n\
         • Pool Metadata: {} total, {} active, {} fresh (< 10min), {} unique tokens\n\
         • Price History: {} total entries, {} recent (< 24h), {} unique tokens",
        total_pools,
        active_pools,
        fresh_pools,
        unique_tokens,
        total_price_entries,
        recent_price_entries,
        unique_price_tokens
    );

    Ok(summary)
}

/// Get all pool addresses from database (for debug purposes)
pub fn get_all_pool_addresses(limit: usize) -> Result<Vec<String>, String> {
    let service = get_pool_db_service()?;

    let conn = Connection::open(&service.db_path).map_err(|e|
        format!("Failed to open database: {}", e)
    )?;

    let query =
        "SELECT pool_address FROM pool_metadata WHERE is_active = 1 ORDER BY liquidity_usd DESC LIMIT ?1";

    let mut stmt = conn.prepare(query).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt
        .query_map([limit], |row| { Ok(row.get::<_, String>(0)?) })
        .map_err(|e| format!("Failed to execute query: {}", e))?;

    let mut addresses = Vec::new();
    for row in rows {
        addresses.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
    }

    Ok(addresses)
}

/// Get all token mints that have pools in the database (global function)
pub fn get_all_tokens_with_pools() -> Result<Vec<String>, String> {
    let service = get_pool_db_service()?;
    let conn = Connection::open(&service.db_path).map_err(|e|
        format!("Failed to open database: {}", e)
    )?;

    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT token_mint FROM pool_metadata WHERE is_active = 1 ORDER BY token_mint"
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map([], |row| { Ok(row.get::<_, String>(0)?) })
        .map_err(|e| format!("Failed to execute query: {}", e))?;

    let mut tokens = Vec::new();
    for row in rows {
        tokens.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
    }

    Ok(tokens)
}
