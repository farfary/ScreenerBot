//! Chain-qualified OHLCV candle-data version ownership.
//!
//! Cached candles are only trustworthy for the chain whose ingest rules produced
//! them. A stale version therefore wipes that chain's candle/gap rows only.
//! The historical singleton row (`id = 1`) is migrated losslessly onto Solana.

use super::{table_has_column, wipe_candle_data};
use crate::chains::ChainId;
use crate::ohlcvs::types::{OhlcvError, OhlcvResult};
use rusqlite::{params, Connection, OptionalExtension};

/// Version of the OHLCV candle data logic. Bump this whenever a change to how
/// candles are fetched/stored (locally or by the data server) means the existing
/// cached candles are no longer trustworthy and should be re-fetched. On startup
/// a stored version that differs from this constant triggers a one-time wipe of
/// the owning chain's local candle/gap data so every monitored token re-backfills
/// with the current logic — self-healing across app restarts for every user, no
/// manual cache clearing required.
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
pub(super) const OHLCV_DATA_VERSION: i64 = 4;

const CREATE_CHAIN_VERSIONS: &str = "CREATE TABLE IF NOT EXISTS ohlcv_data_versions (
    chain_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL
)";

/// Ensure the data-version table is chain-keyed and that `chain_id`'s stored
/// version matches `OHLCV_DATA_VERSION`. A stale or missing version wipes only
/// that chain. The historical global `id = 1` row is copied onto Solana without
/// changing its version meaning.
pub(super) fn ensure_data_version(conn: &Connection, chain_id: &str) -> OhlcvResult<()> {
    migrate_global_data_version_table(conn)?;
    conn.execute_batch(CREATE_CHAIN_VERSIONS).map_err(|e| {
        OhlcvError::DatabaseError(format!("Failed to initialize OHLCV data version: {e}"))
    })?;

    let stored = conn
        .query_row(
            "SELECT version FROM ohlcv_data_versions WHERE chain_id = ?1",
            params![chain_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to read OHLCV data version: {e}"))
        })?;
    if stored == Some(OHLCV_DATA_VERSION) {
        return Ok(());
    }

    if stored.is_none()
        && chain_id == ChainId::Solana.as_str()
        && inherit_solana_user_version(conn)?
    {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction().map_err(|e| {
        OhlcvError::DatabaseError(format!("Failed to begin OHLCV data-version update: {e}"))
    })?;
    wipe_candle_data(&transaction, chain_id)
        .map_err(|e| OhlcvError::DatabaseError(format!("Failed to wipe candle data: {e}")))?;
    transaction
        .execute(
            "INSERT INTO ohlcv_data_versions (chain_id, version) VALUES (?1, ?2)
             ON CONFLICT(chain_id) DO UPDATE SET version = excluded.version",
            params![chain_id, OHLCV_DATA_VERSION],
        )
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to update OHLCV data version: {e}"))
        })?;
    transaction.commit().map_err(|e| {
        OhlcvError::DatabaseError(format!("Failed to commit OHLCV data-version update: {e}"))
    })
}

fn inherit_solana_user_version(conn: &Connection) -> OhlcvResult<bool> {
    let legacy_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to read legacy OHLCV data version: {e}"))
        })?;
    if legacy_version != OHLCV_DATA_VERSION {
        return Ok(false);
    }
    conn.execute(
        "INSERT INTO ohlcv_data_versions (chain_id, version) VALUES (?1, ?2)",
        params![ChainId::Solana.as_str(), legacy_version],
    )
    .map_err(|e| OhlcvError::DatabaseError(format!("Failed to seed OHLCV data version: {e}")))?;
    Ok(true)
}

