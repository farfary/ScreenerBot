// Database layer for OHLCV module

use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::types::{
    Candle, MintGapAggregate, OhlcvError, OhlcvResult, PoolConfig, Priority, Timeframe,
    TokenOhlcvConfig,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Result as SqliteResult};
use serde_json::json;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct OhlcvDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl OhlcvDatabase {
    /// Initialize the database and create tables
    pub fn new<P: AsRef<Path>>(path: P) -> OhlcvResult<Self> {
        let mut conn = Connection::open(path)
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to open database: {}", e)))?;

        // Enable WAL for better concurrent read/write performance BEFORE moving conn
        let _ = conn.execute("PRAGMA journal_mode=WAL;", []);
        // Set reasonable busy timeout to handle brief contention
        let _ = conn.busy_timeout(std::time::Duration::from_millis(30_000));

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn
            .execute_batch(
                r#"
            -- Pool configurations
            CREATE TABLE IF NOT EXISTS ohlcv_pools (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                dex TEXT NOT NULL,
                liquidity REAL NOT NULL DEFAULT 0.0,
                is_default INTEGER NOT NULL DEFAULT 0,
                last_success TEXT,
                failure_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(mint, pool_address)
            );
            CREATE INDEX IF NOT EXISTS idx_pools_mint ON ohlcv_pools(mint);
            CREATE INDEX IF NOT EXISTS idx_pools_default ON ohlcv_pools(mint, is_default);

            -- UNIFIED CANDLES TABLE (stores ALL native timeframes from API)
            -- Replaces ohlcv_1m and ohlcv_aggregated with single storage
            CREATE TABLE IF NOT EXISTS ohlcv_candles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                timeframe TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                open REAL NOT NULL,
                high REAL NOT NULL,
                low REAL NOT NULL,
                close REAL NOT NULL,
                volume REAL NOT NULL,
                source TEXT NOT NULL DEFAULT 'api',
                fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(mint, pool_address, timeframe, timestamp)
            );
            CREATE INDEX IF NOT EXISTS idx_candles_lookup ON ohlcv_candles(mint, timeframe, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_candles_pool_lookup ON ohlcv_candles(pool_address, timeframe, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_candles_cleanup ON ohlcv_candles(fetched_at);

            -- Gap tracking (per timeframe)
            CREATE TABLE IF NOT EXISTS ohlcv_gaps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                timeframe TEXT NOT NULL,
                start_timestamp INTEGER NOT NULL,
                end_timestamp INTEGER NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                last_attempt TEXT,
                filled INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(mint, pool_address, timeframe, start_timestamp, end_timestamp)
            );
            CREATE INDEX IF NOT EXISTS idx_gaps_unfilled ON ohlcv_gaps(filled, mint, timeframe);
            CREATE INDEX IF NOT EXISTS idx_gaps_retry ON ohlcv_gaps(filled, attempts, last_attempt);

            -- Token monitoring configuration (with backfill tracking)
            CREATE TABLE IF NOT EXISTS ohlcv_monitor_config (
                mint TEXT PRIMARY KEY,
                priority TEXT NOT NULL,
                fetch_interval_seconds INTEGER NOT NULL DEFAULT 60,
                source TEXT NOT NULL DEFAULT 'manual',
                is_active INTEGER NOT NULL DEFAULT 1,
                backfill_1m_complete INTEGER NOT NULL DEFAULT 0,
                backfill_5m_complete INTEGER NOT NULL DEFAULT 0,
                backfill_15m_complete INTEGER NOT NULL DEFAULT 0,
                backfill_1h_complete INTEGER NOT NULL DEFAULT 0,
                backfill_4h_complete INTEGER NOT NULL DEFAULT 0,
                backfill_12h_complete INTEGER NOT NULL DEFAULT 0,
                backfill_1d_complete INTEGER NOT NULL DEFAULT 0,
                backfill_started_at TEXT,
                backfill_completed_at TEXT,
                last_fetch TEXT,
                last_activity TEXT NOT NULL,
                consecutive_empty_fetches INTEGER NOT NULL DEFAULT 0,
                last_pool_discovery_attempt INTEGER,
                consecutive_pool_failures INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_monitor_active ON ohlcv_monitor_config(is_active, priority);
            CREATE INDEX IF NOT EXISTS idx_monitor_backfill ON ohlcv_monitor_config(is_active, backfill_1d_complete);
            "#
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to create tables: {}", e)))?;

        Ok(())
    }

    // ==================== Pool Management ====================

    pub fn upsert_pool(&self, mint: &str, pool: &PoolConfig) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let last_success = pool.last_successful_fetch.map(|dt| dt.to_rfc3339());

        conn
            .execute(
                "INSERT INTO ohlcv_pools (mint, pool_address, dex, liquidity, is_default, last_success, failure_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(mint, pool_address) DO UPDATE SET
                liquidity = excluded.liquidity,
                is_default = excluded.is_default,
                last_success = excluded.last_success,
                failure_count = excluded.failure_count",
                params![
                    mint,
                    &pool.address,
                    &pool.dex,
                    pool.liquidity,
                    pool.is_default as i32,
                    last_success,
                    pool.failure_count
                ]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to upsert pool: {}", e)))?;

        Ok(())
    }

    pub fn delete_pool(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn.execute(
            "DELETE FROM ohlcv_pools WHERE mint = ?1 AND pool_address = ?2",
            params![mint, pool_address],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to delete pool: {}", e)))?;

        Ok(())
    }

    pub fn get_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, dex, liquidity, is_default, last_success, failure_count
                 FROM ohlcv_pools
                 WHERE mint = ?1
                 ORDER BY liquidity DESC",
            )
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to prepare statement: {}", e))
            })?;

        let pools = stmt
            .query_map(params![mint], |row| {
                let last_success_str: Option<String> = row.get(4)?;
                let last_success = last_success_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });

                Ok(PoolConfig {
                    address: row.get(0)?,
                    dex: row.get(1)?,
                    liquidity: row.get(2)?,
                    is_default: row.get::<_, i32>(3)? != 0,
                    last_successful_fetch: last_success,
                    failure_count: row.get(5)?,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect results: {}", e)))?;

        Ok(pools)
    }

    pub fn mark_pool_failure(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn
            .execute(
                "UPDATE ohlcv_pools SET failure_count = failure_count + 1 WHERE mint = ?1 AND pool_address = ?2",
                params![mint, pool_address]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark failure: {}", e)))?;

        Ok(())
    }

    pub fn mark_pool_success(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn
            .execute(
                "UPDATE ohlcv_pools SET failure_count = 0, last_success = ?1 WHERE mint = ?2 AND pool_address = ?3",
                params![Utc::now().to_rfc3339(), mint, pool_address]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark success: {}", e)))?;

        Ok(())
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT MIN(timestamp), MAX(timestamp) FROM ohlcv_candles WHERE mint = ?1 AND pool_address = ?2 AND timeframe = ?3",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {}", e)))?;

        let bounds = stmt
            .query_row(params![mint, pool_address, timeframe.as_str()], |row| {
                let min_ts: Option<i64> = row.get(0)?;
                let max_ts: Option<i64> = row.get(1)?;
                Ok((min_ts, max_ts))
            })
            .optional()
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(bounds.and_then(|(min_ts, max_ts)| match (min_ts, max_ts) {
            (Some(min_val), Some(max_val)) => Some((min_val, max_val)),
            _ => None,
        }))
    }

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn
            .execute(
                "INSERT OR IGNORE INTO ohlcv_gaps (mint, pool_address, timeframe, start_timestamp, end_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![mint, pool_address, timeframe.as_str(), start_timestamp, end_timestamp]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to insert gap: {}", e)))?;

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, start_timestamp, end_timestamp FROM ohlcv_gaps
                 WHERE mint = ?1 AND timeframe = ?2 AND filled = 0
                 ORDER BY start_timestamp DESC
                 LIMIT 100",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {}", e)))?;

        let gaps = stmt
            .query_map(params![mint, timeframe.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {}", e)))?;

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn
            .execute(
                "UPDATE ohlcv_gaps SET filled = 1 
             WHERE mint = ?1 AND pool_address = ?2 AND timeframe = ?3 AND start_timestamp = ?4 AND end_timestamp = ?5",
                params![mint, pool_address, timeframe.as_str(), start_timestamp, end_timestamp]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark gap filled: {}", e)))?;

        Ok(())
    }

    pub fn get_gap_aggregate(&self) -> OhlcvResult<(usize, usize)> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let (gap_count, token_count): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*) as gap_count, COUNT(DISTINCT mint) as token_count
                 FROM ohlcv_gaps WHERE filled = 0",
                params![],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to read gap aggregate: {}", e))
            })?;

        Ok((token_count.max(0) as usize, gap_count.max(0) as usize))
    }

    pub fn get_top_open_gaps(&self, limit: usize) -> OhlcvResult<Vec<MintGapAggregate>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, COUNT(*) as gap_count,
                        MAX(end_timestamp - start_timestamp) as largest_gap,
                        MAX(end_timestamp) as latest_gap
                 FROM ohlcv_gaps
                 WHERE filled = 0
                 GROUP BY mint
                 ORDER BY largest_gap DESC, latest_gap DESC
                 LIMIT ?1",
            )
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to prepare gap summary: {}", e))
            })?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Gap summary query failed: {}", e)))?;

        let aggregates = rows.collect::<SqliteResult<Vec<_>>>().map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to collect gap summary: {}", e))
        })?;

        Ok(aggregates)
    }

    // ==================== Monitor Configuration ====================

    pub fn upsert_monitor_config(&self, config: &TokenOhlcvConfig) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let last_fetch = config.last_fetch.as_ref().map(|dt| dt.to_rfc3339());

        conn
            .execute(
                "INSERT INTO ohlcv_monitor_config (mint, priority, fetch_interval_seconds, last_fetch, last_activity, consecutive_empty_fetches, is_active, last_pool_discovery_attempt, consecutive_pool_failures)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(mint) DO UPDATE SET
                priority = excluded.priority,
                fetch_interval_seconds = excluded.fetch_interval_seconds,
                last_fetch = excluded.last_fetch,
                last_activity = excluded.last_activity,
                consecutive_empty_fetches = excluded.consecutive_empty_fetches,
                is_active = excluded.is_active,
                last_pool_discovery_attempt = excluded.last_pool_discovery_attempt,
                consecutive_pool_failures = excluded.consecutive_pool_failures",
                params![
                    &config.mint,
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to upsert config: {}", e)))?;

        Ok(())
    }

    pub fn get_monitor_config(&self, mint: &str) -> OhlcvResult<Option<TokenOhlcvConfig>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let config: Option<TokenOhlcvConfig> = conn
            .query_row(
                "SELECT priority, fetch_interval_seconds, last_fetch, last_activity, consecutive_empty_fetches, is_active, last_pool_discovery_attempt, consecutive_pool_failures
                 FROM ohlcv_monitor_config WHERE mint = ?1",
                params![mint],
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(config)
    }

    pub fn get_all_active_configs(&self) -> OhlcvResult<Vec<TokenOhlcvConfig>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, priority, fetch_interval_seconds, last_fetch, last_activity, consecutive_empty_fetches, last_pool_discovery_attempt, consecutive_pool_failures
                 FROM ohlcv_monitor_config WHERE is_active = 1
                 ORDER BY priority DESC"
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {}", e)))?;

        let configs = stmt
            .query_map(params![], |row| {
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {}", e)))?;

        Ok(configs)
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| OhlcvError::DatabaseError(format!("Transaction failed: {}", e)))?;

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Commit failed: {}", e)))?;

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

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
            query.push_str(&format!(" AND pool_address = ?{}", param_index));
            params_vec.push(Box::new(pool.to_string()));
            param_index += 1;
        }

        if let Some(from) = from_ts {
            query.push_str(&format!(" AND timestamp >= ?{}", param_index));
            params_vec.push(Box::new(from));
            param_index += 1;
        }

        if let Some(to) = to_ts {
            query.push_str(&format!(" AND timestamp <= ?{}", param_index));
            params_vec.push(Box::new(to));
            param_index += 1;
        }

        query.push_str(" ORDER BY timestamp ASC");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT ?{}", param_index));
            params_vec.push(Box::new(lim));
        }

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| OhlcvError::DatabaseError(format!("Prepare failed: {}", e)))?;

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        candles
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Collect failed: {}", e)))
    }

    /// Check if backfill is complete for timeframe
    pub fn is_backfill_complete(&self, mint: &str, timeframe: Timeframe) -> OhlcvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let column = format!("backfill_{}_complete", timeframe.as_str().replace('-', ""));

        let query = format!(
            "SELECT {} FROM ohlcv_monitor_config WHERE mint = ?1",
            column
        );

        let result: i32 = conn
            .query_row(&query, params![mint], |row| row.get(0))
            .optional()
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?
            .unwrap_or(0);

        Ok(result == 1)
    }

    /// Mark backfill as complete for timeframe
    pub fn mark_backfill_complete(&self, mint: &str, timeframe: Timeframe) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let column = format!("backfill_{}_complete", timeframe.as_str().replace('-', ""));

        let query = format!(
            "UPDATE ohlcv_monitor_config SET {} = 1, updated_at = CURRENT_TIMESTAMP WHERE mint = ?1",
            column
        );

        conn.execute(&query, params![mint])
            .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {}", e)))?;

        Ok(())
    }

    /// Mark all backfills as complete
    pub fn mark_all_backfills_complete(&self, mint: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

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
        .map_err(|e| OhlcvError::DatabaseError(format!("Update failed: {}", e)))?;

        Ok(())
    }

    // ==================== Cleanup ====================

    pub fn cleanup_old_data(&self, retention_days: i64) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let cutoff = (Utc::now() - Duration::days(retention_days)).to_rfc3339();

        let deleted = conn
            .execute(
                "DELETE FROM ohlcv_candles WHERE fetched_at < ?1",
                params![cutoff],
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Cleanup failed: {}", e)))?;

        Ok(deleted)
    }

    // ==================== Metrics ====================

    pub fn get_data_point_count(&self) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_candles", params![], |row| {
                row.get(0)
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }

    pub fn has_data_for_mint(&self, mint: &str) -> OhlcvResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let exists: i64 = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM ohlcv_candles WHERE mint = ?1 LIMIT 1)",
                params![mint],
                |row| row.get(0),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(exists != 0)
    }

    pub fn get_mints_with_data(&self, mints: &[String]) -> OhlcvResult<HashSet<String>> {
        if mints.is_empty() {
            return Ok(HashSet::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

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
                .map_err(|e| OhlcvError::DatabaseError(format!("Query prep failed: {}", e)))?;

            let params: Vec<&dyn rusqlite::ToSql> = chunk
                .iter()
                .map(|mint| mint as &dyn rusqlite::ToSql)
                .collect();

            let rows = stmt
                .query_map(params_from_iter(params.iter().copied()), |row| {
                    row.get::<_, String>(0)
                })
                .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

            for row in rows {
                let mint =
                    row.map_err(|e| OhlcvError::DatabaseError(format!("Row parse failed: {}", e)))?;
                result.insert(mint);
            }
        }

        Ok(result)
    }

    pub fn get_pool_count(&self) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_pools", params![], |row| {
                row.get(0)
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }

    pub fn get_token_count(&self) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT mint) FROM ohlcv_monitor_config WHERE is_active = 1",
                params![],
                |row| row.get(0),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }

    pub fn get_gap_count(&self, filled: bool) -> OhlcvResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_gaps WHERE filled = ?1",
                params![filled as i32],
                |row| row.get(0),
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }
}
