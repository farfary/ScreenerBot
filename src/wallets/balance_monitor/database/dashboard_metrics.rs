//! Wallet balance monitor: dashboard metrics cache operations

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::super::cache::CachedDashboardMetrics;
use super::WalletDatabase;

impl WalletDatabase {
    pub fn get_dashboard_metrics(
        &self,
        window_key: &str,
    ) -> Result<Option<CachedDashboardMetrics>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, \
                    computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, \
                    last_processed_timestamp, last_processed_signature, window_start \
                 FROM wallet_dashboard_metrics WHERE chain_id = ?1 AND wallet_address = ?2 AND window_key = ?3",
            )
            .map_err(|e| format!("Failed to prepare dashboard metrics query: {e}"))?;

        let result = stmt
            .query_row(
                params![self.chain.as_str(), self.subject, window_key],
                |row| {
                    let computed_at_str: String = row.get(6)?;
                    let valid_until_str: String = row.get(7)?;
                    let computed_at = DateTime::parse_from_rfc3339(&computed_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                6,
                                "computed_at".to_owned(),
                                rusqlite::types::Type::Text,
                            )
                        })?;
                    let valid_until = DateTime::parse_from_rfc3339(&valid_until_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|_| {
                            rusqlite::Error::InvalidColumnType(
                                7,
                                "valid_until".to_owned(),
                                rusqlite::types::Type::Text,
                            )
                        })?;

                    let last_processed_ts: Option<String> = row.get(11).ok();
                    let last_processed_timestamp = last_processed_ts
                        .as_deref()
                        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                        .map(|dt| dt.with_timezone(&Utc));

                    let window_start_ts: Option<String> = row.get(13).ok();
                    let window_start = window_start_ts
                        .as_deref()
                        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                        .map(|dt| dt.with_timezone(&Utc));

                    Ok(CachedDashboardMetrics {
                        window_key: row.get(0)?,
                        window_hours: row.get::<_, i64>(1)?,
                        snapshot_limit: row.get::<_, i64>(2)? as usize,
                        token_limit: row.get::<_, i64>(3)? as usize,
                        payload: row.get(4)?,
                        payload_format: row.get(5)?,
                        computed_at,
                        valid_until,
                        computation_duration_ms: row.get(8).ok(),
                        snapshot_count: row.get::<_, i64>(9)? as usize,
                        flow_cache_rows: row.get::<_, i64>(10)? as usize,
                        last_processed_timestamp,
                        last_processed_signature: row.get(12).ok(),
                        window_start,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("Failed to fetch dashboard metrics: {e}"))?;

        Ok(result)
    }

    pub fn upsert_dashboard_metrics(&self, metrics: &CachedDashboardMetrics) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO wallet_dashboard_metrics (
                chain_id, wallet_address, window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format,
                computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows,
                last_processed_timestamp, last_processed_signature, window_start, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, datetime('now'))",
            params![
                self.chain.as_str(),
                self.subject,
                metrics.window_key,
                metrics.window_hours,
                metrics.snapshot_limit as i64,
                metrics.token_limit as i64,
                metrics.payload,
                metrics.payload_format,
                metrics.computed_at.to_rfc3339(),
                metrics.valid_until.to_rfc3339(),
                metrics.computation_duration_ms,
                metrics.snapshot_count as i64,
                metrics.flow_cache_rows as i64,
                metrics
                    .last_processed_timestamp
                    .as_ref()
                    .map(|ts| ts.to_rfc3339()),
                metrics.last_processed_signature,
                metrics.window_start.as_ref().map(|ts| ts.to_rfc3339()),
            ],
        )
        .map_err(|e| format!("Failed to upsert dashboard metrics: {e}"))?;
        Ok(())
    }

    pub fn invalidate_dashboard_metrics(&self, window_key: &str) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM wallet_dashboard_metrics WHERE chain_id = ?1 AND wallet_address = ?2 AND window_key = ?3",
            params![self.chain.as_str(), self.subject, window_key],
        )
        .map_err(|e| format!("Failed to invalidate dashboard metrics: {e}"))?;
        Ok(())
    }

    pub fn cleanup_expired_metrics(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let deleted = conn
            .execute(
                "DELETE FROM wallet_dashboard_metrics WHERE chain_id = ?1 AND wallet_address = ?2 AND valid_until < datetime('now')",
                params![self.chain.as_str(), self.subject],
            )
            .map_err(|e| format!("Failed to cleanup dashboard metrics: {e}"))?;
        Ok(deleted.max(0) as u64)
    }
}
