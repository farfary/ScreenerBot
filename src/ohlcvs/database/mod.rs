//! OHLCV database — SQLite persistence for candlestick data and gap tracking.

mod candles;
mod config;
mod gaps;
mod maintenance;
pub mod types;

pub use types::{ClearAllResult, DatabaseStats, DeleteResult, OhlcvTokenStatus};

use crate::database;
use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Version of the OHLCV candle data logic. Bump this whenever a change to how
/// candles are fetched/stored (locally or by the data server) means the existing
/// cached candles are no longer trustworthy and should be re-fetched. On startup
/// a stored version that differs from this constant triggers a one-time wipe of
/// the local candle/gap data so every monitored token re-backfills with the
/// current logic — self-healing across app restarts for every user, no manual
/// cache clearing required.
///
/// Pool rows and the monitoring list (which tokens to watch) are preserved; only
/// the candle data and gap tracking are cleared and backfill progress is reset.
///
/// Changelog:
///   1 — 2026-07: data-server `fetch_limit` now bridges interior gaps in one
///       fetch (was a fixed refresh window that left permanent holes on cold
///       tokens); wipe stale local caches so they re-pull the healed series.
const OHLCV_DATA_VERSION: i64 = 1;

pub struct OhlcvDatabase {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

impl OhlcvDatabase {
    /// Initialize the database and create tables
    pub fn new<P: AsRef<Path>>(path: P) -> OhlcvResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to open database: {e}")))?;

        // Apply centralized PRAGMA configuration
        database::configure_connection(&conn, database::OHLCVS_DB).map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to configure connection: {e}"))
        })?;

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
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

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
                is_sol_pair INTEGER NOT NULL DEFAULT 1,
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
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to create tables: {e}")))?;

        // Migration: add is_sol_pair to pre-existing ohlcv_pools tables (CREATE IF
        // NOT EXISTS above won't add a column to a table that already exists). Errs
        // harmlessly (duplicate column) once migrated, so ignore the result.
        let _ = conn.execute(
            "ALTER TABLE ohlcv_pools ADD COLUMN is_sol_pair INTEGER NOT NULL DEFAULT 1",
            [],
        );

        // One-time wipe of stale candle caches when the OHLCV data logic version
        // changed (uses SQLite's built-in user_version slot — no extra schema).
        let stored: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to read user_version: {e}")))?;
        if stored != OHLCV_DATA_VERSION {
            let result = wipe_candle_data(&conn).map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to wipe candle data: {e}"))
            })?;
            conn.pragma_update(None, "user_version", OHLCV_DATA_VERSION)
                .map_err(|e| {
                    OhlcvError::DatabaseError(format!("Failed to set user_version: {e}"))
                })?;
            tracing::warn!(
                from = stored,
                to = OHLCV_DATA_VERSION,
                candles_deleted = result.candles_deleted,
                gaps_deleted = result.gaps_deleted,
                tokens_reset = result.tokens_reset,
                "OHLCV data version changed; wiped local candle caches for re-fetch"
            );
        }

        Ok(())
    }

    /// Clear ALL cached OHLCV candles and gaps and reset every token's backfill
    /// progress so monitored tokens re-fetch from scratch. Pools and the
    /// monitoring list are preserved. Backs both the manual "Clear OHLCV Cache"
    /// action and the automatic data-version wipe.
    pub fn clear_all_ohlcv_data(&self) -> OhlcvResult<ClearAllResult> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;
        wipe_candle_data(&conn)
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to clear OHLCV data: {e}")))
    }

    // ==================== Pool Management ====================

    pub fn upsert_pool(&self, mint: &str, pool: &PoolConfig) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let last_success = pool.last_successful_fetch.map(|dt| dt.to_rfc3339());

        conn
            .execute(
                "INSERT INTO ohlcv_pools (mint, pool_address, dex, liquidity, is_default, is_sol_pair, last_success, failure_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(mint, pool_address) DO UPDATE SET
                liquidity = excluded.liquidity,
                is_default = excluded.is_default,
                is_sol_pair = excluded.is_sol_pair,
                last_success = excluded.last_success,
                failure_count = excluded.failure_count",
                params![
                    mint,
                    &pool.address,
                    &pool.dex,
                    pool.liquidity,
                    pool.is_default as i32,
                    pool.is_sol_pair as i32,
                    last_success,
                    pool.failure_count
                ]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to upsert pool: {e}")))?;

        Ok(())
    }

    pub fn delete_pool(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn.execute(
            "DELETE FROM ohlcv_pools WHERE mint = ?1 AND pool_address = ?2",
            params![mint, pool_address],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to delete pool: {e}")))?;

        Ok(())
    }

    pub fn get_pools(&self, mint: &str) -> OhlcvResult<Vec<PoolConfig>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT pool_address, dex, liquidity, is_default, last_success, failure_count, is_sol_pair
                 FROM ohlcv_pools
                 WHERE mint = ?1
                 ORDER BY liquidity DESC",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare statement: {e}")))?;

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
                    is_sol_pair: row.get::<_, i32>(6)? != 0,
                })
            })
            .map_err(|e| OhlcvError::DatabaseError(format!("Query failed: {e}")))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to collect results: {e}")))?;

        Ok(pools)
    }

    pub fn mark_pool_failure(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn
            .execute(
                "UPDATE ohlcv_pools SET failure_count = failure_count + 1 WHERE mint = ?1 AND pool_address = ?2",
                params![mint, pool_address]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark failure: {e}")))?;

        Ok(())
    }

    pub fn mark_pool_success(&self, mint: &str, pool_address: &str) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn
            .execute(
                "UPDATE ohlcv_pools SET failure_count = 0, last_success = ?1 WHERE mint = ?2 AND pool_address = ?3",
                params![Utc::now().to_rfc3339(), mint, pool_address]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark success: {e}")))?;

        Ok(())
    }
}

/// Delete all candles + gaps and reset every monitor row's backfill progress so
/// the token re-backfills from scratch on the next scheduler pass. Operates on an
/// already-locked connection so it can be shared by both the constructor's
/// version wipe and the public `clear_all_ohlcv_data` (which locks first).
fn wipe_candle_data(conn: &Connection) -> SqliteResult<ClearAllResult> {
    let candles_deleted = conn.execute("DELETE FROM ohlcv_candles", [])?;
    let gaps_deleted = conn.execute("DELETE FROM ohlcv_gaps", [])?;
    // Reset backfill progress + activity counters so every monitored token is
    // treated as never-fetched and re-pulls its full series. The monitor rows
    // themselves (and pools) stay so we still know which tokens to watch.
    let tokens_reset = conn.execute(
        "UPDATE ohlcv_monitor_config SET
            backfill_1m_complete = 0,
            backfill_5m_complete = 0,
            backfill_15m_complete = 0,
            backfill_1h_complete = 0,
            backfill_4h_complete = 0,
            backfill_12h_complete = 0,
            backfill_1d_complete = 0,
            backfill_started_at = NULL,
            backfill_completed_at = NULL,
            last_fetch = NULL,
            consecutive_empty_fetches = 0,
            updated_at = CURRENT_TIMESTAMP",
        [],
    )?;
    Ok(ClearAllResult {
        candles_deleted,
        gaps_deleted,
        tokens_reset,
    })
}
