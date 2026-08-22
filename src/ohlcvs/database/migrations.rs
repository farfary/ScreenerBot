//! OHLCV schema migrations — the chain-scope upgrade run by
//! `OhlcvDatabase::create_tables`.

use rusqlite::{params, Connection};

use crate::ohlcvs::types::{OhlcvError, OhlcvResult};

use super::table_has_column;

const CHAIN_SCOPE_MIGRATION: &str = "20260821_chain_scope";

pub(super) fn migrate_chain_scope(conn: &Connection) -> OhlcvResult<()> {
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

pub(super) fn create_chain_indexes(conn: &Connection) -> OhlcvResult<()> {
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

#[cfg(test)]
pub(super) fn test_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "screenerbot-ohlcv-{label}-{}.db",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;
    use crate::ohlcvs::database::OhlcvDatabase;
    use crate::ohlcvs::types::Timeframe;

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