fn migrate_global_data_version_table(conn: &Connection) -> OhlcvResult<()> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'ohlcv_data_versions')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to inspect OHLCV data version table: {e}"))
        })?
        != 0;
    if !exists || table_has_column(conn, "ohlcv_data_versions", "chain_id")? {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction().map_err(|e| {
        OhlcvError::DatabaseError(format!(
            "Failed to begin OHLCV data-version table migration: {e}"
        ))
    })?;
    transaction
        .execute_batch(
            "ALTER TABLE ohlcv_data_versions RENAME TO ohlcv_data_versions_legacy_global;
             CREATE TABLE ohlcv_data_versions (
                 chain_id TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );
             INSERT INTO ohlcv_data_versions (chain_id, version)
                 SELECT 'solana', version FROM ohlcv_data_versions_legacy_global WHERE id = 1;
             DROP TABLE ohlcv_data_versions_legacy_global;",
        )
        .map_err(|e| {
            OhlcvError::DatabaseError(format!("Failed to migrate OHLCV data-version table: {e}"))
        })?;
    transaction.commit().map_err(|e| {
        OhlcvError::DatabaseError(format!(
            "Failed to commit OHLCV data-version table migration: {e}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ohlcvs::database::OhlcvDatabase;
    use crate::ohlcvs::types::{Candle, Timeframe};
    use rusqlite::Connection;

    fn test_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "screenerbot-ohlcv-data-version-{label}-{}.db",
            std::process::id()
        ))
    }

    fn foreign_key_check_is_clean(conn: &Connection) -> bool {
        conn.query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .expect("foreign_key_check")
            .is_none()
    }

    fn insert_raw_candle(conn: &Connection, chain_id: &str, timestamp: i64) {
        conn.execute(
            "INSERT INTO ohlcv_candles (chain_id, mint, pool_address, timeframe, timestamp, open, high, low, close, volume, source)
             VALUES (?1, 'mint', 'pool', '1m', ?2, 1, 1, 1, 1, 1, 'test')",
            params![chain_id, timestamp],
        )
        .unwrap();
    }

    fn candle_count(conn: &Connection, chain_id: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM ohlcv_candles WHERE chain_id = ?1",
            params![chain_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    fn version_rows(conn: &Connection) -> Vec<(String, i64)> {
        let mut stmt = conn
            .prepare("SELECT chain_id, version FROM ohlcv_data_versions ORDER BY chain_id")
            .unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn replace_with_global_version(conn: &Connection, version: i64) {
        conn.execute_batch("DROP TABLE ohlcv_data_versions;")
            .unwrap();
        conn.execute_batch(
            "CREATE TABLE ohlcv_data_versions (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                version INTEGER NOT NULL
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ohlcv_data_versions (id, version) VALUES (1, ?1)",
            params![version],
        )
        .unwrap();
    }

    #[test]
    fn legacy_global_data_version_migrates_idempotently_without_wiping_current_data() {
        let path = test_path("legacy-global");
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
        {
            let conn = db.conn.lock().unwrap();
            replace_with_global_version(&conn, OHLCV_DATA_VERSION);
            assert!(
                !table_has_column(&conn, "ohlcv_data_versions", "chain_id").unwrap(),
                "fixture must use the legacy global id = 1 table"
            );
        }
        drop(db);

        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            assert_eq!(
                version_rows(&conn),
                vec![("solana".to_owned(), OHLCV_DATA_VERSION)]
            );
            assert_eq!(candle_count(&conn, "solana"), 1);
            assert!(foreign_key_check_is_clean(&conn));
        }
        drop(db);

        let reopened = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = reopened.conn.lock().unwrap();
            assert_eq!(
                version_rows(&conn),
                vec![("solana".to_owned(), OHLCV_DATA_VERSION)]
            );
            assert_eq!(candle_count(&conn, "solana"), 1);
            assert!(foreign_key_check_is_clean(&conn));
        }
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stale_version_wipes_only_the_owning_chain() {
        let path = test_path("stale-owning-chain");
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
        {
            let conn = db.conn.lock().unwrap();
            insert_raw_candle(&conn, "foreign", 120);
            conn.execute(
                "UPDATE ohlcv_data_versions SET version = 1 WHERE chain_id = 'solana'",
                [],
            )
            .unwrap();
            assert_eq!(candle_count(&conn, "solana"), 1);
            assert_eq!(candle_count(&conn, "foreign"), 1);
        }
        drop(db);

        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            assert_eq!(candle_count(&conn, "solana"), 0);
            assert_eq!(candle_count(&conn, "foreign"), 1);
            assert_eq!(
                version_rows(&conn),
                vec![("solana".to_owned(), OHLCV_DATA_VERSION)]
            );
            assert!(foreign_key_check_is_clean(&conn));
        }
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn current_version_does_not_wipe_owning_chain_data() {
        let path = test_path("current-no-wipe");
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
        drop(db);

        let reopened = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = reopened.conn.lock().unwrap();
            assert_eq!(candle_count(&conn, "solana"), 1);
            assert_eq!(
                version_rows(&conn),
                vec![("solana".to_owned(), OHLCV_DATA_VERSION)]
            );
            assert!(foreign_key_check_is_clean(&conn));
        }
        drop(reopened);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn stale_legacy_global_version_wipes_solana_and_leaves_synthetic_chain() {
        let path = test_path("stale-legacy-global");
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
        {
            let conn = db.conn.lock().unwrap();
            insert_raw_candle(&conn, "foreign", 120);
            replace_with_global_version(&conn, 1);
        }
        drop(db);

        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            assert_eq!(candle_count(&conn, "solana"), 0);
            assert_eq!(candle_count(&conn, "foreign"), 1);
            assert_eq!(
                version_rows(&conn),
                vec![("solana".to_owned(), OHLCV_DATA_VERSION)]
            );
            assert!(foreign_key_check_is_clean(&conn));
        }
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
