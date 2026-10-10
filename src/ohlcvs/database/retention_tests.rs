// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for chart history eviction: an unmonitored chart loses every row in bounded transactions, while a monitored, reactivated or interrupted chart keeps what it has.

use std::cell::Cell;

use rusqlite::{params, Connection};

use super::retention::{
    active_chart_mints, evict_chart_history, inactive_charts, MAX_ROWS_PER_TRANSACTION,
};
use super::OhlcvDatabase;
use crate::chains::ChainId;

const IDLE_SINCE: &str = "2026-01-01T00:00:00Z";

fn store() -> (tempfile::TempDir, OhlcvDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let db = OhlcvDatabase::new(directory.path().join("ohlcvs.db"), ChainId::Solana)
        .expect("open chart store");
    (directory, db)
}

/// A chart with `candles` one-minute candles, one gap and one pool.
fn insert_chart(conn: &Connection, mint: &str, active: bool, candles: u64) {
    conn.execute(
        "INSERT INTO ohlcv_monitor_config (chain_id, mint, priority, is_active, last_activity)
         VALUES ('solana', ?1, 'low', ?2, ?3)",
        params![mint, active, IDLE_SINCE],
    )
    .expect("insert monitor entry");
    conn.execute(
        "INSERT INTO ohlcv_pools (chain_id, mint, pool_address, dex) VALUES ('solana', ?1, 'pool', 'dex')",
        params![mint],
    )
    .expect("insert pool");
    conn.execute(
        "INSERT INTO ohlcv_gaps (chain_id, mint, pool_address, timeframe, start_timestamp, end_timestamp)
         VALUES ('solana', ?1, 'pool', '1m', 0, 60)",
        params![mint],
    )
    .expect("insert gap");
    conn.execute(
        "WITH RECURSIVE n(i) AS (SELECT 0 UNION ALL SELECT i + 1 FROM n WHERE i + 1 < ?2)
         INSERT INTO ohlcv_candles (chain_id, mint, pool_address, timeframe, timestamp, open, high, low, close, volume)
         SELECT 'solana', ?1, 'pool', '1m', i * 60, 1, 1, 1, 1, 1 FROM n",
        params![mint, candles],
    )
    .expect("insert candles");
}

fn rows_of(conn: &Connection, mint: &str) -> i64 {
    [
        "ohlcv_candles",
        "ohlcv_gaps",
        "ohlcv_pools",
        "ohlcv_monitor_config",
    ]
    .iter()
    .map(|table| {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE mint = ?1"),
            params![mint],
            |row| row.get::<_, i64>(0),
        )
        .expect("count rows")
    })
    .sum()
}

fn never_stop() -> bool {
    false
}

#[test]
fn an_unmonitored_chart_loses_every_row_across_bounded_transactions() {
    let (_directory, db) = store();
    let mut conn = db.conn().expect("connection");
    let candles = MAX_ROWS_PER_TRANSACTION as u64 + 500;
    insert_chart(&conn, "idle", false, candles);
    insert_chart(&conn, "watched", true, 10);

    let eviction = evict_chart_history(&mut conn, ChainId::Solana, "idle", IDLE_SINCE, &never_stop)
        .expect("evict");

    assert!(eviction.completed);
    assert!(!eviction.interrupted);
    assert_eq!(eviction.rows, candles + 3);
    assert_eq!(rows_of(&conn, "idle"), 0);
    assert_eq!(rows_of(&conn, "watched"), 13);
}

#[test]
fn a_monitored_or_reactivated_chart_is_never_evicted() {
    let (_directory, db) = store();
    let mut conn = db.conn().expect("connection");
    insert_chart(&conn, "watched", true, 10);
    insert_chart(&conn, "viewed-again", false, 10);
    conn.execute(
        "UPDATE ohlcv_monitor_config SET last_activity = '2026-06-01T00:00:00Z' WHERE mint = 'viewed-again'",
        [],
    )
    .expect("touch chart");

    for mint in ["watched", "viewed-again"] {
        let eviction =
            evict_chart_history(&mut conn, ChainId::Solana, mint, IDLE_SINCE, &never_stop)
                .expect("evict");
        assert!(eviction.interrupted, "{mint}");
        assert!(!eviction.completed, "{mint}");
        assert_eq!(eviction.rows, 0, "{mint}");
        assert_eq!(rows_of(&conn, mint), 13, "{mint}");
    }
}

#[test]
fn an_interrupted_eviction_keeps_the_monitor_entry_so_the_next_pass_finishes() {
    let (_directory, db) = store();
    let mut conn = db.conn().expect("connection");
    let candles = MAX_ROWS_PER_TRANSACTION as u64 + 500;
    insert_chart(&conn, "idle", false, candles);
    let checks = Cell::new(0);
    let stop_after_one_transaction = || {
        checks.set(checks.get() + 1);
        checks.get() > 1
    };

    let first = evict_chart_history(
        &mut conn,
        ChainId::Solana,
        "idle",
        IDLE_SINCE,
        &stop_after_one_transaction,
    )
    .expect("first transaction");
    assert!(first.interrupted);
    assert!(!first.completed);
    assert_eq!(first.rows, MAX_ROWS_PER_TRANSACTION as u64);

    let (_, listed) = inactive_charts(&conn, ChainId::Solana).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].candles, candles - MAX_ROWS_PER_TRANSACTION as u64);

    let rest = evict_chart_history(&mut conn, ChainId::Solana, "idle", IDLE_SINCE, &never_stop)
        .expect("rest");
    assert!(rest.completed);
    assert_eq!(first.rows + rest.rows, candles + 3);
    assert_eq!(rows_of(&conn, "idle"), 0);
}

#[test]
fn charts_are_listed_by_whether_they_are_monitored() {
    let (_directory, db) = store();
    let conn = db.conn().expect("connection");
    insert_chart(&conn, "watched", true, 7);
    insert_chart(&conn, "idle", false, 3);

    assert_eq!(
        active_chart_mints(&conn, ChainId::Solana).expect("active"),
        ["watched".to_owned()].into()
    );
    let (total, inactive) = inactive_charts(&conn, ChainId::Solana).expect("inactive");
    assert_eq!(total, 10);
    assert_eq!(inactive.len(), 1);
    assert_eq!(inactive[0].mint, "idle");
    assert_eq!(inactive[0].last_activity, IDLE_SINCE);
    assert_eq!(inactive[0].candles, 3);
}
