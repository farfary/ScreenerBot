//! Gap tracking — insert, query, and mark gaps as filled.

use crate::ohlcvs::types::{MintGapAggregate, OhlcvError, OhlcvResult, Timeframe};
use rusqlite::{params, Result as SqliteResult};

use super::OhlcvDatabase;

impl OhlcvDatabase {
    // ==================== Gap Management ====================

    pub fn insert_gap(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn
            .execute(
                "INSERT OR IGNORE INTO ohlcv_gaps (chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![self.chain_id(), mint, pool_address, timeframe.as_str(), start_timestamp, end_timestamp]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to insert gap: {e}")))?;

        Ok(())
    }

    pub fn get_unfilled_gaps(
        &self,
        mint: &str,
        timeframe: Timeframe,
    ) -> OhlcvResult<Vec<(String, i64, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

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

    pub fn mark_gap_filled(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn
            .execute(
                "UPDATE ohlcv_gaps SET filled = 1 
             WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3 AND timeframe = ?4 AND start_timestamp = ?5 AND end_timestamp = ?6",
                params![self.chain_id(), mint, pool_address, timeframe.as_str(), start_timestamp, end_timestamp]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark gap filled: {e}")))?;

        Ok(())
    }

    pub fn get_gap_aggregate(&self) -> OhlcvResult<(usize, usize)> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

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

        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

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
