/// Price caching system for the centralized pricing module
use crate::tokens::types::*;
use crate::logger::{ log, LogTag };
use std::collections::HashMap;
use std::time::{ Duration, Instant };
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use std::fs;
use std::path::Path;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::{ Arc, Mutex };

/// Price cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceCacheEntry {
    pub price: f64,
    pub source: PriceSourceType,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
}

/// Price cache statistics
#[derive(Debug, Clone)]
pub struct PriceCacheStats {
    pub total_entries: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub last_cleanup: Option<DateTime<Utc>>,
    pub memory_usage_bytes: usize,
}

impl PriceCacheStats {
    pub fn get_hit_rate(&self) -> f64 {
        let total_requests = self.cache_hits + self.cache_misses;
        if total_requests == 0 {
            0.0
        } else {
            ((self.cache_hits as f64) / (total_requests as f64)) * 100.0
        }
    }
}

impl std::fmt::Display for PriceCacheStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entries: {}, Hit Rate: {:.1}%, Memory: {:.1}KB",
            self.total_entries,
            self.get_hit_rate(),
            (self.memory_usage_bytes as f64) / 1024.0
        )
    }
}

/// In-memory price cache with persistence
pub struct PriceCache {
    cache: HashMap<String, PriceCacheEntry>,
    stats: PriceCacheStats,
    cache_duration: Duration,
    max_entries: usize,
    cache_file: String,
    last_save: Option<Instant>,
    save_interval: Duration,
}

impl PriceCache {
    /// Create new price cache
    pub fn new() -> Self {
        let mut cache = Self {
            cache: HashMap::new(),
            stats: PriceCacheStats {
                total_entries: 0,
                cache_hits: 0,
                cache_misses: 0,
                last_cleanup: None,
                memory_usage_bytes: 0,
            },
            cache_duration: Duration::from_secs(300), // 5 minutes cache
            max_entries: 10000, // Maximum cached entries
            cache_file: "price_cache.json".to_string(),
            last_save: None,
            save_interval: Duration::from_secs(60), // Save every minute
        };

        // Try to load existing cache
        if let Err(e) = cache.load_from_file() {
            eprintln!("Warning: Failed to load price cache: {}", e);
        }

        cache
    }

    /// Get cached price for a token
    pub fn get_price(&mut self, mint: &str) -> Option<f64> {
        if let Some(entry) = self.cache.get(mint) {
            // Check if cache entry is still valid
            let age = Utc::now().signed_duration_since(entry.timestamp);

            if age.num_seconds() < (self.cache_duration.as_secs() as i64) {
                self.stats.cache_hits += 1;
                return Some(entry.price);
            } else {
                // Remove expired entry
                self.cache.remove(mint);
                self.update_memory_usage();
            }
        }

        self.stats.cache_misses += 1;
        None
    }

    /// Set price in cache
    pub fn set_price(&mut self, mint: &str, price: f64) {
        self.set_price_with_source(mint, price, PriceSourceType::DexScreenerApi, 1.0);
    }

    /// Set price in cache with detailed source information
    pub fn set_price_with_source(
        &mut self,
        mint: &str,
        price: f64,
        source: PriceSourceType,
        confidence: f64
    ) {
        let entry = PriceCacheEntry {
            price,
            source,
            timestamp: Utc::now(),
            confidence,
        };

        self.cache.insert(mint.to_string(), entry);
        self.update_stats();

        // Check if we need to cleanup
        if self.cache.len() > self.max_entries {
            self.cleanup_old_entries();
        }

        // Auto-save periodically
        self.auto_save();
    }

    /// Get multiple prices from cache
    pub fn get_multiple_prices(&mut self, mints: &[String]) -> HashMap<String, f64> {
        let mut prices = HashMap::new();

        for mint in mints {
            if let Some(price) = self.get_price(mint) {
                prices.insert(mint.clone(), price);
            }
        }

        prices
    }

