use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ Connection, params };
use serde::{ Deserialize, Serialize };
use std::sync::Mutex;

/// Full token data with market information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub price_usd: f64,
    pub price_change_24h: f64,
    pub volume_24h: f64,
    pub market_cap: f64,
    pub fdv: f64,
    pub total_supply: f64,
    pub circulating_supply: f64,
    pub liquidity_usd: f64,
    pub top_pool_address: Option<String>,
    pub top_pool_base_reserve: Option<f64>,
    pub top_pool_quote_reserve: Option<f64>,
    pub last_updated: DateTime<Utc>,
}

/// Pool information for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolData {
    pub pool_address: String,
    pub token_mint: String,
    pub base_token_address: String,
    pub quote_token_address: String,
    pub base_token_reserve: f64,
    pub quote_token_reserve: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

/// Market data statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStats {
    pub total_tokens_tracked: u64,
    pub active_tokens: u64,
    pub total_pools: u64,
    pub last_update_run: DateTime<Utc>,
    pub update_rate_per_hour: f64,
}

/// Database connection for market data module
pub struct MarketDatabase {
    conn: Mutex<Connection>,
}

impl MarketDatabase {
    /// Create a new market database connection
    pub fn new() -> Result<Self> {
        let conn = Connection::open("cache_tokens.db").context("Failed to open market database")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.initialize_tables()?;
        Ok(db)
    }

