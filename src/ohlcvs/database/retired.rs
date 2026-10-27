// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Candle and gap tables retired by a data-version change, and their batched purge.
//!
//! A stale data version must not cost the open in proportion to the store: deleting every
//! candle inside the schema transaction stalls the start for as long as the delete runs.
//! The open instead renames the chain's candle and gap tables aside and creates empty ones
//! in the same transaction that records the new version, so no candle of the older version
//! is ever read or merged with a new write. The renamed tables are never queried; the OHLCV
//! monitor empties them in bounded batches and drops each one once it is empty.

use super::migrations::create_chain_indexes;
use super::{OhlcvDatabase, CREATE_CANDLES, CREATE_GAPS};
use crate::database::schema::ddl::{column_names, has_table};
use crate::errors::DatabaseError;
use crate::ohlcvs::error::{OhlcvError, OhlcvResult};
use rusqlite::{params, Connection};

/// Each table a data-version change retires, with the statement that recreates it.
const RETIRED_TABLES: [(&str, &str); 2] = [
    ("ohlcv_candles", CREATE_CANDLES),
    ("ohlcv_gaps", CREATE_GAPS),
];

/// Infix between a table's name and the sequence number of its retired copy.
const RETIRED_INFIX: &str = "_retired_";

/// What one purge step left behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetiredPurge {
    /// A retired table still holds rows; another step is due.
    Pending,
    /// No retired table remains and the live tables carry their indexes.
    Done,
}

/// Rename `chain_id`'s candle and gap tables aside and recreate them empty; a table holding
/// no row of the chain, as in a new store, is left in place. Rows of other
/// chains are copied back through the leading `chain_id` column of each table's unique
/// index, so the work is bounded by those rows, never by the retired chain's. Runs inside
/// the schema transaction of the open.
///
/// The retired tables keep their explicit index names, so `create_chain_indexes` skips the
/// new tables until the purge drops the retired ones and recreates the indexes; until then
/// reads use the unique index, which leads with the chain, mint and pool.
pub(super) fn retire_chain_tables(conn: &Connection, chain_id: &str) -> OhlcvResult<()> {
    for (table, create) in RETIRED_TABLES {
        let holds_rows: bool = conn
            .query_row(
                &format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE chain_id = ?1)"),
                params![chain_id],
                |row| row.get(0),
            )
            .map_err(|e| {
                OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                    "inspect OHLCV table",
                    e,
                ))
            })?;
        if !holds_rows {
            continue;
        }
        let columns = column_names(conn, table)?.join(", ");
        let retired = free_retired_name(conn, table)?;
        conn.execute_batch(&format!(
            "ALTER TABLE {table} RENAME TO {retired}; {create};"
        ))
        .map_err(|e| {
            OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                "retire OHLCV table",
                e,
            ))
        })?;
        conn.execute(
            &format!(
                "INSERT INTO {table} ({columns}) SELECT {columns} FROM {retired}
                 WHERE chain_id < ?1 OR chain_id > ?1"
            ),
            params![chain_id],
        )
        .map_err(|e| {
            OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                "keep other chains' OHLCV rows",
                e,
            ))
        })?;
    }
    Ok(())
}

/// The first `<table>_retired_<n>` name no table holds.
fn free_retired_name(conn: &Connection, table: &str) -> OhlcvResult<String> {
    let mut sequence = 1_u32;
    loop {
        let name = format!("{table}{RETIRED_INFIX}{sequence}");
        if !has_table(conn, &name)? {
            return Ok(name);
        }
        sequence += 1;
    }
}

/// Every retired table in the store, oldest first.
fn retired_tables(conn: &Connection) -> OhlcvResult<Vec<String>> {
    let mut names = Vec::new();
    for (table, _) in RETIRED_TABLES {
        let mut statement = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name GLOB ?1
                 ORDER BY length(name), name",
            )
            .map_err(|e| {
                OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                    "list retired OHLCV tables",
                    e,
                ))
            })?;
        let rows = statement
            .query_map(params![format!("{table}{RETIRED_INFIX}*")], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|e| {
                OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                    "list retired OHLCV tables",
                    e,
                ))
            })?;
        for name in rows {
            names.push(name.map_err(|e| {
                OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                    "list retired OHLCV tables",
                    e,
                ))
            })?);
        }
    }
    Ok(names)
}

