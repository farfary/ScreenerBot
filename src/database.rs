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
            format!("Failed to open database: {}", db_path)
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
                balance INTEGER NOT NULL,
                decimals INTEGER NOT NULL,
                value_usd REAL,
                entry_price REAL,
                current_price REAL,
                pnl REAL,
                pnl_percentage REAL,
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

        // Trading positions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trading_positions (
                id TEXT PRIMARY KEY,
                token_mint TEXT NOT NULL,
                entry_price REAL NOT NULL,
                entry_amount_sol REAL NOT NULL,
                entry_amount_tokens REAL NOT NULL,
                current_price REAL NOT NULL,
                current_value_sol REAL NOT NULL,
                pnl_sol REAL NOT NULL,
                pnl_percentage REAL NOT NULL,
                opened_at TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                status TEXT NOT NULL,
                profit_target REAL NOT NULL,
                time_category TEXT NOT NULL
            )",
            []
        )?;

        // Trading transactions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS trading_transactions (
                id TEXT PRIMARY KEY,
                signature TEXT NOT NULL UNIQUE,
                transaction_type TEXT NOT NULL,
                token_mint TEXT NOT NULL,
                amount_sol REAL NOT NULL,
                amount_tokens REAL NOT NULL,
                price REAL NOT NULL,
                timestamp TEXT NOT NULL,
                block_height INTEGER NOT NULL,
                fee_sol REAL NOT NULL,
                position_id TEXT
            )",
            []
        )?;

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

        // Create indexes for trading tables
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trading_positions_status ON trading_positions(status)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trading_positions_token ON trading_positions(token_mint)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trading_transactions_signature ON trading_transactions(signature)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trading_transactions_position ON trading_transactions(position_id)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trading_transactions_timestamp ON trading_transactions(timestamp)",
            []
        )?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tokens_discovered_at ON tokens(discovered_at)",
            []
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_tokens_is_active ON tokens(is_active)", [])?;

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

        let token_iter = stmt.query_map([mint], |row| { Ok(self.row_to_token_info(row)?) })?;

        for token in token_iter {
            return Ok(Some(token?));
        }

        Ok(None)
    }

    pub fn get_active_tokens(&self, limit: Option<u32>) -> Result<Vec<TokenInfo>> {
        let conn = self.conn.lock().unwrap();
        let query = match limit {
            Some(l) =>
                format!("SELECT * FROM tokens WHERE is_active = 1 ORDER BY last_updated DESC LIMIT {}", l),
            None =>
                "SELECT * FROM tokens WHERE is_active = 1 ORDER BY last_updated DESC".to_string(),
        };

        let mut stmt = conn.prepare(&query)?;
        let token_iter = stmt.query_map([], |row| { Ok(self.row_to_token_info(row)?) })?;

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
            (mint, balance, decimals, value_usd, entry_price, current_price, 
             pnl, pnl_percentage, last_updated)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                position.mint,
                position.balance,
                position.decimals,
                position.value_usd,
                position.entry_price,
                position.current_price,
                position.pnl,
                position.pnl_percentage,
                position.last_updated.to_rfc3339()
            ]
        )?;
        Ok(())
    }

    pub fn get_wallet_positions(&self) -> Result<Vec<WalletPosition>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM wallet_positions ORDER BY value_usd DESC")?;

        let position_iter = stmt.query_map([], |row| { Ok(self.row_to_wallet_position(row)?) })?;

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

        let stats_iter = stmt.query_map([], |row| {
            Ok(DiscoveryStats {
                total_tokens_discovered: row.get(1)?,
                active_tokens: row.get(2)?,
                last_discovery_run: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                discovery_rate_per_hour: row.get(4)?,
            })
        })?;

        for stats in stats_iter {
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
            .query_map([limit], |row| Ok(row.get::<_, String>(0)?))
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

        let price_iter = stmt.query_map([token_address], |row| {
            match self.row_to_token_price(row) {
                Ok(price) => Ok(price),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        for price in price_iter {
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
            balance: row.get(1)?,
            decimals: row.get(2)?,
            value_usd: row.get(3)?,
            entry_price: row.get(4)?,
            current_price: row.get(5)?,
            pnl: row.get(6)?,
            pnl_percentage: row.get(7)?,
            last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                .unwrap()
                .with_timezone(&Utc),
        })
    }

    // Trading positions operations
    pub fn save_trading_position(&self, position: &crate::types::TradingPosition) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO trading_positions 
            (id, token_mint, entry_price, entry_amount_sol, entry_amount_tokens,
             current_price, current_value_sol, pnl_sol, pnl_percentage,
             opened_at, last_updated, status, profit_target, time_category)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                position.id,
                position.token_mint,
                position.entry_price,
                position.entry_amount_sol,
                position.entry_amount_tokens,
                position.current_price,
                position.current_value_sol,
                position.pnl_sol,
                position.pnl_percentage,
                position.opened_at.to_rfc3339(),
                position.last_updated.to_rfc3339(),
                format!("{:?}", position.status),
                position.profit_target,
                format!("{:?}", position.time_category)
            ]
        )?;
        Ok(())
    }

    pub fn get_trading_positions(
        &self,
        status_filter: Option<&str>
    ) -> Result<Vec<crate::types::TradingPosition>> {
        let conn = self.conn.lock().unwrap();
        let query = match status_filter {
            Some(status) =>
                format!("SELECT * FROM trading_positions WHERE status = '{}' ORDER BY opened_at DESC", status),
            None => "SELECT * FROM trading_positions ORDER BY opened_at DESC".to_string(),
        };

        let mut stmt = conn.prepare(&query)?;
        let position_iter = stmt.query_map([], |row| { Ok(self.row_to_trading_position(row)?) })?;

        let mut positions = Vec::new();
        for position in position_iter {
            positions.push(position?);
        }

        Ok(positions)
    }

    pub fn get_trading_position(&self, id: &str) -> Result<Option<crate::types::TradingPosition>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM trading_positions WHERE id = ?1")?;

        let mut position_iter = stmt.query_map([id], |row| {
            Ok(self.row_to_trading_position(row)?)
        })?;

        match position_iter.next() {
            Some(position) => Ok(Some(position?)),
            None => Ok(None),
        }
    }

    // Trading transactions operations
    pub fn save_trading_transaction(&self, transaction: &crate::types::Transaction) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO trading_transactions 
            (id, signature, transaction_type, token_mint, amount_sol, amount_tokens,
             price, timestamp, block_height, fee_sol, position_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                transaction.id,
                transaction.signature,
                format!("{:?}", transaction.transaction_type),
                transaction.token_mint,
                transaction.amount_sol,
                transaction.amount_tokens,
                transaction.price,
                transaction.timestamp.to_rfc3339(),
                transaction.block_height,
                transaction.fee_sol,
                transaction.position_id
            ]
        )?;
        Ok(())
    }

    pub fn get_trading_transactions(
        &self,
        token_mint: Option<&str>,
        position_id: Option<&str>,
        hours: Option<u64>
    ) -> Result<Vec<crate::types::Transaction>> {
        let conn = self.conn.lock().unwrap();

        let mut query = "SELECT * FROM trading_transactions WHERE 1=1".to_string();
        let mut params: Vec<String> = Vec::new();

        if let Some(mint) = token_mint {
            query.push_str(" AND token_mint = ?");
            params.push(mint.to_string());
        }

        if let Some(pos_id) = position_id {
            query.push_str(" AND position_id = ?");
            params.push(pos_id.to_string());
        }

        if let Some(h) = hours {
            let cutoff = chrono::Utc::now() - chrono::Duration::hours(h as i64);
            query.push_str(" AND timestamp > ?");
            params.push(cutoff.to_rfc3339());
        }

        query.push_str(" ORDER BY timestamp DESC");

        let mut stmt = conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|p| p as &dyn rusqlite::ToSql)
            .collect();
        let transaction_iter = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(self.row_to_trading_transaction(row)?)
        })?;

        let mut transactions = Vec::new();
        for transaction in transaction_iter {
            transactions.push(transaction?);
        }

        Ok(transactions)
    }

    fn row_to_trading_position(
        &self,
        row: &Row
    ) -> Result<crate::types::TradingPosition, rusqlite::Error> {
        let status_str: String = row.get(11)?;
        let status = match status_str.as_str() {
            "Open" => crate::types::PositionStatus::Open,
            "Closed" => crate::types::PositionStatus::Closed,
            "PendingClose" => crate::types::PositionStatus::PendingClose,
            _ => crate::types::PositionStatus::Open,
        };

        let time_category_str: String = row.get(13)?;
        let time_category = match time_category_str.as_str() {
            "Quick" => crate::types::TimeCategory::Quick,
            "Medium" => crate::types::TimeCategory::Medium,
            "Long" => crate::types::TimeCategory::Long,
            "Extended" => crate::types::TimeCategory::Extended,
            _ => crate::types::TimeCategory::Quick,
        };

        Ok(crate::types::TradingPosition {
            id: row.get(0)?,
            token_mint: row.get(1)?,
            entry_price: row.get(2)?,
            entry_amount_sol: row.get(3)?,
            entry_amount_tokens: row.get(4)?,
            current_price: row.get(5)?,
            current_value_sol: row.get(6)?,
            pnl_sol: row.get(7)?,
            pnl_percentage: row.get(8)?,
            opened_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                .unwrap()
                .with_timezone(&Utc),
            last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                .unwrap()
                .with_timezone(&Utc),
            status,
            profit_target: row.get(12)?,
            time_category,
        })
    }

    fn row_to_trading_transaction(
        &self,
        row: &Row
    ) -> Result<crate::types::Transaction, rusqlite::Error> {
        let transaction_type_str: String = row.get(2)?;
        let transaction_type = match transaction_type_str.as_str() {
            "Buy" => crate::types::TransactionType::Buy,
            "Sell" => crate::types::TransactionType::Sell,
            "Transfer" => crate::types::TransactionType::Transfer,
            _ => crate::types::TransactionType::Buy,
        };

        Ok(crate::types::Transaction {
            id: row.get(0)?,
            signature: row.get(1)?,
            transaction_type,
            token_mint: row.get(3)?,
            amount_sol: row.get(4)?,
            amount_tokens: row.get(5)?,
            price: row.get(6)?,
            timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                .unwrap()
                .with_timezone(&Utc),
            block_height: row.get(8)?,
            fee_sol: row.get(9)?,
            position_id: row.get(10)?,
        })
    }
}
