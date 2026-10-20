// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the filtering build write: the rejection columns describe the latest complete build only, the clear is bounded and chain-scoped, and the time-range stats count a rejection once when it starts.

use rusqlite::{params, Connection};

use super::retention::MAX_ROWS_PER_TRANSACTION;
use super::{FilterOutcome, FilterRejectionRecord, TokenDatabase};
use crate::chains::ChainId;
use crate::database::WriteTransaction;

const BUILD_ONE: i64 = 7_200;
const BUILD_TWO: i64 = 7_260;
const HOUR_OF_BUILDS: i64 = 7_200;

fn store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("tokens store");
    (directory, db)
}

fn insert_tracking(conn: &Connection, chain: &str, mint: &str) {
    conn.execute(
        "INSERT INTO tokens (chain_id, mint, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
         VALUES (?1, ?2, 1, 1, 1)",
        params![chain, mint],
    )
    .expect("insert token");
    conn.execute(
        "INSERT INTO update_tracking (chain_id, mint) VALUES (?1, ?2)",
        params![chain, mint],
    )
    .expect("insert tracking row");
}

/// A tracking row left by an earlier build: a reason and, optionally, the stamp.
fn insert_rejected(conn: &Connection, chain: &str, mint: &str, reason: &str, stamp: Option<i64>) {
    insert_tracking(conn, chain, mint);
    conn.execute(
        "UPDATE update_tracking
         SET last_rejection_reason = ?3, last_rejection_source = 'market',
             last_rejection_at = 50, filter_evaluated_at = ?4
         WHERE chain_id = ?1 AND mint = ?2",
        params![chain, mint, reason, stamp],
    )
    .expect("seed rejection");
}

fn rejected(mint: &str, reason: &str, since: i64) -> FilterOutcome {
    FilterOutcome {
        mint: mint.to_owned(),
        rejection: Some(FilterRejectionRecord {
            reason: reason.to_owned(),
            source: "market".to_owned(),
            since,
        }),
    }
}

fn passed(mint: &str) -> FilterOutcome {
    FilterOutcome {
        mint: mint.to_owned(),
        rejection: None,
    }
}

/// `(reason, source, at, stamp)` of one tracking row.
type Row = (Option<String>, Option<String>, Option<i64>, Option<i64>);

fn row(conn: &Connection, chain: &str, mint: &str) -> Row {
    conn.query_row(
        "SELECT last_rejection_reason, last_rejection_source, last_rejection_at, filter_evaluated_at
         FROM update_tracking WHERE chain_id = ?1 AND mint = ?2",
        params![chain, mint],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .expect("read tracking row")
}

fn rows_with_a_reason(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM update_tracking WHERE last_rejection_reason IS NOT NULL",
        [],
        |row| row.get(0),
    )
    .expect("count rejections")
}

fn stats(conn: &Connection) -> Vec<(i64, String, String, i64)> {
    let mut statement = conn
        .prepare(
            "SELECT bucket_hour, reason, source, rejection_count FROM rejection_stats
             ORDER BY bucket_hour, reason, source",
        )
        .expect("prepare stats");
    statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("list stats")
        .collect::<rusqlite::Result<_>>()
        .expect("read stats")
}

#[test]
fn a_token_the_latest_build_did_not_evaluate_has_no_rejection() {
    let (_directory, db) = store();
    let conn = db.conn().expect("connection");
    insert_tracking(&conn, "solana", "a");
    insert_tracking(&conn, "solana", "b");
    insert_rejected(&conn, "solana", "unstamped", "no_decimals", None);

    let first = db
        .record_filter_build(
            BUILD_ONE,
            &[
                rejected("a", "low_liquidity", BUILD_ONE),
                rejected("b", "low_liquidity", BUILD_ONE),
            ],
        )
        .expect("first build");
    assert_eq!(first.outcomes, 2);
    assert_eq!(
        first.cleared, 1,
        "the unstamped row of an earlier core goes"
    );
    assert_eq!(row(&conn, "solana", "unstamped").0, None);

    let second = db
        .record_filter_build(BUILD_TWO, &[rejected("a", "low_liquidity", BUILD_ONE)])
        .expect("second build");
    assert_eq!(second.cleared, 1);

    assert_eq!(
        row(&conn, "solana", "b"),
        (None, None, None, Some(BUILD_ONE))
    );
    assert_eq!(
        row(&conn, "solana", "a"),
        (
            Some("low_liquidity".to_owned()),
            Some("market".to_owned()),
            Some(BUILD_ONE),
            Some(BUILD_TWO)
        )
    );
}

