// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Gap tracking — insert, query, and record fill attempts and resolutions.
//!
//! A resolved row (`filled = 1`) means the source answered for every bucket of
//! its range: missing buckets inside it are source-confirmed no-trade buckets.
//! Detection never re-opens a range contained in a resolved row.

use crate::database::WriteTransaction;
use crate::ohlcvs::types::{MintGapAggregate, OhlcvError, OhlcvResult, Timeframe};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};

use super::{GapRecord, OhlcvDatabase};

/// RFC 3339 text stored in `ohlcv_gaps.last_attempt`.
fn attempt_text(unix_secs: i64) -> String {
    DateTime::<Utc>::from_timestamp(unix_secs, 0)
        .unwrap_or_default()
        .to_rfc3339()
}

fn parse_attempt(text: Option<String>) -> Option<i64> {
    text.and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.timestamp())
}

impl OhlcvDatabase {
    // ==================== Gap Management ====================

    /// Register a detected gap unless a resolved row already contains it.
    pub fn insert_gap(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> OhlcvResult<()> {
        let conn = self.conn()?;

        conn.execute(
            "INSERT OR IGNORE INTO ohlcv_gaps
                 (chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6
             WHERE NOT EXISTS (
                 SELECT 1 FROM ohlcv_gaps
                 WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4
                   AND filled = 1 AND start_timestamp <= ?5 AND end_timestamp >= ?6
             )",
            params![
                self.chain_id(),
                mint,
                pool_address,
                timeframe.as_str(),
                start_timestamp,
                end_timestamp
            ],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to insert gap: {e}")))?;

        Ok(())
    }

    /// Unfilled gaps of one pool that end at or after `since` and have fewer
    /// than `max_attempts` attempts, ordered by timeframe and start.
    pub fn get_open_gaps(
        &self,
        mint: &str,
        pool_address: &str,
        since: i64,
        max_attempts: u32,
    ) -> OhlcvResult<Vec<GapRecord>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT timeframe, start_timestamp, end_timestamp, attempts, last_attempt
                 FROM ohlcv_gaps
                 WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND filled = 0
                   AND attempts < ?4 AND end_timestamp >= ?5
                 ORDER BY timeframe, start_timestamp, end_timestamp",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {e}")))?;

        let rows = stmt
            .query_map(
                params![
                    self.chain_id(),
                    mint,
                    pool_address,
                    i64::from(max_attempts),
                    since
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {e}")))?;

        Ok(rows
            .into_iter()
            .filter_map(|(timeframe, start, end, attempts, last_attempt)| {
                Some(GapRecord {
                    timeframe: Timeframe::from_str(&timeframe)?,
                    start_timestamp: start,
                    end_timestamp: end,
                    attempts: u32::try_from(attempts.max(0)).unwrap_or(u32::MAX),
                    last_attempt: parse_attempt(last_attempt),
                })
            })
            .collect())
    }

    /// Record an inconclusive fill attempt on every unfilled row inside
    /// `[start, end]`: one more attempt, its time and the reason.
    #[allow(clippy::too_many_arguments)]
    pub fn record_gap_attempt(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
        attempted_at: i64,
        error: &str,
    ) -> OhlcvResult<usize> {
        let conn = self.conn()?;

        conn.execute(
            "UPDATE ohlcv_gaps
             SET attempts = attempts + 1, last_attempt = ?7, error_message = ?8
             WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4
               AND filled = 0 AND start_timestamp >= ?5 AND end_timestamp <= ?6",
            params![
                self.chain_id(),
                mint,
                pool_address,
                timeframe.as_str(),
                start_timestamp,
                end_timestamp,
                attempt_text(attempted_at),
                error
            ],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to record gap attempt: {e}")))
    }

    /// Resolve `[start, end]`: the source answered for every bucket in it. Marks
    /// the unfilled rows inside it filled and stores a resolved row spanning the
    /// whole range, so detection does not re-open any hole inside it.
    pub fn resolve_gap_span(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
        attempted_at: i64,
    ) -> OhlcvResult<()> {
        let mut conn = self.conn()?;
        let tx = conn
            .write_tx()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to begin: {e}")))?;
        let key = GapKey {
            chain_id: self.chain_id(),
            mint,
            pool_address,
            timeframe,
        };
        key.close_rows(&tx, start_timestamp, end_timestamp, attempted_at)?;
        key.upsert_row(
            &tx,
            start_timestamp,
            end_timestamp,
            true,
            1,
            attempted_at,
            None,
        )?;
        tx.commit()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to resolve gap span: {e}")))
    }

    /// Record an answer that covered only `[covered_start, end]` of `[start, end]`.
    /// The covered part is resolved; `[start, remainder_end]` stays open as one row
    /// carrying `attempts` unchanged, because the answer made progress.
    #[allow(clippy::too_many_arguments)]
    pub fn split_gap_span(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
        covered_start: i64,
        remainder_end: i64,
        attempts: u32,
        attempted_at: i64,
    ) -> OhlcvResult<()> {
        let mut conn = self.conn()?;
        let tx = conn
            .write_tx()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to begin: {e}")))?;
        let key = GapKey {
            chain_id: self.chain_id(),
            mint,
            pool_address,
            timeframe,
        };
        key.close_rows(&tx, start_timestamp, end_timestamp, attempted_at)?;
        key.upsert_row(
            &tx,
            covered_start,
            end_timestamp,
            true,
            1,
            attempted_at,
            None,
        )?;
        key.upsert_row(
            &tx,
            start_timestamp,
            remainder_end,
            false,
            attempts,
            attempted_at,
            Some("source answer covered part of the span"),
        )?;
        tx.commit()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to split gap span: {e}")))
    }

    pub fn get_unfilled_gaps(
        &self,
        mint: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Vec<(String, i64, i64)>> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, start_timestamp, end_timestamp FROM ohlcv_gaps
                 WHERE chain_id = ?1 AND mint = ?2 AND timeframe = ?3 AND filled = 0
                 ORDER BY start_timestamp DESC
                 LIMIT 100",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {e}")))?;

