// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token rejection storage — records why tokens were filtered out with timestamps.

use std::collections::BTreeMap;

use crate::errors::DatabaseError;
use rusqlite::{params, Connection};

use crate::tokens::types::TokenResult;
use crate::tokens::Error;

use super::retention::{MAX_ROWS_PER_TRANSACTION, TRANSACTION_PAUSE};
use super::TokenDatabase;
use crate::database::WriteTransaction;

/// The verdict of one filtering build on one token: `None` when the token passed.
#[derive(Debug, Clone)]
pub struct FilterOutcome {
    pub mint: String,
    pub rejection: Option<FilterRejectionRecord>,
}

/// A rejection as the token row stores it.
#[derive(Debug, Clone)]
pub struct FilterRejectionRecord {
    pub reason: String,
    pub source: String,
    /// Stamp of the first build in the unbroken run of builds that rejected the token
    /// for this reason.
    pub since: i64,
}

/// What [`TokenDatabase::record_filter_build`] wrote.
#[derive(Debug, Clone, Copy)]
pub struct FilterBuildWrite {
    /// Outcomes written.
    pub outcomes: usize,
    /// Rejections cleared from tokens the build did not evaluate.
    pub cleared: u64,
}

/// One row of the recent rejections list: mint, reason, source, Unix time, symbol, name
/// and logo.
pub type RecentRejectionRow = (
    String,
    String,
    String,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl TokenDatabase {
    /// Record one complete filtering build as the chain's rejection state.
    ///
    /// Every outcome is written in one transaction (all-or-nothing): the evaluated
    /// token's rejection columns take the build's verdict and `filter_evaluated_at` takes
    /// `evaluated_at`, and `rejection_stats` counts each rejection that starts in this
    /// build once, in the hour of `evaluated_at`. Only after that transaction committed
    /// are the rejections of tokens the build did not evaluate (any row whose stamp is
    /// not `evaluated_at`) cleared, in bounded transactions, so a failed outcomes
    /// transaction leaves the previous build's state whole and an interrupted clear is
    /// finished by the next build.
    pub fn record_filter_build(
        &self,
        evaluated_at: i64,
        outcomes: &[FilterOutcome],
    ) -> TokenResult<FilterBuildWrite> {
        let mut conn = self.conn()?;
        self.write_filter_outcomes(&mut conn, evaluated_at, outcomes)?;
        let cleared = self.clear_unevaluated_rejections(&mut conn, evaluated_at)?;
        Ok(FilterBuildWrite {
            outcomes: outcomes.len(),
            cleared,
        })
    }

    fn write_filter_outcomes(
        &self,
        conn: &mut Connection,
        evaluated_at: i64,
        outcomes: &[FilterOutcome],
    ) -> TokenResult<()> {
        let failed = |operation: &'static str| {
            move |error| DatabaseError::classify_sqlite_failure(operation, error)
        };
        let mut started: BTreeMap<(&str, &str), i64> = BTreeMap::new();
        let transaction = conn.write_tx().map_err(failed("begin filter outcomes"))?;
        {
            let mut statement = transaction
                .prepare_cached(
                    "UPDATE update_tracking
                     SET last_rejection_reason = ?3, last_rejection_source = ?4,
                         last_rejection_at = ?5, filter_evaluated_at = ?6
                     WHERE chain_id = ?1 AND mint = ?2",
                )
                .map_err(failed("prepare filter outcomes"))?;
            for outcome in outcomes {
                let (reason, source, since) = match &outcome.rejection {
                    Some(rejection) => (
                        Some(rejection.reason.as_str()),
                        Some(rejection.source.as_str()),
                        Some(rejection.since),
                    ),
                    None => (None, None, None),
                };
                statement
                    .execute(params![
                        self.chain_id(),
                        outcome.mint,
                        reason,
                        source,
                        since,
                        evaluated_at
                    ])
                    .map_err(failed("write filter outcome"))?;
                if let Some(rejection) = &outcome.rejection {
                    if rejection.since == evaluated_at {
                        *started
                            .entry((rejection.reason.as_str(), rejection.source.as_str()))
                            .or_default() += 1;
                    }
                }
            }
        }
        {
            let mut statement = transaction
                .prepare_cached(
                    "INSERT INTO rejection_stats (chain_id, bucket_hour, reason, source, rejection_count, unique_tokens, first_seen, last_seen)
                     VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)
                     ON CONFLICT(chain_id, bucket_hour, reason, source) DO UPDATE SET
                         rejection_count = rejection_count + excluded.rejection_count,
                         last_seen = excluded.last_seen",
                )
                .map_err(failed("prepare rejection stats"))?;
            for ((reason, source), count) in started {
                statement
                    .execute(params![
                        self.chain_id(),
                        evaluated_at / 3600 * 3600,
                        reason,
                        source,
                        count,
                        evaluated_at
                    ])
                    .map_err(failed("write rejection stats"))?;
            }
        }
        transaction
            .commit()
            .map_err(failed("commit filter outcomes"))?;
        Ok(())
    }

    /// Clear the rejection columns of every row whose stamp is not `evaluated_at`, in
    /// transactions of at most `MAX_ROWS_PER_TRANSACTION` rows. The partial rejection
    /// index holds only the rows that currently carry a reason, so the walk touches
    /// nothing else.
    fn clear_unevaluated_rejections(
        &self,
        conn: &mut Connection,
        evaluated_at: i64,
    ) -> TokenResult<u64> {
        let failed = |operation: &'static str| {
            move |error| DatabaseError::classify_sqlite_failure(operation, error)
        };
        let mut cleared = 0u64;
        loop {
            let transaction = conn.write_tx().map_err(failed("begin rejection clear"))?;
            let changed = transaction
                .execute(
                    "UPDATE update_tracking
                     SET last_rejection_reason = NULL, last_rejection_source = NULL,
                         last_rejection_at = NULL
                     WHERE rowid IN (
                         SELECT rowid FROM update_tracking INDEXED BY idx_tracking_rejection_reason_at
                         WHERE chain_id = ?1 AND last_rejection_reason IS NOT NULL
                           AND filter_evaluated_at IS NOT ?2
                         LIMIT ?3)",
                    params![self.chain_id(), evaluated_at, MAX_ROWS_PER_TRANSACTION],
                )
                .map_err(failed("clear stale rejections"))? as u64;
            transaction
                .commit()
                .map_err(failed("commit rejection clear"))?;
            cleared += changed;
            if changed < MAX_ROWS_PER_TRANSACTION {
                return Ok(cleared);
            }
            std::thread::sleep(TRANSACTION_PAUSE);
        }
    }

    /// Get rejection statistics grouped by reason

    pub fn get_rejection_stats(&self) -> TokenResult<Vec<(String, String, i64)>> {
        self.get_rejection_stats_with_time_filter(None, None)
    }

    /// Get rejection statistics grouped by reason with optional time filter
    /// Queries update_tracking table for UNIQUE tokens rejected in time range
    /// This is the correct semantic - counting unique tokens, not cumulative events

    pub fn get_rejection_stats_with_time_filter(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self.conn()?;

        // Build query with optional time filters on last_rejection_at
        let mut query = "SELECT 
                    last_rejection_reason, 
                    last_rejection_source, 
                    COUNT(*) as count 
                 FROM update_tracking 
                 WHERE chain_id = :chain_id AND last_rejection_reason IS NOT NULL"
            .to_string();

        if start_time.is_some() {
            query.push_str(" AND last_rejection_at >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND last_rejection_at <= :end_time");
        }

        query
            .push_str(" GROUP BY last_rejection_reason, last_rejection_source ORDER BY count DESC");

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Bind parameters
        let chain_id = self.chain_id();
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = vec![(":chain_id", &chain_id)];
        if let Some(ref start) = start_time {
            params.push((":start_time", start));
        }
        if let Some(ref end) = end_time {
            params.push((":end_time", end));
        }

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Get list of rejected tokens with pagination and optional filtering

    pub fn get_recent_rejections(&self, limit: usize) -> TokenResult<Vec<RecentRejectionRow>> {
        let conn = self.conn()?;

        let query = "SELECT ut.mint, ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at,
                            t.symbol, t.name,
                            COALESCE(
                              (SELECT image_url FROM market_dexscreener WHERE chain_id = ut.chain_id AND mint = ut.mint AND image_url IS NOT NULL AND image_url != '' LIMIT 1),
                              (SELECT image_url FROM market_geckoterminal WHERE chain_id = ut.chain_id AND mint = ut.mint AND image_url IS NOT NULL AND image_url != '' LIMIT 1)
                            ) AS image_url
                     FROM update_tracking ut
                     LEFT JOIN tokens t ON ut.chain_id = t.chain_id AND ut.mint = t.mint
                     WHERE ut.chain_id = ?1 AND ut.last_rejection_reason IS NOT NULL
                     ORDER BY ut.last_rejection_at DESC LIMIT ?2";

        let mut stmt = conn.prepare(query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare".to_owned(),
                message: e.to_string(),
            })
        })?;

        let chain_id = self.chain_id();
        let limit_i64 = limit as i64;
        let rows = stmt
            .query_map(params![chain_id, limit_i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| Error::RowDecode {
                detail: e.to_string(),
            })?);
        }

        Ok(results)
    }

    /// Query rejected tokens with optional filtering and pagination
    pub fn get_rejected_tokens(
        &self,
        reason_filter: Option<String>,
        source_filter: Option<String>,
        search_filter: Option<String>,
        limit: usize,
        offset: usize,
    ) -> TokenResult<Vec<(String, String, String, i64)>> {
        let conn = self.conn()?;

        let mut query = if search_filter.is_some() {
            "SELECT ut.mint, ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at 
             FROM update_tracking ut 
             LEFT JOIN tokens t ON ut.chain_id = t.chain_id AND ut.mint = t.mint
             WHERE ut.chain_id = :chain_id AND ut.last_rejection_reason IS NOT NULL".to_owned()
        } else {
            "SELECT mint, last_rejection_reason, last_rejection_source, last_rejection_at 
             FROM update_tracking 
             WHERE chain_id = :chain_id AND last_rejection_reason IS NOT NULL"
                .to_string()
        };

        if reason_filter.is_some() {
            query.push_str(if search_filter.is_some() {
                " AND ut.last_rejection_reason = :reason"
            } else {
                " AND last_rejection_reason = :reason"
            });
        }

        if source_filter.is_some() {
            query.push_str(if search_filter.is_some() {
                " AND ut.last_rejection_source = :source"
            } else {
                " AND last_rejection_source = :source"
            });
        }

        if search_filter.is_some() {
            query.push_str(
                " AND (ut.mint LIKE :search OR t.symbol LIKE :search OR t.name LIKE :search)",
            );
        }

        query.push_str(if search_filter.is_some() {
            " ORDER BY ut.last_rejection_at DESC LIMIT :limit OFFSET :offset"
        } else {
            " ORDER BY last_rejection_at DESC LIMIT :limit OFFSET :offset"
        });

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Failed to prepare".to_owned(),
                message: e.to_string(),
            })
        })?;

        // Build params dynamically - only include params that are in the query
        let chain_id = self.chain_id();
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = vec![(":chain_id", &chain_id)];
        if let Some(ref reason) = reason_filter {
            params.push((":reason", reason));
        }
        if let Some(ref source) = source_filter {
            params.push((":source", source));
        }

        let search_pattern;
        if let Some(ref search) = search_filter {
            search_pattern = format!("%{search}%");
            params.push((":search", &search_pattern));
        }

        let limit_i64 = limit as i64;
        let offset_i64 = offset as i64;
        params.push((":limit", &limit_i64));
        params.push((":offset", &offset_i64));

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                ))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Insert rejection event into history table (for time-range analytics)

    /// Get rejection statistics for a specific time range

    /// Cleanup old rejection history entries (keep last N hours)
    /// This is critical for database size management - rejection history grows ~5GB/day

    pub fn cleanup_rejection_history(&self, hours_to_keep: i64) -> TokenResult<usize> {
        let conn = self.conn()?;

        let cutoff = chrono::Utc::now().timestamp() - (hours_to_keep * 60 * 60);

        let deleted = conn
            .execute(
                "DELETE FROM rejection_history WHERE chain_id = ?1 AND rejected_at < ?2",
                params![self.chain_id(), cutoff],
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Failed to cleanup rejection history".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(deleted)
    }

    /// Upsert rejection stat into aggregated hourly bucket table
    /// This replaces per-event logging with O(1) aggregation

    /// Get rejection statistics from aggregated table for a time range

    pub fn get_rejection_stats_aggregated(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self.conn()?;

        let mut query =
            "SELECT reason, source, SUM(rejection_count) as total FROM rejection_stats WHERE chain_id = :chain_id"
                .to_string();

        if start_time.is_some() {
            query.push_str(" AND bucket_hour >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND bucket_hour <= :end_time");
        }

        query.push_str(" GROUP BY reason, source ORDER BY total DESC");

        let mut stmt = conn.prepare(&query).map_err(|e| {
            Error::Database(DatabaseError::Query {
                operation: "Prepare failed".to_owned(),
                message: e.to_string(),
            })
        })?;

        let chain_id = self.chain_id();
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = vec![(":chain_id", &chain_id)];
        if let Some(ref start) = start_time {
            params.push((":start_time", start));
        }
        if let Some(ref end) = end_time {
            params.push((":end_time", end));
        }

        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Query failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }
        Ok(results)
    }

    /// Cleanup old aggregated rejection stats (keep last N hours)

    pub fn cleanup_rejection_stats(&self, hours_to_keep: i64) -> TokenResult<usize> {
        let conn = self.conn()?;

        let cutoff = chrono::Utc::now().timestamp() - (hours_to_keep * 3600);

        let deleted = conn
            .execute(
                "DELETE FROM rejection_stats WHERE chain_id = ?1 AND bucket_hour < ?2",
                params![self.chain_id(), cutoff],
            )
            .map_err(|e| {
                Error::Database(DatabaseError::Query {
                    operation: "Delete rejection stats failed".to_owned(),
                    message: e.to_string(),
                })
            })?;

        Ok(deleted)
    }
}
