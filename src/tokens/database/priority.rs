//! Token priority database — stores and queries token processing priority levels.

use rusqlite::{params, params_from_iter};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::tokens::types::{Priority, TokenError, TokenResult};

use super::TokenDatabase;

impl TokenDatabase {
    /// Fetch token mints with the given priority level
    pub fn get_tokens_by_priority(&self, priority: i32, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT mint FROM update_tracking 
                 WHERE chain_id = ?1 AND priority = ?2
                 AND (last_error_at IS NULL OR last_error_at < strftime('%s','now') - 180)
                 AND (market_error_type IS NULL OR market_error_type != 'permanent')
                 ORDER BY market_data_last_updated_at ASC NULLS FIRST 
                 LIMIT ?3",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mints = stmt
            .query_map(params![self.chain_id(), priority, limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {e}")))
    }

    /// Get oldest non-blacklisted tokens (excludes permanently failed market data tokens)

    pub fn get_oldest_non_blacklisted(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
             LEFT JOIN blacklist b ON t.chain_id = b.chain_id AND t.mint = b.mint
             LEFT JOIN update_tracking u ON t.chain_id = u.chain_id AND t.mint = u.mint
             WHERE t.chain_id = ?1 AND b.mint IS NULL
             AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
             ORDER BY COALESCE(u.market_data_last_updated_at, 0) ASC
             LIMIT ?2",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mints = stmt
            .query_map(params![self.chain_id(), limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {e}")))
    }

    /// Update priority for a token

    pub fn update_priority(&self, mint: &str, priority: i32) -> TokenResult<()> {
        // Validate priority value (Bug #29 fix)
        let valid_priorities = [10, 25, 40, 55, 60, 75, 100];
        if !valid_priorities.contains(&priority) {
            return Err(TokenError::Database(format!(
                "Invalid priority value: {}. Must be one of: 10, 25, 40, 55, 60, 75, 100",
                priority
            )));
        }

        let conn = self.conn()?;

        conn.execute(
            "UPDATE update_tracking SET priority = ?1 WHERE chain_id = ?2 AND mint = ?3",
            params![priority, self.chain_id(), mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to update priority: {e}")))?;

        Ok(())
    }

    /// Update priority for multiple tokens in a single transaction
    pub fn batch_update_priority(&self, mints: &[String], priority: i32) -> TokenResult<usize> {
        if mints.is_empty() {
            return Ok(0);
        }

        // Validate priority value (Bug #29 fix)
        let valid_priorities = [10, 25, 40, 55, 60, 75, 100];
        if !valid_priorities.contains(&priority) {
            return Err(TokenError::Database(format!(
                "Invalid priority value: {}. Must be one of: 10, 25, 40, 55, 60, 75, 100",
                priority
            )));
        }

        let mut conn = self.conn()?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {e}")))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE update_tracking SET priority = ?1 WHERE chain_id = ?2 AND mint = ?3",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {e}")))?;

            for mint in mints {
                match stmt.execute(params![priority, self.chain_id(), mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_update_priority error for {mint}: {e}"),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {e}")))?;

        Ok(updated)
    }

    /// Batch update rejection status for multiple tokens (PERF optimization)
    /// updates: Vec of (mint, reason, source, rejected_at)

    pub fn get_priorities_for_tokens(&self, mints: &[String]) -> TokenResult<HashMap<String, i32>> {
        if mints.is_empty() {
            return Ok(HashMap::new());
        }

        let conn = self.conn()?;

        let mut placeholders = String::new();
        for (idx, _) in mints.iter().enumerate() {
            if idx > 0 {
                placeholders.push(',');
            }
            placeholders.push('?');
        }

        let query = format!(
            "SELECT mint, priority FROM update_tracking WHERE chain_id = ? AND mint IN ({})",
            placeholders
        );

        let mint_refs: Vec<&str> = std::iter::once(self.chain_id())
            .chain(mints.iter().map(String::as_str))
            .collect();

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let rows = stmt
            .query_map(params_from_iter(mint_refs.into_iter()), |row| {
                let mint: String = row.get(0)?;
                let priority: i32 = row.get(1)?;
                Ok((mint, priority))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        let mut result = HashMap::new();
        for row in rows {
            let (mint, priority) =
                row.map_err(|e| TokenError::Database(format!("Row parse failed: {e}")))?;
            result.insert(mint, priority);
        }

        Ok(result)
    }

    /// Get counts of tokens at each priority level
    pub fn summarize_priorities(&self) -> TokenResult<Vec<(i32, u64)>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT priority, COUNT(*) FROM update_tracking WHERE chain_id = ?1 GROUP BY priority ORDER BY priority DESC",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let rows = stmt
            .query_map(params![self.chain_id()], |row| {
                let priority: i32 = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((priority, count.max(0) as u64))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect priority summary: {e}")))
    }

    /// Get the current priority level for a specific token
    pub fn get_priority(&self, mint: &str) -> TokenResult<Priority> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare("SELECT priority FROM update_tracking WHERE chain_id = ?1 AND mint = ?2")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let priority: i32 = stmt
            .query_row(params![self.chain_id(), mint], |row| row.get(0))
            .unwrap_or(10); // Default to Low priority

        Ok(Priority::from_value(priority))
    }
}