        let gaps = stmt
            .query_map(params![self.chain_id(), mint, timeframe.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {e}")))?;

        Ok(gaps)
    }

    pub fn get_gap_aggregate(&self) -> OhlcvResult<(usize, usize)> {
        let conn = self.conn()?;

        let (gap_count, token_count): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*) as gap_count, COUNT(DISTINCT mint) as token_count
                 FROM ohlcv_gaps WHERE chain_id = ?1 AND filled = 0",
                params![self.chain_id()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to read gap aggregate: {e}")))?;

        Ok((token_count.max(0) as usize, gap_count.max(0) as usize))
    }

    pub fn get_top_open_gaps(&self, limit: usize) -> OhlcvResult<Vec<MintGapAggregate>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, COUNT(*) as gap_count,
                        MAX(end_timestamp - start_timestamp) as largest_gap,
                        MAX(end_timestamp) as latest_gap
                 FROM ohlcv_gaps
                 WHERE chain_id = ?1 AND filled = 0
                 GROUP BY mint
                 ORDER BY largest_gap DESC, latest_gap DESC
                 LIMIT ?2",
            )
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to prepare gap summary: {e}"))
            })?;

        let rows = stmt
            .query_map(params![self.chain_id(), limit as i64], |row| {
                let mint: String = row.get(0)?;
                let open_gaps: i64 = row.get(1)?;
                let largest_gap: Option<i64> = row.get(2)?;
                let latest_gap: Option<i64> = row.get(3)?;

                Ok(MintGapAggregate {
                    mint,
                    open_gaps: open_gaps.max(0) as usize,
                    largest_gap_seconds: largest_gap,
                    latest_gap_end: latest_gap,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Gap summary query failed: {e}")))?;

        let aggregates = rows.collect::<SqliteResult<Vec<_>>>().map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to collect gap summary: {e}"))
        })?;

        Ok(aggregates)
    }
}

/// The (chain, mint, pool, timeframe) scope of gap rows written together.
struct GapKey<'a> {
    chain_id: &'static str,
    mint: &'a str,
    pool_address: &'a str,
    timeframe: Timeframe,
}

