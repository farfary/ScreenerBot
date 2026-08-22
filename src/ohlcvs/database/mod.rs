//! OHLCV database — SQLite persistence for candlestick data and gap tracking.

mod candles;
mod config;
mod data_version;
mod gaps;
mod maintenance;
mod migrations;
pub mod types;

pub use types::{ClearAllResult, DatabaseStats, DeleteResult, OhlcvTokenStatus};

use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig};
use crate::{chains::ChainId, database};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

use migrations::{create_chain_indexes, migrate_chain_scope};

pub struct OhlcvDatabase {
    pub(crate) conn: Arc<Mutex<Connection>>,
    pub(crate) chain: ChainId,
}

impl OhlcvDatabase {
    /// Initialize the database and create tables
    pub fn new<P: AsRef<Path>>(path: P, chain: ChainId) -> OhlcvResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to open database: {e}")))?;

        // Apply centralized PRAGMA configuration
        database::configure_connection(&conn, database::OHLCVS_DB).map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to configure connection: {e}"))
        })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            chain,
        };

        db.create_tables()?;
        Ok(db)
    }

    pub(crate) const fn chain(&self) -> ChainId {
        self.chain
    }

    pub(crate) const fn chain_id(&self) -> &'static str {
        self.chain.as_str()
    }

    fn create_tables(&self) -> OhlcvResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OhlcvError::DatabaseError(format!("Lock error: {e}")))?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                migration_id TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            -- Pool configurations
            CREATE TABLE IF NOT EXISTS ohlcv_pools (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                dex TEXT NOT NULL,
                liquidity REAL NOT NULL DEFAULT 0.0,
                is_default INTEGER NOT NULL DEFAULT 0,
                is_sol_pair INTEGER NOT NULL DEFAULT 1,
                last_success TEXT,
                failure_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(chain_id, mint, pool_address)
            );

            -- UNIFIED CANDLES TABLE (stores ALL native timeframes from API)
            -- Replaces ohlcv_1m and ohlcv_aggregated with single storage
            CREATE TABLE IF NOT EXISTS ohlcv_candles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL,
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
                UNIQUE(chain_id, mint, pool_address, timeframe, timestamp)
            );

            -- Gap tracking (per timeframe)
            CREATE TABLE IF NOT EXISTS ohlcv_gaps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL,
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
                UNIQUE(chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
            );

            -- Token monitoring configuration (with backfill tracking)
            CREATE TABLE IF NOT EXISTS ohlcv_monitor_config (
                chain_id TEXT NOT NULL,
                mint TEXT NOT NULL,
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
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (chain_id, mint)
            );
            "#,
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to create tables: {e}")))?;

        // Migration: add is_sol_pair to pre-existing ohlcv_pools tables (CREATE IF
        // NOT EXISTS above won't add a column to a table that already exists). Errs
        // harmlessly (duplicate column) once migrated, so ignore the result.
        let _ = conn.execute(
            "ALTER TABLE ohlcv_pools ADD COLUMN is_sol_pair INTEGER NOT NULL DEFAULT 1",
            [],
        );

        migrate_chain_scope(&conn)?;
        create_chain_indexes(&conn)?;
        data_version::ensure_data_version(&conn, self.chain_id())?;

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
        wipe_candle_data(&conn, self.chain_id())
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
                "INSERT INTO ohlcv_pools (chain_id, mint, pool_address, dex, liquidity, is_default, is_sol_pair, last_success, failure_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(chain_id, mint, pool_address) DO UPDATE SET
                liquidity = excluded.liquidity,
                is_default = excluded.is_default,
                is_sol_pair = excluded.is_sol_pair,
                last_success = excluded.last_success,
                failure_count = excluded.failure_count",
                params![
                    self.chain_id(), mint,
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
            "DELETE FROM ohlcv_pools WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3",
            params![self.chain_id(), mint, pool_address],
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
                 WHERE chain_id = ?1 AND mint = ?2
                 ORDER BY liquidity DESC",
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to prepare statement: {e}")))?;

        let pools = stmt
            .query_map(params![self.chain_id(), mint], |row| {
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
                "UPDATE ohlcv_pools SET failure_count = failure_count + 1 WHERE chain_id = ?1 AND mint = ?2 AND pool_address = ?3",
                params![self.chain_id(), mint, pool_address]
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
                "UPDATE ohlcv_pools SET failure_count = 0, last_success = ?1 WHERE chain_id = ?2 AND mint = ?3 AND pool_address = ?4",
                params![Utc::now().to_rfc3339(), self.chain_id(), mint, pool_address]
            )
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to mark success: {e}")))?;

        Ok(())
    }
}

pub(super) fn table_has_column(conn: &Connection, table: &str, column: &str) -> OhlcvResult<bool> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to inspect {table}: {e}")))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to inspect {table}: {e}")))?
        .collect::<SqliteResult<Vec<_>>>()
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to inspect {table}: {e}")))?;
    Ok(columns.iter().any(|name| name == column))
}

/// Delete all candles + gaps and reset every monitor row's backfill progress so
/// the token re-backfills from scratch on the next scheduler pass. Operates on an
/// already-locked connection so it can be shared by both the constructor's
/// version wipe and the public `clear_all_ohlcv_data` (which locks first).
pub(super) fn wipe_candle_data(conn: &Connection, chain_id: &str) -> SqliteResult<ClearAllResult> {
    let candles_deleted = conn.execute(
        "DELETE FROM ohlcv_candles WHERE chain_id = ?1",
        params![chain_id],
    )?;
    let gaps_deleted = conn.execute(
        "DELETE FROM ohlcv_gaps WHERE chain_id = ?1",
        params![chain_id],
    )?;
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
            updated_at = CURRENT_TIMESTAMP
         WHERE chain_id = ?1",
        params![chain_id],
    )?;
    Ok(ClearAllResult {
        candles_deleted,
        gaps_deleted,
        tokens_reset,
    })
}

#[cfg(test)]
mod tests {
    use super::migrations::test_path;
    use super::*;
    use crate::ohlcvs::types::{Candle, Timeframe};

    #[test]
    fn chain_bound_database_ignores_raw_foreign_rows_and_keeps_user_version() {
        let path = test_path("foreign");
        let _ = std::fs::remove_file(&path);
        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        db.insert_candles_batch(
            "mint",
            "pool",
            Timeframe::Minute1,
            &[Candle::new(60, 1.0, 1.0, 1.0, 1.0, 1.0)],
            "test",
        )
        .unwrap();
        let conn = db.conn.lock().unwrap();
        conn.execute("INSERT INTO ohlcv_candles (chain_id, mint, pool_address, timeframe, timestamp, open, high, low, close, volume, source) VALUES ('foreign', 'mint', 'pool', '1m', 120, 2, 2, 2, 2, 1, 'test')", []).unwrap();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, 0);
        drop(conn);
        assert_eq!(
            db.get_candles("mint", Some("pool"), Timeframe::Minute1, None, None, None)
                .unwrap()
                .len(),
            1
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