    /// Set multiple prices in cache
    pub fn set_multiple_prices(&mut self, prices: HashMap<String, f64>) {
        for (mint, price) in prices {
            self.set_price(&mint, price);
        }
    }

    /// Clear all cached prices
    pub fn clear(&mut self) {
        self.cache.clear();
        self.update_stats();
    }

    /// Remove expired entries from cache
    pub fn cleanup_old_entries(&mut self) {
        let now = Utc::now();
        let cache_duration_secs = self.cache_duration.as_secs() as i64;

        self.cache.retain(|_, entry| {
            let age = now.signed_duration_since(entry.timestamp);
            age.num_seconds() < cache_duration_secs
        });

        self.stats.last_cleanup = Some(now);
        self.update_stats();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> PriceCacheStats {
        self.stats.clone()
    }

    /// Update internal statistics
    fn update_stats(&mut self) {
        self.stats.total_entries = self.cache.len();
        self.update_memory_usage();
    }

    /// Estimate memory usage
    fn update_memory_usage(&mut self) {
        let estimated_size = self.cache.len() * (44 + 32 + 64); // Rough estimate
        self.stats.memory_usage_bytes = estimated_size;
    }

    /// Auto-save cache to file periodically
    fn auto_save(&mut self) {
        let should_save = if let Some(last_save) = self.last_save {
            last_save.elapsed() >= self.save_interval
        } else {
            true
        };

        if should_save {
            if let Err(e) = self.save_to_file() {
                eprintln!("Warning: Failed to save price cache: {}", e);
            } else {
                self.last_save = Some(Instant::now());
            }
        }
    }

    /// Save cache to JSON file
    pub fn save_to_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct CacheData {
            cache: HashMap<String, PriceCacheEntry>,
            saved_at: DateTime<Utc>,
        }

        let data = CacheData {
            cache: self.cache.clone(),
            saved_at: Utc::now(),
        };

        let json = serde_json::to_string_pretty(&data)?;
        fs::write(&self.cache_file, json)?;

        Ok(())
    }

    /// Load cache from JSON file
    pub fn load_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(&self.cache_file).exists() {
            return Ok(()); // No cache file yet
        }

        #[derive(Deserialize)]
        struct CacheData {
            cache: HashMap<String, PriceCacheEntry>,
            saved_at: Option<DateTime<Utc>>,
        }

        let content = fs::read_to_string(&self.cache_file)?;
        let data: CacheData = serde_json::from_str(&content)?;

        // Only load entries that are still valid
        let now = Utc::now();
        let cache_duration_secs = self.cache_duration.as_secs() as i64;

        for (mint, entry) in data.cache {
            let age = now.signed_duration_since(entry.timestamp);
            if age.num_seconds() < cache_duration_secs {
                self.cache.insert(mint, entry);
            }
        }