impl GapKey<'_> {
    /// Mark every unfilled row inside `[start, end]` resolved on this attempt.
    fn close_rows(
        &self,
        conn: &Connection,
        start_timestamp: i64,
        end_timestamp: i64,
        attempted_at: i64,
    ) -> OhlcvResult<()> {
        conn.execute(
            "UPDATE ohlcv_gaps
             SET filled = 1, attempts = attempts + 1, last_attempt = ?7, error_message = NULL
             WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4
               AND filled = 0 AND start_timestamp >= ?5 AND end_timestamp <= ?6",
            params![
                self.chain_id,
                self.mint,
                self.pool_address,
                self.timeframe.as_str(),
                start_timestamp,
                end_timestamp,
                attempt_text(attempted_at)
            ],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to close gap rows: {e}")))?;
        Ok(())
    }

    /// Insert or overwrite the row keyed by `[start, end]`.
    #[allow(clippy::too_many_arguments)]
    fn upsert_row(
        &self,
        conn: &Connection,
        start_timestamp: i64,
        end_timestamp: i64,
        filled: bool,
        attempts: u32,
        attempted_at: i64,
        error: Option<&str>,
    ) -> OhlcvResult<()> {
        conn.execute(
            "INSERT INTO ohlcv_gaps
                 (chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp,
                  attempts, last_attempt, filled, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
             DO UPDATE SET filled = excluded.filled, attempts = excluded.attempts,
                           last_attempt = excluded.last_attempt,
                           error_message = excluded.error_message",
            params![
                self.chain_id,
                self.mint,
                self.pool_address,
                self.timeframe.as_str(),
                start_timestamp,
                end_timestamp,
                i64::from(attempts),
                attempt_text(attempted_at),
                i64::from(filled),
                error
            ],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to write gap row: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::migrations::test_path;
    use super::*;
    use crate::chains::ChainId;

    const MIN: i64 = 60;
    const T0: i64 = 1_700_000_000 - 1_700_000_000 % 86_400;
    const NOW: i64 = T0 + 86_400;

    fn open_db(label: &str) -> (OhlcvDatabase, std::path::PathBuf) {
        let path = test_path(label);
        let _ = std::fs::remove_file(&path);
        (OhlcvDatabase::new(&path, ChainId::Solana).unwrap(), path)
    }

    fn close_db(db: OhlcvDatabase, path: std::path::PathBuf) {
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    fn insert(db: &OhlcvDatabase, start: i64, end: i64) {
        db.insert_gap("mint", "pool", Timeframe::Minute1, start, end)
            .unwrap();
    }

    fn open(db: &OhlcvDatabase) -> Vec<(i64, i64, u32)> {
        db.get_open_gaps("mint", "pool", 0, u32::MAX)
            .unwrap()
            .into_iter()
            .map(|g| (g.start_timestamp, g.end_timestamp, g.attempts))
            .collect()
    }

    fn row(db: &OhlcvDatabase, start: i64, end: i64) -> (i64, i64, Option<String>, Option<String>) {
        let conn = db.conn().unwrap();
        conn.query_row(
            "SELECT filled, attempts, last_attempt, error_message FROM ohlcv_gaps
             WHERE mint = 'mint' AND pool_address = 'pool' AND timeframe = '1m'
               AND start_timestamp = ?1 AND end_timestamp = ?2",
            params![start, end],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
    }

    #[test]
    fn attempts_record_count_time_and_reason_on_every_row_in_the_span() {
        let (db, path) = open_db("gap-attempts");
        insert(&db, T0, T0 + MIN);
        insert(&db, T0 + 5 * MIN, T0 + 5 * MIN);
        insert(&db, T0 + 90 * MIN, T0 + 90 * MIN);

        let touched = db
            .record_gap_attempt(
                "mint",
                "pool",
                Timeframe::Minute1,
                T0,
                T0 + 5 * MIN,
                NOW,
                "no answer",
            )
            .unwrap();
        assert_eq!(touched, 2);
        let (filled, attempts, last, error) = row(&db, T0, T0 + MIN);
        assert_eq!((filled, attempts), (0, 1));
        assert_eq!(parse_attempt(last), Some(NOW));
        assert_eq!(error.as_deref(), Some("no answer"));
        // Outside the span: untouched.
        assert_eq!(row(&db, T0 + 90 * MIN, T0 + 90 * MIN).1, 0);

        // The attempt bound filters exhausted rows out of the open set.
        assert_eq!(
            db.get_open_gaps("mint", "pool", 0, 1).unwrap().len(),
            1,
            "only the untouched row is below one attempt"
        );
        // The window bound filters rows that ended before `since`.
        assert_eq!(
            db.get_open_gaps("mint", "pool", T0 + 6 * MIN, u32::MAX)
                .unwrap()
                .len(),
            1
        );
        close_db(db, path);
    }

    #[test]
    fn a_resolved_span_is_never_reopened_by_detection() {
        let (db, path) = open_db("gap-resolve");
        insert(&db, T0 + MIN, T0 + MIN);
        insert(&db, T0 + 4 * MIN, T0 + 6 * MIN);

        db.resolve_gap_span("mint", "pool", Timeframe::Minute1, T0, T0 + 10 * MIN, NOW)
            .unwrap();
        assert!(open(&db).is_empty());
        assert_eq!(row(&db, T0 + MIN, T0 + MIN).0, 1);
        assert_eq!(row(&db, T0, T0 + 10 * MIN).0, 1);

        // Same keys and any hole inside the resolved range stay resolved.
        insert(&db, T0 + MIN, T0 + MIN);
        insert(&db, T0 + 4 * MIN, T0 + 6 * MIN);
        insert(&db, T0 + 8 * MIN, T0 + 9 * MIN);
        assert!(open(&db).is_empty());

        // A hole reaching outside it is new.
        insert(&db, T0 + 9 * MIN, T0 + 11 * MIN);
        assert_eq!(open(&db), vec![(T0 + 9 * MIN, T0 + 11 * MIN, 0)]);
        close_db(db, path);
    }

    #[test]
    fn a_partial_answer_resolves_the_covered_part_and_keeps_the_remainder_open() {
        let (db, path) = open_db("gap-split");
        let (start, end) = (T0, T0 + 3_000 * MIN);
        insert(&db, start, end);
        db.record_gap_attempt(
            "mint",
            "pool",
            Timeframe::Minute1,
            start,
            end,
            NOW - 600,
            "x",
        )
        .unwrap();

        let covered = T0 + 2_000 * MIN;
        db.split_gap_span(
            "mint",
            "pool",
            Timeframe::Minute1,
            start,
            end,
            covered,
            covered - MIN,
            1,
            NOW,
        )
        .unwrap();

        // The remainder carries the attempts the span had before this answer.
        assert_eq!(open(&db), vec![(start, covered - MIN, 1)]);
        // Holes inside the covered part are not re-detected.
        insert(&db, covered + 5 * MIN, covered + 7 * MIN);
        assert_eq!(open(&db).len(), 1);
        // Re-detecting the remainder does not duplicate it.
        insert(&db, start, covered - MIN);
        assert_eq!(open(&db).len(), 1);
        close_db(db, path);
    }
}
