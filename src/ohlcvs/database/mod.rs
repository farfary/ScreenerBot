//! OHLCV database — SQLite persistence for candlestick data and gap tracking.

mod candles;
mod config;
mod gaps;
mod maintenance;
pub mod types;

pub use types::{ClearAllResult, DatabaseStats, DeleteResult, OhlcvTokenStatus};

use crate::ohlcvs::types::{OhlcvError, OhlcvResult, PoolConfig};
use crate::{chains::ChainId, database};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
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
///   2 — 2026-07: backfill now requests full depth (`max_backfill_candles` = 1000
///       per timeframe, was per-tf ~30-day caps) and the data server deep-pages
///       history backward. Existing caches capped at ~30 days had their backfill
///       flags marked complete, so they would never re-deepen — wipe them so
///       every token re-pulls the now-deep coarse-frame series.
///   3 — 2026-07: candle timestamps are now snapped to the canonical UTC bucket
///       at ingest. Providers disagreed on the 12h anchor (GeckoTerminal phases
///       12h at +10h, ts % 43200 == 36000; others use the midnight grid), so the
///       stored 12h series interleaved two grids ~2h apart and rendered corrupt.
///       Wipe so every token re-pulls onto the single normalized grid.
///   4 — 2026-07: empty no-trade candles (volume == 0) are no longer recorded,
///       and reads/status are scoped to the single resolved pool (no cross-pool
///       combining). Wipe so existing zero-volume rows and any stale other-pool
///       candles are cleared and re-pulled clean.
const OHLCV_DATA_VERSION: i64 = 4;
const CHAIN_SCOPE_MIGRATION: &str = "20260821_chain_scope";

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
        ensure_data_version(&conn)?;

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

