// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Authority reputation storage — owns the `authority_reputation` rows: the blocked-address
//! load on startup, the replacement of the data service's creator list, and the cleanup of
//! automatically written rows.

use rusqlite::params;

use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::TokenResult;
use crate::tokens::Error;

/// `authority_reputation.source` of the rows written from the data service's creator list.
/// Distinct from `auto` (removed on every start) and from manually added rows.
pub const CREATOR_HISTORY_SOURCE: &str = "creator_history";

/// Removes this chain's `creator_history` rows last refreshed before the third parameter.
const DELETE_STALE_CREATOR_ROWS: &str = "DELETE FROM authority_reputation
     WHERE chain_id = ?1 AND source = ?2 AND last_updated_at < ?3";

/// One wallet the data service reports as a serial liquidity remover: `rugged` of its
/// `tokens` ended with the liquidity removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatorReputationRow {
    pub address: String,
    pub tokens: u32,
    pub rugged: u32,
}

impl TokenDatabase {
    /// Load all blocked authority addresses from the database.
    /// Called on startup by the tokens service.
    pub fn load_blocked_authorities(&self) -> TokenResult<Vec<String>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT address FROM authority_reputation WHERE chain_id = ?1 AND is_blocked = 1",
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Prepare error".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let rows = stmt
            .query_map(params![self.chain_id()], |row| row.get::<_, String>(0))
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query error".to_owned(),
                    message: e.to_string(),
                })
            })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Row decode error".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Delete every automatically written reputation row of this chain.
    ///
    /// Rows written by the retired discovery count any current rejection as
    /// evidence against an authority, so they are not reliable; only rows from
    /// another source are kept.
    ///
    /// Returns the number of removed rows.
    pub fn remove_auto_reputation(&self) -> TokenResult<usize> {
        let conn = self.conn()?;

        conn.execute(
            "DELETE FROM authority_reputation WHERE chain_id = ?1 AND source = 'auto'",
            rusqlite::params![self.chain_id()],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Remove automatic authority reputation error".to_owned(),
                message: e.to_string(),
            })
        })
    }

    /// Make this chain's `creator_history` rows equal `rows`, in one transaction.
    ///
    /// A listed wallet is inserted blocked or refreshed (its `first_seen_at` is kept), a
    /// `creator_history` row missing from `rows` is deleted so its block lifts, and a row of
    /// any other source is never written, overwritten or deleted. `confidence` is the share of
    /// the wallet's tokens that ended with the liquidity removed.
    ///
    /// Returns the number of rows written.
    pub fn replace_creator_reputation(
        &self,
        rows: &[CreatorReputationRow],
        now: i64,
    ) -> TokenResult<usize> {
        let failed = |operation: &'static str| {
            move |error| DatabaseError::classify_sqlite_failure(operation, error)
        };
        let mut conn = self.conn()?;
        let transaction = conn
            .write_tx()
            .map_err(failed("begin creator reputation replacement"))?;
        let mut written = 0;
        {
            let mut upsert = transaction
                .prepare_cached(
                    "INSERT INTO authority_reputation (chain_id, address, authority_type,
                         total_token_count, flagged_token_count, confidence, is_blocked, source,
                         first_seen_at, last_updated_at)
                     VALUES (?1, ?2, 'creator', ?3, ?4, ?5, 1, ?6, ?7, ?7)
                     ON CONFLICT(chain_id, address) DO UPDATE SET
                         total_token_count = excluded.total_token_count,
                         flagged_token_count = excluded.flagged_token_count,
                         confidence = excluded.confidence,
                         is_blocked = 1,
                         last_updated_at = excluded.last_updated_at
                     WHERE authority_reputation.source = excluded.source",
                )
                .map_err(failed("prepare creator reputation upsert"))?;
            for row in rows {
                let confidence = f64::from(row.rugged) / f64::from(row.tokens.max(1));
                written += upsert
                    .execute(params![
                        self.chain_id(),
                        row.address,
                        row.tokens,
                        row.rugged,
                        confidence,
                        CREATOR_HISTORY_SOURCE,
                        now
                    ])
                    .map_err(failed("write creator reputation"))?;
            }
        }
        transaction
            .execute(
                DELETE_STALE_CREATOR_ROWS,
                params![self.chain_id(), CREATOR_HISTORY_SOURCE, now],
            )
            .map_err(failed("delete lifted creator reputation"))?;
        transaction
            .commit()
            .map_err(failed("commit creator reputation replacement"))?;
        Ok(written)
    }

    /// Delete this chain's `creator_history` rows last refreshed before `older_than`, the
    /// expiry for a list that can no longer be refreshed.
    ///
    /// Returns the number of removed rows.
    pub fn expire_creator_reputation(&self, older_than: i64) -> TokenResult<usize> {
        let conn = self.conn()?;

        conn.execute(
            DELETE_STALE_CREATOR_ROWS,
            params![self.chain_id(), CREATOR_HISTORY_SOURCE, older_than],
        )
        .map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Expire creator reputation error".to_owned(),
                message: e.to_string(),
            })
        })
    }
}