        self.update_stats();
        Ok(())
    }

    /// Get cached prices sorted by timestamp (newest first)
    pub fn get_recent_prices(&self, limit: usize) -> Vec<(String, PriceCacheEntry)> {
        let mut entries: Vec<_> = self.cache
            .iter()
            .map(|(mint, entry)| (mint.clone(), entry.clone()))
            .collect();

        entries.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
        entries.truncate(limit);
        entries
    }

    /// Get prices from specific source
    pub fn get_prices_by_source(&self, source: PriceSourceType) -> HashMap<String, f64> {
        self.cache
            .iter()
            .filter_map(|(mint, entry)| {
                if std::mem::discriminant(&entry.source) == std::mem::discriminant(&source) {
                    Some((mint.clone(), entry.price))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get cache hit rate
    pub fn get_hit_rate(&self) -> f64 {
        self.stats.get_hit_rate()
    }

    /// Check if cache contains a specific token
    pub fn contains(&self, mint: &str) -> bool {
        self.cache.contains_key(mint)
    }

    /// Get age of cached entry
    pub fn get_entry_age(&self, mint: &str) -> Option<Duration> {
        self.cache.get(mint).map(|entry| {
            let age = Utc::now().signed_duration_since(entry.timestamp);
            Duration::from_secs(age.num_seconds() as u64)
        })
    }

    /// Force save cache to file
    pub fn force_save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_to_file()
    }
}

// =============================================================================
// TOKEN DATABASE (SQLite)
// =============================================================================

use rusqlite::{ Connection, params, Result as SqliteResult };
use crate::tokens::types::ApiToken;

/// SQLite database for token storage and caching
#[derive(Clone)]
pub struct TokenDatabase {
    connection: Arc<Mutex<Connection>>,
}

// Manually implement Send and Sync since Arc<Mutex<Connection>> is Send + Sync
unsafe impl Send for TokenDatabase {}
unsafe impl Sync for TokenDatabase {}

impl TokenDatabase {
    /// Create new token database instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::open("tokens.db")?;

        // Create tables if they don't exist
        connection.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                decimals INTEGER DEFAULT 9,
                chain_id TEXT NOT NULL,
                dex_id TEXT,
                pair_address TEXT,
                pair_url TEXT,
                price_native REAL NOT NULL,
                price_usd REAL NOT NULL,
                price_sol REAL,
                liquidity_usd REAL,
                liquidity_base REAL,
                liquidity_quote REAL,
                volume_h24 REAL,
                volume_h6 REAL,
                volume_h1 REAL,
                volume_m5 REAL,
                txns_h24_buys INTEGER,
                txns_h24_sells INTEGER,
                txns_h6_buys INTEGER,
                txns_h6_sells INTEGER,
                txns_h1_buys INTEGER,
                txns_h1_sells INTEGER,
                txns_m5_buys INTEGER,
                txns_m5_sells INTEGER,
                price_change_h24 REAL,
                price_change_h6 REAL,
                price_change_h1 REAL,
                price_change_m5 REAL,
                fdv REAL,
                market_cap REAL,
                pair_created_at INTEGER,
                boosts_active INTEGER,
                info_image_url TEXT,
                labels TEXT,
                last_updated TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            []
        )?;

        // Create indexes for better performance
        connection.execute("CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol)", [])?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_liquidity ON tokens(liquidity_usd DESC)",
            []
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_last_updated ON tokens(last_updated)",
            []
        )?;

        // Migration: Add decimals column if it doesn't exist (for existing databases)
        let _ = connection.execute("ALTER TABLE tokens ADD COLUMN decimals INTEGER DEFAULT 9", []);

        // Only log on first initialization - reduce log spam
        static DATABASE_INITIALIZED: std::sync::Once = std::sync::Once::new();
        DATABASE_INITIALIZED.call_once(|| {
            log(LogTag::System, "DATABASE", "Token database initialized");
        });

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Add new tokens to database
    pub async fn add_tokens(&self, tokens: &[ApiToken]) -> Result<(), Box<dyn std::error::Error>> {
        for token in tokens {
            self.insert_or_update_token(token)?;
        }

        log(LogTag::System, "DATABASE", &format!("Added/updated {} tokens", tokens.len()));

        Ok(())
    }

    /// Update existing tokens in database
    pub async fn update_tokens(
        &self,
        tokens: &[ApiToken]
    ) -> Result<(), Box<dyn std::error::Error>> {
        for token in tokens {
            self.insert_or_update_token(token)?;
        }

        log(LogTag::System, "DATABASE", &format!("Updated {} tokens", tokens.len()));

        Ok(())
    }

    /// Get all tokens from database
    pub async fn get_all_tokens(&self) -> Result<Vec<ApiToken>, Box<dyn std::error::Error>> {
        let connection = self.connection
            .lock()
            .map_err(
                |e|
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Database lock error: {}", e)
                        )
                    ) as Box<dyn std::error::Error>
            )?;
        let mut stmt = connection.prepare("SELECT * FROM tokens ORDER BY liquidity_usd DESC")?;

        let token_iter = stmt.query_map([], |row| { Ok(self.row_to_token(row)?) })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get tokens by mints
    pub async fn get_tokens_by_mints(
        &self,
        mints: &[String]
    ) -> Result<Vec<ApiToken>, Box<dyn std::error::Error>> {
        let mut tokens = Vec::new();

        for mint in mints {
            if let Some(token) = self.get_token_by_mint(mint)? {
                tokens.push(token);
            }
        }

        Ok(tokens)
    }

    /// Get single token by mint
    pub fn get_token_by_mint(
        &self,
        mint: &str
    ) -> Result<Option<ApiToken>, Box<dyn std::error::Error>> {
        let connection = self.connection
            .lock()
            .map_err(
                |e|
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Database lock error: {}", e)
                        )
                    ) as Box<dyn std::error::Error>
            )?;
        let mut stmt = connection.prepare("SELECT * FROM tokens WHERE mint = ?1")?;

        let mut rows = stmt.query_map(params![mint], |row| { Ok(self.row_to_token(row)?) })?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Insert or update token in database
    fn insert_or_update_token(&self, token: &ApiToken) -> Result<(), Box<dyn std::error::Error>> {
        let labels_json = token.labels
            .as_ref()
            .map(|labels| serde_json::to_string(labels).unwrap_or_default())
            .unwrap_or_default();

        let connection = self.connection
            .lock()
            .map_err(
                |e|
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Database lock error: {}", e)
                        )
                    ) as Box<dyn std::error::Error>
            )?;
        connection.execute(
            "INSERT OR REPLACE INTO tokens (
                mint, symbol, name, chain_id, dex_id, pair_address, pair_url,
                price_native, price_usd, price_sol,
                liquidity_usd, liquidity_base, liquidity_quote,
                volume_h24, volume_h6, volume_h1, volume_m5,
                txns_h24_buys, txns_h24_sells, txns_h6_buys, txns_h6_sells,
                txns_h1_buys, txns_h1_sells, txns_m5_buys, txns_m5_sells,
                price_change_h24, price_change_h6, price_change_h1, price_change_m5,
                fdv, market_cap, pair_created_at, boosts_active,
                info_image_url, labels, last_updated
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36
            )",
            params![
                token.mint,
                token.symbol,
                token.name,
                token.chain_id,
                token.dex_id,
                token.pair_address,
                token.pair_url,
                token.price_native,
                token.price_usd,
                token.price_sol,
                token.liquidity.as_ref().and_then(|l| l.usd),
                token.liquidity.as_ref().and_then(|l| l.base),
                token.liquidity.as_ref().and_then(|l| l.quote),
                token.volume.as_ref().and_then(|v| v.h24),
                token.volume.as_ref().and_then(|v| v.h6),
                token.volume.as_ref().and_then(|v| v.h1),
                token.volume.as_ref().and_then(|v| v.m5),
                token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h24.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h6.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.h1.as_ref().and_then(|h| h.sells)),
                token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.buys)),
                token.txns.as_ref().and_then(|t| t.m5.as_ref().and_then(|h| h.sells)),
                token.price_change.as_ref().and_then(|p| p.h24),
                token.price_change.as_ref().and_then(|p| p.h6),
                token.price_change.as_ref().and_then(|p| p.h1),
                token.price_change.as_ref().and_then(|p| p.m5),
                token.fdv,
                token.market_cap,
                token.pair_created_at,
                token.boosts.as_ref().and_then(|b| b.active),
                token.info.as_ref().and_then(|i| i.image_url.clone()),
                labels_json,
                token.last_updated.to_rfc3339()
            ]
        )?;

        Ok(())
    }

    /// Convert database row to ApiToken
    fn row_to_token(&self, row: &rusqlite::Row) -> SqliteResult<ApiToken> {
        let labels_json: String = row.get("labels")?;
        let labels = if labels_json.is_empty() {
            None
        } else {
            serde_json::from_str(&labels_json).ok()
        };

        let last_updated_str: String = row.get("last_updated")?;
        let last_updated = chrono::DateTime
            ::parse_from_rfc3339(&last_updated_str)
            .map_err(|e|
                rusqlite::Error::InvalidColumnType(
                    0,
                    "last_updated".to_string(),
                    rusqlite::types::Type::Text
                )
            )?
            .with_timezone(&chrono::Utc);

        Ok(ApiToken {
            mint: row.get("mint")?,
            symbol: row.get("symbol")?,
            name: row.get("name")?,
            chain_id: row.get("chain_id")?,
            dex_id: row.get("dex_id")?,
            pair_address: row.get("pair_address")?,
            pair_url: row.get("pair_url")?,
            price_native: row.get("price_native")?,
            price_usd: row.get("price_usd")?,
            price_sol: row.get("price_sol")?,
            liquidity: Some(crate::tokens::types::LiquidityInfo {
                usd: row.get("liquidity_usd")?,
                base: row.get("liquidity_base")?,
                quote: row.get("liquidity_quote")?,
            }),
            volume: Some(crate::tokens::types::VolumeStats {
                h24: row.get("volume_h24")?,
                h6: row.get("volume_h6")?,
                h1: row.get("volume_h1")?,
                m5: row.get("volume_m5")?,
            }),
            txns: Some(crate::tokens::types::TxnStats {
                h24: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_h24_buys")?,
                    sells: row.get("txns_h24_sells")?,
                }),
                h6: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_h6_buys")?,
                    sells: row.get("txns_h6_sells")?,
                }),
                h1: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_h1_buys")?,
                    sells: row.get("txns_h1_sells")?,
                }),
                m5: Some(crate::tokens::types::TxnPeriod {
                    buys: row.get("txns_m5_buys")?,
                    sells: row.get("txns_m5_sells")?,
                }),
            }),
            price_change: Some(crate::tokens::types::PriceChangeStats {
                h24: row.get("price_change_h24")?,
                h6: row.get("price_change_h6")?,
                h1: row.get("price_change_h1")?,
                m5: row.get("price_change_m5")?,
            }),
            fdv: row.get("fdv")?,
            market_cap: row.get("market_cap")?,
            pair_created_at: row.get("pair_created_at")?,
            boosts: Some(crate::tokens::types::BoostInfo {
                active: row.get("boosts_active")?,
            }),
            info: Some(crate::tokens::types::TokenInfo {
                image_url: row.get("info_image_url")?,
                websites: None, // Not stored in simplified schema
                socials: None, // Not stored in simplified schema
            }),
            labels,
            last_updated,
        })
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error>> {
        let connection = self.connection
            .lock()
            .map_err(
                |e|
                    Box::new(
                        std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Database lock error: {}", e)
                        )
                    ) as Box<dyn std::error::Error>
            )?;
        let mut stmt = connection.prepare("SELECT COUNT(*) FROM tokens")?;
        let total_tokens: i64 = stmt.query_row([], |row| row.get(0))?;

        let mut stmt = connection.prepare("SELECT COUNT(*) FROM tokens WHERE liquidity_usd > 100")?;
        let tokens_with_liquidity: i64 = stmt.query_row([], |row| row.get(0))?;

        Ok(DatabaseStats {
            total_tokens: total_tokens as usize,
            tokens_with_liquidity: tokens_with_liquidity as usize,
            last_updated: chrono::Utc::now(),
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_tokens: usize,
    pub tokens_with_liquidity: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

// =============================================================================
// DECIMAL CACHING FUNCTIONALITY
// =============================================================================

/// Extract decimals from a mint account's data
fn extract_decimals_from_mint_account(
    account_data: &[u8]
) -> Result<u8, Box<dyn std::error::Error>> {
    // Solana mint account layout:
    // - mint_authority: 36 bytes (32 bytes pubkey + 4 bytes COption)
    // - supply: 8 bytes
    // - decimals: 1 byte
    // - is_initialized: 1 byte
    // - freeze_authority: 36 bytes (32 bytes pubkey + 4 bytes COption)

    if account_data.len() < 82 {
        return Err("Invalid mint account data length".into());
    }

    // Decimals is at offset 44 (36 + 8)
    let decimals = account_data[44];
    Ok(decimals)
}

/// Get token decimals from database cache, returns None if not cached
/// This function creates its own database connection to avoid threading issues
pub fn get_token_decimals_cached(mint: &str) -> Option<u8> {
    // Create a temporary database connection to avoid threading issues
    if let Ok(connection) = Connection::open("tokens.db") {
        if let Ok(mut stmt) = connection.prepare("SELECT decimals FROM tokens WHERE mint = ?1") {
            if
                let Ok(decimals) = stmt.query_row([mint], |row| {
                    let decimals: i64 = row.get(0)?;
                    Ok(decimals as u8)
                })
            {
                return Some(decimals);
            }
        }
    }
    None
}

/// Update token decimals in the database
fn update_token_decimals_sync(
    db: &TokenDatabase,
    mint: &str,
    decimals: u8
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = db.connection
        .lock()
        .map_err(
            |e|
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Database lock error: {}", e)
                    )
                ) as Box<dyn std::error::Error>
        )?;

    // First try to update existing record
    let updated = connection.execute(
        "UPDATE tokens SET decimals = ?1 WHERE mint = ?2",
        params![decimals as i64, mint]
    )?;

    // If no existing record, insert a minimal record
    if updated == 0 {
        connection.execute(
            "INSERT OR IGNORE INTO tokens (mint, symbol, name, decimals, chain_id, price_native, price_usd, last_updated) 
             VALUES (?1, ?2, ?3, ?4, 'solana', 0.0, 0.0, ?5)",
            params![
                mint,
                format!("{}...{}", &mint[..4], &mint[mint.len() - 4..]),
                "Unknown Token",
                decimals as i64,
                chrono::Utc::now().to_rfc3339()
            ]
        )?;
    }
    Ok(())
}

