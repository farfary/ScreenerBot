//! Candle storage and retrieval — time bounds, batch inserts, and backfill tracking.

use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Timeframe};
use rusqlite::{params, OptionalExtension};

use super::OhlcvDatabase;

impl OhlcvDatabase {
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
                "SELECT MIN(timestamp), MAX(timestamp) FROM ohlcv_candles WHERE mint = ?1 AND pool_address = ?2 AND timeframe = ?3",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {e}")))?;

        let bounds = stmt
            .query_row(params![mint, pool_address, timeframe.as_str()], |row| {
                let min_ts: Option<i64> = row.get(0)?;
                let max_ts: Option<i64> = row.get(1)?;
                Ok((min_ts, max_ts))
            })
            .optional()
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(bounds.and_then(|(min_ts, max_ts)| match (min_ts, max_ts) {
            (Some(min_val), Some(max_val)) => Some((min_val, max_val)),
            _ => None,
        }))
    }

    // ==================== Unified Candles Storage ====================

    /// Insert batch of candles for specific timeframe
    pub fn insert_candles_batch(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        candles: &[Candle],
        source: &str,
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
        let mut inserted = 0;

        for candle in candles {
            let result = tx.execute(
                "INSERT INTO ohlcv_candles 
                 (mint, pool_address, timeframe, timestamp, open, high, low, close, volume, source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(mint, pool_address, timeframe, timestamp) DO NOTHING",
                params![
                    mint,
                    pool_address,
                    timeframe_str,
                    candle.timestamp,
                    candle.open,
                    candle.high,
                    candle.low,
                    candle.close,
                    candle.volume,
                    source,
                ],
            );

            if let Ok(rows) = result {
                inserted += rows;
            }
        }

        tx.commit()
            .map_err(|e| OhlcvError::DatabaseError(format!("Commit failed: {e}")))?;

        Ok(inserted)
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
             WHERE mint = ? AND timeframe = ?",
        );

        let mut param_index = 3;
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
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

        query.push_str(" ORDER BY timestamp ASC");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT ?{param_index}"));
            params_vec.push(Box::new(lim));
        }

        let mut stmt = conn
            .prepare(&query)
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

    /// Per-timeframe candle count and latest timestamp for a token, in a single
    /// grouped query. Used by the chart status indicator so the dialog can show
    /// exactly which timeframes have data without firing one probe request per
    /// timeframe. Returns (timeframe_str, candle_count, latest_timestamp).
    pub fn get_timeframe_summary(
        &self,
        mint: &str,
    ) -> OhlcvResult<Vec<(String, i64, Option<i64>)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT timeframe, COUNT(*) AS cnt, MAX(timestamp) AS latest
                 FROM ohlcv_candles
                 WHERE mint = ?1
                 GROUP BY timeframe",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![mint], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Collect failed: {e}")))
    }

    /// Per-timeframe time of the most recent candle write (unix secs), i.e. the
    /// last successful fetch that produced new candles. `fetched_at` is stored as
    /// a TEXT timestamp, so convert to epoch in SQL.
    pub fn get_timeframe_last_new_data(&self, mint: &str) -> OhlcvResult<Vec<(String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT timeframe, CAST(strftime('%s', MAX(fetched_at)) AS INTEGER) AS last_new
                 FROM ohlcv_candles
                 WHERE mint = ?1
                 GROUP BY timeframe",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![mint], |row| {
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
                 FROM ohlcv_monitor_config WHERE mint = ?1",
                params![mint],
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
            "SELECT {} FROM ohlcv_monitor_config WHERE mint = ?1",
            column
        );

        let result: i32 = conn
            .query_row(&query, params![mint], |row| row.get(0))
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
            "UPDATE ohlcv_monitor_config SET {} = 1, updated_at = CURRENT_TIMESTAMP WHERE mint = ?1",
            column
        );

        conn.execute(&query, params![mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {e}")))?;

        Ok(())
    }

    /// Mark backfill as incomplete for timeframe
    pub fn mark_backfill_incomplete(&self, mint: &str, timeframe: Timeframe) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let column = format!("backfill_{}_complete", timeframe.as_str().replace('-', ""));

        let query = format!(
            "UPDATE ohlcv_monitor_config SET {} = 0, updated_at = CURRENT_TIMESTAMP WHERE mint = ?1",
            column
        );

        conn.execute(&query, params![mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {e}")))?;

        Ok(())
    }

    /// Mark all backfills as complete
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
             WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {e}")))?;

        Ok(())
    }
}
