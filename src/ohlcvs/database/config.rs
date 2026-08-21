//! Monitor configuration — upsert, query, and list token monitoring configs.

use crate::ohlcvs::types::{OhlcvError, OhlcvResult, Priority, TokenOhlcvConfig};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Result as SqliteResult};

use super::OhlcvDatabase;

impl OhlcvDatabase {
    // ==================== Monitor Configuration ====================

    pub fn upsert_monitor_config(&self, config: &TokenOhlcvConfig) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let last_fetch = config.last_fetch.as_ref().map(|dt| dt.to_rfc3339());

        conn
            .execute(
                "INSERT INTO ohlcv_monitor_config (chain_id, mint, priority, fetch_interval_seconds, last_fetch, last_activity, consecutive_empty_fetches, is_active, last_pool_discovery_attempt, consecutive_pool_failures)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(chain_id, mint) DO UPDATE SET
                priority = excluded.priority,
                fetch_interval_seconds = excluded.fetch_interval_seconds,
                last_fetch = excluded.last_fetch,
                last_activity = excluded.last_activity,
                consecutive_empty_fetches = excluded.consecutive_empty_fetches,
                is_active = excluded.is_active,
                last_pool_discovery_attempt = excluded.last_pool_discovery_attempt,
                consecutive_pool_failures = excluded.consecutive_pool_failures",
                params![
                    self.chain_id(), &config.mint,
                    config.priority.as_str(),
                    config.fetch_frequency.as_secs() as i64,
                    last_fetch,
                    config.last_activity.to_rfc3339(),
                    config.consecutive_empty_fetches,
                    config.is_active as i32,
                    config.last_pool_discovery_attempt,
                    config.consecutive_pool_failures
                ]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to upsert config: {e}")))?;

        Ok(())
    }

    pub fn get_monitor_config(&self, mint: &str) -> OhlcvResult<Option<TokenOhlcvConfig>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let config: Option<TokenOhlcvConfig> = conn
            .query_row(
                "SELECT priority, fetch_interval_seconds, last_fetch, last_activity, consecutive_empty_fetches, is_active, last_pool_discovery_attempt, consecutive_pool_failures
                 FROM ohlcv_monitor_config WHERE chain_id = ?1 AND mint = ?2",
                params![self.chain_id(), mint],
                |row| {
                    let priority_str: String = row.get(0)?;
                    let priority = Priority::from_str(&priority_str).unwrap_or(Priority::Low);
                    let fetch_secs: i64 = row.get(1)?;
                    let last_fetch_str: Option<String> = row.get(2)?;
                    let last_fetch = last_fetch_str.and_then(|s| {
                        DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    });
                    let last_activity_str: String = row.get(3)?;
                    let last_activity = DateTime::parse_from_rfc3339(&last_activity_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let consecutive_empty: u32 = row.get(4)?;
                    let is_active: i32 = row.get(5)?;
                    let last_pool_attempt: Option<i64> = row.get(6)?;
                    let pool_failures: u32 = row.get(7)?;

                    let mut config = TokenOhlcvConfig::new(mint.to_string(), priority);
                    config.fetch_frequency = std::time::Duration::from_secs(fetch_secs as u64);
                    config.last_activity = last_activity;
                    config.last_fetch = last_fetch;
                    config.consecutive_empty_fetches = consecutive_empty;
                    config.is_active = is_active != 0;
                    config.last_pool_discovery_attempt = last_pool_attempt;
                    config.consecutive_pool_failures = pool_failures;

                    Ok(config)
                }
            )
            .optional()
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?;

        Ok(config)
    }

    pub fn get_all_active_configs(&self) -> OhlcvResult<Vec<TokenOhlcvConfig>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, priority, fetch_interval_seconds, last_fetch, last_activity, consecutive_empty_fetches, last_pool_discovery_attempt, consecutive_pool_failures
                 FROM ohlcv_monitor_config WHERE chain_id = ?1 AND is_active = 1
                 ORDER BY priority DESC"
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {e}")))?;

        let configs = stmt
            .query_map(params![self.chain_id()], |row| {
                let mint: String = row.get(0)?;
                let priority_str: String = row.get(1)?;
                let priority = Priority::from_str(&priority_str).unwrap_or(Priority::Low);
                let fetch_secs: i64 = row.get(2)?;
                let last_fetch_str: Option<String> = row.get(3)?;
                let last_fetch = last_fetch_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });
                let last_activity_str: String = row.get(4)?;
                let last_activity = DateTime::parse_from_rfc3339(&last_activity_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let consecutive_empty: u32 = row.get(5)?;
                let last_pool_attempt: Option<i64> = row.get(6)?;
                let pool_failures: u32 = row.get(7)?;

                let mut config = TokenOhlcvConfig::new(mint, priority);
                config.fetch_frequency = std::time::Duration::from_secs(fetch_secs as u64);
                config.last_fetch = last_fetch;
                config.last_activity = last_activity;
                config.consecutive_empty_fetches = consecutive_empty;
                config.last_pool_discovery_attempt = last_pool_attempt;
                config.consecutive_pool_failures = pool_failures;

                Ok(config)
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {e}")))?;

        Ok(configs)
    }
}
