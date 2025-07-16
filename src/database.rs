use crate::types::{ TokenInfo, WalletPosition, DiscoveryStats };
use crate::pricing::{ TokenInfo as PricingTokenInfo, TokenPrice, PoolInfo, PoolType };
use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ params, Connection, Row };
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

// Implement Send and Sync for Database
unsafe impl Send for Database {}
unsafe impl Sync for Database {}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path).with_context(||
            format!("Failed to open database: {db_path}")
        )?;

        let db = Self { conn: Mutex::new(conn) };
        db.initialize_tables()?;
        Ok(db)
    }

    fn initialize_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Tokens table
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

        // Wallet positions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS wallet_positions (
                mint TEXT PRIMARY KEY,
                name TEXT,
                symbol TEXT,
                balance INTEGER NOT NULL,
                decimals INTEGER NOT NULL,
                value_sol REAL,
                entry_price_sol REAL,
                current_price_sol REAL,
                pnl_sol REAL,
                pnl_percentage REAL,
                realized_pnl_sol REAL,
                unrealized_pnl_sol REAL,
                total_invested_sol REAL,
                average_entry_price_sol REAL,
                last_updated TEXT NOT NULL
            )",
            []
        )?;

        // Discovery stats table
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

        // Add new tables for pricing module
        // Token prices table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS token_prices (
                address TEXT PRIMARY KEY,
                price_usd REAL NOT NULL,
                price_sol REAL,
                market_cap REAL,
                volume_24h REAL NOT NULL,
                liquidity_usd REAL NOT NULL,
                source TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                last_updated TEXT NOT NULL
            )",
            []
        )?;

        // Pool information table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pools (
                address TEXT PRIMARY KEY,
                pool_type TEXT NOT NULL,
                token_0 TEXT NOT NULL,
                token_1 TEXT NOT NULL,
                reserve_0 INTEGER NOT NULL,
                reserve_1 INTEGER NOT NULL,
                liquidity_usd REAL NOT NULL,
                volume_24h REAL NOT NULL,
                fee_tier REAL,
                last_updated TEXT NOT NULL
            )",
            []
        )?;

        // Token info extended table
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

        // Transaction tracking table - SOL-based pricing
        conn.execute(
            "CREATE TABLE IF NOT EXISTS wallet_transactions (
                signature TEXT PRIMARY KEY,
                mint TEXT NOT NULL,
                transaction_type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                price_sol REAL,
                value_sol REAL,
                sol_amount INTEGER,
                fee INTEGER,
                block_time INTEGER NOT NULL,
                slot INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            []
        )?;

        // Migrate old USD-based columns to SOL-based if they exist
        let _ = conn.execute("ALTER TABLE wallet_transactions ADD COLUMN price_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_transactions ADD COLUMN value_sol REAL", []);

        // Create indexes for pricing tables
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_token_prices_timestamp ON token_prices(timestamp)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pools_liquidity ON pools(liquidity_usd DESC)",
            []
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_pools_tokens ON pools(token_0, token_1)", [])?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_discovered_at ON tokens(discovered_at)",
            []
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_tokens_is_active ON tokens(is_active)", [])?;

        // Create indexes for transaction tracking
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transactions_mint ON wallet_transactions(mint)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transactions_block_time ON wallet_transactions(block_time)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_transactions_type ON wallet_transactions(transaction_type)",
            []
        )?;

        // Migrate existing tables to SOL-based calculations
        // Add new SOL-based columns if they don't exist
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN value_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN entry_price_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN current_price_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN pnl_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN realized_pnl_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN unrealized_pnl_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_positions ADD COLUMN total_invested_sol REAL", []);
        let _ = conn.execute(
            "ALTER TABLE wallet_positions ADD COLUMN average_entry_price_sol REAL",
            []
        );

        // Add SOL-based columns to transactions table
        let _ = conn.execute("ALTER TABLE wallet_transactions ADD COLUMN price_sol REAL", []);
        let _ = conn.execute("ALTER TABLE wallet_transactions ADD COLUMN value_sol REAL", []);

        Ok(())
    }

    // Token operations
    pub fn save_token(&self, token: &TokenInfo) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tokens 
            (mint, symbol, name, decimals, supply, market_cap, price, volume_24h, 
             liquidity, pool_address, discovered_at, last_updated, is_active)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                token.mint,
                token.symbol,
                token.name,
                token.decimals,
                token.supply,
                token.market_cap,
                token.price,
                token.volume_24h,
                token.liquidity,
                token.pool_address,
                token.discovered_at.to_rfc3339(),
                token.last_updated.to_rfc3339(),
                if token.is_active {
                    1
                } else {
                    0
                }
            ]
        )?;
        Ok(())
    }

    pub fn get_token(&self, mint: &str) -> Result<Option<TokenInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tokens WHERE mint = ?1")?;
        let mut token_iter = stmt.query_map([mint], |row| { self.row_to_token_info(row) })?;

        if let Some(token) = token_iter.next() {
            return Ok(Some(token?));
        }

        Ok(None)
    }

    pub fn get_active_tokens(&self, limit: Option<u32>) -> Result<Vec<TokenInfo>> {
        let conn = self.conn.lock().unwrap();
        let query = match limit {
            Some(l) =>
                format!(
                    "SELECT * FROM tokens WHERE is_active = 1 ORDER BY last_updated DESC LIMIT {l}"
                ),
            None =>
                "SELECT * FROM tokens WHERE is_active = 1 ORDER BY last_updated DESC".to_string(),
        };

        let mut stmt = conn.prepare(&query)?;
        let token_iter = stmt.query_map([], |row| { self.row_to_token_info(row) })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    pub fn mark_token_inactive(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tokens SET is_active = 0, last_updated = ?1 WHERE mint = ?2",
            params![Utc::now().to_rfc3339(), mint]
        )?;
        Ok(())
    }

    // Wallet position operations
    pub fn save_wallet_position(&self, position: &WalletPosition) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO wallet_positions 
            (mint, name, symbol, balance, decimals, value_sol, entry_price_sol, current_price_sol, 
             pnl_sol, pnl_percentage, realized_pnl_sol, unrealized_pnl_sol, 
             total_invested_sol, average_entry_price_sol, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                position.mint,
                position.name,
                position.symbol,
                position.balance,
                position.decimals,
                position.value_sol,
                position.entry_price_sol,
                position.current_price_sol,
                position.pnl_sol,
                position.pnl_percentage,
                position.realized_pnl_sol,
                position.unrealized_pnl_sol,
                position.total_invested_sol,
                position.average_entry_price_sol,
                position.last_updated.to_rfc3339()
            ]
        )?;
        Ok(())
    }

    pub fn get_wallet_positions(&self) -> Result<Vec<WalletPosition>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT mint, name, symbol, balance, decimals, value_sol, entry_price_sol, current_price_sol, 
             pnl_sol, pnl_percentage, realized_pnl_sol, unrealized_pnl_sol, 
             total_invested_sol, average_entry_price_sol, last_updated 
             FROM wallet_positions ORDER BY value_sol DESC"
        )?;

        let position_iter = stmt.query_map([], |row| { self.row_to_wallet_position(row) })?;

        let mut positions = Vec::new();
        for position in position_iter {
            positions.push(position?);
        }

        Ok(positions)
    }

    // Discovery stats operations
    pub fn save_discovery_stats(&self, stats: &DiscoveryStats) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO discovery_stats 
            (total_tokens_discovered, active_tokens, last_discovery_run, 
             discovery_rate_per_hour, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                stats.total_tokens_discovered,
                stats.active_tokens,
                stats.last_discovery_run.to_rfc3339(),
                stats.discovery_rate_per_hour,
                Utc::now().to_rfc3339()
            ]
        )?;
        Ok(())
    }

    pub fn get_latest_discovery_stats(&self) -> Result<Option<DiscoveryStats>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM discovery_stats ORDER BY created_at DESC LIMIT 1"
        )?;

        let mut stats_iter = stmt.query_map([], |row| {
            Ok(DiscoveryStats {
                total_tokens_discovered: row.get(1)?,
                active_tokens: row.get(2)?,
                last_discovery_run: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                discovery_rate_per_hour: row.get(4)?,
            })
        })?;

        if let Some(stats) = stats_iter.next() {
            return Ok(Some(stats?));
        }

        Ok(None)
    }

    // Cleanup operations
    pub fn cleanup_old_tokens(&self, max_age_days: u64) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff_date = Utc::now() - chrono::Duration::days(max_age_days as i64);

        let rows_affected = conn.execute(
            "DELETE FROM tokens WHERE discovered_at < ?1 AND is_active = 0",
            params![cutoff_date.to_rfc3339()]
        )?;

        Ok(rows_affected as u64)
    }

    pub fn get_token_count(&self) -> Result<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))?;

        let active: u64 = conn.query_row(
            "SELECT COUNT(*) FROM tokens WHERE is_active = 1",
            [],
            |row| row.get(0)
        )?;

        Ok((total, active))
    }

    // New methods for pricing module
    pub async fn update_token_info(&self, token_info: &PricingTokenInfo) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Update token info
        conn.execute(
            "INSERT OR REPLACE INTO token_info_extended 
            (address, name, symbol, decimals, total_supply, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                token_info.address,
                token_info.name,
                token_info.symbol,
                token_info.decimals,
                token_info.total_supply,
                token_info.last_updated
            ]
        )?;

        // Update price if available
        if let Some(price) = &token_info.price {
            self.update_token_price_internal(&conn, price)?;
        }

        // Update pools
        for pool in &token_info.pools {
            self.update_pool_info_internal(&conn, pool)?;
        }

        Ok(())
    }

    fn update_token_price_internal(&self, conn: &Connection, price: &TokenPrice) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO token_prices 
            (address, price_usd, price_sol, market_cap, volume_24h, liquidity_usd, source, timestamp, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                price.address,
                price.price_usd,
                price.price_sol,
                price.market_cap,
                price.volume_24h,
                price.liquidity_usd,
                format!("{:?}", price.source),
                price.timestamp,
                Utc::now().to_rfc3339()
            ]
        )?;
        Ok(())
    }

    fn update_pool_info_internal(&self, conn: &Connection, pool: &PoolInfo) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO pools 
            (address, pool_type, token_0, token_1, reserve_0, reserve_1, liquidity_usd, volume_24h, fee_tier, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                pool.address,
                format!("{:?}", pool.pool_type),
                pool.token_0,
                pool.token_1,
                pool.reserve_0,
                pool.reserve_1,
                pool.liquidity_usd,
                pool.volume_24h,
                pool.fee_tier,
                pool.last_updated
            ]
        )?;
        Ok(())
    }

    pub async fn get_top_tokens_by_liquidity(&self, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.address FROM token_info_extended t
             LEFT JOIN (
                 SELECT token_0 as token, SUM(liquidity_usd) as total_liquidity
                 FROM pools 
                 GROUP BY token_0
                 UNION ALL
                 SELECT token_1 as token, SUM(liquidity_usd) as total_liquidity
                 FROM pools 
                 GROUP BY token_1
             ) p ON t.address = p.token
             ORDER BY COALESCE(p.total_liquidity, 0) DESC
             LIMIT ?1"
        )?;

        let token_addresses: Result<Vec<String>, rusqlite::Error> = stmt
            .query_map([limit], |row| row.get::<_, String>(0))
            .unwrap()
            .collect();

        Ok(token_addresses?)
    }

    pub async fn get_token_pools(&self, token_address: &str) -> Result<Vec<PoolInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM pools WHERE token_0 = ?1 OR token_1 = ?1 ORDER BY liquidity_usd DESC"
        )?;

        let pool_iter = stmt.query_map([token_address], |row| {
            match self.row_to_pool_info(row) {
                Ok(pool) => Ok(pool),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool?);
        }

        Ok(pools)
    }

    pub async fn get_cached_token_price(&self, token_address: &str) -> Result<Option<TokenPrice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM token_prices WHERE address = ?1")?;

        let mut price_iter = stmt.query_map([token_address], |row| {
            match self.row_to_token_price(row) {
                Ok(price) => Ok(price),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        if let Some(price) = price_iter.next() {
            return Ok(Some(price?));
        }

        Ok(None)
    }

    fn row_to_pool_info(&self, row: &Row) -> Result<PoolInfo, anyhow::Error> {
        let pool_type_str: String = row.get("pool_type")?;
        let pool_type = match pool_type_str.as_str() {
            "Raydium" => PoolType::Raydium,
            "PumpFun" => PoolType::PumpFun,
            "Meteora" => PoolType::Meteora,
            "Orca" => PoolType::Orca,
            "Serum" => PoolType::Serum,
            _ => PoolType::Unknown(pool_type_str),
        };

        Ok(PoolInfo {
            address: row.get("address")?,
            pool_type,
            token_0: row.get("token_0")?,
            token_1: row.get("token_1")?,
            reserve_0: row.get("reserve_0")?,
            reserve_1: row.get("reserve_1")?,
            liquidity_usd: row.get("liquidity_usd")?,
            volume_24h: row.get("volume_24h")?,
            fee_tier: row.get("fee_tier")?,
            last_updated: row.get("last_updated")?,
        })
    }

    fn row_to_token_price(&self, row: &Row) -> Result<TokenPrice, anyhow::Error> {
        use crate::pricing::PriceSource;

        let source_str: String = row.get("source")?;
        let source = match source_str.as_str() {
            "GeckoTerminal" => PriceSource::GeckoTerminal,
            "PoolCalculation" => PriceSource::PoolCalculation,
            "Cache" => PriceSource::Cache,
            _ => PriceSource::Cache,
        };

        Ok(TokenPrice {
            address: row.get("address")?,
            price_usd: row.get("price_usd")?,
            price_sol: row.get("price_sol")?,
            market_cap: row.get("market_cap")?,
            volume_24h: row.get("volume_24h")?,
            liquidity_usd: row.get("liquidity_usd")?,
            timestamp: row.get("timestamp")?,
            source,
            is_cache: true,
        })
    }

    // Helper methods
    fn row_to_token_info(&self, row: &Row) -> Result<TokenInfo, rusqlite::Error> {
        Ok(TokenInfo {
            mint: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            decimals: row.get(3)?,
            supply: row.get(4)?,
            market_cap: row.get(5)?,
            price: row.get(6)?,
            volume_24h: row.get(7)?,
            liquidity: row.get(8)?,
            pool_address: row.get(9)?,
            discovered_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .unwrap()
                .with_timezone(&Utc),
            last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                .unwrap()
                .with_timezone(&Utc),
            is_active: row.get::<_, i32>(12)? == 1,
        })
    }

    fn row_to_wallet_position(&self, row: &Row) -> Result<WalletPosition, rusqlite::Error> {
        Ok(WalletPosition {
            mint: row.get(0)?,
            name: row.get(1)?,
            symbol: row.get(2)?,
            balance: row.get(3)?,
            decimals: row.get(4)?,
            value_sol: row.get(5)?,
            entry_price_sol: row.get(6)?,
            current_price_sol: row.get(7)?,
            pnl_sol: row.get(8)?,
            pnl_percentage: row.get(9)?,
            realized_pnl_sol: row.get(10)?,
            unrealized_pnl_sol: row.get(11)?,
            total_invested_sol: row.get(12)?,
            average_entry_price_sol: row.get(13)?,
            last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(14)?)
                .unwrap()
                .with_timezone(&Utc),
        })
    }

    // Transaction tracking methods
    pub fn save_wallet_transaction(
        &self,
        transaction: &crate::types::WalletTransaction
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let transaction_type_str = match transaction.transaction_type {
            crate::types::TransactionType::Buy => "Buy",
            crate::types::TransactionType::Sell => "Sell",
            crate::types::TransactionType::Transfer => "Transfer",
            crate::types::TransactionType::Receive => "Receive",
        };

        conn
            .execute(
                "INSERT OR REPLACE INTO wallet_transactions 
             (signature, mint, transaction_type, amount, price_sol, value_sol, sol_amount, fee, block_time, slot, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    transaction.signature,
                    transaction.mint,
                    transaction_type_str,
                    transaction.amount as i64,
                    transaction.price_sol,
                    transaction.value_sol,
                    transaction.sol_amount.map(|x| x as i64),
                    transaction.fee.map(|x| x as i64),
                    transaction.block_time,
                    transaction.slot as i64,
                    transaction.created_at.to_rfc3339()
                ]
            )
            .context("Failed to save wallet transaction")?;

        Ok(())
    }

    /// Check if a transaction already exists in the database
    pub fn transaction_exists(&self, signature: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM wallet_transactions WHERE signature = ?1"
        )?;
        let count: i64 = stmt.query_row([signature], |row| row.get(0))?;
        Ok(count > 0)
    }

    /// Get all wallet transactions for a specific mint, ordered by block_time
    pub fn get_wallet_transactions_for_mint(
        &self,
        mint: &str
    ) -> Result<Vec<crate::types::WalletTransaction>> {
        let conn = self.conn.lock().unwrap();
        let query =
            "SELECT signature, mint, transaction_type, amount, price_sol, value_sol, sol_amount, fee, block_time, slot, created_at 
                     FROM wallet_transactions 
                     WHERE mint = ?1 
                     ORDER BY block_time ASC";

        let mut stmt = conn.prepare(query)?;
        let transaction_iter = stmt.query_map([mint], |row| {
            self.row_to_wallet_transaction_with_sol(row)
        })?;

        let mut transactions = Vec::new();
        for transaction in transaction_iter {
            transactions.push(transaction?);
        }
        Ok(transactions)
    }

    /// Get all unique mints from transaction history
    pub fn get_all_transaction_mints(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let query = "SELECT DISTINCT mint FROM wallet_transactions ORDER BY mint";

        let mut stmt = conn.prepare(query)?;
        let mint_iter = stmt.query_map([], |row| { row.get::<_, String>(0) })?;

        let mut mints = Vec::new();
        for mint in mint_iter {
            mints.push(mint?);
        }
        Ok(mints)
    }

    /// Get the latest transaction signature (for pagination)
    pub fn get_latest_transaction_signature(&self) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let query =
            "SELECT signature FROM wallet_transactions ORDER BY block_time DESC, slot DESC LIMIT 1";

        let mut stmt = conn.prepare(query)?;
        match stmt.query_row([], |row| row.get::<_, String>(0)) {
            Ok(signature) => Ok(Some(signature)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get historical price for a token at a specific timestamp
    pub fn get_historical_price(&self, mint: &str, block_time: i64) -> Result<f64> {
        let conn = self.conn.lock().unwrap();

        // Try to find a price within 1 hour of the requested time
        let query =
            "SELECT price_sol FROM token_prices 
                     WHERE address = ?1 AND ABS(timestamp - ?2) <= 3600 
                     ORDER BY ABS(timestamp - ?2) LIMIT 1";

        let mut stmt = conn.prepare(query)?;
        match
            stmt.query_row(params![mint, block_time], |row| {
                Ok(row.get::<_, Option<f64>>(0)?.unwrap_or(0.0))
            })
        {
            Ok(price) => Ok(price),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0.0),
            Err(e) => Err(e.into()),
        }
    }

    /// Get transaction count for debugging
    pub fn get_transaction_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM wallet_transactions")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// Clean old transactions (keep only the most recent N transactions)
    pub fn clean_old_transactions(&self, keep_count: usize) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        // Delete all but the most recent transactions
        let query =
            "DELETE FROM wallet_transactions 
                     WHERE signature NOT IN (
                         SELECT signature FROM wallet_transactions 
                         ORDER BY block_time DESC, slot DESC 
                         LIMIT ?1
                     )";

        let deleted = conn.execute(query, params![keep_count as i64])?;
        Ok(deleted as i64)
    }

    /// Row to WalletTransaction with SOL-based fields
    fn row_to_wallet_transaction_with_sol(
        &self,
        row: &Row
    ) -> Result<crate::types::WalletTransaction, rusqlite::Error> {
        let transaction_type_str: String = row.get(2)?;
        let transaction_type = match transaction_type_str.as_str() {
            "Buy" => crate::types::TransactionType::Buy,
            "Sell" => crate::types::TransactionType::Sell,
            "Transfer" => crate::types::TransactionType::Transfer,
            "Receive" => crate::types::TransactionType::Receive,
            _ => crate::types::TransactionType::Transfer,
        };

        Ok(crate::types::WalletTransaction {
            signature: row.get(0)?,
            mint: row.get(1)?,
            transaction_type,
            amount: row.get::<_, i64>(3)? as u64,
            price_sol: row.get(4)?,
            value_sol: row.get(5)?,
            sol_amount: row.get::<_, Option<i64>>(6)?.map(|x| x as u64),
            fee: row.get::<_, Option<i64>>(7)?.map(|x| x as u64),
            block_time: row.get(8)?,
            slot: row.get::<_, i64>(9)? as u64,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .unwrap()
                .with_timezone(&Utc),
        })
    }
}
