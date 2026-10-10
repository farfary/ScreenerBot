// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Eviction of token rows that have not been used for a long time: a keyset walk over the discovery index, deleted with their market, security, tracking, media, pool and rejection rows in bounded transactions.

use std::collections::HashSet;
use std::time::Duration;

use rusqlite::{params, Connection};

use crate::chains::ChainId;
use crate::database::WriteTransaction;
use crate::errors::DatabaseError;

/// Rows one eviction transaction deletes at most before it commits, so the store's
/// write lock is never held for long.
pub const MAX_ROWS_PER_TRANSACTION: u64 = 2_000;
/// Tokens examined per transaction.
const CANDIDATES_PER_TRANSACTION: u32 = 500;
/// Pause between two transactions, so writers waiting on the store get their turn.
const TRANSACTION_PAUSE: Duration = Duration::from_millis(25);

/// The tables holding a token's rows, children first: market, security and tracking
/// rows restrict the token's deletion.
const TOKEN_CHILD_TABLES: [&str; 7] = [
    "market_dexscreener",
    "market_geckoterminal",
    "security_rugcheck",
    "update_tracking",
    "token_media",
    "token_pools",
    "rejection_history",
];

/// Candidates past the keyset cursor `(first_discovered_at, rowid)`, newest discovery
/// first, in the order of the discovery index, each with the latest time any of its
/// data was fetched or updated and whether a decision made in the app keeps it. Every
/// join reaches its table through that table's key, and rows are only ever deleted, so
/// the cursor never skips a token.
pub(super) const CANDIDATES_SQL: &str = "
    SELECT t.rowid, t.mint, t.first_discovered_at,
           MAX(t.metadata_last_fetched_at,
               COALESCE(d.market_data_last_fetched_at, 0),
               COALESCE(g.market_data_last_fetched_at, 0),
               COALESCE(s.security_data_last_fetched_at, 0),
               COALESCE(u.market_data_last_updated_at, 0),
               COALESCE(u.security_data_last_updated_at, 0)) AS last_used,
           EXISTS (SELECT 1 FROM token_favorites f
                   WHERE f.chain_id = t.chain_id AND f.mint = t.mint)
           OR EXISTS (SELECT 1 FROM blacklist b
                      WHERE b.chain_id = t.chain_id AND b.mint = t.mint
                        AND b.source = 'manual') AS kept
    FROM tokens t INDEXED BY idx_tokens_discovered
    LEFT JOIN market_dexscreener d ON d.chain_id = t.chain_id AND d.mint = t.mint
    LEFT JOIN market_geckoterminal g ON g.chain_id = t.chain_id AND g.mint = t.mint
    LEFT JOIN security_rugcheck s ON s.chain_id = t.chain_id AND s.mint = t.mint
    LEFT JOIN update_tracking u ON u.chain_id = t.chain_id AND u.mint = t.mint
    WHERE t.chain_id = ?1 AND t.first_discovered_at <= ?2
      AND (t.first_discovered_at < ?2 OR t.rowid > ?3)
    ORDER BY t.first_discovered_at DESC, t.rowid
    LIMIT ?4";

/// What one token eviction did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TokenEviction {
    pub examined: u64,
    pub tokens: u64,
    pub rows: u64,
    /// Whether the walk stopped early because shutdown was requested.
    pub interrupted: bool,
}

