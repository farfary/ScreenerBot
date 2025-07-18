use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ Connection, params, OptionalExtension };
use serde::{ Deserialize, Serialize };
use std::sync::Mutex;

/// Full token data with market information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub price_sol: f64,
    pub price_change_24h: f64,
    pub volume_24h: f64,
    pub market_cap: f64,
    pub fdv: f64,
    pub total_supply: f64,
    pub circulating_supply: f64,
    pub liquidity_sol: f64,
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
    pub liquidity_sol: f64,
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

/// Liquidity history entry for tracking changes over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityHistory {
    pub id: i64,
    pub token_address: String,
    pub liquidity_sol: f64,
    pub timestamp: DateTime<Utc>,
    pub source: String, // 'dexscreener', 'gecko', etc.
}

/// Token blacklist entry for rugged/dead tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBlacklist {
    pub token_address: String,
    pub reason: String, // 'rug_detected', 'manual', 'low_liquidity'
    pub blacklisted_at: DateTime<Utc>,
    pub peak_liquidity: Option<f64>,
    pub final_liquidity: Option<f64>,
    pub drop_percentage: Option<f64>,
}

/// Rug detection event for tracking suspicious activities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugDetectionEvent {
    pub id: i64,
    pub token_address: String,
    pub event_type: String, // 'liquidity_drop', 'volume_spike', 'reserve_imbalance'
    pub before_value: f64,
    pub after_value: f64,
    pub percentage_change: f64,
    pub detected_at: DateTime<Utc>,
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
        // Create tokens table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                decimals INTEGER NOT NULL,
                price_sol REAL NOT NULL,
                price_change_24h REAL NOT NULL,
                volume_24h REAL NOT NULL,
                market_cap REAL NOT NULL,
                fdv REAL NOT NULL,
                total_supply REAL NOT NULL,
                circulating_supply REAL NOT NULL,
                liquidity_sol REAL NOT NULL,
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
                liquidity_sol REAL NOT NULL,
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
            "CREATE INDEX IF NOT EXISTS idx_pools_liquidity ON pools(liquidity_sol DESC)",
            []
        )?;

        // Create liquidity history table for rug detection
        conn.execute(
            "CREATE TABLE IF NOT EXISTS liquidity_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_address TEXT NOT NULL,
                liquidity_sol REAL NOT NULL,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL,
                FOREIGN KEY (token_address) REFERENCES tokens(mint)
            )",
            []
        )?;

        // Create token blacklist table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_blacklist (
                token_address TEXT PRIMARY KEY,
                reason TEXT NOT NULL,
                blacklisted_at TEXT NOT NULL,
                peak_liquidity REAL,
                final_liquidity REAL,
                drop_percentage REAL
            )",
            []
        )?;

        // Create rug detection events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS rug_detection_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_address TEXT NOT NULL,
                event_type TEXT NOT NULL,
                before_value REAL NOT NULL,
                after_value REAL NOT NULL,
                percentage_change REAL NOT NULL,
                detected_at TEXT NOT NULL
            )",
            []
        )?;

        // Create indexes for the new tables
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_liquidity_history_token ON liquidity_history(token_address)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_liquidity_history_timestamp ON liquidity_history(timestamp DESC)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rug_events_token ON rug_detection_events(token_address)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rug_events_timestamp ON rug_detection_events(detected_at DESC)",
            []
        )?;

        Ok(())
    }

    /// Record liquidity history for rug detection
    pub fn record_liquidity_history(
        &self,
        token_address: &str,
        liquidity_sol: f64,
        source: &str
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO liquidity_history (token_address, liquidity_sol, timestamp, source)
             VALUES (?1, ?2, ?3, ?4)",
            params![token_address, liquidity_sol, Utc::now().to_rfc3339(), source]
        )?;

        Ok(())
    }

    /// Get liquidity history for a token
    pub fn get_liquidity_history(
        &self,
        token_address: &str,
        hours_back: i64
    ) -> Result<Vec<LiquidityHistory>> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::hours(hours_back);

        let mut stmt = conn.prepare(
            "SELECT id, token_address, liquidity_sol, timestamp, source
             FROM liquidity_history
             WHERE token_address = ?1 AND timestamp >= ?2
             ORDER BY timestamp DESC"
        )?;

        let history_iter = stmt.query_map(params![token_address, cutoff_time.to_rfc3339()], |row| {
            Ok(LiquidityHistory {
                id: row.get(0)?,
                token_address: row.get(1)?,
                liquidity_sol: row.get(2)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                source: row.get(4)?,
            })
        })?;

        let mut history = Vec::new();
        for entry in history_iter {
            history.push(entry?);
        }

        Ok(history)
    }

    /// Get peak liquidity for a token within a time window
    pub fn get_peak_liquidity(&self, token_address: &str, hours_back: i64) -> Result<Option<f64>> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::hours(hours_back);

        let peak: Option<f64> = conn
            .query_row(
                "SELECT MAX(liquidity_sol) FROM liquidity_history
             WHERE token_address = ?1 AND timestamp >= ?2",
                params![token_address, cutoff_time.to_rfc3339()],
                |row| row.get(0)
            )
            .optional()?;

        Ok(peak)
    }

    /// Add token to blacklist
    pub fn add_to_blacklist(&self, blacklist_entry: &TokenBlacklist) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO token_blacklist 
             (token_address, reason, blacklisted_at, peak_liquidity, final_liquidity, drop_percentage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                blacklist_entry.token_address,
                blacklist_entry.reason,
                blacklist_entry.blacklisted_at.to_rfc3339(),
                blacklist_entry.peak_liquidity,
                blacklist_entry.final_liquidity,
                blacklist_entry.drop_percentage
            ]
        )?;

        Ok(())
    }

    /// Remove token from blacklist
    pub fn remove_from_blacklist(&self, token_address: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "DELETE FROM token_blacklist WHERE token_address = ?1",
            params![token_address]
        )?;

        Ok(())
    }

    /// Check if token is blacklisted
    pub fn is_blacklisted(&self, token_address: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM token_blacklist WHERE token_address = ?1",
            params![token_address],
            |row| row.get(0)
        )?;

        Ok(count > 0)
    }

    /// Get blacklist entry for a token
    pub fn get_blacklist_entry(&self, token_address: &str) -> Result<Option<TokenBlacklist>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT token_address, reason, blacklisted_at, peak_liquidity, final_liquidity, drop_percentage
             FROM token_blacklist WHERE token_address = ?1"
        )?;

        let mut blacklist_iter = stmt.query_map(params![token_address], |row| {
            Ok(TokenBlacklist {
                token_address: row.get(0)?,
                reason: row.get(1)?,
                blacklisted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                peak_liquidity: row.get(3)?,
                final_liquidity: row.get(4)?,
                drop_percentage: row.get(5)?,
            })
        })?;

        if let Some(entry) = blacklist_iter.next() {
            return Ok(Some(entry?));
        }

        Ok(None)
    }

    /// Record rug detection event
    pub fn record_rug_event(&self, event: &RugDetectionEvent) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO rug_detection_events 
             (token_address, event_type, before_value, after_value, percentage_change, detected_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.token_address,
                event.event_type,
                event.before_value,
                event.after_value,
                event.percentage_change,
                event.detected_at.to_rfc3339()
            ]
        )?;

        Ok(())
    }

    /// Get recent rug detection events for a token
    pub fn get_rug_events(
        &self,
        token_address: &str,
        hours_back: i64
    ) -> Result<Vec<RugDetectionEvent>> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::hours(hours_back);

        let mut stmt = conn.prepare(
            "SELECT id, token_address, event_type, before_value, after_value, percentage_change, detected_at
             FROM rug_detection_events
             WHERE token_address = ?1 AND detected_at >= ?2
             ORDER BY detected_at DESC"
        )?;

        let events_iter = stmt.query_map(params![token_address, cutoff_time.to_rfc3339()], |row| {
            Ok(RugDetectionEvent {
                id: row.get(0)?,
                token_address: row.get(1)?,
                event_type: row.get(2)?,
                before_value: row.get(3)?,
                after_value: row.get(4)?,
                percentage_change: row.get(5)?,
                detected_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut events = Vec::new();
        for event in events_iter {
            events.push(event?);
        }

        Ok(events)
    }

    /// Clean old liquidity history (keep only last 7 days)
    pub fn cleanup_old_liquidity_history(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::days(7);

        conn.execute(
            "DELETE FROM liquidity_history WHERE timestamp < ?1",
            params![cutoff_time.to_rfc3339()]
        )?;

        Ok(())
    }

    /// Save token data to database
    pub fn save_token(&self, token: &TokenData) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO tokens (
                mint, symbol, name, decimals, price_sol, price_change_24h, volume_24h,
                market_cap, fdv, total_supply, circulating_supply, liquidity_sol,
                top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                token.mint,
                token.symbol,
                token.name,
                token.decimals,
                token.price_sol,
                token.price_change_24h,
                token.volume_24h,
                token.market_cap,
                token.fdv,
                token.total_supply,
                token.circulating_supply,
                token.liquidity_sol,
                token.top_pool_address,
                token.top_pool_base_reserve,
                token.top_pool_quote_reserve,
                token.last_updated.to_rfc3339()
            ]
        )?;

        // Record liquidity history for rug detection
        if token.liquidity_sol > 0.0 {
            conn.execute(
                "INSERT INTO liquidity_history (token_address, liquidity_sol, timestamp, source)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    token.mint,
                    token.liquidity_sol,
                    token.last_updated.to_rfc3339(),
                    "token_save"
                ]
            )?;
        }

        Ok(())
    }

    /// Save pool data to database
    pub fn save_pool(&self, pool: &PoolData) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO pools (
                pool_address, token_mint, base_token_address, quote_token_address,
                base_token_reserve, quote_token_reserve, liquidity_sol, volume_24h,
                created_at, last_updated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                pool.pool_address,
                pool.token_mint,
                pool.base_token_address,
                pool.quote_token_address,
                pool.base_token_reserve,
                pool.quote_token_reserve,
                pool.liquidity_sol,
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
            "SELECT mint, symbol, name, decimals, price_sol, price_change_24h, volume_24h,
                    market_cap, fdv, total_supply, circulating_supply, liquidity_sol,
                    top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
             FROM tokens WHERE mint = ?1"
        )?;

        let mut token_iter = stmt.query_map(params![mint], |row| {
            Ok(TokenData {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                price_sol: row.get(4)?,
                price_change_24h: row.get(5)?,
                volume_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                total_supply: row.get(9)?,
                circulating_supply: row.get(10)?,
                liquidity_sol: row.get(11)?,
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
            "SELECT mint, symbol, name, decimals, price_sol, price_change_24h, volume_24h,
                    market_cap, fdv, total_supply, circulating_supply, liquidity_sol,
                    top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
             FROM tokens ORDER BY last_updated DESC"
        )?;

        let token_iter = stmt.query_map([], |row| {
            Ok(TokenData {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                price_sol: row.get(4)?,
                price_change_24h: row.get(5)?,
                volume_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                total_supply: row.get(9)?,
                circulating_supply: row.get(10)?,
                liquidity_sol: row.get(11)?,
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
            "SELECT mint, symbol, name, decimals, price_sol, price_change_24h, volume_24h,
                    market_cap, fdv, total_supply, circulating_supply, liquidity_sol,
                    top_pool_address, top_pool_base_reserve, top_pool_quote_reserve, last_updated
             FROM tokens WHERE volume_24h > 0 ORDER BY volume_24h DESC LIMIT ?1"
        )?;

        let token_iter = stmt.query_map(params![limit], |row| {
            Ok(TokenData {
                mint: row.get(0)?,
                symbol: row.get(1)?,
                name: row.get(2)?,
                decimals: row.get(3)?,
                price_sol: row.get(4)?,
                price_change_24h: row.get(5)?,
                volume_24h: row.get(6)?,
                market_cap: row.get(7)?,
                fdv: row.get(8)?,
                total_supply: row.get(9)?,
                circulating_supply: row.get(10)?,
                liquidity_sol: row.get(11)?,
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
                    base_token_reserve, quote_token_reserve, liquidity_sol, volume_24h,
                    created_at, last_updated
             FROM pools WHERE token_mint = ?1 ORDER BY liquidity_sol DESC"
        )?;

        let pool_iter = stmt.query_map(params![mint], |row| {
            Ok(PoolData {
                pool_address: row.get(0)?,
                token_mint: row.get(1)?,
                base_token_address: row.get(2)?,
                quote_token_address: row.get(3)?,
                base_token_reserve: row.get(4)?,
                quote_token_reserve: row.get(5)?,
                liquidity_sol: row.get(6)?,
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

    /// Get all active tokens (for monitoring purposes)
    pub fn get_active_tokens(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();

        // Get tokens updated in the last 24 hours (active tokens)
        let cutoff_time = Utc::now() - chrono::Duration::hours(24);

        let mut stmt = conn.prepare(
            "SELECT mint FROM tokens 
             WHERE last_updated >= ?1 
             AND liquidity_usd > 0
             ORDER BY liquidity_usd DESC"
        )?;

        let token_iter = stmt.query_map(params![cutoff_time.to_rfc3339()], |row|
            Ok(row.get::<_, String>(0)?)
        )?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
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
