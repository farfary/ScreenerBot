//! Cleanup, metrics, and data management — housekeeping and statistics queries.

use crate::ohlcvs::types::{OhlcvError, OhlcvResult};
use chrono::{Duration, Utc};
use rusqlite::{params, params_from_iter, Result as SqliteResult};
use std::collections::HashSet;

use super::OhlcvDatabase;

impl OhlcvDatabase {
    // ==================== Cleanup ====================

    /// Historical OHLCV candle data is preserved forever.
    ///
    /// This function intentionally does NOT delete candle data. Historical data is valuable for:
    /// - Chart gap visualization (users can see where data was missing)
    /// - Historical analysis and backtesting
    /// - GeckoTerminal can backfill gaps on demand when needed
    ///
    /// The `retention_days` parameter is kept for backward compatibility but is ignored.
    /// For database size management, use `cleanup_filled_gaps()` instead which removes
    /// filled gap tracking records that are no longer needed.
    #[allow(unused_variables)]
    pub fn cleanup_old_data(&self, retention_days: i64) -> OhlcvResult<usize> {
        // Intentional no-op: OHLCV candle data is preserved forever
        // Gap visualization is preferred over automatic data deletion
        Ok(0)
    }

    /// Cleanup old filled gap records to manage database size.
    ///
    /// Gap records that are marked as filled (successfully backfilled) and older than
    /// the specified retention days are deleted. Unfilled gaps are always preserved
    /// for tracking and retry purposes.
    ///
    /// Note: This only affects gap tracking records, NOT actual candle data.
    pub fn cleanup_filled_gaps(&self, retention_days: i64) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let cutoff = (Utc::now() - Duration::days(retention_days)).to_rfc3339();

        let deleted = conn
            .execute(
                "DELETE FROM ohlcv_gaps WHERE filled = 1 AND created_at < ?1",
                params![cutoff],
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Gap cleanup failed: {e}")))?;

        Ok(deleted)
    }

    // ==================== Metrics ====================