/// How many tokens `chain` holds, and how many of them were discovered before each
/// of `cutoffs` (unix seconds). Counted on the discovery index alone.
pub fn discovery_counts(
    conn: &Connection,
    chain: ChainId,
    cutoffs: &[i64],
) -> Result<(u64, Vec<u64>), DatabaseError> {
    let failed = |error| DatabaseError::classify_sqlite_failure("count tokens by discovery", error);
    let as_count = |count: i64| u64::try_from(count).unwrap_or(0);
    let total = conn
        .query_row(
            "SELECT COUNT(*) FROM tokens WHERE chain_id = ?1",
            params![chain.as_str()],
            |row| row.get(0),
        )
        .map(as_count)
        .map_err(failed)?;
    let mut statement = conn
        .prepare_cached(
            "SELECT COUNT(*) FROM tokens WHERE chain_id = ?1 AND first_discovered_at < ?2",
        )
        .map_err(failed)?;
    let before = cutoffs
        .iter()
        .map(|cutoff| {
            statement
                .query_row(params![chain.as_str(), cutoff], |row| row.get(0))
                .map(as_count)
                .map_err(failed)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((total, before))
}

/// Delete every token of `chain` last used before `cutoff` (unix seconds), except
/// favorites, manually blacklisted tokens and `protected` mints, together with all
/// of its rows. A token's last use is the latest fetch or update of its metadata,
/// market, security or tracking data, so a token discovered before the cutoff that
/// is still refreshed stays. Each transaction examines a bounded set of candidates
/// and deletes at most about `MAX_ROWS_PER_TRANSACTION` rows; `stop` is checked
/// between transactions.
pub fn evict_tokens_unused_before(
    conn: &mut Connection,
    chain: ChainId,
    cutoff: i64,
    protected: &HashSet<String>,
    stop: &dyn Fn() -> bool,
) -> Result<TokenEviction, DatabaseError> {
    let failed = |operation: &'static str| {
        move |error| DatabaseError::classify_sqlite_failure(operation, error)
    };
    let mut eviction = TokenEviction::default();
    let mut cursor: (i64, i64) = (cutoff, i64::MAX);
    loop {
        if stop() {
            eviction.interrupted = true;
            return Ok(eviction);
        }
        let transaction = conn.write_tx().map_err(failed("begin token eviction"))?;
        let candidates: Vec<(i64, String, i64, i64, bool)> = {
            let mut statement = transaction
                .prepare_cached(CANDIDATES_SQL)
                .map_err(failed("prepare token eviction candidates"))?;
            let rows = statement
                .query_map(
                    params![
                        chain.as_str(),
                        cursor.0,
                        cursor.1,
                        CANDIDATES_PER_TRANSACTION
                    ],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .map_err(failed("list token eviction candidates"))?;
            rows.collect::<rusqlite::Result<_>>()
                .map_err(failed("read token eviction candidates"))?
        };
        let exhausted = candidates.len() < CANDIDATES_PER_TRANSACTION as usize;
        let mut rows_in_transaction = 0u64;
        let mut evicted = Vec::new();
        for (rowid, mint, discovered_at, last_used, kept) in candidates {
            eviction.examined += 1;
            cursor = (discovered_at, rowid);
            if last_used >= cutoff || kept || protected.contains(&mint) {
                continue;
            }
            rows_in_transaction += delete_token(&transaction, chain, &mint)?;
            evicted.push(mint);
            if rows_in_transaction >= MAX_ROWS_PER_TRANSACTION {
                break;
            }
        }
        transaction
            .commit()
            .map_err(failed("commit token eviction"))?;
        eviction.tokens += evicted.len() as u64;
        eviction.rows += rows_in_transaction;
        for mint in &evicted {
            crate::tokens::store::invalidate_token_snapshot(chain, mint);
        }
        if exhausted && rows_in_transaction < MAX_ROWS_PER_TRANSACTION {
            return Ok(eviction);
        }
        std::thread::sleep(TRANSACTION_PAUSE);
    }
}

/// Delete one token and every row it owns; returns the rows deleted.
fn delete_token(
    transaction: &rusqlite::Transaction<'_>,
    chain: ChainId,
    mint: &str,
) -> Result<u64, DatabaseError> {
    let mut rows = 0u64;
    for table in TOKEN_CHILD_TABLES.into_iter().chain(["tokens"]) {
        let mut statement = transaction
            .prepare_cached(&format!(
                "DELETE FROM {table} WHERE chain_id = ?1 AND mint = ?2"
            ))
            .map_err(|error| {
                DatabaseError::classify_sqlite_failure("prepare token delete", error)
            })?;
        rows += statement
            .execute(params![chain.as_str(), mint])
            .map_err(|error| DatabaseError::classify_sqlite_failure("delete token rows", error))?
            as u64;
    }
    Ok(rows)
}
