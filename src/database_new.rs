use crate::types::{ TokenInfo, WalletPosition, DiscoveryStats };
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
}