#[test]
fn a_passed_token_has_no_rejection_and_carries_the_stamp() {
    let (_directory, db) = store();
    let conn = db.conn().expect("connection");
    insert_rejected(&conn, "solana", "a", "low_liquidity", Some(BUILD_ONE));

    db.record_filter_build(BUILD_TWO, &[passed("a")])
        .expect("build");

    assert_eq!(
        row(&conn, "solana", "a"),
        (None, None, None, Some(BUILD_TWO))
    );
}

#[test]
fn an_empty_build_clears_every_rejection() {
    let (_directory, db) = store();
    let conn = db.conn().expect("connection");
    insert_rejected(&conn, "solana", "a", "low_liquidity", Some(BUILD_ONE));
    insert_rejected(&conn, "solana", "b", "no_decimals", Some(BUILD_ONE));

    let write = db.record_filter_build(BUILD_TWO, &[]).expect("empty build");

    assert_eq!((write.outcomes, write.cleared), (0, 2));
    assert_eq!(rows_with_a_reason(&conn), 0);
}

#[test]
fn the_clear_runs_in_bounded_transactions() {
    let (_directory, db) = store();
    let mut conn = db.conn().expect("connection");
    let stale_rows = MAX_ROWS_PER_TRANSACTION * 2 + 500;
    {
        let transaction = conn.write_tx().expect("seed transaction");
        for index in 0..stale_rows {
            insert_rejected(
                &transaction,
                "solana",
                &format!("mint-{index}"),
                "low_liquidity",
                Some(BUILD_ONE),
            );
        }
        transaction.commit().expect("commit seed");
    }

    let write = db.record_filter_build(BUILD_TWO, &[]).expect("build");

    assert_eq!(write.cleared, stale_rows);
    assert_eq!(rows_with_a_reason(&conn), 0);
}

#[test]
fn rows_of_another_chain_are_never_touched() {
    let (_directory, db) = store();
    let conn = db.conn().expect("connection");
    insert_rejected(&conn, "other", "a", "low_liquidity", Some(BUILD_ONE));
    insert_rejected(&conn, "solana", "a", "low_liquidity", Some(BUILD_ONE));

    db.record_filter_build(BUILD_TWO, &[passed("a")])
        .expect("build");

    assert_eq!(
        row(&conn, "other", "a"),
        (
            Some("low_liquidity".to_owned()),
            Some("market".to_owned()),
            Some(50),
            Some(BUILD_ONE)
        )
    );
    assert_eq!(
        row(&conn, "solana", "a"),
        (None, None, None, Some(BUILD_TWO))
    );
}

#[test]
fn rejection_stats_count_a_token_once_when_its_rejection_starts() {
    let (_directory, db) = store();
    let conn = db.conn().expect("connection");
    insert_tracking(&conn, "solana", "a");
    insert_tracking(&conn, "solana", "b");

    db.record_filter_build(
        BUILD_ONE,
        &[
            rejected("a", "low_liquidity", BUILD_ONE),
            rejected("b", "low_liquidity", BUILD_ONE),
        ],
    )
    .expect("first build");
    db.record_filter_build(
        BUILD_TWO,
        &[
            rejected("a", "low_liquidity", BUILD_ONE),
            rejected("b", "low_liquidity", BUILD_ONE),
        ],
    )
    .expect("the same rejections in the next build");
    assert_eq!(
        stats(&conn),
        vec![(
            HOUR_OF_BUILDS,
            "low_liquidity".to_owned(),
            "market".to_owned(),
            2
        )],
        "a continuing rejection is not counted again"
    );

    db.record_filter_build(
        BUILD_TWO + 60,
        &[rejected("a", "no_decimals", BUILD_TWO + 60)],
    )
    .expect("a reason change");
    assert_eq!(
        stats(&conn),
        vec![
            (
                HOUR_OF_BUILDS,
                "low_liquidity".to_owned(),
                "market".to_owned(),
                2
            ),
            (
                HOUR_OF_BUILDS,
                "no_decimals".to_owned(),
                "market".to_owned(),
                1
            ),
        ],
        "a new reason counts again"
    );
}
