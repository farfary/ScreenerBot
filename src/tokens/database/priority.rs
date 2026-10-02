// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token priority database — stores and queries token processing priority levels.

use crate::errors::DatabaseError;
use rusqlite::{params, params_from_iter};
use std::collections::HashMap;

use crate::logger::{self, LogTag};
use crate::tokens::types::{Priority, TokenResult};
use crate::tokens::Error;

use super::TokenDatabase;
use crate::database::WriteTransaction;

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
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mints = stmt
            .query_map(params![self.chain_id(), priority, limit], |row| row.get(0))
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        mints.collect::<Result<Vec<_>, _>>().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to collect".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Get oldest non-blacklisted tokens (excludes permanently failed market data tokens).
    ///
    /// Ordered by `COALESCE(market_data_last_updated_at, 0) ASC`: a token without an
    /// `update_tracking` row sorts as 0, and ties come back in no defined order. A single
    /// `tokens LEFT JOIN update_tracking` sorted on that expression read every token and
    /// sorted it in a temp b-tree (2.2s at 553k tokens), so the result is built in two
    /// bounded steps instead:
    /// 1. Tokens with no `update_tracking` row. `update_tracking` references `tokens` by
    ///    foreign key, so the per-chain row-count difference is exactly how many exist.
    ///    The anti-join is a full pass over `tokens` that no index can serve, so it runs
    ///    only when that difference is non-zero and stops once all of them are found.
    /// 2. Tracked rows walked in key order through `idx_tracking_market_age_active`,
    ///    stopping at the remaining limit.
    ///
    /// Market timestamps are unix seconds, never negative, so every untracked token
    /// (key 0) sorts no later than any tracked row and step 1 precedes step 2.
    pub fn get_oldest_non_blacklisted(&self, limit: usize) -> TokenResult<Vec<String>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self.conn()?;
        let query_error = |operation: &str, e: rusqlite::Error| {
            Error::Database(DatabaseError::Query {
                operation: operation.to_owned(),
                message: e.to_string(),
            })
        };

        let (token_count, tracked_count): (i64, i64) = conn
            .query_row(
                "SELECT (SELECT COUNT(*) FROM tokens WHERE chain_id = ?1),
                        (SELECT COUNT(*) FROM update_tracking WHERE chain_id = ?1)",
                params![self.chain_id()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| query_error("Failed to count tracked tokens", e))?;

        // A negative difference means orphaned tracking rows, which the foreign key
        // forbids; the count then says nothing about untracked tokens, so scan unbounded.
        let untracked_limit = match token_count - tracked_count {
            0 => 0,
            missing if missing > 0 => limit.min(usize::try_from(missing).unwrap_or(limit)),
            _ => limit,
        };

        let mut mints: Vec<String> = Vec::with_capacity(limit);

        if untracked_limit > 0 {
            let mut stmt = conn
                .prepare(
                    "SELECT t.mint FROM tokens t
                     WHERE t.chain_id = ?1
                     AND NOT EXISTS (SELECT 1 FROM update_tracking u
                                     WHERE u.chain_id = t.chain_id AND u.mint = t.mint)
                     AND NOT EXISTS (SELECT 1 FROM blacklist b
                                     WHERE b.chain_id = t.chain_id AND b.mint = t.mint)
                     LIMIT ?2",
                )
                .map_err(|e| query_error("Failed to prepare untracked tokens", e))?;
            let rows = stmt
                .query_map(params![self.chain_id(), untracked_limit], |row| row.get(0))
                .map_err(|e| query_error("Untracked tokens query failed", e))?;
            for row in rows {
                mints.push(row.map_err(|e| query_error("Failed to collect untracked tokens", e))?);
            }
        }

        let remaining = limit - mints.len();
        if remaining > 0 {
            let mut stmt = conn
                .prepare(
                    "SELECT u.mint FROM update_tracking u
                     WHERE u.chain_id = ?1
                     AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
                     AND NOT EXISTS (SELECT 1 FROM blacklist b
                                     WHERE b.chain_id = u.chain_id AND b.mint = u.mint)
                     ORDER BY COALESCE(u.market_data_last_updated_at, 0) ASC
                     LIMIT ?2",
                )
                .map_err(|e| query_error("Failed to prepare tracked tokens", e))?;
            let rows = stmt
                .query_map(params![self.chain_id(), remaining], |row| row.get(0))
                .map_err(|e| query_error("Tracked tokens query failed", e))?;
            for row in rows {
                mints.push(row.map_err(|e| query_error("Failed to collect tracked tokens", e))?);
            }
        }

        Ok(mints)
    }

    /// Update priority for a token

    pub fn update_priority(&self, mint: &str, priority: i32) -> TokenResult<()> {
        // Validate priority value (Bug #29 fix)
        let valid_priorities = [10, 25, 40, 55, 60, 75, 100];
        if !valid_priorities.contains(&priority) {
            return Err(Error::InvalidPriority { value: priority });
        }

        let conn = self.conn()?;

        conn.execute(
            "UPDATE update_tracking SET priority = ?1 WHERE chain_id = ?2 AND mint = ?3",
            params![priority, self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to update priority".to_owned(),
                message: e.to_string(),
            })
        })?;

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
            return Err(Error::InvalidPriority { value: priority });
        }

        let mut conn = self.conn()?;

        let tx = conn.write_tx().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Transaction start failed".to_owned(),
                message: e.to_string(),
            })
        })?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE update_tracking SET priority = ?1 WHERE chain_id = ?2 AND mint = ?3",
                )
                .map_err(|e| {
                    Error::Database(DatabaseError::Query {
                        operation: "Prepare failed".to_owned(),
                        message: e.to_string(),
                    })
                })?;

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

        tx.commit().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Transaction commit failed".to_owned(),
                message: e.to_string(),
            })
        })?;

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

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare".to_owned(),
                message: e.to_string(),
            })
        })?;

        let rows = stmt
            .query_map(params_from_iter(mint_refs.into_iter()), |row| {
                let mint: String = row.get(0)?;
                let priority: i32 = row.get(1)?;
                Ok((mint, priority))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut result = HashMap::new();
        for row in rows {
            let (mint, priority) = row.map_err(|e| Error::RowDecode {
                detail: e.to_string(),
            })?;
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
            .map_err(|e| Error::Database(DatabaseError::Query { operation: "Failed to prepare".to_owned(), message: e.to_string() }))?;

        let rows = stmt
            .query_map(params![self.chain_id()], |row| {
                let priority: i32 = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((priority, count.max(0) as u64))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to collect priority summary".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Get the current priority level for a specific token
    pub fn get_priority(&self, mint: &str) -> TokenResult<Priority> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare("SELECT priority FROM update_tracking WHERE chain_id = ?1 AND mint = ?2")
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let priority: i32 = stmt
            .query_row(params![self.chain_id(), mint], |row| row.get(0))
            .unwrap_or(10); // Default to Low priority

        Ok(Priority::from_value(priority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;
    use std::collections::HashSet;

    /// The single-join selection `get_oldest_non_blacklisted` replaced, kept as the oracle.
    const SINGLE_JOIN_ORACLE: &str = "SELECT t.mint FROM tokens t
             LEFT JOIN blacklist b ON t.chain_id = b.chain_id AND t.mint = b.mint
             LEFT JOIN update_tracking u ON t.chain_id = u.chain_id AND t.mint = u.mint
             WHERE t.chain_id = ?1 AND b.mint IS NULL
             AND (u.market_error_type IS NULL OR u.market_error_type != 'permanent')
             ORDER BY COALESCE(u.market_data_last_updated_at, 0) ASC
             LIMIT ?2";

    fn insert_token(conn: &rusqlite::Connection, chain: &str, mint: &str) {
        conn.execute(
            "INSERT INTO tokens (chain_id, mint, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES (?1, ?2, 1, 1, 1)",
            params![chain, mint],
        )
        .expect("insert token");
    }

    fn insert_tracking(
        conn: &rusqlite::Connection,
        chain: &str,
        mint: &str,
        updated_at: Option<i64>,
        error_type: Option<&str>,
    ) {
        conn.execute(
            "INSERT INTO update_tracking (chain_id, mint, market_data_last_updated_at, market_error_type)
             VALUES (?1, ?2, ?3, ?4)",
            params![chain, mint, updated_at, error_type],
        )
        .expect("insert tracking");
    }

    fn insert_blacklist(conn: &rusqlite::Connection, mint: &str) {
        conn.execute(
            "INSERT INTO blacklist (chain_id, mint, reason, source, added_at) VALUES ('solana', ?1, 'test', 'test', 1)",
            params![mint],
        )
        .expect("insert blacklist");
    }

    fn sort_key(conn: &rusqlite::Connection, mint: &str) -> i64 {
        conn.query_row(
            "SELECT COALESCE((SELECT market_data_last_updated_at FROM update_tracking
                              WHERE chain_id = 'solana' AND mint = ?1), 0)",
            params![mint],
            |row| row.get(0),
        )
        .expect("read sort key")
    }

    fn oracle(conn: &rusqlite::Connection, limit: usize) -> Vec<String> {
        let mut stmt = conn.prepare(SINGLE_JOIN_ORACLE).expect("prepare oracle");
        let rows = stmt
            .query_map(params!["solana", limit], |row| row.get(0))
            .expect("run oracle");
        rows.collect::<Result<Vec<String>, _>>()
            .expect("collect oracle")
    }

    /// For every limit, the result must be the oracle's result up to the order of rows that
    /// share a sort key: the same sort-key sequence, only eligible mints, no duplicates, and
    /// exactly the oracle's set once the limit covers every eligible token.
    fn assert_matches_oracle(db: &TokenDatabase, conn: &rusqlite::Connection) {
        let eligible: HashSet<String> = oracle(conn, usize::MAX >> 1).into_iter().collect();
        for limit in 0..=eligible.len() + 2 {
            let expected = oracle(conn, limit);
            let actual = db
                .get_oldest_non_blacklisted(limit)
                .expect("get_oldest_non_blacklisted");

            let expected_keys: Vec<i64> = expected.iter().map(|m| sort_key(conn, m)).collect();
            let actual_keys: Vec<i64> = actual.iter().map(|m| sort_key(conn, m)).collect();
            assert_eq!(
                actual_keys, expected_keys,
                "sort keys differ at limit {limit}"
            );

            let unique: HashSet<&String> = actual.iter().collect();
            assert_eq!(
                unique.len(),
                actual.len(),
                "duplicate mint at limit {limit}"
            );
            assert!(
                actual.iter().all(|m| eligible.contains(m)),
                "ineligible mint at limit {limit}: {actual:?}"
            );
            if limit >= eligible.len() {
                let actual_set: HashSet<String> = actual.into_iter().collect();
                assert_eq!(actual_set, eligible, "full result differs at limit {limit}");
            }
        }
    }

    #[test]
    fn oldest_non_blacklisted_matches_single_join_oracle() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("tokens.db");
        let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana)
            .expect("open tokens database");
        let conn = db.conn().expect("pooled connection");

        // Never tracked: sort as 0.
        for mint in ["untracked-a", "untracked-b", "untracked-blacklisted"] {
            insert_token(&conn, "solana", mint);
        }
        // Tracked with a NULL timestamp: also 0, tied with the untracked tokens.
        for mint in ["null-a", "null-b"] {
            insert_token(&conn, "solana", mint);
            insert_tracking(&conn, "solana", mint, None, None);
        }
        // Tracked old and new, including a tie and a non-permanent error.
        for (mint, at, error) in [
            ("old-a", 100, None),
            ("old-b", 100, None),
            ("old-c", 200, None),
            ("errored", 300, Some("rate_limited")),
            ("new-a", 5_000, None),
            ("new-b", 9_000, None),
            ("permanent-old", 50, Some("permanent")),
            ("blacklisted-old", 10, None),
        ] {
            insert_token(&conn, "solana", mint);
            insert_tracking(&conn, "solana", mint, Some(at), error);
        }
        insert_token(&conn, "solana", "permanent-null");
        insert_tracking(&conn, "solana", "permanent-null", None, Some("permanent"));
        insert_token(&conn, "solana", "blacklisted-null");
        insert_tracking(&conn, "solana", "blacklisted-null", None, None);
        for mint in [
            "untracked-blacklisted",
            "blacklisted-old",
            "blacklisted-null",
        ] {
            insert_blacklist(&conn, mint);
        }
        // Another chain's rows must never surface.
        insert_token(&conn, "ethereum", "foreign-null");
        insert_tracking(&conn, "ethereum", "foreign-null", None, None);
        insert_token(&conn, "ethereum", "foreign-untracked");

        // Untracked tokens present: the anti-join step runs.
        assert_matches_oracle(&db, &conn);

        // Every token tracked: the row counts match and the anti-join step is skipped.
        for mint in ["untracked-a", "untracked-b", "untracked-blacklisted"] {
            insert_tracking(&conn, "solana", mint, None, None);
        }
        assert_matches_oracle(&db, &conn);
    }
}
