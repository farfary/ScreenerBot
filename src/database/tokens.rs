use crate::database::models::{ DatabaseResult, QueryParams, TrackedToken };
use crate::database::connection::Database;
use crate::types::TokenInfo;
use anyhow::Result;
use chrono::{ DateTime, Utc };
use rusqlite::{ params, Row };

impl Database {
    /// Save a token to the database
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

    /// Get a token by its mint address
    pub fn get_token(&self, mint: &str) -> Result<Option<TokenInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM tokens WHERE mint = ?1")?;
        let mut token_iter = stmt.query_map([mint], |row| self.row_to_token_info(row))?;

        if let Some(token) = token_iter.next() {
            return Ok(Some(token?));
        }

        Ok(None)
    }

    /// Get active tokens with optional limit
    pub fn get_active_tokens(&self, limit: Option<u32>) -> Result<Vec<TokenInfo>> {
        let conn = self.conn.lock().unwrap();
        let query = match limit {
            Some(l) =>
                format!("SELECT * FROM tokens WHERE is_active = 1 ORDER BY last_updated DESC LIMIT {}", l),
            None =>
                "SELECT * FROM tokens WHERE is_active = 1 ORDER BY last_updated DESC".to_string(),
        };

        let mut stmt = conn.prepare(&query)?;
        let token_iter = stmt.query_map([], |row| self.row_to_token_info(row))?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get tokens with advanced filtering
    pub async fn get_tokens_filtered(
        &self,
        params: &QueryParams
    ) -> DatabaseResult<Vec<TokenInfo>> {
        let conn = self.conn.lock().unwrap();

        let mut query = "SELECT * FROM tokens WHERE 1=1".to_string();
        let mut query_params = Vec::new();

        // Add filters
        if let Some(min_liquidity) = params.min_liquidity {
            query.push_str(" AND liquidity >= ?");
            query_params.push(min_liquidity.to_string());
        }

        if let Some(max_age_hours) = params.max_age_hours {
            let cutoff_time = Utc::now() - chrono::Duration::hours(max_age_hours as i64);
            query.push_str(" AND last_updated >= ?");
            query_params.push(cutoff_time.to_rfc3339());
        }

        // Add ordering
        if let Some(order_by) = &params.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
            if params.order_desc {
                query.push_str(" DESC");
            }
        }

        // Add limit and offset
        if let Some(limit) = params.limit {
            query.push_str(&format!(" LIMIT {}", limit));
            if let Some(offset) = params.offset {
                query.push_str(&format!(" OFFSET {}", offset));
            }
        }

        let mut stmt = conn.prepare(&query)?;
        let token_iter = stmt.query_map(rusqlite::params_from_iter(query_params), |row| {
            self.row_to_token_info(row)
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Mark a token as inactive
    pub fn mark_token_inactive(&self, mint: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tokens SET is_active = 0, last_updated = ?1 WHERE mint = ?2",
            params![Utc::now().to_rfc3339(), mint]
        )?;
        Ok(())
    }

    /// Mark a token as active
    pub async fn mark_token_active(&self, mint: &str) -> DatabaseResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tokens SET is_active = 1, last_updated = ?1 WHERE mint = ?2",
            params![Utc::now().to_rfc3339(), mint]
        )?;
        Ok(())
    }

    /// Get token count (total and active)
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

    /// Get tracked tokens with their latest market data
    pub async fn get_tracked_tokens(&self) -> Result<Vec<TrackedToken>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT tp.token_address, tp.liquidity_usd, tp.volume_24h, tp.price_usd, tp.timestamp
             FROM token_prices tp
             INNER JOIN (
                 SELECT token_address, MAX(timestamp) as latest_timestamp
                 FROM token_prices
                 GROUP BY token_address
             ) latest ON tp.token_address = latest.token_address AND tp.timestamp = latest.latest_timestamp
             WHERE tp.liquidity_usd > 0"
        )?;

        let token_iter = stmt.query_map([], |row| {
            Ok(TrackedToken {
                address: row.get(0)?,
                liquidity_usd: row.get(1)?,
                volume_24h: row.get(2)?,
                price_usd: row.get(3)?,
                last_updated: row.get(4)?,
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get top tokens by liquidity
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

    /// Delete a token from the database
    pub async fn delete_token(&self, mint: &str) -> DatabaseResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM tokens WHERE mint = ?1", params![mint])?;
        Ok(())
    }

    /// Update token metadata
    pub async fn update_token_metadata(
        &self,
        mint: &str,
        name: &str,
        symbol: &str
    ) -> DatabaseResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tokens SET name = ?1, symbol = ?2, last_updated = ?3 WHERE mint = ?4",
            params![name, symbol, Utc::now().to_rfc3339(), mint]
        )?;
        Ok(())
    }

    /// Helper method to convert database row to TokenInfo
    pub(crate) fn row_to_token_info(&self, row: &Row) -> Result<TokenInfo, rusqlite::Error> {
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
}
