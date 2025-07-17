use crate::database::models::DatabaseResult;
use crate::database::connection::Database;
use crate::market_data::{ PoolInfo, PoolType, TokenInfo as PricingTokenInfo };
use anyhow::Result;
use rusqlite::{ params, Row };

impl Database {
    /// Update token info including pools
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

    /// Update pool information
    pub async fn update_pool_info(&self, pool: &PoolInfo) -> DatabaseResult<()> {
        let conn = self.conn.lock().unwrap();
        self.update_pool_info_internal(&conn, pool)?;
        Ok(())
    }

    /// Get pools for a specific token
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
            pools.push(pool.map_err(|_| anyhow::anyhow!("Failed to parse pool row"))?);
        }

        Ok(pools)
    }

    /// Get all pools with optional filtering
    pub async fn get_pools(
        &self,
        min_liquidity: Option<f64>,
        limit: Option<u32>
    ) -> DatabaseResult<Vec<PoolInfo>> {
        let conn = self.conn.lock().unwrap();

        let mut query = "SELECT * FROM pools".to_string();
        let mut params = Vec::new();

        if let Some(min_liq) = min_liquidity {
            query.push_str(" WHERE liquidity_usd >= ?");
            params.push(min_liq);
        }

        query.push_str(" ORDER BY liquidity_usd DESC");

        if let Some(lim) = limit {
            query.push_str(" LIMIT ?");
            params.push(lim as f64);
        }

        let mut stmt = conn.prepare(&query)?;
        let pool_iter = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            match self.row_to_pool_info(row) {
                Ok(pool) => Ok(pool),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool.map_err(|_| anyhow::anyhow!("Failed to parse pool row"))?);
        }

        Ok(pools)
    }

    /// Get top pools by liquidity
    pub async fn get_top_pools_by_liquidity(&self, limit: u32) -> DatabaseResult<Vec<PoolInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM pools ORDER BY liquidity_usd DESC LIMIT ?1")?;

        let pool_iter = stmt.query_map([limit], |row| {
            match self.row_to_pool_info(row) {
                Ok(pool) => Ok(pool),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool.map_err(|_| anyhow::anyhow!("Failed to parse pool row"))?);
        }

        Ok(pools)
    }

    /// Get pools by type
    pub async fn get_pools_by_type(&self, pool_type: &PoolType) -> DatabaseResult<Vec<PoolInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM pools WHERE pool_type = ?1 ORDER BY liquidity_usd DESC"
        )?;

        let pool_iter = stmt.query_map([format!("{:?}", pool_type)], |row| {
            match self.row_to_pool_info(row) {
                Ok(pool) => Ok(pool),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        let mut pools = Vec::new();
        for pool in pool_iter {
            pools.push(pool.map_err(|_| anyhow::anyhow!("Failed to parse pool row"))?);
        }

        Ok(pools)
    }

    /// Delete a pool
    pub async fn delete_pool(&self, pool_address: &str) -> DatabaseResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM pools WHERE address = ?1", params![pool_address])?;
        Ok(())
    }

    /// Get pool statistics
    pub async fn get_pool_statistics(&self) -> DatabaseResult<PoolStatistics> {
        let conn = self.conn.lock().unwrap();

        let total_pools: u64 = conn.query_row("SELECT COUNT(*) FROM pools", [], |row| row.get(0))?;

        let total_liquidity: f64 = conn.query_row(
            "SELECT COALESCE(SUM(liquidity_usd), 0) FROM pools",
            [],
            |row| row.get(0)
        )?;

        let avg_liquidity: f64 = conn.query_row(
            "SELECT COALESCE(AVG(liquidity_usd), 0) FROM pools",
            [],
            |row| row.get(0)
        )?;

        let pool_types: Vec<(String, u64)> = {
            let mut stmt = conn.prepare(
                "SELECT pool_type, COUNT(*) as count FROM pools GROUP BY pool_type"
            )?;
            let type_iter = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?;

            let mut types = Vec::new();
            for type_result in type_iter {
                types.push(type_result?);
            }
            types
        };

        Ok(PoolStatistics {
            total_pools,
            total_liquidity,
            avg_liquidity,
            pool_types,
        })
    }

    /// Internal helper to update pool info
    pub(crate) fn update_pool_info_internal(
        &self,
        conn: &rusqlite::Connection,
        pool: &PoolInfo
    ) -> Result<()> {
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

    /// Helper method to convert database row to PoolInfo
    pub(crate) fn row_to_pool_info(&self, row: &Row) -> Result<PoolInfo, anyhow::Error> {
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
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_pools: u64,
    pub total_liquidity: f64,
    pub avg_liquidity: f64,
    pub pool_types: Vec<(String, u64)>,
}
