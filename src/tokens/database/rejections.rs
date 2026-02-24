use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::filtering::RejectedToken;
use crate::logger::{self, LogTag};
use crate::tokens::types::{TokenError, TokenResult};

use super::TokenDatabase;

impl TokenDatabase {
    pub fn update_rejection_status(
        &self,
        mint: &str,
        reason: &str,
        source: &str,
        rejected_at: i64,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        conn.execute(
            "UPDATE update_tracking SET 
                last_rejection_reason = ?1, 
                last_rejection_source = ?2, 
                last_rejection_at = ?3 
             WHERE mint = ?4",
            params![reason, source, rejected_at, mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to update rejection status: {e}")))?;

        Ok(())
    }

    /// Clear rejection status for a token that passed filtering
    pub fn clear_rejection_status(&self, mint: &str) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        conn.execute(
            "UPDATE update_tracking SET 
                last_rejection_reason = NULL, 
                last_rejection_source = NULL, 
                last_rejection_at = NULL 
             WHERE mint = ?1",
            params![mint],
        )
        .map_err(|e| TokenError::Database(format!("Failed to clear rejection status: {e}")))?;

        Ok(())
    }

    /// Batch clear rejection status for multiple tokens (PERF optimization)
    /// Uses a single transaction instead of spawning individual tasks

    pub fn batch_clear_rejection_status(&self, mints: &[String]) -> TokenResult<usize> {
        if mints.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {e}")))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE update_tracking SET 
                        last_rejection_reason = NULL, 
                        last_rejection_source = NULL, 
                        last_rejection_at = NULL 
                     WHERE mint = ?1",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {e}")))?;

            for mint in mints {
                match stmt.execute(params![mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        // Log but continue - don't fail entire batch
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_clear_rejection_status error for {mint}: {e}"),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {e}")))?;

        Ok(updated)
    }

    /// Batch update priority for multiple tokens (PERF optimization)
    /// Uses a single transaction instead of spawning individual tasks

    pub fn batch_update_rejection_status(
        &self,
        updates: &[(String, String, String, i64)],
    ) -> TokenResult<usize> {
        if updates.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {e}")))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "UPDATE update_tracking SET 
                        last_rejection_reason = ?1, 
                        last_rejection_source = ?2, 
                        last_rejection_at = ?3 
                     WHERE mint = ?4",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {e}")))?;

            for (mint, reason, source, rejected_at) in updates {
                match stmt.execute(params![reason, source, rejected_at, mint]) {
                    Ok(rows) => updated += rows,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_update_rejection_status error for {mint}: {e}"),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {e}")))?;

        Ok(updated)
    }

    /// Batch upsert rejection stats (PERF optimization)
    /// stats: Vec of (reason, source, timestamp)
    pub fn batch_upsert_rejection_stats(
        &self,
        stats: &[(String, String, i64)],
    ) -> TokenResult<usize> {
        if stats.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let tx = conn
            .transaction()
            .map_err(|e| TokenError::Database(format!("Transaction start failed: {e}")))?;

        let mut updated = 0;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO rejection_stats (bucket_hour, reason, source, rejection_count, unique_tokens, first_seen, last_seen)
                     VALUES (?1, ?2, ?3, 1, 1, ?4, ?4)
                     ON CONFLICT(bucket_hour, reason, source) DO UPDATE SET
                         rejection_count = rejection_count + 1,
                         last_seen = ?4",
                )
                .map_err(|e| TokenError::Database(format!("Prepare failed: {e}")))?;

            for (reason, source, timestamp) in stats {
                // Round timestamp to hour bucket
                let bucket_hour = (timestamp / 3600) * 3600;
                match stmt.execute(params![bucket_hour, reason, source, timestamp]) {
                    Ok(_) => updated += 1,
                    Err(e) => {
                        logger::warning(
                            LogTag::Tokens,
                            &format!("batch_upsert_rejection_stats error: {e}"),
                        );
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| TokenError::Database(format!("Transaction commit failed: {e}")))?;

        Ok(updated)
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
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        // Build query with optional time filters on last_rejection_at
        let mut query = "SELECT 
                    last_rejection_reason, 
                    last_rejection_source, 
                    COUNT(*) as count 
                 FROM update_tracking 
                 WHERE last_rejection_reason IS NOT NULL"
            .to_string();

        if start_time.is_some() {
            query.push_str(" AND last_rejection_at >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND last_rejection_at <= :end_time");
        }

        query
            .push_str(" GROUP BY last_rejection_reason, last_rejection_source ORDER BY count DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        // Bind parameters
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
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
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Get list of rejected tokens with pagination and optional filtering

    pub fn get_recent_rejections(
        &self,
        limit: usize,
    ) -> TokenResult<Vec<(String, String, String, i64, Option<String>)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let query = "SELECT ut.mint, ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at, t.symbol 
                     FROM update_tracking ut 
                     LEFT JOIN tokens t ON ut.mint = t.mint 
                     WHERE ut.last_rejection_reason IS NOT NULL 
                     ORDER BY ut.last_rejection_at DESC LIMIT :limit";

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let limit_i64 = limit as i64;
        let rows = stmt
            .query_map(&[(":limit", &limit_i64)], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| TokenError::Database(format!("Row failed: {e}")))?);
        }

        Ok(results)
    }

    pub fn get_rejected_tokens(
        &self,
        reason_filter: Option<String>,
        source_filter: Option<String>,
        search_filter: Option<String>,
        limit: usize,
        offset: usize,
    ) -> TokenResult<Vec<(String, String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut query = if search_filter.is_some() {
            "SELECT ut.mint, ut.last_rejection_reason, ut.last_rejection_source, ut.last_rejection_at 
             FROM update_tracking ut 
             LEFT JOIN tokens t ON ut.mint = t.mint 
             WHERE ut.last_rejection_reason IS NOT NULL".to_owned()
        } else {
            "SELECT mint, last_rejection_reason, last_rejection_source, last_rejection_at 
             FROM update_tracking 
             WHERE last_rejection_reason IS NOT NULL"
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

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        // Build params dynamically - only include params that are in the query
        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
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
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Insert rejection event into history table (for time-range analytics)

    pub fn insert_rejection_history(
        &self,
        mint: &str,
        reason: &str,
        source: &str,
        rejected_at: i64,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        conn.execute(
            "INSERT INTO rejection_history (mint, reason, source, rejected_at) VALUES (?1, ?2, ?3, ?4)",
            params![mint, reason, source, rejected_at],
        )
        .map_err(|e| TokenError::Database(format!("Failed to insert rejection history: {e}")))?;

        Ok(())
    }

    /// Get rejection statistics for a specific time range

    pub fn get_rejection_stats_for_range(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        // If no time range specified, fall back to current rejection stats (update_tracking table)
        if start_time.is_none() && end_time.is_none() {
            return self.get_rejection_stats();
        }

        // Query rejection_history table for time-range stats
        let mut query =
            "SELECT reason, source, COUNT(*) as count FROM rejection_history WHERE 1=1".to_owned();

        if start_time.is_some() {
            query.push_str(" AND rejected_at >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND rejected_at <= :end_time");
        }

        query.push_str(" GROUP BY reason, source ORDER BY count DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Failed to prepare: {e}")))?;

        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
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
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

        let mut results = Vec::new();
        for row in rows {
            if let Ok(entry) = row {
                results.push(entry);
            }
        }

        Ok(results)
    }

    /// Cleanup old rejection history entries (keep last N hours)
    /// This is critical for database size management - rejection history grows ~5GB/day

    pub fn cleanup_rejection_history(&self, hours_to_keep: i64) -> TokenResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let cutoff = chrono::Utc::now().timestamp() - (hours_to_keep * 60 * 60);

        let deleted = conn
            .execute(
                "DELETE FROM rejection_history WHERE rejected_at < ?1",
                params![cutoff],
            )
            .map_err(|e| {
                TokenError::Database(format!("Failed to cleanup rejection history: {e}"))
            })?;

        Ok(deleted)
    }

    /// Upsert rejection stat into aggregated hourly bucket table
    /// This replaces per-event logging with O(1) aggregation

    pub fn upsert_rejection_stat(
        &self,
        reason: &str,
        source: &str,
        timestamp: i64,
    ) -> TokenResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        // Round timestamp to hour bucket
        let bucket_hour = (timestamp / 3600) * 3600;

        conn.execute(
            "INSERT INTO rejection_stats (bucket_hour, reason, source, rejection_count, unique_tokens, first_seen, last_seen)
             VALUES (?1, ?2, ?3, 1, 1, ?4, ?4)
             ON CONFLICT(bucket_hour, reason, source) DO UPDATE SET
                 rejection_count = rejection_count + 1,
                 last_seen = ?4",
            params![bucket_hour, reason, source, timestamp],
        )
        .map_err(|e| TokenError::Database(format!("Upsert rejection stat failed: {e}")))?;

        Ok(())
    }

    /// Get rejection statistics from aggregated table for a time range

    pub fn get_rejection_stats_aggregated(
        &self,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> TokenResult<Vec<(String, String, i64)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let mut query =
            "SELECT reason, source, SUM(rejection_count) as total FROM rejection_stats WHERE 1=1"
                .to_string();

        if start_time.is_some() {
            query.push_str(" AND bucket_hour >= :start_time");
        }
        if end_time.is_some() {
            query.push_str(" AND bucket_hour <= :end_time");
        }

        query.push_str(" GROUP BY reason, source ORDER BY total DESC");

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| TokenError::Database(format!("Prepare failed: {e}")))?;

        let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
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
            .map_err(|e| TokenError::Database(format!("Query failed: {e}")))?;

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
        let conn = self
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let cutoff = chrono::Utc::now().timestamp() - (hours_to_keep * 3600);

        let deleted = conn
            .execute(
                "DELETE FROM rejection_stats WHERE bucket_hour < ?1",
                params![cutoff],
            )
            .map_err(|e| TokenError::Database(format!("Delete rejection stats failed: {e}")))?;

        Ok(deleted)
    }
}
