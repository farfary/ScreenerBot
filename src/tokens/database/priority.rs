use rusqlite::{params, params_from_iter};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::tokens::types::{Priority, TokenError, TokenResult};

use super::TokenDatabase;

impl TokenDatabase {
    pub fn get_tokens_by_priority(&self, priority: i32, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint FROM update_tracking 
                 WHERE priority = ?1
                 AND (last_error_at IS NULL OR last_error_at < strftime('%s','now') - 180)
                 AND (market_error_type IS NULL OR market_error_type != 'permanent')
                 ORDER BY market_data_last_updated_at ASC NULLS FIRST 
                 LIMIT ?2",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mints = stmt
            .query_map(params![priority, limit], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        mints
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect: {e}")))
    }

    /// Get oldest non-blacklisted tokens (excludes permanently failed market data tokens)

    pub fn get_oldest_non_blacklisted(&self, limit: usize) -> TokenResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT t.mint FROM tokens t
             LEFT JOIN blacklist b ON t.mint = b.mint
             LEFT JOIN update_tracking u ON t.mint = u.mint
             WHERE b.mint IS NULL
             AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
             ORDER BY COALESCE(u.market_data_last_updated_at, 0) ASC
             LIMIT ?1",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mints = stmt
            .query_map(params![limit], |row| row.get(0))
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

        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        conn.execute(
            "UPDATE update_tracking SET priority = ?1 WHERE mint = ?2",
            params![priority, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to update priority: {e}")))?;

        Ok(())
    }

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

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {e}")))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached("UPDATE update_tracking SET priority = ?1 WHERE mint = ?2")
                .map_err(|e| TokenError::Database(format!("Prepare failed: {e}")))?;

            for mint in mints {
                match stmt.execute(params![priority, mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_update_priority error for {}: {}", mint, e),
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

        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut placeholders = String::new();
        for (idx, _) in mints.iter().enumerate() {
            if idx > 0 {
                placeholders.push(',');
            }
            placeholders.push('?');
        }

        let query = format!(
            "SELECT mint, priority FROM update_tracking WHERE mint IN ({})",
            placeholders
        );

        let mint_refs: Vec<&str> = mints.iter().map(|mint| mint.as_str()).collect();

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

    pub fn summarize_priorities(&self) -> TokenResult<Vec<(i32, u64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT priority, COUNT(*) FROM update_tracking GROUP BY priority ORDER BY priority DESC",
            )
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let priority: i32 = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((priority, count.max(0) as u64))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| TokenError::Database(format!("Failed to collect priority summary: {e}")))
    }

    pub fn get_priority(&self, mint: &str) -> TokenResult<Priority> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT priority FROM update_tracking WHERE mint = ?1")
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let priority: i32 = stmt
            .query_row(params![mint], |row| row.get(0))
            .unwrap_or(10); // Default to Low priority

        Ok(Priority::from_value(priority))
    }
}
