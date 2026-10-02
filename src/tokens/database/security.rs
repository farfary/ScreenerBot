// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token security data storage — persists rugcheck scores and safety analysis.

use crate::errors::DatabaseError;
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::tokens::store;
use crate::tokens::types::{RugcheckData, SecurityRisk, TokenHolder, TokenResult};
use crate::tokens::Error;

use super::TokenDatabase;

impl TokenDatabase {
    /// Insert or update Rugcheck security data for a token
    pub fn upsert_rugcheck_data(&self, mint: &str, data: &RugcheckData) -> TokenResult<()> {
        let conn = self.conn()?;

        let risks_json = serde_json::to_string(&data.risks).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to serialize risks".to_owned(),
                message: e.to_string(),
            })
        })?;
        let holders_json = serde_json::to_string(&data.top_holders).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to serialize holders".to_owned(),
                message: e.to_string(),
            })
        })?;
        let markets_json = data
            .markets
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to serialize markets".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let rugged_flag = if data.rugged { 1 } else { 0 };

        // Check if this is first insert (for first_fetched_at tracking)
        let is_first_insert: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM security_rugcheck WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |row| {
                    let count: i64 = row.get(0)?;
                    Ok(count == 0)
                },
            )
            .unwrap_or(true);

        let now_ts = data.security_data_last_fetched_at.timestamp();
        let first_fetched_ts = if is_first_insert {
            now_ts
        } else {
            // Preserve existing first_fetched_at on updates
            conn.query_row(
                "SELECT security_data_first_fetched_at FROM security_rugcheck WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(now_ts)
        };

        let is_mutable_flag = data.is_mutable.map(|b| if b { 1 } else { 0 });

        conn.execute(
            "INSERT INTO security_rugcheck (
                chain_id, mint,
                token_type,
                token_decimals,
                score,
                score_normalised,
                score_description,
                mint_authority,
                freeze_authority,
                update_authority,
                is_mutable,
                top_10_holders_pct,
                total_supply,
                total_holders,
                total_lp_providers,
                graph_insiders_detected,
                total_market_liquidity,
                total_stable_liquidity,
                creator_balance_pct,
                transfer_fee_pct,
                transfer_fee_max_amount,
                transfer_fee_authority,
                rugged,
                risks,
                top_holders,
                markets,
                security_data_last_fetched_at,
                security_data_first_fetched_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28
             )
             ON CONFLICT(chain_id, mint) DO UPDATE SET
                token_type = excluded.token_type,
                token_decimals = excluded.token_decimals,
                score = excluded.score,
                score_normalised = excluded.score_normalised,
                score_description = excluded.score_description,
                mint_authority = excluded.mint_authority,
                freeze_authority = excluded.freeze_authority,
                update_authority = excluded.update_authority,
                is_mutable = excluded.is_mutable,
                top_10_holders_pct = excluded.top_10_holders_pct,
                total_supply = excluded.total_supply,
                total_holders = excluded.total_holders,
                total_lp_providers = excluded.total_lp_providers,
                graph_insiders_detected = excluded.graph_insiders_detected,
                total_market_liquidity = excluded.total_market_liquidity,
                total_stable_liquidity = excluded.total_stable_liquidity,
                creator_balance_pct = excluded.creator_balance_pct,
                transfer_fee_pct = excluded.transfer_fee_pct,
                transfer_fee_max_amount = excluded.transfer_fee_max_amount,
                transfer_fee_authority = excluded.transfer_fee_authority,
                rugged = excluded.rugged,
                risks = excluded.risks,
                top_holders = excluded.top_holders,
                markets = excluded.markets,
                security_data_last_fetched_at = excluded.security_data_last_fetched_at",
            params![
                self.chain_id(),
                mint,
                &data.token_type,
                data.token_decimals,
                data.score,
                data.score_normalised,
                &data.score_description,
                &data.mint_authority,
                &data.freeze_authority,
                &data.update_authority,
                is_mutable_flag,
                data.top_10_holders_pct,
                &data.total_supply,
                data.total_holders,
                data.total_lp_providers,
                data.graph_insiders_detected,
                data.total_market_liquidity,
                data.total_stable_liquidity,
                data.creator_balance_pct,
                data.transfer_fee_pct,
                data.transfer_fee_max_amount,
                &data.transfer_fee_authority,
                rugged_flag,
                risks_json,
                holders_json,
                markets_json,
                now_ts,
                first_fetched_ts,
            ],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to upsert Rugcheck data".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Update in-memory cache
        store::store_rugcheck(self.chain(), mint, data);

        Ok(())
    }

    /// Fetch Rugcheck security data for a token
    pub fn get_rugcheck_data(&self, mint: &str) -> TokenResult<Option<RugcheckData>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT
                    token_type,
                    token_decimals,
                    score,
                    score_normalised,
                    score_description,
                    mint_authority,
                    freeze_authority,
                    top_10_holders_pct,
                    total_supply,
                    total_holders,
                    total_lp_providers,
                    graph_insiders_detected,
                    total_market_liquidity,
                    total_stable_liquidity,
                    creator_balance_pct,
                    transfer_fee_pct,
                    transfer_fee_max_amount,
                    transfer_fee_authority,
                    rugged,
                    risks,
                    top_holders,
                    markets,
                    security_data_last_fetched_at,
                    security_data_first_fetched_at,
                    update_authority,
                    is_mutable
                 FROM security_rugcheck WHERE chain_id = ?1 AND mint = ?2",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to prepare".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let result = stmt.query_row(params![self.chain_id(), mint], |row| {
            let risks_json: String = row.get(19)?;
            let holders_json: String = row.get(20)?;
            let markets_json: Option<String> = row.get(21)?;
            let fetched_ts: i64 = row.get(22)?;
            let first_fetched_ts: i64 = row.get(23)?;
            let rugged_flag: Option<i64> = row.get(18)?;
            let is_rugged = rugged_flag.unwrap_or_default() != 0;
            let is_mutable_flag: Option<i64> = row.get(25)?;
            let is_mutable = is_mutable_flag.map(|f| f != 0);

            let risks: Vec<SecurityRisk> = serde_json::from_str(&risks_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let holders: Vec<TokenHolder> = serde_json::from_str(&holders_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let markets = markets_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(RugcheckData {
                token_type: row.get(0)?,
                token_decimals: row.get(1)?,
                score: row.get(2)?,
                score_normalised: row.get(3)?,
                score_description: row.get(4)?,
                mint_authority: row.get(5)?,
                freeze_authority: row.get(6)?,
                update_authority: row.get(24)?,
                is_mutable,
                top_10_holders_pct: row.get(7)?,
                total_supply: row.get(8)?,
                total_holders: row.get(9)?,
                total_lp_providers: row.get(10)?,
                graph_insiders_detected: row.get(11)?,
                total_market_liquidity: row.get(12)?,
                total_stable_liquidity: row.get(13)?,
                creator_balance_pct: row.get(14)?,
                transfer_fee_pct: row.get(15)?,
                transfer_fee_max_amount: row.get(16)?,
                transfer_fee_authority: row.get(17)?,
                rugged: is_rugged,
                risks,
                top_holders: holders,
                markets,
                security_data_last_fetched_at: DateTime::from_timestamp(fetched_ts, 0)
                    .unwrap_or_else(|| Utc::now()),
                security_data_first_fetched_at: DateTime::from_timestamp(first_fetched_ts, 0)
                    .unwrap_or_else(|| {
                        DateTime::from_timestamp(fetched_ts, 0).unwrap_or_else(|| Utc::now())
                    }),
            })
        });

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(DatabaseError::Query {
                operation: "Query failed".to_owned(),
                message: e.to_string(),
            })),
        }
    }

    /// Fetch token mints that have no security assessment.
    ///
    /// Eligible: no `security_rugcheck` row, not blacklisted, and the security error state
    /// allows a retry — never tried, a temporary error past its exponential backoff
    /// (`120 * 2^min(error_count - 1, 10)` seconds), or a permanent error older than 7 days. Ordered by retry class, then `first_discovered_at ASC`; ties come back in
    /// no defined order:
    /// 1. never tried, discovered in the last 24h
    /// 2. never tried, older
    /// 3. temporary errors
    /// 4. permanent errors
    ///
    /// A single `tokens` join ordered on the class expression read every token and sorted
    /// it in a temp b-tree (3.4s at 555k tokens), so each class is read in order and the
    /// read stops at the remaining limit:
    /// - Classes 1 and 2 walk `idx_tokens_discovery_mint` from the 24h cutoff and from
    ///   the start respectively. "Never tried" means no `update_tracking` row or one with
    ///   a NULL `security_error_type`.
    /// - Classes 3 and 4 enter through `idx_tracking_security_error`, which holds only
    ///   the rows that carry a security error.
    pub fn get_tokens_without_security_data(&self, limit: usize) -> TokenResult<Vec<String>> {
        const UNTRIED_RECENT: &str = "SELECT t.mint FROM tokens t
             WHERE t.chain_id = ?1 AND t.first_discovered_at > ?2 - 86400
             AND NOT EXISTS (SELECT 1 FROM security_rugcheck sr
                             WHERE sr.chain_id = t.chain_id AND sr.mint = t.mint)
             AND NOT EXISTS (SELECT 1 FROM blacklist b
                             WHERE b.chain_id = t.chain_id AND b.mint = t.mint)
             AND NOT EXISTS (SELECT 1 FROM update_tracking ut
                             WHERE ut.chain_id = t.chain_id AND ut.mint = t.mint
                             AND ut.security_error_type IS NOT NULL)
             ORDER BY t.first_discovered_at ASC
             LIMIT ?3";
        const UNTRIED_OLDER: &str = "SELECT t.mint FROM tokens t
             WHERE t.chain_id = ?1 AND t.first_discovered_at <= ?2 - 86400
             AND NOT EXISTS (SELECT 1 FROM security_rugcheck sr
                             WHERE sr.chain_id = t.chain_id AND sr.mint = t.mint)
             AND NOT EXISTS (SELECT 1 FROM blacklist b
                             WHERE b.chain_id = t.chain_id AND b.mint = t.mint)
             AND NOT EXISTS (SELECT 1 FROM update_tracking ut
                             WHERE ut.chain_id = t.chain_id AND ut.mint = t.mint
                             AND ut.security_error_type IS NOT NULL)
             ORDER BY t.first_discovered_at ASC
             LIMIT ?3";
        const RETRY_DUE: &str = "SELECT t.mint FROM update_tracking ut
             JOIN tokens t ON t.chain_id = ut.chain_id AND t.mint = ut.mint
             WHERE ut.chain_id = ?1 AND ut.security_error_type IS NOT NULL
             AND (
                 (ut.security_error_type = 'temporary'
                  AND ut.last_security_error_at < ?2 - (120 * (1 << MIN(ut.security_error_count - 1, 10))))
                 OR (ut.security_error_type = 'permanent'
                     AND ut.last_security_error_at < ?2 - 604800)
             )
             AND NOT EXISTS (SELECT 1 FROM security_rugcheck sr
                             WHERE sr.chain_id = ut.chain_id AND sr.mint = ut.mint)
             AND NOT EXISTS (SELECT 1 FROM blacklist b
                             WHERE b.chain_id = ut.chain_id AND b.mint = ut.mint)
             ORDER BY CASE WHEN ut.security_error_type = 'temporary' THEN 3 ELSE 4 END,
                      t.first_discovered_at ASC
             LIMIT ?3";

        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self.conn()?;
        let now = Utc::now().timestamp();
        let query_error = |operation: &str, e: rusqlite::Error| {
            Error::Database(DatabaseError::Query {
                operation: operation.to_owned(),
                message: e.to_string(),
            })
        };

        let mut mints: Vec<String> = Vec::with_capacity(limit);
        for (sql, class) in [
            (UNTRIED_RECENT, "recent untried tokens"),
            (UNTRIED_OLDER, "older untried tokens"),
            (RETRY_DUE, "security retries"),
        ] {
            let remaining = limit - mints.len();
            if remaining == 0 {
                break;
            }
            let mut stmt = conn
                .prepare(sql)
                .map_err(|e| query_error(&format!("Failed to prepare {class}"), e))?;
            let rows = stmt
                .query_map(params![self.chain_id(), now, remaining], |row| row.get(0))
                .map_err(|e| query_error(&format!("{class} query failed"), e))?;
            for row in rows {
                mints.push(row.map_err(|e| query_error(&format!("Failed to collect {class}"), e))?);
            }
        }

        Ok(mints)
    }

    /// Record a failed market update attempt with error type tracking
    ///
    /// Error types:
    /// - "temporary": Transient errors (rate limit, network issues) - retry with backoff
    /// - "permanent": Token not listed on any exchange - stop retrying after threshold
    ///

    pub fn record_security_error(
        &self,
        mint: &str,
        message: &str,
        error_type: &str,
    ) -> TokenResult<()> {
        let conn = self.conn()?;

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE update_tracking SET 
                security_error_count = security_error_count + 1,
                last_security_error = ?1,
                last_security_error_at = ?2,
                security_error_type = ?3
             WHERE chain_id = ?4 AND mint = ?5",
            params![message, now, error_type, self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to record security error".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }

    /// Clear security error tracking (called after successful fetch)
    pub fn clear_security_error(&self, mint: &str) -> TokenResult<()> {
        let conn = self.conn()?;

        conn.execute(
            "UPDATE update_tracking SET 
                security_error_count = 0,
                last_security_error = NULL,
                last_security_error_at = NULL,
                security_error_type = NULL
             WHERE chain_id = ?1 AND mint = ?2",
            params![self.chain_id(), mint],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to clear security error".to_owned(),
                message: e.to_string(),
            })
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;
    use std::collections::HashSet;

    /// The single-join selection `get_tokens_without_security_data` replaced, kept as the
    /// oracle.
    const SINGLE_JOIN_ORACLE: &str = "SELECT t.mint FROM tokens t
             LEFT JOIN security_rugcheck sr ON t.chain_id = sr.chain_id AND t.mint = sr.mint
             LEFT JOIN blacklist b ON t.chain_id = b.chain_id AND t.mint = b.mint
             LEFT JOIN update_tracking ut ON t.chain_id = ut.chain_id AND t.mint = ut.mint
             WHERE t.chain_id = ?1 AND sr.mint IS NULL
             AND b.mint IS NULL
             AND (
                 ut.security_error_type IS NULL
                 OR (ut.security_error_type = 'temporary'
                     AND ut.last_security_error_at < ?2 - (120 * (1 << MIN(ut.security_error_count - 1, 10))))
                 OR (ut.security_error_type = 'permanent'
                     AND ut.last_security_error_at < ?2 - 604800)
             )
             ORDER BY
                 CASE
                     WHEN ut.security_error_type IS NULL AND t.first_discovered_at > ?2 - 86400 THEN 1
                     WHEN ut.security_error_type IS NULL THEN 2
                     WHEN ut.security_error_type = 'temporary' THEN 3
                     ELSE 4
                 END,
                 t.first_discovered_at ASC
             LIMIT ?3";

    const HOUR: i64 = 3_600;
    const DAY: i64 = 86_400;

    fn insert_token(conn: &rusqlite::Connection, chain: &str, mint: &str, discovered_at: i64) {
        conn.execute(
            "INSERT INTO tokens (chain_id, mint, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES (?1, ?2, ?3, 1, 1)",
            params![chain, mint, discovered_at],
        )
        .expect("insert token");
    }

    fn insert_tracking(
        conn: &rusqlite::Connection,
        chain: &str,
        mint: &str,
        error: Option<(&str, i64, i64)>,
    ) {
        let (error_type, error_count, error_at) = match error {
            Some((kind, count, at)) => (Some(kind), count, Some(at)),
            None => (None, 0, None),
        };
        conn.execute(
            "INSERT INTO update_tracking (chain_id, mint, security_error_type, security_error_count, last_security_error_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![chain, mint, error_type, error_count, error_at],
        )
        .expect("insert tracking");
    }

    fn insert_rugcheck(conn: &rusqlite::Connection, mint: &str) {
        conn.execute(
            "INSERT INTO security_rugcheck (chain_id, mint, security_data_last_fetched_at, security_data_first_fetched_at)
             VALUES ('solana', ?1, 1, 1)",
            params![mint],
        )
        .expect("insert rugcheck");
    }

    fn insert_blacklist(conn: &rusqlite::Connection, mint: &str) {
        conn.execute(
            "INSERT INTO blacklist (chain_id, mint, reason, source, added_at) VALUES ('solana', ?1, 'test', 'test', 1)",
            params![mint],
        )
        .expect("insert blacklist");
    }

    /// (retry class, first_discovered_at) as the oracle orders it.
    fn sort_key(conn: &rusqlite::Connection, mint: &str, now: i64) -> (i64, i64) {
        conn.query_row(
            "SELECT CASE
                        WHEN ut.security_error_type IS NULL AND t.first_discovered_at > ?2 - 86400 THEN 1
                        WHEN ut.security_error_type IS NULL THEN 2
                        WHEN ut.security_error_type = 'temporary' THEN 3
                        ELSE 4
                    END,
                    t.first_discovered_at
             FROM tokens t
             LEFT JOIN update_tracking ut ON t.chain_id = ut.chain_id AND t.mint = ut.mint
             WHERE t.chain_id = 'solana' AND t.mint = ?1",
            params![mint, now],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read sort key")
    }

    fn oracle(conn: &rusqlite::Connection, now: i64, limit: usize) -> Vec<String> {
        let mut stmt = conn.prepare(SINGLE_JOIN_ORACLE).expect("prepare oracle");
        let rows = stmt
            .query_map(params!["solana", now, limit], |row| row.get(0))
            .expect("run oracle");
        rows.collect::<Result<Vec<String>, _>>()
            .expect("collect oracle")
    }

    /// For every limit, the result must be the oracle's result up to the order of rows that
    /// share a sort key: the same sort-key sequence, only eligible mints, no duplicates, and
    /// exactly the oracle's set once the limit covers every eligible token.
    fn assert_matches_oracle(db: &TokenDatabase, conn: &rusqlite::Connection, now: i64) {
        let eligible: HashSet<String> = oracle(conn, now, usize::MAX >> 1).into_iter().collect();
        for limit in 0..=eligible.len() + 2 {
            let expected = oracle(conn, now, limit);
            let actual = db
                .get_tokens_without_security_data(limit)
                .expect("get_tokens_without_security_data");

            let expected_keys: Vec<(i64, i64)> =
                expected.iter().map(|m| sort_key(conn, m, now)).collect();
            let actual_keys: Vec<(i64, i64)> =
                actual.iter().map(|m| sort_key(conn, m, now)).collect();
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
    fn tokens_without_security_data_match_single_join_oracle() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("tokens.db");
        let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana)
            .expect("open tokens database");
        let conn = db.conn().expect("pooled connection");

        // Timestamps sit hours away from every cutoff, so the second between the oracle's
        // `now` and the method's own cannot move a row across one.
        let now = Utc::now().timestamp();

        // Class 1: never tried, discovered in the last 24h; untracked or tracked without
        // an error, including a tie.
        for (mint, discovered, tracked) in [
            ("recent-untracked", now - 2 * HOUR, false),
            ("recent-tracked", now - 5 * HOUR, true),
            ("recent-tie-a", now - 3 * HOUR, true),
            ("recent-tie-b", now - 3 * HOUR, false),
        ] {
            insert_token(&conn, "solana", mint, discovered);
            if tracked {
                insert_tracking(&conn, "solana", mint, None);
            }
        }
        // Class 2: never tried, older than 24h.
        for (mint, discovered, tracked) in [
            ("older-untracked", now - 3 * DAY, false),
            ("older-tracked", now - 10 * DAY, true),
            ("older-tie-a", now - 5 * DAY, true),
            ("older-tie-b", now - 5 * DAY, true),
        ] {
            insert_token(&conn, "solana", mint, discovered);
            if tracked {
                insert_tracking(&conn, "solana", mint, None);
            }
        }
        // Classes 3 and 4: errors past and within their retry delay. Backoff is
        // 120 * 2^min(count - 1, 10) seconds; 7 days for permanent errors.
        for (mint, discovered, error) in [
            ("temp-due", now - 2 * HOUR, ("temporary", 1, now - HOUR)),
            ("temp-due-old", now - 20 * DAY, ("temporary", 3, now - HOUR)),
            (
                "temp-zero-count",
                now - 4 * DAY,
                ("temporary", 0, now - HOUR),
            ),
            (
                "temp-capped-due",
                now - 6 * DAY,
                ("temporary", 40, now - 2 * DAY),
            ),
            (
                "temp-capped-waiting",
                now - 6 * DAY,
                ("temporary", 40, now - HOUR),
            ),
            ("temp-waiting", now - 3 * DAY, ("temporary", 8, now - HOUR)),
            ("perm-due", now - 30 * DAY, ("permanent", 1, now - 8 * DAY)),
            (
                "perm-due-recent",
                now - 2 * HOUR,
                ("permanent", 2, now - 9 * DAY),
            ),
            (
                "perm-waiting",
                now - 30 * DAY,
                ("permanent", 1, now - 6 * DAY),
            ),
            ("unknown-type", now - 2 * DAY, ("other", 1, now - 30 * DAY)),
        ] {
            insert_token(&conn, "solana", mint, discovered);
            insert_tracking(&conn, "solana", mint, Some(error));
        }
        // Assessed or blacklisted: never selected, whatever their tracking state.
        for (mint, discovered, error) in [
            ("assessed-recent", now - HOUR, None),
            ("assessed-older", now - 40 * DAY, None),
            (
                "assessed-temp",
                now - 2 * DAY,
                Some(("temporary", 1, now - DAY)),
            ),
            ("blacklisted-recent", now - HOUR, None),
            ("blacklisted-older", now - 40 * DAY, None),
            (
                "blacklisted-perm",
                now - 40 * DAY,
                Some(("permanent", 1, now - 9 * DAY)),
            ),
        ] {
            insert_token(&conn, "solana", mint, discovered);
            insert_tracking(&conn, "solana", mint, error);
        }
        for mint in ["assessed-recent", "assessed-older", "assessed-temp"] {
            insert_rugcheck(&conn, mint);
        }
        for mint in [
            "blacklisted-recent",
            "blacklisted-older",
            "blacklisted-perm",
        ] {
            insert_blacklist(&conn, mint);
        }
        // Another chain's rows must never surface.
        insert_token(&conn, "ethereum", "foreign-recent", now - HOUR);
        insert_token(&conn, "ethereum", "foreign-temp", now - 2 * DAY);
        insert_tracking(
            &conn,
            "ethereum",
            "foreign-temp",
            Some(("temporary", 1, now - DAY)),
        );

        assert_matches_oracle(&db, &conn, now);

        // Every never-tried token assessed: only the retry classes remain.
        for mint in [
            "recent-untracked",
            "recent-tracked",
            "recent-tie-a",
            "recent-tie-b",
            "older-untracked",
            "older-tracked",
            "older-tie-a",
            "older-tie-b",
        ] {
            insert_rugcheck(&conn, mint);
        }
        assert_matches_oracle(&db, &conn, now);
    }
}
