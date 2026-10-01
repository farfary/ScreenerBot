//! Candle storage and retrieval — time bounds, batch inserts, and backfill tracking.

use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Timeframe};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use super::{OhlcvDatabase, StoredBucket, TimeframeSummary};

impl OhlcvDatabase {
    /// Source label of candles the monitor derives locally from stored 1m rows.
    /// Every other label is a native series from a provider or the Data Server.
    pub const AGGREGATE_SOURCE: &'static str = "monitor_aggregate";

    /// Source label of native candles fetched per timeframe (backfill and the
    /// monitor's native refresh).
    pub const NATIVE_SOURCE: &'static str = "backfill";

    /// Whether a candle carries a real trade and is kept at ingest.
    pub fn is_storable(candle: &Candle) -> bool {
        candle.volume.is_finite() && candle.volume > 0.0
    }

    /// Canonical UTC bucket start of `timestamp` for `timeframe`.
    pub fn bucket_start(timestamp: i64, timeframe: Timeframe) -> i64 {
        let bucket = timeframe.to_seconds();
        if bucket > 0 {
            (timestamp / bucket) * bucket
        } else {
            timestamp
        }
    }

    // ==================== Time Bounds ====================

    pub fn get_time_bounds(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Option<(i64, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT MIN(timestamp), MAX(timestamp) FROM ohlcv_candles WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {e}")))?;

        let bounds = stmt
            .query_row(
                params![self.chain_id(), mint, pool_address, timeframe.as_str()],
                |row| {
                    let min_ts: Option<i64> = row.get(0)?;
                    let max_ts: Option<i64> = row.get(1)?;
                    Ok((min_ts, max_ts))
                },
            )
            .optional()
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(bounds.and_then(|(min_ts, max_ts)| match (min_ts, max_ts) {
            (Some(min_val), Some(max_val)) => Some((min_val, max_val)),
            _ => None,
        }))
    }

    // ==================== Unified Candles Storage ====================

    /// Upsert a batch of candles for one timeframe and return how many rows were
    /// inserted or changed. See [`Self::insert_candles_batch_at`] for the rules.
    pub fn insert_candles_batch(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        candles: &[Candle],
        source: &str,
    ) -> OhlcvResult<usize> {
        self.insert_candles_batch_at(
            mint,
            pool_address,
            timeframe,
            candles,
            source,
            Utc::now().timestamp(),
        )
    }

    /// Upsert a batch of candles for one timeframe, judging bucket closure
    /// against `now` (unix secs).
    ///
    /// A stored bucket is last-write-wins, with one exception: a
    /// [`Self::AGGREGATE_SOURCE`] write never replaces a native row (any other
    /// source) whose bucket has closed (`timestamp + bucket <= now`). A closed
    /// native candle is the provider's final answer, while a local 1m aggregate
    /// is only as complete as the 1m rows stored when it was computed. An
    /// identical row is left untouched, so the count and `fetched_at` reflect
    /// real data changes only.
    fn insert_candles_batch_at(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        candles: &[Candle],
        source: &str,
        now: i64,
    ) -> OhlcvResult<usize> {
        if candles.is_empty() {
            return Ok(0);
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| OhlcvError::DatabaseError(format!("Transaction failed: {e}")))?;

        let timeframe_str = timeframe.as_str();
        // Snap every timestamp to the canonical UTC-anchored bucket for this
        // timeframe (floor to the interval), matching OhlcvAggregator's
        // `(ts / bucket) * bucket` convention. Different OHLCV providers anchor
        // some timeframes on different grids — notably 12h: GeckoTerminal returns
        // 12h candles phased at +10h (ts % 43200 == 36000) while SolanaTracker,
        // the derived-from-1m aggregator, and every other timeframe use the
        // midnight grid (offset 0). Storing both raw phases in the same
        // (mint,pool,timeframe) series interleaves candles ~2h apart and renders
        // a corrupted chart with impossible price jumps. Normalizing here forces
        // a single grid for all sources, so the chart matches TradingView /
        // DexScreener (which also anchor at 00:00/12:00 UTC).
        let bucket = timeframe.to_seconds();
        let mut changed = 0;

        for candle in candles {
            // Never record an empty no-trade candle. Candles are SOL-denominated,
            // so a real trade always carries volume > 0; volume == 0 means no
            // swaps happened in that period and the provider merely carried the
            // price forward. Storing those paints fake price action on the chart,
            // so we drop them (the series shows an honest gap instead, like
            // TradingView/DexScreener on an illiquid pair).
            if !Self::is_storable(candle) {
                continue;
            }
            let aligned_ts = Self::bucket_start(candle.timestamp, timeframe);
            let result = tx.execute(
                "INSERT INTO ohlcv_candles
                 (chain_id, mint, pool_address, timeframe, timestamp, open, high, low, close, volume, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(chain_id, mint, pool_address, timeframe, timestamp) DO UPDATE SET
                    open = excluded.open,
                    high = excluded.high,
                    low = excluded.low,
                    close = excluded.close,
                    volume = excluded.volume,
                    source = excluded.source,
                    fetched_at = CURRENT_TIMESTAMP
                 WHERE NOT (
                        excluded.source = ?12
                        AND ohlcv_candles.source != ?12
                        AND ohlcv_candles.timestamp + ?13 <= ?14
                    )
                    AND (ohlcv_candles.open != excluded.open
                        OR ohlcv_candles.high != excluded.high
                        OR ohlcv_candles.low != excluded.low
                        OR ohlcv_candles.close != excluded.close
                        OR ohlcv_candles.volume != excluded.volume
                        OR ohlcv_candles.source != excluded.source)",
                params![
                    self.chain_id(), mint,
                    pool_address,
                    timeframe_str,
                    aligned_ts,
                    candle.open,
                    candle.high,
                    candle.low,
                    candle.close,
                    candle.volume,
                    source,
                    Self::AGGREGATE_SOURCE,
                    bucket,
                    now,
                ],
            );

            if let Ok(rows) = result {
                changed += rows;
            }
        }

        tx.commit()
            .map_err(|e| OhlcvError::DatabaseError(format!("Commit failed: {e}")))?;

        Ok(changed)
    }

    /// Newest stored bucket for one pool and timeframe that came from a native
    /// source, i.e. excluding [`Self::AGGREGATE_SOURCE`] rows. Coverage and
    /// catch-up sizing are measured against this, because local aggregates fill
    /// the live edge and would otherwise hide a hole in the native series.
    pub fn get_latest_native_timestamp(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Option<i64>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn.query_row(
            "SELECT timestamp FROM ohlcv_candles
             WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4
               AND source != ?5
             ORDER BY timestamp DESC LIMIT 1",
            params![
                self.chain_id(),
                mint,
                pool_address,
                timeframe.as_str(),
                Self::AGGREGATE_SOURCE
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))
    }

    /// The stored row of the bucket starting at `timestamp` for one pool and
    /// timeframe, or `None` when the bucket has no row.
    pub fn get_stored_bucket(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        timestamp: i64,
    ) -> OhlcvResult<Option<StoredBucket>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn.query_row(
            "SELECT source != ?6, CAST(strftime('%s', fetched_at) AS INTEGER) FROM ohlcv_candles
             WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4
               AND timestamp = ?5",
            params![
                self.chain_id(),
                mint,
                pool_address,
                timeframe.as_str(),
                timestamp,
                Self::AGGREGATE_SOURCE
            ],
            |row| {
                Ok(StoredBucket {
                    native: row.get(0)?,
                    fetched_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))
    }

    /// Get candles for specific timeframe
    pub fn get_candles(
        &self,
        mint: &str,
        pool_address: Option<&str>,
        timeframe: Timeframe,
        from_ts: Option<i64>,
        to_ts: Option<i64>,
        limit: Option<usize>,
    ) -> OhlcvResult<Vec<Candle>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let timeframe_str = timeframe.as_str();

        let mut query = String::from(
            "SELECT timestamp, open, high, low, close, volume 
             FROM ohlcv_candles 
             WHERE chain_id = ? AND mint = ? AND timeframe = ?",
        );

        let mut param_index = 4;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(self.chain_id().to_string()),
            Box::new(mint.to_string()),
            Box::new(timeframe_str.to_string()),
        ];

        if let Some(pool) = pool_address {
            query.push_str(&format!(" AND pool_address = ?{param_index}"));
            params_vec.push(Box::new(pool.to_string()));
            param_index += 1;
        }

        if let Some(from) = from_ts {
            query.push_str(&format!(" AND timestamp >= ?{param_index}"));
            params_vec.push(Box::new(from));
            param_index += 1;
        }

        if let Some(to) = to_ts {
            query.push_str(&format!(" AND timestamp <= ?{param_index}"));
            params_vec.push(Box::new(to));
            param_index += 1;
        }

        // A chart wants the NEWEST `limit` candles, returned oldest-first (ASC) for
        // rendering. A plain `ORDER BY timestamp ASC LIMIT N` returns the OLDEST N
        // instead — so an active token with more history than `limit` would show
        // ancient candles and miss recent price action, and the result would differ
        // from the in-memory cache path (which takes the most-recent N), making the
        // same request flip between recent and ancient depending on cache state.
        // Select the newest N via a DESC-limited subquery, then re-sort ASC. With no
        // limit, return the full series in ASC order.
        let final_query = if let Some(lim) = limit {
            params_vec.push(Box::new(lim));
            format!(
                "SELECT timestamp, open, high, low, close, volume FROM (
                     {query} ORDER BY timestamp DESC LIMIT ?{param_index}
                 ) ORDER BY timestamp ASC"
            )
        } else {
            format!("{query} ORDER BY timestamp ASC")
        };

        let mut stmt = conn
            .prepare(&final_query)
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let candles = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(Candle {
                    timestamp: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        candles
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Collect failed: {e}")))
    }

    /// Per-timeframe candle count and latest timestamp for a token ON A SINGLE
    /// POOL, in one grouped query. Feeds the chart status indicator. It MUST be
    /// scoped to the same pool the chart reads (`get_ohlcv_data`) — counting
    /// across every pool_address would combine candles from different pools
    /// (e.g. an old pool the token has since migrated away from) and report a
    /// count the chart never shows.
    pub fn get_timeframe_summary(
        &self,
        mint: &str,
        pool_address: &str,
    ) -> OhlcvResult<Vec<TimeframeSummary>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT timeframe, COUNT(*) AS cnt, MIN(timestamp) AS earliest, MAX(timestamp) AS latest
                 FROM ohlcv_candles
                 WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3
                 GROUP BY timeframe",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![self.chain_id(), mint, pool_address], |row| {
                Ok(TimeframeSummary {
                    timeframe: row.get(0)?,
                    candles: row.get(1)?,
                    earliest: row.get(2)?,
                    latest: row.get(3)?,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Collect failed: {e}")))
    }

    /// Per-timeframe count of candles on one pool whose bucket overlaps `[from, to]` (unix
    /// secs). A bucket overlaps when it starts after `from - bucket_seconds`, so the candle
    /// that CONTAINS `from` counts. Answers "does this timeframe still hold that span?", which
    /// the newest-candle summary cannot once stored depth has rolled past it.
    pub fn count_candles_in_range(
        &self,
        mint: &str,
        pool_address: &str,
        from: i64,
        to: i64,
    ) -> OhlcvResult<Vec<(String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT COUNT(*) FROM ohlcv_candles
                 WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4
                   AND timestamp > ?5 AND timestamp <= ?6",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        Timeframe::all()
            .into_iter()
            .map(|tf| {
                let seconds = tf.to_seconds();
                stmt.query_row(
                    params![
                        self.chain_id(),
                        mint,
                        pool_address,
                        tf.as_str(),
                        from - seconds,
                        to
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .map(|count| (tf.as_str().to_string(), count))
                .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))
            })
            .collect()
    }

    /// Delete every candle stored under a pool that is no longer the token's
    /// resolved pool. Called when pool discovery drops a pool from a token so a
    /// stale pool's price series can never resurface or be combined with the
    /// current pool's candles. Returns the number of rows removed.
    pub fn delete_candles_for_pool(&self, mint: &str, pool_address: &str) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let removed = conn
            .execute(
                "DELETE FROM ohlcv_candles WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3",
                params![self.chain_id(), mint, pool_address],
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Delete failed: {e}")))?;

        Ok(removed)
    }

    /// Per-timeframe time of the most recent candle write (unix secs), i.e. the
    /// last successful fetch that produced new candles. `fetched_at` is stored as
    /// a TEXT timestamp, so convert to epoch in SQL.
    pub fn get_timeframe_last_new_data(
        &self,
        mint: &str,
        pool_address: &str,
    ) -> OhlcvResult<Vec<(String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT timeframe, CAST(strftime('%s', MAX(fetched_at)) AS INTEGER) AS last_new
                 FROM ohlcv_candles
                 WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3
                 GROUP BY timeframe",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![self.chain_id(), mint, pool_address], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        let mut out = Vec::new();
        for row in rows {
            let (tf, last): (String, Option<i64>) =
                row.map_err(|e| OhlcvError::DatabaseError(format!("Row failed: {e}")))?;
            if let Some(ts) = last {
                out.push((tf, ts));
            }
        }
        Ok(out)
    }

    /// When this token's OHLCV was last checked (any fetch attempt), from the
    /// monitor config's `last_fetch` (TEXT) as unix secs.
    pub fn get_last_checked_at(&self, mint: &str) -> OhlcvResult<Option<i64>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let res: Option<i64> = conn
            .query_row(
                "SELECT CAST(strftime('%s', last_fetch) AS INTEGER)
                 FROM ohlcv_monitor_config WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        Ok(res)
    }

    /// Check if backfill is complete for timeframe
    pub fn is_backfill_complete(&self, mint: &str, timeframe: Timeframe) -> OhlcvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let column = format!("backfill_{}_complete", timeframe.as_str().replace('-', ""));

        let query = format!(
            "SELECT {} FROM ohlcv_monitor_config WHERE chain_id = ?1 AND mint = ?2",
            column
        );

        let result: i32 = conn
            .query_row(&query, params![self.chain_id(), mint], |row| row.get(0))
            .optional()
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .unwrap_or_default();

        Ok(result == 1)
    }

    /// Mark backfill as complete for timeframe
    pub fn mark_backfill_complete(&self, mint: &str, timeframe: Timeframe) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let column = format!("backfill_{}_complete", timeframe.as_str().replace('-', ""));

        let query = format!(
            "UPDATE ohlcv_monitor_config SET {} = 1, updated_at = CURRENT_TIMESTAMP WHERE chain_id = ?1 AND mint = ?2",
            column
        );

        conn.execute(&query, params![self.chain_id(), mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {e}")))?;

        Ok(())
    }

    /// Mark backfill as incomplete for timeframe. The token is no longer fully
    /// backfilled, so `backfill_completed_at` is cleared with it.
    pub fn mark_backfill_incomplete(&self, mint: &str, timeframe: Timeframe) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let column = format!("backfill_{}_complete", timeframe.as_str().replace('-', ""));

        let query = format!(
            "UPDATE ohlcv_monitor_config SET {} = 0, backfill_completed_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE chain_id = ?1 AND mint = ?2",
            column
        );

        conn.execute(&query, params![self.chain_id(), mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {e}")))?;

        Ok(())
    }

    /// Mark all backfills as complete. `backfill_completed_at` records the
    /// transition to complete, so a token that is already complete is left
    /// untouched.
    pub fn mark_all_backfills_complete(&self, mint: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn.execute(
            "UPDATE ohlcv_monitor_config SET 
             backfill_1m_complete = 1,
             backfill_5m_complete = 1,
             backfill_15m_complete = 1,
             backfill_1h_complete = 1,
             backfill_4h_complete = 1,
             backfill_12h_complete = 1,
             backfill_1d_complete = 1,
             backfill_completed_at = CURRENT_TIMESTAMP,
             updated_at = CURRENT_TIMESTAMP
             WHERE chain_id = ?1 AND mint = ?2 AND backfill_completed_at IS NULL",
            params![self.chain_id(), mint],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::migrations::test_path;
    use super::*;
    use crate::chains::ChainId;
    use crate::ohlcvs::types::{Priority, TokenOhlcvConfig};

    const HOUR: i64 = 3_600;
    const NOW: i64 = 1_700_000_000 - 1_700_000_000 % HOUR + 1_800;
    const FORMING: i64 = NOW - NOW % HOUR;
    const CLOSED: i64 = FORMING - HOUR;

    fn open_db(label: &str) -> (OhlcvDatabase, std::path::PathBuf) {
        let path = test_path(label);
        let _ = std::fs::remove_file(&path);
        (OhlcvDatabase::new(&path, ChainId::Solana).unwrap(), path)
    }

    fn close_db(db: OhlcvDatabase, path: std::path::PathBuf) {
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    fn candle(ts: i64, close: f64) -> Candle {
        Candle::new(ts, 1.0, close.max(1.0), 0.5, close, 10.0)
    }

    fn upsert(db: &OhlcvDatabase, candles: &[Candle], source: &str) -> usize {
        db.insert_candles_batch_at("mint", "pool", Timeframe::Hour1, candles, source, NOW)
            .unwrap()
    }

    fn stored(db: &OhlcvDatabase, ts: i64) -> (f64, String) {
        let conn = db.conn.lock().unwrap();
        conn.query_row(
            "SELECT close, source FROM ohlcv_candles WHERE mint = 'mint' AND pool_address = 'pool' AND timeframe = '1h' AND timestamp = ?1",
            params![ts],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    #[test]
    fn stored_bucket_reports_native_rows_and_their_write_time() {
        let (db, path) = open_db("stored_bucket");
        upsert(&db, &[candle(CLOSED, 2.0)], OhlcvDatabase::NATIVE_SOURCE);
        upsert(
            &db,
            &[candle(FORMING, 2.0)],
            OhlcvDatabase::AGGREGATE_SOURCE,
        );

        let bucket = |ts| {
            db.get_stored_bucket("mint", "pool", Timeframe::Hour1, ts)
                .unwrap()
        };
        let closed = bucket(CLOSED).unwrap();
        assert!(closed.native);
        assert!(closed.fetched_at.is_some());
        assert!(!bucket(FORMING).unwrap().native);
        assert_eq!(bucket(CLOSED - HOUR), None);
        close_db(db, path);
    }

    #[test]
    fn native_over_native_updates_a_closed_bucket() {
        let (db, path) = open_db("upsert-native-native");
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 2.0)], OhlcvDatabase::NATIVE_SOURCE),
            1
        );
        assert_eq!(upsert(&db, &[candle(CLOSED, 3.0)], "monitor"), 1);
        assert_eq!(stored(&db, CLOSED), (3.0, "monitor".to_string()));
        close_db(db, path);
    }

    #[test]
    fn aggregate_over_aggregate_updates_a_closed_bucket() {
        let (db, path) = open_db("upsert-aggregate-aggregate");
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 2.0)], OhlcvDatabase::AGGREGATE_SOURCE),
            1
        );
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 3.0)], OhlcvDatabase::AGGREGATE_SOURCE),
            1
        );
        assert_eq!(
            stored(&db, CLOSED),
            (3.0, OhlcvDatabase::AGGREGATE_SOURCE.to_string())
        );
        close_db(db, path);
    }

    #[test]
    fn aggregate_never_overwrites_a_closed_native_bucket() {
        let (db, path) = open_db("upsert-aggregate-closed-native");
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 2.0)], OhlcvDatabase::NATIVE_SOURCE),
            1
        );
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 3.0)], OhlcvDatabase::AGGREGATE_SOURCE),
            0
        );
        assert_eq!(
            stored(&db, CLOSED),
            (2.0, OhlcvDatabase::NATIVE_SOURCE.to_string())
        );
        // A native rewrite of an aggregate row is always taken.
        assert_eq!(
            upsert(
                &db,
                &[candle(CLOSED - HOUR, 4.0)],
                OhlcvDatabase::AGGREGATE_SOURCE
            ),
            1
        );
        assert_eq!(
            upsert(
                &db,
                &[candle(CLOSED - HOUR, 5.0)],
                OhlcvDatabase::NATIVE_SOURCE
            ),
            1
        );
        assert_eq!(
            stored(&db, CLOSED - HOUR),
            (5.0, OhlcvDatabase::NATIVE_SOURCE.to_string())
        );
        close_db(db, path);
    }

    #[test]
    fn aggregate_updates_a_forming_native_bucket() {
        let (db, path) = open_db("upsert-aggregate-forming-native");
        // A mid-bucket provider timestamp is snapped onto the forming bucket.
        assert_eq!(
            upsert(
                &db,
                &[candle(FORMING + 60, 2.0)],
                OhlcvDatabase::NATIVE_SOURCE
            ),
            1
        );
        assert_eq!(
            upsert(
                &db,
                &[candle(FORMING, 3.0)],
                OhlcvDatabase::AGGREGATE_SOURCE
            ),
            1
        );
        assert_eq!(
            stored(&db, FORMING),
            (3.0, OhlcvDatabase::AGGREGATE_SOURCE.to_string())
        );
        // The native newest ignores the aggregate that now holds the forming bucket.
        assert_eq!(
            db.get_latest_native_timestamp("mint", "pool", Timeframe::Hour1)
                .unwrap(),
            None
        );
        close_db(db, path);
    }

    #[test]
    fn identical_rows_and_empty_candles_change_nothing() {
        let (db, path) = open_db("upsert-identical-empty");
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 2.0)], OhlcvDatabase::NATIVE_SOURCE),
            1
        );
        assert_eq!(
            upsert(&db, &[candle(CLOSED, 2.0)], OhlcvDatabase::NATIVE_SOURCE),
            0
        );
        let mut empty = candle(CLOSED, 9.0);
        empty.volume = 0.0;
        let mut negative = candle(FORMING, 9.0);
        negative.volume = -1.0;
        let mut non_finite = candle(FORMING, 9.0);
        non_finite.volume = f64::NAN;
        assert_eq!(
            upsert(
                &db,
                &[empty, negative, non_finite],
                OhlcvDatabase::NATIVE_SOURCE
            ),
            0
        );
        assert_eq!(
            stored(&db, CLOSED),
            (2.0, OhlcvDatabase::NATIVE_SOURCE.to_string())
        );
        assert_eq!(
            db.get_latest_native_timestamp("mint", "pool", Timeframe::Hour1)
                .unwrap(),
            Some(CLOSED)
        );
        close_db(db, path);
    }

    #[test]
    fn backfill_completed_at_is_written_only_on_the_transition_to_complete() {
        let (db, path) = open_db("backfill-completed-at");
        db.upsert_monitor_config(&TokenOhlcvConfig::new("mint".to_string(), Priority::High))
            .unwrap();
        let completed_at = |db: &OhlcvDatabase| -> Option<String> {
            let conn = db.conn.lock().unwrap();
            conn.query_row(
                "SELECT backfill_completed_at FROM ohlcv_monitor_config WHERE mint = 'mint'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        let set_completed_at = |db: &OhlcvDatabase, value: &str| {
            let conn = db.conn.lock().unwrap();
            conn.execute(
                "UPDATE ohlcv_monitor_config SET backfill_completed_at = ?1 WHERE mint = 'mint'",
                params![value],
            )
            .unwrap();
        };

        db.mark_all_backfills_complete("mint").unwrap();
        assert!(completed_at(&db).is_some());
        set_completed_at(&db, "2000-01-01 00:00:00");
        db.mark_all_backfills_complete("mint").unwrap();
        assert_eq!(completed_at(&db).as_deref(), Some("2000-01-01 00:00:00"));

        db.mark_backfill_incomplete("mint", Timeframe::Hour4)
            .unwrap();
        assert_eq!(completed_at(&db), None);
        assert!(!db.is_backfill_complete("mint", Timeframe::Hour4).unwrap());
        db.mark_all_backfills_complete("mint").unwrap();
        assert!(completed_at(&db).is_some());
        assert!(db.is_backfill_complete("mint", Timeframe::Hour4).unwrap());
        close_db(db, path);
    }
}
