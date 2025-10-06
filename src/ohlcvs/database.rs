// Database layer for OHLCV module

use crate::ohlcvs::types::{
    OhlcvDataPoint,
    OhlcvError,
    OhlcvResult,
    PoolConfig,
    Priority,
    Timeframe,
    TokenOhlcvConfig,
};
use chrono::{ DateTime, Duration, Utc };
use rusqlite::{ params, Connection, OptionalExtension, Result as SqliteResult };
use std::path::Path;
use std::sync::{ Arc, Mutex };

pub struct OhlcvDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl OhlcvDatabase {
    /// Initialize the database and create tables
    pub fn new<P: AsRef<Path>>(path: P) -> OhlcvResult<Self> {
        let conn = Connection::open(path).map_err(|e|
            OhlcvError::DatabaseError(format!("Failed to open database: {}", e))
        )?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> OhlcvResult<()> {
        let conn = self.conn
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

            -- Raw 1-minute data
            CREATE TABLE IF NOT EXISTS ohlcv_1m (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                open REAL NOT NULL,
                high REAL NOT NULL,
                low REAL NOT NULL,
                close REAL NOT NULL,
                volume REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(mint, pool_address, timestamp)
            );
            CREATE INDEX IF NOT EXISTS idx_1m_mint_timestamp ON ohlcv_1m(mint, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_1m_cleanup ON ohlcv_1m(created_at);
            CREATE INDEX IF NOT EXISTS idx_1m_pool_timestamp ON ohlcv_1m(pool_address, timestamp DESC);

            -- Aggregated data cache
            CREATE TABLE IF NOT EXISTS ohlcv_aggregated (
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
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(mint, pool_address, timeframe, timestamp)
            );
            CREATE INDEX IF NOT EXISTS idx_agg_lookup ON ohlcv_aggregated(mint, timeframe, timestamp DESC);

            -- Gap tracking
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
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(mint, pool_address, timeframe, start_timestamp, end_timestamp)
            );
            CREATE INDEX IF NOT EXISTS idx_gaps_unfilled ON ohlcv_gaps(filled, mint);

            -- Token monitoring configuration
            CREATE TABLE IF NOT EXISTS ohlcv_monitor_config (
                mint TEXT PRIMARY KEY,
                priority TEXT NOT NULL,
                fetch_interval_seconds INTEGER NOT NULL,
                last_fetch TEXT,
                last_activity TEXT NOT NULL,
                consecutive_empty_fetches INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 1,
                last_pool_discovery_attempt INTEGER,
                consecutive_pool_failures INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_monitor_active ON ohlcv_monitor_config(is_active, priority);
            "#
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to create tables: {}", e)))?;

        Ok(())
    }

    // ==================== Pool Management ====================

    pub fn upsert_pool(&self, mint: &str, pool: &PoolConfig) -> OhlcvResult<()> {
        let conn = self.conn
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

    pub fn get_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, dex, liquidity, is_default, last_success, failure_count
                 FROM ohlcv_pools
                 WHERE mint = ?1
                 ORDER BY liquidity DESC"
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
        let conn = self.conn
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
        let conn = self.conn
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

    // ==================== OHLCV Data Storage ====================

    pub fn insert_1m_data(
        &self,
        mint: &str,
        pool_address: &str,
        data: &[OhlcvDataPoint]
    ) -> OhlcvResult<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to start transaction: {}", e))
            })?;

        let mut inserted = 0;
        for point in data {
            let result = tx.execute(
                "INSERT OR IGNORE INTO ohlcv_1m (mint, pool_address, timestamp, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    mint,
                    pool_address,
                    point.timestamp,
                    point.open,
                    point.high,
                    point.low,
                    point.close,
                    point.volume
                ]
            );

            if let Ok(rows) = result {
                inserted += rows;
            }
        }

        tx.commit().map_err(|e| OhlcvError::DatabaseError(format!("Failed to commit: {}", e)))?;