    /// Initialize market database tables
    fn initialize_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create tokens table with full market data
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                decimals INTEGER NOT NULL,
                price_usd REAL NOT NULL,
                price_change_24h REAL NOT NULL,
                volume_24h REAL NOT NULL,
                market_cap REAL NOT NULL,
                fdv REAL NOT NULL,
                total_supply REAL NOT NULL,
                circulating_supply REAL NOT NULL,
                liquidity_usd REAL NOT NULL,
                top_pool_address TEXT,
                top_pool_base_reserve REAL,
                top_pool_quote_reserve REAL,
                last_updated TEXT NOT NULL
            )",
            []
        )?;

        // Create pools table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pools (
                pool_address TEXT PRIMARY KEY,
                token_mint TEXT NOT NULL,
                base_token_address TEXT NOT NULL,
                quote_token_address TEXT NOT NULL,
                base_token_reserve REAL NOT NULL,
                quote_token_reserve REAL NOT NULL,
                liquidity_usd REAL NOT NULL,
                volume_24h REAL NOT NULL,
                created_at TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                FOREIGN KEY (token_mint) REFERENCES tokens(mint)
            )",
            []
        )?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_last_updated ON tokens(last_updated)",
            []
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_pools_token_mint ON pools(token_mint)", [])?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pools_liquidity ON pools(liquidity_usd DESC)",
            []
        )?;

        Ok(())
    }

    /// Save token data to database
    pub fn save_token(&self, token: &TokenData) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO tokens (
                mint, symbol, name, decimals, price_usd, price_change_24h, volume_24h,
                market_cap, fdv, total_supply, circulating_supply, liquidity_usd,
                top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                token.mint,
                token.symbol,
                token.name,
                token.decimals,
                token.price_usd,
                token.price_change_24h,
                token.volume_24h,
                token.market_cap,
                token.fdv,
                token.total_supply,
                token.circulating_supply,
                token.liquidity_usd,
                token.top_pool_address,
                token.top_pool_base_reserve,
                token.top_pool_quote_reserve,
                token.last_updated.to_rfc3339()
            ]
        )?;

        Ok(())
    }

    /// Save pool data to database
    pub fn save_pool(&self, pool: &PoolData) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO pools (
                pool_address, token_mint, base_token_address, quote_token_address,
                base_token_reserve, quote_token_reserve, liquidity_usd, volume_24h,
                created_at, last_updated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                pool.pool_address,
                pool.token_mint,
                pool.base_token_address,
                pool.quote_token_address,
                pool.base_token_reserve,
                pool.quote_token_reserve,
                pool.liquidity_usd,
                pool.volume_24h,
                pool.created_at.to_rfc3339(),
                pool.last_updated.to_rfc3339()
            ]
        )?;

        Ok(())
    }

    /// Get token data by mint address
    pub fn get_token(&self, mint: &str) -> Result<Option<TokenData>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT mint, symbol, name, decimals, price_usd, price_change_24h, volume_24h,
                    market_cap, fdv, total_supply, circulating_supply, liquidity_usd,
                    top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
             FROM tokens WHERE mint = ?1"
        )?;

        let mut token_iter = stmt.query_map(params![mint], |row| {
            Ok(TokenData {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                price_usd: row.get(4)?,
                price_change_24h: row.get(5)?,
                volume_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                total_supply: row.get(9)?,
                circulating_supply: row.get(10)?,
                liquidity_usd: row.get(11)?,
                top_pool_address: row.get(12)?,
                top_pool_base_reserve: row.get(13)?,
                top_pool_quote_reserve: row.get(14)?,
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        if let Some(token) = token_iter.next() {
            return Ok(Some(token?));
        }

        Ok(None)
    }

    /// Get all tracked tokens
    pub fn get_all_tokens(&self) -> Result<Vec<TokenData>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT mint, symbol, name, decimals, price_usd, price_change_24h, volume_24h,
                    market_cap, fdv, total_supply, circulating_supply, liquidity_usd,
                    top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
             FROM tokens ORDER BY last_updated DESC"
        )?;

        let token_iter = stmt.query_map([], |row| {
            Ok(TokenData {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                price_usd: row.get(4)?,
                price_change_24h: row.get(5)?,
                volume_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                total_supply: row.get(9)?,
                circulating_supply: row.get(10)?,
                liquidity_usd: row.get(11)?,
                top_pool_address: row.get(12)?,
                top_pool_base_reserve: row.get(13)?,
                top_pool_quote_reserve: row.get(14)?,
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get top tokens by volume (24h) - used for trader monitoring
    pub fn get_top_tokens_by_volume(&self, limit: usize) -> Result<Vec<TokenData>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT mint, symbol, name, decimals, price_usd, price_change_24h, volume_24h,
                    market_cap, fdv, total_supply, circulating_supply, liquidity_usd,
                    top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
             FROM tokens WHERE volume_24h > 0 ORDER BY volume_24h DESC LIMIT ?1"
        )?;

        let token_iter = stmt.query_map(params![limit], |row| {
            Ok(TokenData {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                price_usd: row.get(4)?,
                price_change_24h: row.get(5)?,
                volume_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                total_supply: row.get(9)?,
                circulating_supply: row.get(10)?,
                liquidity_usd: row.get(11)?,
                top_pool_address: row.get(12)?,
                top_pool_base_reserve: row.get(13)?,
                top_pool_quote_reserve: row.get(14)?,
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get pools for a token
    pub fn get_token_pools(&self, mint: &str) -> Result<Vec<PoolData>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT pool_address, token_mint, base_token_address, quote_token_address,
                    base_token_reserve, quote_token_reserve, liquidity_usd, volume_24h,
                    created_at, last_updated
             FROM pools WHERE token_mint = ?1 ORDER BY liquidity_usd DESC"
        )?;

        let pool_iter = stmt.query_map(params![mint], |row| {
            Ok(PoolData {
                pool_address: row.get(0)?,
                token_mint: row.get(1)?,
                base_token_address: row.get(2)?,
                quote_token_address: row.get(3)?,
                base_token_reserve: row.get(4)?,
                quote_token_reserve: row.get(5)?,
                liquidity_usd: row.get(6)?,
                volume_24h: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool?);
        }

        Ok(pools)
    }

    /// Get market statistics
    pub fn get_stats(&self) -> Result<MarketStats> {
        let conn = self.conn.lock().unwrap();

        let total_tokens: u64 = conn.query_row("SELECT COUNT(*) FROM tokens", [], |row|
            row.get(0)
        )?;

        let active_tokens: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tokens WHERE last_updated >= datetime('now', '-1 hour')",
            [],
            |row| row.get(0)
        )?;

        let total_pools: u64 = conn.query_row("SELECT COUNT(*) FROM pools", [], |row| row.get(0))?;

        // Calculate update rate for last 24 hours
        let recent_updates: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tokens WHERE last_updated >= datetime('now', '-24 hours')",
            [],
            |row| row.get(0)
        )?;

        Ok(MarketStats {
            total_tokens_tracked: total_tokens,
            active_tokens,
            total_pools,
            last_update_run: Utc::now(),
            update_rate_per_hour: (recent_updates as f64) / 24.0,
        })
    }

    /// Get token count
    pub fn get_token_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Delete a token and its pools
    pub fn delete_token(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM pools WHERE token_mint = ?1", params![mint])?;
        conn.execute("DELETE FROM tokens WHERE mint = ?1", params![mint])?;

        Ok(())
    }

    /// Clear all data (for testing/reset purposes)
    pub fn clear_all_data(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("DELETE FROM pools", [])?;
        conn.execute("DELETE FROM tokens", [])?;

        Ok(())
    }
}

// Thread safety
unsafe impl Send for MarketDatabase {}
unsafe impl Sync for MarketDatabase {}
