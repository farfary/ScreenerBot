use crate::pool::types::*;
use anyhow::{ Context, Result };
use chrono::{ DateTime, Utc };
use rusqlite::{ params, Connection, Row };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::Mutex;

/// Database for storing pool information and reserves history
pub struct PoolDatabase {
    connection: Mutex<Connection>,
}

impl PoolDatabase {
    pub fn new() -> Result<Self> {
        let conn = Connection::open("pool.db").context("Failed to open pool database")?;

        let db = Self {
            connection: Mutex::new(conn),
        };

        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        // Pool info table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pool_info (
                pool_address TEXT PRIMARY KEY,
                pool_type TEXT NOT NULL,
                base_token_mint TEXT NOT NULL,
                quote_token_mint TEXT NOT NULL,
                base_token_decimals INTEGER NOT NULL,
                quote_token_decimals INTEGER NOT NULL,
                liquidity_usd REAL NOT NULL DEFAULT 0.0,
                fee_rate REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL,
                last_updated TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            )",
            []
        )?;

        // Pool reserves history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pool_reserves (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pool_address TEXT NOT NULL,
                base_token_amount INTEGER NOT NULL,
                quote_token_amount INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                slot INTEGER NOT NULL,
                FOREIGN KEY (pool_address) REFERENCES pool_info (pool_address)
            )",
            []
        )?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pool_reserves_address_timestamp 
             ON pool_reserves (pool_address, timestamp)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pool_reserves_timestamp 
             ON pool_reserves (timestamp)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pool_info_tokens 
             ON pool_info (base_token_mint, quote_token_mint)",
            []
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pool_info_type 
             ON pool_info (pool_type)",
            []
        )?;

        println!("✅ Pool database tables created successfully");
        Ok(())
    }

    /// Save pool information
    pub fn save_pool_info(&self, pool: &PoolInfo) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO pool_info (
                pool_address, pool_type, base_token_mint, quote_token_mint,
                base_token_decimals, quote_token_decimals, liquidity_usd, fee_rate,
                created_at, last_updated, is_active
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                pool.pool_address,
                pool.pool_type.to_string(),
                pool.base_token_mint,
                pool.quote_token_mint,
                pool.base_token_decimals,
                pool.quote_token_decimals,
                pool.liquidity_usd,
                pool.fee_rate,
                pool.created_at.to_rfc3339(),
                pool.last_updated.to_rfc3339(),
                pool.is_active as i32
            ]
        )?;

        Ok(())
    }

    /// Save pool reserves
    pub fn save_pool_reserves(&self, reserves: &PoolReserve) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        conn.execute(
            "INSERT INTO pool_reserves (
                pool_address, base_token_amount, quote_token_amount, timestamp, slot
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                reserves.pool_address,
                reserves.base_token_amount as i64,
                reserves.quote_token_amount as i64,
                reserves.timestamp.to_rfc3339(),
                reserves.slot as i64
            ]
        )?;

        Ok(())
    }

    /// Get pool information by address
    pub fn get_pool_info(&self, pool_address: &str) -> Result<Option<PoolInfo>> {
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT pool_address, pool_type, base_token_mint, quote_token_mint,
                    base_token_decimals, quote_token_decimals, liquidity_usd, fee_rate,
                    created_at, last_updated, is_active
             FROM pool_info 
             WHERE pool_address = ?1"
        )?;

        let pool_iter = stmt.query_map(params![pool_address], |row| {
            Ok(PoolInfo {
                pool_address: row.get(0)?,
                pool_type: PoolType::from(row.get::<_, String>(1)?.as_str()),
                base_token_mint: row.get(2)?,
                quote_token_mint: row.get(3)?,
                base_token_decimals: row.get(4)?,
                quote_token_decimals: row.get(5)?,
                liquidity_usd: row.get(6)?,
                fee_rate: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&Utc),
                is_active: row.get::<_, i32>(10)? == 1,
            })
        })?;

        for pool in pool_iter {
            return Ok(Some(pool?));
        }

        Ok(None)
    }

    /// Get latest reserves for a pool
    pub fn get_latest_reserves(&self, pool_address: &str) -> Result<Option<PoolReserve>> {
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT pool_address, base_token_amount, quote_token_amount, timestamp, slot
             FROM pool_reserves 
             WHERE pool_address = ?1 
             ORDER BY timestamp DESC 
             LIMIT 1"
        )?;

        let reserve_iter = stmt.query_map(params![pool_address], |row| {
            Ok(PoolReserve {
                pool_address: row.get(0)?,
                base_token_amount: row.get::<_, i64>(1)? as u64,
                quote_token_amount: row.get::<_, i64>(2)? as u64,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                slot: row.get::<_, i64>(4)? as u64,
            })
        })?;

        for reserve in reserve_iter {
            return Ok(Some(reserve?));
        }

        Ok(None)
    }

    /// Get pools for a specific token
    pub fn get_token_pools(&self, token_mint: &str) -> Result<Vec<PoolInfo>> {
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT pool_address, pool_type, base_token_mint, quote_token_mint,
                    base_token_decimals, quote_token_decimals, liquidity_usd, fee_rate,
                    created_at, last_updated, is_active
             FROM pool_info 
             WHERE (base_token_mint = ?1 OR quote_token_mint = ?1) AND is_active = 1
             ORDER BY liquidity_usd DESC"
        )?;

        let pool_iter = stmt.query_map(params![token_mint], |row| {
            Ok(PoolInfo {
                pool_address: row.get(0)?,
                pool_type: PoolType::from(row.get::<_, String>(1)?.as_str()),
                base_token_mint: row.get(2)?,
                quote_token_mint: row.get(3)?,
                base_token_decimals: row.get(4)?,
                quote_token_decimals: row.get(5)?,
                liquidity_usd: row.get(6)?,
                fee_rate: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&Utc),
                is_active: row.get::<_, i32>(10)? == 1,
            })
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool?);
        }

        Ok(pools)
    }

    /// Get pool history for a specific time range
    pub fn get_pool_history(
        &self,
        pool_address: &str,
        hours_back: i64
    ) -> Result<Vec<PoolReserve>> {
        let conn = self.connection.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::hours(hours_back);

        let mut stmt = conn.prepare(
            "SELECT pool_address, base_token_amount, quote_token_amount, timestamp, slot
             FROM pool_reserves 
             WHERE pool_address = ?1 AND timestamp >= ?2 
             ORDER BY timestamp DESC"
        )?;

        let reserve_iter = stmt.query_map(params![pool_address, cutoff_time.to_rfc3339()], |row| {
            Ok(PoolReserve {
                pool_address: row.get(0)?,
                base_token_amount: row.get::<_, i64>(1)? as u64,
                quote_token_amount: row.get::<_, i64>(2)? as u64,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap()
                    .with_timezone(&Utc),
                slot: row.get::<_, i64>(4)? as u64,
            })
        })?;

        let mut reserves = Vec::new();
        for reserve in reserve_iter {
            reserves.push(reserve?);
        }

        Ok(reserves)
    }

    /// Get all active pools
    pub fn get_all_active_pools(&self) -> Result<Vec<PoolInfo>> {
        let conn = self.connection.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT pool_address, pool_type, base_token_mint, quote_token_mint,
                    base_token_decimals, quote_token_decimals, liquidity_usd, fee_rate,
                    created_at, last_updated, is_active
             FROM pool_info 
             WHERE is_active = 1
             ORDER BY liquidity_usd DESC"
        )?;

        let pool_iter = stmt.query_map([], |row| {
            Ok(PoolInfo {
                pool_address: row.get(0)?,
                pool_type: PoolType::from(row.get::<_, String>(1)?.as_str()),
                base_token_mint: row.get(2)?,
                quote_token_mint: row.get(3)?,
                base_token_decimals: row.get(4)?,
                quote_token_decimals: row.get(5)?,
                liquidity_usd: row.get(6)?,
                fee_rate: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&Utc),
                is_active: row.get::<_, i32>(10)? == 1,
            })
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool?);
        }

        Ok(pools)
    }

    /// Get statistics about pools
    pub fn get_stats(&self) -> Result<PoolStats> {
        let conn = self.connection.lock().unwrap();

        // Get total pools
        let total_pools: u64 = conn.query_row("SELECT COUNT(*) FROM pool_info", [], |row|
            row.get(0)
        )?;

        // Get active pools
        let active_pools: u64 = conn.query_row(
            "SELECT COUNT(*) FROM pool_info WHERE is_active = 1",
            [],
            |row| row.get(0)
        )?;

        // Get pools by type
        let mut stmt = conn.prepare(
            "SELECT pool_type, COUNT(*) FROM pool_info WHERE is_active = 1 GROUP BY pool_type"
        )?;

        let type_iter = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
        })?;

        let mut pools_by_type = HashMap::new();
        for result in type_iter {
            let (pool_type, count) = result?;
            pools_by_type.insert(pool_type, count);
        }

        // Get total reserves history
        let total_reserves_history: u64 = conn.query_row(
            "SELECT COUNT(*) FROM pool_reserves",
            [],
            |row| row.get(0)
        )?;

        // Calculate update rate (reserves per hour in last 24 hours)
        let update_rate_per_hour = conn.query_row(
            "SELECT COUNT(*) FROM pool_reserves WHERE timestamp >= datetime('now', '-24 hours')",
            [],
            |row| {
                let count: u64 = row.get(0)?;
                Ok((count as f64) / 24.0)
            }
        )?;

        Ok(PoolStats {
            total_pools,
            active_pools,
            pools_by_type,
            total_reserves_history,
            last_update: Utc::now(),
            update_rate_per_hour,
        })
    }

    /// Clean old reserves data (keep only last N days)
    pub fn cleanup_old_reserves(&self, keep_days: i64) -> Result<u64> {
        let conn = self.connection.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::days(keep_days);

        let affected_rows = conn.execute(
            "DELETE FROM pool_reserves WHERE timestamp < ?1",
            params![cutoff_time.to_rfc3339()]
        )?;

        Ok(affected_rows as u64)
    }

    /// Deactivate a pool
    pub fn deactivate_pool(&self, pool_address: &str) -> Result<()> {
        let conn = self.connection.lock().unwrap();

        conn.execute(
            "UPDATE pool_info SET is_active = 0, last_updated = ?1 WHERE pool_address = ?2",
            params![Utc::now().to_rfc3339(), pool_address]
        )?;

        Ok(())
    }

    /// Get pools that haven't been updated in X hours
    pub fn get_stale_pools(&self, hours: i64) -> Result<Vec<PoolInfo>> {
        let conn = self.connection.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::hours(hours);

        let mut stmt = conn.prepare(
            "SELECT pool_address, pool_type, base_token_mint, quote_token_mint,
                    base_token_decimals, quote_token_decimals, liquidity_usd, fee_rate,
                    created_at, last_updated, is_active
             FROM pool_info 
             WHERE is_active = 1 AND last_updated < ?1
             ORDER BY last_updated ASC"
        )?;

        let pool_iter = stmt.query_map(params![cutoff_time.to_rfc3339()], |row| {
            Ok(PoolInfo {
                pool_address: row.get(0)?,
                pool_type: PoolType::from(row.get::<_, String>(1)?.as_str()),
                base_token_mint: row.get(2)?,
                quote_token_mint: row.get(3)?,
                base_token_decimals: row.get(4)?,
                quote_token_decimals: row.get(5)?,
                liquidity_usd: row.get(6)?,
                fee_rate: row.get(7)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_updated: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .with_timezone(&Utc),
                is_active: row.get::<_, i32>(10)? == 1,
            })
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool?);
        }

        Ok(pools)
    }
}