/// Update token decimals in the database (sync version with own connection)
pub fn update_token_decimals_sync_standalone(
    mint: &str,
    decimals: u8
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary database connection to avoid threading issues
    if let Ok(connection) = Connection::open("tokens.db") {
        // First try to update existing record
        connection.execute(
            "UPDATE tokens SET decimals = ?1 WHERE mint = ?2",
            params![decimals as i64, mint]
        )?;

        // If no rows were updated, insert a new record
        connection.execute(
            "INSERT OR IGNORE INTO tokens (mint, symbol, name, decimals, chain_id, price_native, price_usd, last_updated) 
             VALUES (?1, ?2, ?3, ?4, 'solana', 0.0, 0.0, datetime('now'))",
            params![
                mint,
                "UNKNOWN", // placeholder symbol
                "UNKNOWN", // placeholder name
                decimals as i64
            ]
        )?;
    }
    Ok(())
}

/// Fetch decimals for multiple mints using getMultipleAccounts RPC call
/// Updates the database cache
pub async fn fetch_or_cache_decimals(
    rpc_client: &RpcClient,
    mints: &[String]
) -> Result<std::collections::HashMap<String, u8>, Box<dyn std::error::Error>> {
    use std::collections::HashMap;

    let mut result = HashMap::new();
    let mut mints_to_fetch = Vec::new();

    // Check database cache first
    for mint in mints {
        if let Some(decimals) = get_token_decimals_cached(mint) {
            result.insert(mint.clone(), decimals);
        } else {
            mints_to_fetch.push(mint.clone());
        }
    }

    if mints_to_fetch.is_empty() {
        return Ok(result);
    }

    log(
        LogTag::Monitor,
        "INFO",
        &format!("Fetching decimals for {} new mints from chain", mints_to_fetch.len())
    );

    // Convert mint strings to Pubkeys
    let mut valid_mints = Vec::new();
    let mut pubkeys = Vec::new();

    for mint_str in &mints_to_fetch {
        match Pubkey::from_str(mint_str) {
            Ok(pubkey) => {
                valid_mints.push(mint_str.clone());
                pubkeys.push(pubkey);
            }
            Err(e) => {
                log(LogTag::Monitor, "WARN", &format!("Invalid mint address {}: {}", mint_str, e));
                // Use default decimals of 9 for invalid addresses
                result.insert(mint_str.clone(), 9);
                if let Err(e) = update_token_decimals_sync_standalone(mint_str, 9) {
                    log(
                        LogTag::Monitor,
                        "WARN",
                        &format!("Failed to cache decimals for {}: {}", mint_str, e)
                    );
                }
            }
        }
    }

    if pubkeys.is_empty() {
        return Ok(result);
    }

    // Fetch multiple accounts in batches (max 100 per request)
    const BATCH_SIZE: usize = 100;
    let mut processed_valid_mints = 0;

    for chunk in pubkeys.chunks(BATCH_SIZE) {
        let chunk_mints: Vec<String> = valid_mints
            .iter()
            .skip(processed_valid_mints)
            .take(chunk.len())
            .cloned()
            .collect();

        match rpc_client.get_multiple_accounts(chunk) {
            Ok(accounts) => {
                for (i, account_opt) in accounts.iter().enumerate() {
                    let mint_str = &chunk_mints[i];

                    match account_opt {
                        Some(account) => {
                            match extract_decimals_from_mint_account(&account.data) {
                                Ok(decimals) => {
                                    result.insert(mint_str.clone(), decimals);
                                    if
                                        let Err(e) = update_token_decimals_sync_standalone(
                                            mint_str,
                                            decimals
                                        )
                                    {
                                        log(
                                            LogTag::Monitor,
                                            "WARN",
                                            &format!(
                                                "Failed to cache decimals for {}: {}",
                                                mint_str,
                                                e
                                            )
                                        );
                                    }
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Monitor,
                                        "WARN",
                                        &format!(
                                            "Failed to parse mint account for {}: {}, using default 9",
                                            mint_str,
                                            e
                                        )
                                    );
                                    result.insert(mint_str.clone(), 9);
                                    if
                                        let Err(e) = update_token_decimals_sync_standalone(
                                            mint_str,
                                            9
                                        )
                                    {
                                        log(
                                            LogTag::Monitor,
                                            "WARN",
                                            &format!(
                                                "Failed to cache decimals for {}: {}",
                                                mint_str,
                                                e
                                            )
                                        );
                                    }
                                }
                            }
                        }
                        None => {
                            log(
                                LogTag::Monitor,
                                "WARN",
                                &format!("Mint account not found for {}, using default 9", mint_str)
                            );
                            result.insert(mint_str.clone(), 9);
                            if let Err(e) = update_token_decimals_sync_standalone(mint_str, 9) {
                                log(
                                    LogTag::Monitor,
                                    "WARN",
                                    &format!("Failed to cache decimals for {}: {}", mint_str, e)
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to fetch mint accounts: {}, using default 9 for all", e)
                );
                // Fallback to default decimals for this batch
                for mint_str in &chunk_mints {
                    result.insert(mint_str.clone(), 9);
                    if let Err(e) = update_token_decimals_sync_standalone(mint_str, 9) {
                        log(
                            LogTag::Monitor,
                            "WARN",
                            &format!("Failed to cache decimals for {}: {}", mint_str, e)
                        );
                    }
                }
            }
        }

        // Increment the counter for the next batch
        processed_valid_mints += chunk.len();
    }

    Ok(result)
}