    pub fn get_data_point_count(&self) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_candles", params![], |row| {
                row.get(0)
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(count as usize)
    }

    pub fn has_data_for_mint(&self, mint: &str) -> OhlcvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let exists: i64 = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM ohlcv_candles WHERE mint = ?1 LIMIT 1)",
                params![mint],
                |row| row.get(0),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(exists != 0)
    }

    pub fn get_mints_with_data(&self, mints: &[String]) -> OhlcvResult<HashSet<String>> {
        if mints.is_empty() {
            return Ok(HashSet::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        const CHUNK_SIZE: usize = 512;
        let mut result = HashSet::with_capacity(mints.len());

        for chunk in mints.chunks(CHUNK_SIZE) {
            let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT DISTINCT mint FROM ohlcv_candles WHERE mint IN ({})",
                placeholders
            );

            let mut stmt = conn
                .prepare(&query)
                .map_err(|e| OhlcvError::DatabaseError(format!("Query prep failed: {e}")))?;

            let params: Vec<&dyn rusqlite::ToSql> = chunk
                .iter()
                .map(|mint| mint as &dyn rusqlite::ToSql)
                .collect();

            let rows = stmt
                .query_map(params_from_iter(params.iter().copied()), |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

            for row in rows {
                let mint =
                    row.map_err(|e| OhlcvError::DatabaseError(format!("Row parse failed: {e}")))?;
                result.insert(mint);
            }
        }

        Ok(result)
    }

    pub fn get_pool_count(&self) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_pools", params![], |row| {
                row.get(0)
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(count as usize)
    }

    pub fn get_token_count(&self) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT mint) FROM ohlcv_monitor_config WHERE is_active = 1",
                params![],
                |row| row.get(0),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(count as usize)
    }

    pub fn get_gap_count(&self, filled: bool) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_gaps WHERE filled = ?1",
                params![filled as i32],
                |row| row.get(0),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(count as usize)
    }

    /// Get comprehensive OHLCV token listing with status information
    pub fn get_all_tokens_with_status(&self) -> OhlcvResult<Vec<OhlcvTokenStatus>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        // Query combines monitor config with candle counts and gap info
        let mut stmt = conn
            .prepare(
                r#"
            SELECT 
                m.mint,
                m.priority,
                m.fetch_interval_seconds,
                m.is_active,
                m.last_fetch,
                m.last_activity,
                m.consecutive_empty_fetches,
                m.consecutive_pool_failures,
                m.backfill_1m_complete,
                m.backfill_5m_complete,
                m.backfill_15m_complete,
                m.backfill_1h_complete,
                m.backfill_4h_complete,
                m.backfill_12h_complete,
                m.backfill_1d_complete,
                m.created_at,
                m.updated_at,
                COALESCE(c.candle_count, 0) as candle_count,
                COALESCE(c.earliest_ts, 0) as earliest_timestamp,
                COALESCE(c.latest_ts, 0) as latest_timestamp,
                COALESCE(g.open_gaps, 0) as open_gaps,
                COALESCE(p.pool_count, 0) as pool_count
            FROM ohlcv_monitor_config m
            LEFT JOIN (
                SELECT mint, COUNT(*) as candle_count, 
                       MIN(timestamp) as earliest_ts, 
                       MAX(timestamp) as latest_ts 
                FROM ohlcv_candles 
                GROUP BY mint
            ) c ON m.mint = c.mint
            LEFT JOIN (
                SELECT mint, COUNT(*) as open_gaps 
                FROM ohlcv_gaps 
                WHERE filled = 0 
                GROUP BY mint
            ) g ON m.mint = g.mint
            LEFT JOIN (
                SELECT mint, COUNT(*) as pool_count 
                FROM ohlcv_pools 
                GROUP BY mint
            ) p ON m.mint = p.mint
            ORDER BY m.is_active DESC, m.priority DESC, m.last_activity DESC
            "#,
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {e}")))?;

        let tokens = stmt
            .query_map(params![], |row| {
                Ok(OhlcvTokenStatus {
                    mint: row.get(0)?,
                    priority: row.get(1)?,
                    fetch_interval_seconds: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    last_fetch: row.get(4)?,
                    last_activity: row.get(5)?,
                    consecutive_empty_fetches: row.get(6)?,
                    consecutive_pool_failures: row.get(7)?,
                    backfill_1m: row.get::<_, i32>(8)? != 0,
                    backfill_5m: row.get::<_, i32>(9)? != 0,
                    backfill_15m: row.get::<_, i32>(10)? != 0,
                    backfill_1h: row.get::<_, i32>(11)? != 0,
                    backfill_4h: row.get::<_, i32>(12)? != 0,
                    backfill_12h: row.get::<_, i32>(13)? != 0,
                    backfill_1d: row.get::<_, i32>(14)? != 0,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                    candle_count: row.get(17)?,
                    earliest_timestamp: row.get(18)?,
                    latest_timestamp: row.get(19)?,
                    open_gaps: row.get(20)?,
                    pool_count: row.get(21)?,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {e}")))?;

        Ok(tokens)
    }

    /// Delete all OHLCV data for a specific token
    pub fn delete_token_data(&self, mint: &str) -> OhlcvResult<DeleteResult> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let candles_deleted: usize = conn
            .execute("DELETE FROM ohlcv_candles WHERE mint = ?1", params![mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Delete candles failed: {e}")))?;

        let gaps_deleted: usize = conn
            .execute("DELETE FROM ohlcv_gaps WHERE mint = ?1", params![mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Delete gaps failed: {e}")))?;

        let pools_deleted: usize = conn
            .execute("DELETE FROM ohlcv_pools WHERE mint = ?1", params![mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Delete pools failed: {e}")))?;

        let config_deleted: usize = conn
            .execute(
                "DELETE FROM ohlcv_monitor_config WHERE mint = ?1",
                params![mint],
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Delete config failed: {e}")))?;

        Ok(DeleteResult {
            candles_deleted,
            gaps_deleted,
            pools_deleted,
            config_deleted,
        })
    }

    /// Delete OHLCV data for tokens that have been inactive for a specified duration
    pub fn delete_inactive_tokens(&self, inactive_hours: i64) -> OhlcvResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let cutoff = Utc::now() - Duration::hours(inactive_hours);
        let cutoff_str = cutoff.to_rfc3339();

        // Find tokens to delete
        let mut stmt = conn
            .prepare(
                "SELECT mint FROM ohlcv_monitor_config 
                 WHERE is_active = 0 AND last_activity < ?1",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {e}")))?;

        let mints: Vec<String> = stmt
            .query_map(params![cutoff_str], |row| row.get(0))
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Collect failed: {e}")))?;

        drop(stmt);

        // Delete each token's data
        for mint in &mints {
            conn.execute("DELETE FROM ohlcv_candles WHERE mint = ?1", params![mint])
                .ok();
            conn.execute("DELETE FROM ohlcv_gaps WHERE mint = ?1", params![mint])
                .ok();
            conn.execute("DELETE FROM ohlcv_pools WHERE mint = ?1", params![mint])
                .ok();
            conn.execute(
                "DELETE FROM ohlcv_monitor_config WHERE mint = ?1",
                params![mint],
            )
            .ok();
        }

        Ok(mints)
    }

    /// Get database size info
    pub fn get_database_stats(&self) -> OhlcvResult<DatabaseStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let total_candles: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_candles", params![], |row| {
                row.get(0)
            })
            .unwrap_or_default();

        let total_gaps: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_gaps", params![], |row| {
                row.get(0)
            })
            .unwrap_or_default();

        let total_pools: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_pools", params![], |row| {
                row.get(0)
            })
            .unwrap_or_default();

        let total_configs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_monitor_config",
                params![],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let active_configs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_monitor_config WHERE is_active = 1",
                params![],
                |row| row.get(0),
            )
            .unwrap_or_default();

        let page_count: i64 = conn
            .query_row("PRAGMA page_count", params![], |row| row.get(0))
            .unwrap_or_default();

        let page_size: i64 = conn
            .query_row("PRAGMA page_size", params![], |row| row.get(0))
            .unwrap_or(4096);

        let database_size_bytes = page_count * page_size;

        Ok(DatabaseStats {
            total_candles: total_candles as usize,
            total_gaps: total_gaps as usize,
            total_pools: total_pools as usize,
            total_configs: total_configs as usize,
            active_configs: active_configs as usize,
            database_size_bytes: database_size_bytes as u64,
        })
    }
}

/// Status information for a single OHLCV token
#[derive(Debug, Clone)]
pub struct OhlcvTokenStatus {
    pub mint: String,
    pub priority: String,
    pub fetch_interval_seconds: i64,
    pub is_active: bool,
    pub last_fetch: Option<String>,
    pub last_activity: String,
    pub consecutive_empty_fetches: i64,
    pub consecutive_pool_failures: i64,
    pub backfill_1m: bool,
    pub backfill_5m: bool,
    pub backfill_15m: bool,
    pub backfill_1h: bool,
    pub backfill_4h: bool,
    pub backfill_12h: bool,
    pub backfill_1d: bool,
    pub created_at: String,
    pub updated_at: String,
    pub candle_count: i64,
    pub earliest_timestamp: i64,
    pub latest_timestamp: i64,
    pub open_gaps: i64,
    pub pool_count: i64,
}

/// Result of a delete operation
#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub candles_deleted: usize,
    pub gaps_deleted: usize,
    pub pools_deleted: usize,
    pub config_deleted: usize,
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_candles: usize,
    pub total_gaps: usize,
    pub total_pools: usize,
    pub total_configs: usize,
    pub active_configs: usize,
    pub database_size_bytes: u64,
}