fn migrate_chain_scope(conn: &Connection) -> OhlcvResult<()> {
    let applied =
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE migration_id = ?1)",
            params![CHAIN_SCOPE_MIGRATION],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to inspect OHLCV migrations: {e}"))
        })? != 0;
    if applied {
        return Ok(());
    }

    let pools_scoped = table_has_column(conn, "ohlcv_pools", "chain_id")?;
    let candles_scoped = table_has_column(conn, "ohlcv_candles", "chain_id")?;
    let gaps_scoped = table_has_column(conn, "ohlcv_gaps", "chain_id")?;
    let config_scoped = table_has_column(conn, "ohlcv_monitor_config", "chain_id")?;
    if pools_scoped && candles_scoped && gaps_scoped && config_scoped {
        conn.execute(
            "INSERT INTO schema_migrations (migration_id) VALUES (?1)",
            params![CHAIN_SCOPE_MIGRATION],
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to record OHLCV migration: {e}")))?;
        return Ok(());
    }

    let legacy_counts: [i64; 4] = [
        "ohlcv_pools",
        "ohlcv_candles",
        "ohlcv_gaps",
        "ohlcv_monitor_config",
    ]
    .map(|table| {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
    })
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| OhlcvError::DatabaseError(format!("Failed to count legacy OHLCV rows: {e}")))?
    .try_into()
    .expect("fixed OHLCV migration table list");

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to start OHLCV migration: {e}")))?;
    tx.execute_batch(r#"
        ALTER TABLE ohlcv_pools RENAME TO ohlcv_pools_legacy;
        ALTER TABLE ohlcv_candles RENAME TO ohlcv_candles_legacy;
        ALTER TABLE ohlcv_gaps RENAME TO ohlcv_gaps_legacy;
        ALTER TABLE ohlcv_monitor_config RENAME TO ohlcv_monitor_config_legacy;
        CREATE TABLE ohlcv_pools (id INTEGER PRIMARY KEY AUTOINCREMENT, chain_id TEXT NOT NULL, mint TEXT NOT NULL, pool_address TEXT NOT NULL, dex TEXT NOT NULL, liquidity REAL NOT NULL DEFAULT 0.0, is_default INTEGER NOT NULL DEFAULT 0, is_sol_pair INTEGER NOT NULL DEFAULT 1, last_success TEXT, failure_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(chain_id, mint, pool_address));
        CREATE TABLE ohlcv_candles (id INTEGER PRIMARY KEY AUTOINCREMENT, chain_id TEXT NOT NULL, mint TEXT NOT NULL, pool_address TEXT NOT NULL, timeframe TEXT NOT NULL, timestamp INTEGER NOT NULL, open REAL NOT NULL, high REAL NOT NULL, low REAL NOT NULL, close REAL NOT NULL, volume REAL NOT NULL, source TEXT NOT NULL DEFAULT 'api', fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(chain_id, mint, pool_address, timeframe, timestamp));
        CREATE TABLE ohlcv_gaps (id INTEGER PRIMARY KEY AUTOINCREMENT, chain_id TEXT NOT NULL, mint TEXT NOT NULL, pool_address TEXT NOT NULL, timeframe TEXT NOT NULL, start_timestamp INTEGER NOT NULL, end_timestamp INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, last_attempt TEXT, filled INTEGER NOT NULL DEFAULT 0, error_message TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp));
        CREATE TABLE ohlcv_monitor_config (chain_id TEXT NOT NULL, mint TEXT NOT NULL, priority TEXT NOT NULL, fetch_interval_seconds INTEGER NOT NULL DEFAULT 60, source TEXT NOT NULL DEFAULT 'manual', is_active INTEGER NOT NULL DEFAULT 1, backfill_1m_complete INTEGER NOT NULL DEFAULT 0, backfill_5m_complete INTEGER NOT NULL DEFAULT 0, backfill_15m_complete INTEGER NOT NULL DEFAULT 0, backfill_1h_complete INTEGER NOT NULL DEFAULT 0, backfill_4h_complete INTEGER NOT NULL DEFAULT 0, backfill_12h_complete INTEGER NOT NULL DEFAULT 0, backfill_1d_complete INTEGER NOT NULL DEFAULT 0, backfill_started_at TEXT, backfill_completed_at TEXT, last_fetch TEXT, last_activity TEXT NOT NULL, consecutive_empty_fetches INTEGER NOT NULL DEFAULT 0, last_pool_discovery_attempt INTEGER, consecutive_pool_failures INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, PRIMARY KEY (chain_id, mint));
        INSERT INTO ohlcv_pools (chain_id,mint,pool_address,dex,liquidity,is_default,is_sol_pair,last_success,failure_count,created_at) SELECT 'solana',mint,pool_address,dex,liquidity,is_default,is_sol_pair,last_success,failure_count,created_at FROM ohlcv_pools_legacy;
        INSERT INTO ohlcv_candles (chain_id,mint,pool_address,timeframe,timestamp,open,high,low,close,volume,source,fetched_at) SELECT 'solana',mint,pool_address,timeframe,timestamp,open,high,low,close,volume,source,fetched_at FROM ohlcv_candles_legacy;
        INSERT INTO ohlcv_gaps (chain_id,mint,pool_address,timeframe,start_timestamp,end_timestamp,attempts,last_attempt,filled,error_message,created_at) SELECT 'solana',mint,pool_address,timeframe,start_timestamp,end_timestamp,attempts,last_attempt,filled,error_message,created_at FROM ohlcv_gaps_legacy;
        INSERT INTO ohlcv_monitor_config (chain_id,mint,priority,fetch_interval_seconds,source,is_active,backfill_1m_complete,backfill_5m_complete,backfill_15m_complete,backfill_1h_complete,backfill_4h_complete,backfill_12h_complete,backfill_1d_complete,backfill_started_at,backfill_completed_at,last_fetch,last_activity,consecutive_empty_fetches,last_pool_discovery_attempt,consecutive_pool_failures,created_at,updated_at) SELECT 'solana',mint,priority,fetch_interval_seconds,source,is_active,backfill_1m_complete,backfill_5m_complete,backfill_15m_complete,backfill_1h_complete,backfill_4h_complete,backfill_12h_complete,backfill_1d_complete,backfill_started_at,backfill_completed_at,last_fetch,last_activity,consecutive_empty_fetches,last_pool_discovery_attempt,consecutive_pool_failures,created_at,updated_at FROM ohlcv_monitor_config_legacy;
        DROP TABLE ohlcv_pools_legacy; DROP TABLE ohlcv_candles_legacy; DROP TABLE ohlcv_gaps_legacy; DROP TABLE ohlcv_monitor_config_legacy;
        CREATE INDEX idx_pools_mint ON ohlcv_pools(chain_id, mint); CREATE INDEX idx_pools_default ON ohlcv_pools(chain_id, mint, is_default);
        CREATE INDEX idx_candles_lookup ON ohlcv_candles(chain_id, mint, timeframe, timestamp DESC); CREATE INDEX idx_candles_pool_lookup ON ohlcv_candles(chain_id, pool_address, timeframe, timestamp DESC); CREATE INDEX idx_candles_cleanup ON ohlcv_candles(fetched_at);
        CREATE INDEX idx_gaps_unfilled ON ohlcv_gaps(chain_id, filled, mint, timeframe); CREATE INDEX idx_gaps_retry ON ohlcv_gaps(chain_id, filled, attempts, last_attempt);
        CREATE INDEX idx_monitor_active ON ohlcv_monitor_config(chain_id, is_active, priority); CREATE INDEX idx_monitor_backfill ON ohlcv_monitor_config(chain_id, is_active, backfill_1d_complete);
    "#).map_err(|e| OhlcvError::DatabaseError(format!("Failed to migrate OHLCV tables: {e}")))?;
    for (table, expected) in [
        "ohlcv_pools",
        "ohlcv_candles",
        "ohlcv_gaps",
        "ohlcv_monitor_config",
    ]
    .into_iter()
    .zip(legacy_counts)
    {
        let actual: i64 = tx
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE chain_id = 'solana'"),
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to verify migrated {table}: {e}"))
            })?;
        if actual != expected {
            return Err(OhlcvError::DatabaseError(format!(
                "OHLCV migration row count mismatch for {table}: expected {expected}, got {actual}"
            )));
        }
    }
    {
        let mut fk = tx.prepare("PRAGMA foreign_key_check").map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to verify OHLCV foreign keys: {e}"))
        })?;
        if fk
            .query([])
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to run OHLCV foreign key check: {e}"))
            })?
            .next()
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to read OHLCV foreign key check: {e}"))
            })?
            .is_some()
        {
            return Err(OhlcvError::DatabaseError(
                "OHLCV foreign key check failed after migration".to_owned(),
            ));
        }
    }
    tx.execute(
        "INSERT INTO schema_migrations (migration_id) VALUES (?1)",
        params![CHAIN_SCOPE_MIGRATION],
    )
    .map_err(|e| OhlcvError::DatabaseError(format!("Failed to record OHLCV migration: {e}")))?;
    tx.commit()
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to commit OHLCV migration: {e}")))
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> OhlcvResult<bool> {
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

fn create_chain_indexes(conn: &Connection) -> OhlcvResult<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_pools_mint ON ohlcv_pools(chain_id, mint);
         CREATE INDEX IF NOT EXISTS idx_pools_default ON ohlcv_pools(chain_id, mint, is_default);
         CREATE INDEX IF NOT EXISTS idx_candles_lookup ON ohlcv_candles(chain_id, mint, timeframe, timestamp DESC);
         CREATE INDEX IF NOT EXISTS idx_candles_pool_lookup ON ohlcv_candles(chain_id, pool_address, timeframe, timestamp DESC);
         CREATE INDEX IF NOT EXISTS idx_candles_cleanup ON ohlcv_candles(fetched_at);
         CREATE INDEX IF NOT EXISTS idx_gaps_unfilled ON ohlcv_gaps(chain_id, filled, mint, timeframe);
         CREATE INDEX IF NOT EXISTS idx_gaps_retry ON ohlcv_gaps(chain_id, filled, attempts, last_attempt);
         CREATE INDEX IF NOT EXISTS idx_monitor_active ON ohlcv_monitor_config(chain_id, is_active, priority);
         CREATE INDEX IF NOT EXISTS idx_monitor_backfill ON ohlcv_monitor_config(chain_id, is_active, backfill_1d_complete);",
    )
    .map_err(|e| OhlcvError::DatabaseError(format!("Failed to create OHLCV indexes: {e}")))
}

