use crate::database::DatabaseResult;
use crate::database::connection::Database;
use crate::market_data::{ TokenPrice, PriceSource };
use anyhow::Result;
use chrono::Utc;
use rusqlite::{ params, Row };

impl Database {
    /// Store token price in database
    pub async fn store_token_price(&self, token_address: &str, price: &TokenPrice) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO token_prices (
                token_address, price_usd, price_sol, market_cap, volume_24h, 
                liquidity_usd, source, timestamp, is_cache
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                token_address,
                price.price_usd,
                price.price_sol,
                price.market_cap,
                price.volume_24h,
                price.liquidity_usd,
                format!("{:?}", price.source),
                price.timestamp,
                if price.is_cached {
                    1
                } else {
                    0
                }
            ]
        )?;

        Ok(())
    }

    /// Get cached token price
    pub async fn get_cached_token_price(&self, token_address: &str) -> Result<Option<TokenPrice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM token_prices WHERE token_address = ?1 ORDER BY timestamp DESC LIMIT 1"
        )?;

        let mut price_iter = stmt.query_map([token_address], |row| {
            match self.row_to_token_price(row) {
                Ok(price) => Ok(price),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        if let Some(price) = price_iter.next() {
            return Ok(Some(price.map_err(|_| rusqlite::Error::InvalidQuery)?));
        }

        Ok(None)
    }

    /// Get token price history
    pub async fn get_token_price_history(
        &self,
        token_address: &str,
        limit: u32
    ) -> Result<Vec<TokenPrice>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT token_address, price_usd, price_sol, market_cap, volume_24h, 
                    liquidity_usd, timestamp, source, is_cache
             FROM token_prices 
             WHERE token_address = ?1 
             ORDER BY timestamp DESC 
             LIMIT ?2"
        )?;

        let price_iter = stmt.query_map(params![token_address, limit], |row| {
            match self.row_to_token_price(row) {
                Ok(price) => Ok(price),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        let mut prices = Vec::new();
        for price in price_iter {
            prices.push(price.map_err(|_| anyhow::anyhow!("Failed to parse price row"))?);
        }

        Ok(prices)
    }

    /// Get latest prices for multiple tokens
    pub async fn get_latest_prices(
        &self,
        token_addresses: &[String]
    ) -> DatabaseResult<Vec<TokenPrice>> {
        let conn = self.conn.lock().unwrap();

        let placeholders = token_addresses
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query =
            format!("SELECT tp.* FROM token_prices tp
             INNER JOIN (
                 SELECT token_address, MAX(timestamp) as latest_timestamp
                 FROM token_prices
                 WHERE token_address IN ({})
                 GROUP BY token_address
             ) latest ON tp.token_address = latest.token_address AND tp.timestamp = latest.latest_timestamp", placeholders);

        let mut stmt = conn.prepare(&query)?;
        let price_iter = stmt.query_map(rusqlite::params_from_iter(token_addresses.iter()), |row| {
            match self.row_to_token_price(row) {
                Ok(price) => Ok(price),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        let mut prices = Vec::new();
        for price in price_iter {
            prices.push(price.map_err(|_| anyhow::anyhow!("Failed to parse price row"))?);
        }

        Ok(prices)
    }

    /// Update token price internal helper
    pub(crate) fn update_token_price_internal(
        &self,
        conn: &rusqlite::Connection,
        price: &TokenPrice
    ) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO token_prices 
            (token_address, price_usd, price_sol, market_cap, volume_24h, liquidity_usd, source, timestamp, is_cache)
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
                if price.is_cached {
                    1
                } else {
                    0
                }
            ]
        )?;
        Ok(())
    }

    /// Get price statistics for a token
    pub async fn get_price_statistics(
        &self,
        token_address: &str,
        hours: u32
    ) -> DatabaseResult<Option<PriceStatistics>> {
        let conn = self.conn.lock().unwrap();

        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
        let mut stmt = conn.prepare(
            "SELECT 
                AVG(price_usd) as avg_price,
                MIN(price_usd) as min_price,
                MAX(price_usd) as max_price,
                COUNT(*) as data_points
             FROM token_prices 
             WHERE token_address = ?1 AND timestamp >= ?2"
        )?;

        let mut stats_iter = stmt.query_map(params![token_address, cutoff_time.timestamp()], |row| {
            Ok(PriceStatistics {
                avg_price: row.get(0)?,
                min_price: row.get(1)?,
                max_price: row.get(2)?,
                data_points: row.get(3)?,
            })
        })?;

        if let Some(stats) = stats_iter.next() {
            return Ok(Some(stats?));
        }

        Ok(None)
    }

    /// Clean up old price data
    pub async fn cleanup_old_prices(&self, max_age_hours: u64) -> DatabaseResult<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_age_hours as i64);

        let rows_affected = conn.execute(
            "DELETE FROM token_prices WHERE timestamp < ?1",
            params![cutoff_time.timestamp()]
        )?;

        Ok(rows_affected as u64)
    }

    /// Get current token price
    pub async fn get_token_price(&self, token_address: &str) -> Result<Option<TokenPrice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT token_address, price_usd, price_sol, market_cap, volume_24h, 
                    liquidity_usd, timestamp, source, is_cache
             FROM token_prices 
             WHERE token_address = ?1 
             ORDER BY timestamp DESC 
             LIMIT 1"
        )?;

        let mut price_iter = stmt.query_map([token_address], |row| {
            match self.row_to_token_price(row) {
                Ok(price) => Ok(price),
                Err(_) => Err(rusqlite::Error::InvalidQuery),
            }
        })?;

        if let Some(price) = price_iter.next() {
            return Ok(Some(price.map_err(|_| anyhow::anyhow!("Failed to parse price row"))?));
        }

        Ok(None)
    }

    /// Helper method to convert database row to TokenPrice
    pub(crate) fn row_to_token_price(&self, row: &Row) -> Result<TokenPrice, anyhow::Error> {
        let source_str: String = row.get("source")?;
        let source = match source_str.as_str() {
            "GeckoTerminal" => PriceSource::GeckoTerminal,
            "PoolCalculation" => PriceSource::PoolCalculation,
            "Cache" => PriceSource::Cache,
            _ => PriceSource::Cache,
        };

        Ok(TokenPrice {
            address: row.get("token_address")?,
            price_usd: row.get("price_usd")?,
            price_sol: row.get("price_sol")?,
            market_cap: row.get("market_cap")?,
            volume_24h: row.get("volume_24h")?,
            liquidity_usd: row.get("liquidity_usd")?,
            timestamp: row.get("timestamp")?,
            source,
            is_cached: row.get::<_, i32>("is_cache")? == 1,
        })
    }
}

/// Price statistics for a token over a time period
#[derive(Debug, Clone)]
pub struct PriceStatistics {
    pub avg_price: f64,
    pub min_price: f64,
    pub max_price: f64,
    pub data_points: u32,
}
