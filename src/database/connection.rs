use crate::database::models::{ DatabaseConfig, DatabaseResult };
use anyhow::{ Context, Result };
use rusqlite::{ Connection, params };
use std::sync::Mutex;

/// Main database connection wrapper
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

// Implement Send and Sync for Database
unsafe impl Send for Database {}
unsafe impl Sync for Database {}

impl Database {
    /// Create a new database connection
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path).with_context(||
            format!("Failed to open database: {}", db_path)
        )?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.initialize_tables()?;
        Ok(db)
    }

    /// Create database with custom configuration
    pub fn with_config(config: &DatabaseConfig) -> Result<Self> {
        Self::new(&config.path)
    }

    /// Initialize all required database tables
    fn initialize_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create tables in order of dependencies
        self.create_tokens_table(&conn)?;
        self.create_discovery_stats_table(&conn)?;
        self.create_token_prices_table(&conn)?;
        self.create_blacklisted_tokens_table(&conn)?;
        self.create_token_priorities_table(&conn)?;
        self.create_token_info_extended_table(&conn)?;
        self.create_pools_table(&conn)?;

        // Create indexes for performance
        self.create_indexes(&conn)?;

        Ok(())
    }

    /// Create tokens table
    fn create_tokens_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                mint TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                name TEXT NOT NULL,
                decimals INTEGER NOT NULL,
                supply INTEGER NOT NULL,
                market_cap REAL,
                price REAL,
                volume_24h REAL,
                liquidity REAL,
                pool_address TEXT,
                discovered_at TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            )",
            []
        )?;
        Ok(())
    }

    /// Create discovery stats table
    fn create_discovery_stats_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS discovery_stats (
                id INTEGER PRIMARY KEY,
                total_tokens_discovered INTEGER NOT NULL,
                active_tokens INTEGER NOT NULL,
                last_discovery_run TEXT NOT NULL,
                discovery_rate_per_hour REAL NOT NULL,
                created_at TEXT NOT NULL
            )",
            []
        )?;
        Ok(())
    }

    /// Create token prices table
    fn create_token_prices_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_prices (
                token_address TEXT NOT NULL,
                price_usd REAL NOT NULL,
                price_sol REAL,
                market_cap REAL,
                volume_24h REAL NOT NULL,
                liquidity_usd REAL NOT NULL,
                timestamp INTEGER NOT NULL,
                source TEXT NOT NULL,
                is_cache INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (token_address, timestamp)
            )",
            []
        )?;
        Ok(())
    }

    /// Create blacklisted tokens table
    fn create_blacklisted_tokens_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blacklisted_tokens (
                token_address TEXT PRIMARY KEY,
                reason TEXT NOT NULL,
                blacklisted_at TEXT NOT NULL,
                last_liquidity REAL NOT NULL
            )",
            []
        )?;
        Ok(())
    }

    /// Create token priorities table
    fn create_token_priorities_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_priorities (
                token_address TEXT PRIMARY KEY,
                liquidity_usd REAL NOT NULL,
                volume_24h REAL NOT NULL,
                priority_score REAL NOT NULL,
                update_interval_secs INTEGER NOT NULL,
                last_updated TEXT NOT NULL,
                consecutive_failures INTEGER NOT NULL DEFAULT 0
            )",
            []
        )?;
        Ok(())
    }

    /// Create token info extended table
    fn create_token_info_extended_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_info_extended (
                address TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                symbol TEXT NOT NULL,
                decimals INTEGER NOT NULL,
                total_supply INTEGER,
                last_updated TEXT NOT NULL
            )",
            []
        )?;
        Ok(())
    }

    /// Create pools table
    fn create_pools_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pools (
                address TEXT PRIMARY KEY,
                pool_type TEXT NOT NULL,
                reserve_0 INTEGER NOT NULL,
                reserve_1 INTEGER NOT NULL,
                token_0 TEXT NOT NULL,
                token_1 TEXT NOT NULL,
                liquidity_usd REAL NOT NULL,
                volume_24h REAL NOT NULL,
                fee_tier REAL,
                last_updated INTEGER NOT NULL
            )",
            []
        )?;
        Ok(())
    }

    /// Create database indexes for performance
    fn create_indexes(&self, conn: &Connection) -> Result<()> {
        // Token prices indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_prices_timestamp ON token_prices(timestamp)",
            []
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_prices_liquidity ON token_prices(liquidity_usd)",
            []
        )?;

        // Token priorities indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_priorities_score ON token_priorities(priority_score)",
            []
        )?;

        // Pools indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pools_liquidity ON pools(liquidity_usd DESC)",
            []
        )?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_pools_tokens ON pools(token_0, token_1)", [])?;

        // Tokens indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_discovered_at ON tokens(discovered_at)",
            []
        )?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_tokens_is_active ON tokens(is_active)", [])?;

        Ok(())
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> DatabaseResult<super::models::DatabaseStats> {
        let conn = self.conn.lock().unwrap();

        let total_tokens: u64 = conn.query_row("SELECT COUNT(*) FROM tokens", [], |row|
            row.get(0)
        )?;

        let active_tokens: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tokens WHERE is_active = 1",
            [],
            |row| row.get(0)
        )?;

        let blacklisted_tokens: u64 = conn.query_row(
            "SELECT COUNT(*) FROM blacklisted_tokens",
            [],
            |row| row.get(0)
        )?;

        let total_pools: u64 = conn.query_row("SELECT COUNT(*) FROM pools", [], |row| row.get(0))?;

        let total_price_records: u64 = conn.query_row("SELECT COUNT(*) FROM token_prices", [], |row|
            row.get(0)
        )?;

        Ok(super::models::DatabaseStats {
            total_tokens,
            active_tokens,
            blacklisted_tokens,
            total_pools,
            total_price_records,
            last_updated: chrono::Utc::now(),
        })
    }

    /// Clean up old data
    pub async fn cleanup_old_data(&self, max_age_days: u64) -> DatabaseResult<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);

        let rows_affected = conn.execute(
            "DELETE FROM tokens WHERE discovered_at < ?1 AND is_active = 0",
            params![cutoff_date.to_rfc3339()]
        )?;

        Ok(rows_affected as u64)
    }
}
