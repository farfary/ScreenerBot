use crate::database::models::{ DatabaseResult, BlacklistedToken };
use crate::database::connection::Database;
use anyhow::Result;
use chrono::{ DateTime, Utc };
use rusqlite::params;

impl Database {
    /// Store blacklisted token information
    pub async fn store_blacklisted_token(
        &self,
        token_address: &str,
        reason: &str,
        liquidity: f64
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO blacklisted_tokens (
                token_address, reason, blacklisted_at, last_liquidity
            ) VALUES (?1, ?2, ?3, ?4)",
            params![token_address, reason, Utc::now().to_rfc3339(), liquidity]
        )?;

        Ok(())
    }

    /// Get blacklisted tokens (addresses only)
    pub async fn get_blacklisted_tokens(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT token_address FROM blacklisted_tokens")?;
        let token_iter = stmt.query_map([], |row| Ok(row.get::<_, String>(0)?))?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Get detailed blacklisted token information
    pub async fn get_blacklisted_tokens_detailed(&self) -> DatabaseResult<Vec<BlacklistedToken>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT token_address, reason, blacklisted_at, last_liquidity FROM blacklisted_tokens ORDER BY blacklisted_at DESC"
        )?;
        let token_iter = stmt.query_map([], |row| {
            Ok(BlacklistedToken {
                token_address: row.get(0)?,
                reason: row.get(1)?,
                blacklisted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_liquidity: row.get(3)?,
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Remove token from blacklist
    pub async fn remove_from_blacklist(&self, token_address: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "DELETE FROM blacklisted_tokens WHERE token_address = ?1",
            params![token_address]
        )?;

        Ok(())
    }

    /// Check if token is blacklisted
    pub async fn is_token_blacklisted(&self, token_address: &str) -> DatabaseResult<bool> {
        let conn = self.conn.lock().unwrap();

        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM blacklisted_tokens WHERE token_address = ?1",
            params![token_address],
            |row| row.get(0)
        )?;

        Ok(count > 0)
    }

    /// Get blacklisted token details
    pub async fn get_blacklisted_token_details(
        &self,
        token_address: &str
    ) -> DatabaseResult<Option<BlacklistedToken>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT token_address, reason, blacklisted_at, last_liquidity FROM blacklisted_tokens WHERE token_address = ?1"
        )?;

        let mut token_iter = stmt.query_map([token_address], |row| {
            Ok(BlacklistedToken {
                token_address: row.get(0)?,
                reason: row.get(1)?,
                blacklisted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_liquidity: row.get(3)?,
            })
        })?;

        if let Some(token) = token_iter.next() {
            return Ok(Some(token?));
        }

        Ok(None)
    }

    /// Get blacklisted tokens by reason
    pub async fn get_blacklisted_tokens_by_reason(
        &self,
        reason: &str
    ) -> DatabaseResult<Vec<BlacklistedToken>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT token_address, reason, blacklisted_at, last_liquidity FROM blacklisted_tokens WHERE reason = ?1 ORDER BY blacklisted_at DESC"
        )?;

        let token_iter = stmt.query_map([reason], |row| {
            Ok(BlacklistedToken {
                token_address: row.get(0)?,
                reason: row.get(1)?,
                blacklisted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                    .unwrap()
                    .with_timezone(&Utc),
                last_liquidity: row.get(3)?,
            })
        })?;

        let mut tokens = Vec::new();
        for token in token_iter {
            tokens.push(token?);
        }

        Ok(tokens)
    }

    /// Update blacklist reason
    pub async fn update_blacklist_reason(
        &self,
        token_address: &str,
        new_reason: &str
    ) -> DatabaseResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE blacklisted_tokens SET reason = ?1 WHERE token_address = ?2",
            params![new_reason, token_address]
        )?;

        Ok(())
    }

    /// Get blacklist statistics
    pub async fn get_blacklist_statistics(&self) -> DatabaseResult<BlacklistStatistics> {
        let conn = self.conn.lock().unwrap();

        let total_blacklisted: u64 = conn.query_row(
            "SELECT COUNT(*) FROM blacklisted_tokens",
            [],
            |row| row.get(0)
        )?;

        let reasons_count: Vec<(String, u64)> = {
            let mut stmt = conn.prepare(
                "SELECT reason, COUNT(*) as count FROM blacklisted_tokens GROUP BY reason ORDER BY count DESC"
            )?;
            let reason_iter = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
            })?;

            let mut reasons = Vec::new();
            for reason in reason_iter {
                reasons.push(reason?);
            }
            reasons
        };

        let avg_liquidity: f64 = conn.query_row(
            "SELECT COALESCE(AVG(last_liquidity), 0) FROM blacklisted_tokens",
            [],
            |row| row.get(0)
        )?;

        let recent_blacklisted: u64 = {
            let cutoff_time = Utc::now() - chrono::Duration::hours(24);
            conn.query_row(
                "SELECT COUNT(*) FROM blacklisted_tokens WHERE blacklisted_at >= ?1",
                params![cutoff_time.to_rfc3339()],
                |row| row.get(0)
            )?
        };

        Ok(BlacklistStatistics {
            total_blacklisted,
            reasons_count,
            avg_liquidity,
            recent_blacklisted,
        })
    }

    /// Clean up old blacklisted tokens
    pub async fn cleanup_old_blacklisted_tokens(&self, max_age_days: u64) -> DatabaseResult<u64> {
        let conn = self.conn.lock().unwrap();
        let cutoff_date = Utc::now() - chrono::Duration::days(max_age_days as i64);

        let rows_affected = conn.execute(
            "DELETE FROM blacklisted_tokens WHERE blacklisted_at < ?1",
            params![cutoff_date.to_rfc3339()]
        )?;

        Ok(rows_affected as u64)
    }
}

/// Blacklist statistics
#[derive(Debug, Clone)]
pub struct BlacklistStatistics {
    pub total_blacklisted: u64,
    pub reasons_count: Vec<(String, u64)>,
    pub avg_liquidity: f64,
    pub recent_blacklisted: u64,
}