fn ensure_data_version(conn: &Connection) -> OhlcvResult<()> {
    let version_table_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'ohlcv_data_versions')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to inspect OHLCV data version table: {e}")))?
        != 0;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS ohlcv_data_versions (id INTEGER PRIMARY KEY CHECK (id = 1), version INTEGER NOT NULL)")
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to initialize OHLCV data version: {e}")))?;
    let stored = conn
        .query_row(
            "SELECT version FROM ohlcv_data_versions WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to read OHLCV data version: {e}"))
        })?;
    if stored.is_none() && !version_table_exists {
        let legacy_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to read legacy OHLCV data version: {e}"))
            })?;
        if legacy_version == OHLCV_DATA_VERSION {
            conn.execute(
                "INSERT INTO ohlcv_data_versions (id, version) VALUES (1, ?1)",
                params![legacy_version],
            )
            .map_err(|e| {
                OhlcvError::DatabaseError(format!("Failed to seed OHLCV data version: {e}"))
            })?;
            return Ok(());
        }
    }
    if stored != Some(OHLCV_DATA_VERSION) {
        wipe_candle_data(conn, ChainId::Solana.as_str())
            .map_err(|e| OhlcvError::DatabaseError(format!("Failed to wipe candle data: {e}")))?;
        conn.execute("INSERT INTO ohlcv_data_versions (id, version) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET version = excluded.version", params![OHLCV_DATA_VERSION]).map_err(|e| OhlcvError::DatabaseError(format!("Failed to update OHLCV data version: {e}")))?;
    }
    Ok(())
}