        Ok(inserted)
    }

    pub fn get_1m_data(
        &self,
        mint: &str,
        pool_address: Option<&str>,
        from_timestamp: Option<i64>,
        to_timestamp: Option<i64>,
        limit: usize
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut query = String::from(
            "SELECT timestamp, open, high, low, close, volume FROM ohlcv_1m WHERE mint = ?1"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(mint.to_string())];

        if let Some(pool) = pool_address {
            let placeholder = params_vec.len() + 1;
            query.push_str(&format!(" AND pool_address = ?{}", placeholder));
            params_vec.push(Box::new(pool.to_string()));
        }

        if let Some(from) = from_timestamp {
            let placeholder = params_vec.len() + 1;
            query.push_str(&format!(" AND timestamp >= ?{}", placeholder));
            params_vec.push(Box::new(from));
        }

        if let Some(to) = to_timestamp {
            let placeholder = params_vec.len() + 1;
            query.push_str(&format!(" AND timestamp <= ?{}", placeholder));
            params_vec.push(Box::new(to));
        }

        let placeholder = params_vec.len() + 1;
        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ?{}", placeholder));
        params_vec.push(Box::new(limit as i64));

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {}", e)))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|p| p.as_ref())
            .collect();

        let data = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(OhlcvDataPoint {
                    timestamp: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {}", e)))?;

        Ok(data)
    }

    // ==================== Aggregated Data Cache ====================

    pub fn cache_aggregated_data(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        data: &[OhlcvDataPoint]
    ) -> OhlcvResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to start transaction: {}", e))
            })?;

        for point in data {
            tx
                .execute(
                    "INSERT OR REPLACE INTO ohlcv_aggregated (mint, pool_address, timeframe, timestamp, open, high, low, close, volume)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        mint,
                        pool_address,
                        timeframe.as_str(),
                        point.timestamp,
                        point.open,
                        point.high,
                        point.low,
                        point.close,
                        point.volume
                    ]
                )
                .map_err(|e| OhlcvError::DatabaseError(format!("Insert failed: {}", e)))?;
        }

        tx.commit().map_err(|e| OhlcvError::DatabaseError(format!("Commit failed: {}", e)))?;

        Ok(())
    }

    pub fn get_aggregated_data(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        from_timestamp: Option<i64>,
        to_timestamp: Option<i64>,
        limit: usize
    ) -> OhlcvResult<Vec<OhlcvDataPoint>> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut query = String::from(
            "SELECT timestamp, open, high, low, close, volume FROM ohlcv_aggregated 
             WHERE mint = ?1 AND pool_address = ?2 AND timeframe = ?3"
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(mint.to_string()),
            Box::new(pool_address.to_string()),
            Box::new(timeframe.as_str().to_string())
        ];

        if let Some(from) = from_timestamp {
            let placeholder = params_vec.len() + 1;
            query.push_str(&format!(" AND timestamp >= ?{}", placeholder));
            params_vec.push(Box::new(from));
        }

        if let Some(to) = to_timestamp {
            let placeholder = params_vec.len() + 1;
            query.push_str(&format!(" AND timestamp <= ?{}", placeholder));
            params_vec.push(Box::new(to));
        }

        let placeholder = params_vec.len() + 1;
        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT ?{}", placeholder));
        params_vec.push(Box::new(limit as i64));

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {}", e)))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec
            .iter()
            .map(|p| p.as_ref())
            .collect();

        let data = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(OhlcvDataPoint {
                    timestamp: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect: {}", e)))?;

        Ok(data)
    }

    // ==================== Gap Management ====================

    pub fn insert_gap(
        &self,
        mint: &str,
        pool_address: &str,
        timeframe: Timeframe,
        start_timestamp: i64,
        end_timestamp: i64
    ) -> OhlcvResult<()> {
        let conn = self.conn
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
        timeframe: Timeframe
    ) -> OhlcvResult<Vec<(String, i64, i64)>> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, start_timestamp, end_timestamp FROM ohlcv_gaps
                 WHERE mint = ?1 AND timeframe = ?2 AND filled = 0
                 ORDER BY start_timestamp DESC
                 LIMIT 100"
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare: {}", e)))?;

        let gaps = stmt
            .query_map(params![mint, timeframe.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
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
        end_timestamp: i64
    ) -> OhlcvResult<()> {
        let conn = self.conn
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

    // ==================== Monitor Configuration ====================

    pub fn upsert_monitor_config(&self, config: &TokenOhlcvConfig) -> OhlcvResult<()> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        conn
            .execute(
                "INSERT INTO ohlcv_monitor_config (mint, priority, fetch_interval_seconds, last_activity, consecutive_empty_fetches, is_active, last_pool_discovery_attempt, consecutive_pool_failures)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(mint) DO UPDATE SET
                priority = excluded.priority,
                fetch_interval_seconds = excluded.fetch_interval_seconds,
                last_activity = excluded.last_activity,
                consecutive_empty_fetches = excluded.consecutive_empty_fetches,
                is_active = excluded.is_active,
                last_pool_discovery_attempt = excluded.last_pool_discovery_attempt,
                consecutive_pool_failures = excluded.consecutive_pool_failures",
                params![
                    &config.mint,
                    config.priority.as_str(),
                    config.fetch_frequency.as_secs() as i64,
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
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let config: Option<TokenOhlcvConfig> = conn
            .query_row(
                "SELECT priority, fetch_interval_seconds, last_activity, consecutive_empty_fetches, is_active, last_pool_discovery_attempt, consecutive_pool_failures
                 FROM ohlcv_monitor_config WHERE mint = ?1",
                params![mint],
                |row| {
                    let priority_str: String = row.get(0)?;
                    let priority = Priority::from_str(&priority_str).unwrap_or(Priority::Low);
                    let fetch_secs: i64 = row.get(1)?;
                    let last_activity_str: String = row.get(2)?;
                    let last_activity = DateTime::parse_from_rfc3339(&last_activity_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let consecutive_empty: u32 = row.get(3)?;
                    let is_active: i32 = row.get(4)?;
                    let last_pool_attempt: Option<i64> = row.get(5)?;
                    let pool_failures: u32 = row.get(6)?;

                    let mut config = TokenOhlcvConfig::new(mint.to_string(), priority);
                    config.fetch_frequency = std::time::Duration::from_secs(fetch_secs as u64);
                    config.last_activity = last_activity;
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
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT mint, priority, fetch_interval_seconds, last_activity, consecutive_empty_fetches, last_pool_discovery_attempt, consecutive_pool_failures
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
                let last_activity_str: String = row.get(3)?;
                let last_activity = DateTime::parse_from_rfc3339(&last_activity_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let consecutive_empty: u32 = row.get(4)?;
                let last_pool_attempt: Option<i64> = row.get(5)?;
                let pool_failures: u32 = row.get(6)?;

                let mut config = TokenOhlcvConfig::new(mint, priority);
                config.fetch_frequency = std::time::Duration::from_secs(fetch_secs as u64);
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

    // ==================== Cleanup ====================

    pub fn cleanup_old_data(&self, retention_days: i64) -> OhlcvResult<usize> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let cutoff = (Utc::now() - Duration::days(retention_days)).to_rfc3339();

        let deleted = conn
            .execute("DELETE FROM ohlcv_1m WHERE created_at < ?1", params![cutoff])
            .map_err(|e| OhlcvError::DatabaseError(format!("Cleanup failed: {}", e)))?;

        // Also clean aggregated cache
        conn
            .execute("DELETE FROM ohlcv_aggregated WHERE created_at < ?1", params![cutoff])
            .map_err(|e| OhlcvError::DatabaseError(format!("Cleanup failed: {}", e)))?;

        Ok(deleted)
    }

    // ==================== Metrics ====================

    pub fn get_data_point_count(&self) -> OhlcvResult<usize> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_1m", params![], |row| row.get(0))
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }

    pub fn has_data_for_mint(&self, mint: &str) -> OhlcvResult<bool> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let exists: i64 = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM ohlcv_1m WHERE mint = ?1 LIMIT 1)",
                params![mint],
                |row| row.get(0)
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(exists != 0)
    }

    pub fn get_pool_count(&self) -> OhlcvResult<usize> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_pools", params![], |row| { row.get(0) })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }

    pub fn get_token_count(&self) -> OhlcvResult<usize> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT mint) FROM ohlcv_monitor_config WHERE is_active = 1",
                params![],
                |row| row.get(0)
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }

    pub fn get_gap_count(&self, filled: bool) -> OhlcvResult<usize> {
        let conn = self.conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {}", e)))?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_gaps WHERE filled = ?1",
                params![filled as i32],
                |row| row.get(0)
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {}", e)))?;

        Ok(count as usize)
    }
}