impl OhlcvDatabase {
    /// One bounded step of the retired-table purge: delete up to `batch` rows of the first
    /// retired table, or drop it when it is already empty. Once none remains, the live
    /// tables get the indexes the retired ones held.
    pub fn purge_retired_batch(&self, batch: usize) -> OhlcvResult<RetiredPurge> {
        let conn = self.conn()?;
        let Some(table) = retired_tables(&conn)?.into_iter().next() else {
            create_chain_indexes(&conn)?;
            return Ok(RetiredPurge::Done);
        };
        let deleted = conn
            .execute(
                &format!("DELETE FROM {table} WHERE rowid IN (SELECT rowid FROM {table} LIMIT ?1)"),
                params![i64::try_from(batch).unwrap_or(i64::MAX)],
            )
            .map_err(|e| {
                OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                    "purge retired OHLCV rows",
                    e,
                ))
            })?;
        if deleted == 0 {
            conn.execute_batch(&format!("DROP TABLE {table}"))
                .map_err(|e| {
                    OhlcvError::Database(DatabaseError::classify_sqlite_failure(
                        "drop retired OHLCV table",
                        e,
                    ))
                })?;
        }
        Ok(RetiredPurge::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;
    use crate::ohlcvs::database::migrations::test_path;
    use crate::ohlcvs::types::{Candle, Timeframe};

    const STALE_ROWS: i64 = 2_500;

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn index_table(conn: &Connection, index: &str) -> Option<String> {
        conn.query_row(
            "SELECT tbl_name FROM sqlite_master WHERE type = 'index' AND name = ?1",
            params![index],
            |row| row.get(0),
        )
        .ok()
    }

    /// A Solana store holding `STALE_ROWS` candles, a gap and a completed backfill under
    /// version 5, plus one candle of another chain.
    fn stale_store(path: &std::path::Path) {
        let db = OhlcvDatabase::new(path, ChainId::Solana).unwrap();
        let candles: Vec<Candle> = (0..STALE_ROWS)
            .map(|i| Candle::new(i * 60, 1.0, 1.0, 1.0, 1.0, 1.0))
            .collect();
        db.insert_candles_batch("mint", "pool", Timeframe::Minute1, &candles, "test")
            .unwrap();
        let conn = db.conn().unwrap();
        conn.execute_batch(
            "INSERT INTO ohlcv_candles (chain_id, mint, pool_address, timeframe, timestamp, open, high, low, close, volume, source)
                 VALUES ('foreign', 'mint', 'pool', '1m', 60, 2, 2, 2, 2, 1, 'test');
             INSERT INTO ohlcv_gaps (chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
                 VALUES ('solana', 'mint', 'pool', '1m', 0, 600);
             INSERT INTO ohlcv_monitor_config (chain_id, mint, priority, last_activity, backfill_1m_complete)
                 VALUES ('solana', 'mint', 'high', '2026-01-01T00:00:00Z', 1);
             UPDATE ohlcv_data_versions SET version = 5 WHERE chain_id = 'solana';",
        )
        .unwrap();
    }

    /// The open of a stale store moves the old rows aside without deleting them, serves
    /// none of them, takes new candles at the timestamps they held, and resets backfill so
    /// the series is fetched again. The batched purge then empties and drops every retired
    /// table and restores the live tables' indexes.
    #[test]
    fn a_stale_store_opens_without_deleting_and_the_purge_finishes_in_batches() {
        let path = test_path("retired-purge");
        let _ = std::fs::remove_file(&path);
        stale_store(&path);

        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = db.conn().unwrap();
            assert_eq!(
                count(&conn, "SELECT COUNT(*) FROM ohlcv_candles_retired_1"),
                STALE_ROWS + 1,
                "the open retires the rows instead of deleting them"
            );
            assert_eq!(count(&conn, "SELECT COUNT(*) FROM ohlcv_gaps_retired_1"), 1);
            assert_eq!(
                count(
                    &conn,
                    "SELECT COUNT(*) FROM ohlcv_candles WHERE chain_id = 'solana'"
                ),
                0
            );
            assert_eq!(
                count(
                    &conn,
                    "SELECT COUNT(*) FROM ohlcv_candles WHERE chain_id = 'foreign'"
                ),
                1,
                "another chain's rows stay live"
            );
            assert_eq!(count(&conn, "SELECT COUNT(*) FROM ohlcv_gaps"), 0);
            assert_eq!(
                count(
                    &conn,
                    "SELECT backfill_1m_complete FROM ohlcv_monitor_config WHERE mint = 'mint'"
                ),
                0,
                "backfill restarts so the series is fetched again"
            );
            assert_eq!(
                count(
                    &conn,
                    "SELECT version FROM ohlcv_data_versions WHERE chain_id = 'solana'"
                ),
                super::super::data_version::OHLCV_DATA_VERSION
            );
            assert_eq!(
                index_table(&conn, "idx_candles_lookup").as_deref(),
                Some("ohlcv_candles_retired_1")
            );
        }
        assert!(db
            .get_candles("mint", Some("pool"), Timeframe::Minute1, None, None, None)
            .unwrap()
            .is_empty());

        // A refetch writes at a timestamp the old version held; nothing old merges into it.
        db.insert_candles_batch(
            "mint",
            "pool",
            Timeframe::Minute1,
            &[Candle::new(60, 3.0, 3.0, 3.0, 3.0, 2.0)],
            "test",
        )
        .unwrap();
        let served = db
            .get_candles("mint", Some("pool"), Timeframe::Minute1, None, None, None)
            .unwrap();
        assert_eq!(served.len(), 1);
        assert_eq!(served[0].close, 3.0);

        let mut steps = 0;
        while db.purge_retired_batch(1_000).unwrap() == RetiredPurge::Pending {
            steps += 1;
            assert!(steps < 100, "the purge must finish");
        }
        assert!(
            steps > 3,
            "the purge deletes in batches, took {steps} steps"
        );

        let conn = db.conn().unwrap();
        assert!(retired_tables(&conn).unwrap().is_empty());
        for index in [
            "idx_candles_lookup",
            "idx_candles_pool_lookup",
            "idx_candles_cleanup",
        ] {
            assert_eq!(index_table(&conn, index).as_deref(), Some("ohlcv_candles"));
        }
        for index in ["idx_gaps_unfilled", "idx_gaps_retry"] {
            assert_eq!(index_table(&conn, index).as_deref(), Some("ohlcv_gaps"));
        }
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM ohlcv_candles"), 2);
        drop(conn);
        assert_eq!(db.purge_retired_batch(1_000).unwrap(), RetiredPurge::Done);
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    /// A version change while an earlier retired table still awaits its purge retires the
    /// live tables under the next free name, and the purge removes both.
    #[test]
    fn a_second_change_before_the_purge_retires_under_the_next_name() {
        let path = test_path("retired-twice");
        let _ = std::fs::remove_file(&path);
        stale_store(&path);
        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        db.insert_candles_batch(
            "mint",
            "pool",
            Timeframe::Minute1,
            &[Candle::new(60, 1.0, 1.0, 1.0, 1.0, 1.0)],
            "test",
        )
        .unwrap();
        db.conn()
            .unwrap()
            .execute_batch(
                "INSERT INTO ohlcv_gaps (chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
                     VALUES ('solana', 'mint', 'pool', '1m', 0, 600);
                 UPDATE ohlcv_data_versions SET version = 5 WHERE chain_id = 'solana';",
            )
            .unwrap();
        drop(db);

        let db = OhlcvDatabase::new(&path, ChainId::Solana).unwrap();
        {
            let conn = db.conn().unwrap();
            assert_eq!(
                retired_tables(&conn).unwrap(),
                vec![
                    "ohlcv_candles_retired_1".to_owned(),
                    "ohlcv_candles_retired_2".to_owned(),
                    "ohlcv_gaps_retired_1".to_owned(),
                    "ohlcv_gaps_retired_2".to_owned(),
                ]
            );
        }
        while db.purge_retired_batch(10_000).unwrap() == RetiredPurge::Pending {}
        assert!(retired_tables(&db.conn().unwrap()).unwrap().is_empty());
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