/// Delete all candles + gaps and reset every monitor row's backfill progress so
/// the token re-backfills from scratch on the next scheduler pass. Operates on an
/// already-locked connection so it can be shared by both the constructor's
/// version wipe and the public `clear_all_ohlcv_data` (which locks first).
fn wipe_candle_data(conn: &Connection, chain_id: &str) -> SqliteResult<ClearAllResult> {
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
    use super::*;
    use crate::ohlcvs::types::{Candle, Timeframe};

    fn test_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "screenerbot-ohlcv-{label}-{}.db",
            std::process::id()
        ))
    }

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

    #[test]
    fn schema_migration_ledger_is_idempotent() {
        let path = test_path("ledger");
        let _ = std::fs::remove_file(&path);
        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        let count: i64 = db
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE migration_id = ?1",
                params![CHAIN_SCOPE_MIGRATION],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        drop(db);
        let reopened = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        let count: i64 = reopened
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE migration_id = ?1",
                params![CHAIN_SCOPE_MIGRATION],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_schema_migrates_to_solana_without_wiping_current_data() {
        let path = test_path("legacy");
        let _ = std::fs::remove_file(&path);
        let legacy = Connection::open(&path).unwrap();
        legacy.execute_batch(
            "CREATE TABLE ohlcv_pools (id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL, dex TEXT NOT NULL, liquidity REAL NOT NULL DEFAULT 0.0, is_default INTEGER NOT NULL DEFAULT 0, last_success TEXT, failure_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(mint, pool_address));
             CREATE TABLE ohlcv_candles (id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL, timeframe TEXT NOT NULL, timestamp INTEGER NOT NULL, open REAL NOT NULL, high REAL NOT NULL, low REAL NOT NULL, close REAL NOT NULL, volume REAL NOT NULL, source TEXT NOT NULL DEFAULT 'api', fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(mint, pool_address, timeframe, timestamp));
             CREATE TABLE ohlcv_gaps (id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL, timeframe TEXT NOT NULL, start_timestamp INTEGER NOT NULL, end_timestamp INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, last_attempt TEXT, filled INTEGER NOT NULL DEFAULT 0, error_message TEXT, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(mint, pool_address, timeframe, start_timestamp, end_timestamp));
             CREATE TABLE ohlcv_monitor_config (mint TEXT PRIMARY KEY, priority TEXT NOT NULL, fetch_interval_seconds INTEGER NOT NULL DEFAULT 60, source TEXT NOT NULL DEFAULT 'manual', is_active INTEGER NOT NULL DEFAULT 1, backfill_1m_complete INTEGER NOT NULL DEFAULT 0, backfill_5m_complete INTEGER NOT NULL DEFAULT 0, backfill_15m_complete INTEGER NOT NULL DEFAULT 0, backfill_1h_complete INTEGER NOT NULL DEFAULT 0, backfill_4h_complete INTEGER NOT NULL DEFAULT 0, backfill_12h_complete INTEGER NOT NULL DEFAULT 0, backfill_1d_complete INTEGER NOT NULL DEFAULT 0, backfill_started_at TEXT, backfill_completed_at TEXT, last_fetch TEXT, last_activity TEXT NOT NULL, consecutive_empty_fetches INTEGER NOT NULL DEFAULT 0, last_pool_discovery_attempt INTEGER, consecutive_pool_failures INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);",
        ).unwrap();
        legacy.execute("INSERT INTO ohlcv_pools (mint, pool_address, dex, liquidity) VALUES ('mint', 'pool', 'dex', 1)", []).unwrap();
        legacy.execute("INSERT INTO ohlcv_candles (mint, pool_address, timeframe, timestamp, open, high, low, close, volume) VALUES ('mint', 'pool', '1m', 60, 1, 1, 1, 1, 1)", []).unwrap();
        legacy.execute("INSERT INTO ohlcv_gaps (mint, pool_address, timeframe, start_timestamp, end_timestamp) VALUES ('mint', 'pool', '1m', 120, 180)", []).unwrap();
        legacy.execute("INSERT INTO ohlcv_monitor_config (mint, priority, last_activity) VALUES ('mint', 'high', CURRENT_TIMESTAMP)", []).unwrap();
        legacy.pragma_update(None, "user_version", 4).unwrap();
        drop(legacy);

        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        assert_eq!(db.get_pools("mint").unwrap().len(), 1);
        assert_eq!(
            db.get_candles("mint", Some("pool"), Timeframe::Minute1, None, None, None)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.get_unfilled_gaps("mint", Timeframe::Minute1)
                .unwrap()
                .len(),
            1
        );
        assert!(db.get_monitor_config("mint").unwrap().is_some());
        let conn = db.conn.lock().unwrap();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let migrated_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_candles WHERE chain_id = 'solana'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(user_version, 4);
        assert_eq!(migrated_rows, 1);
        drop(conn);
        drop(db);
        let reopened = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        assert_eq!(
            reopened
                .get_candles("mint", Some("pool"), Timeframe::Minute1, None, None, None)
                .unwrap()
                .len(),
            1
        );
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }
}
