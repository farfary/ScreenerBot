// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The open that moves paper copies to the shared paper book, through the production
//! opener from every historical copy store shape: the copy paper book stays as read-only
//! history, its paper spend moves to the archive, every task starts a paper run, and
//! decisions and activity stay byte for byte. A second open changes nothing.

use crate::common;

use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::trader::copy::CopyDatabase;
use std::path::Path;

const LEGACY: &str = r#"{"outcome":"paper_filled","fill":{"model":"legacy"},"marker":"legacy"}"#;
const LIVE: &str = r#"{"outcome":"live_submitted","marker":"live"}"#;

/// A store a v0.2.13 core wrote, stamped v7, with a paper task, its estimate-model
/// paper book and spend, and one legacy paper and one live decision with their activity.
fn released_v7(path: &Path) {
    let conn = common::seed_store(
        path,
        screenerbot::database::COPY_TRADING_DB,
        include_str!("fixtures/v0.2.13-copy.sql"),
    );
    conn.execute_batch(
        "INSERT INTO copy_tasks (id, chain_id, target_address, label, enabled, mode_json, \
             sizing_json, exit_mode_json, exit_policy_json, max_sol_per_trade, max_sol_per_token, \
             total_budget_sol, min_target_trade_sol, max_target_trade_sol, buy_once_per_token, \
             slippage_pct, created_at, updated_at) \
         VALUES (7, 'solana', 'target', NULL, 1, '\"paper\"', '{\"kind\":\"fixed\",\"sol\":0.1}', \
             '\"buy_only\"', '{}', 0.125, 0.5, 2.75, 0.01, 9.5, 0, 1.0, \
             '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
         INSERT INTO copy_spend (task_id, mode, mint, spent_sol, buy_count, updated_at) \
         VALUES (7, 'paper', 'mint', 0.375, 3, '2026-01-02T00:00:00Z');
         INSERT INTO copy_paper_positions (task_id, mint, token_amount, cost_basis_sol, \
             invested_sol, realized_proceeds_sol, realized_cost_sol, buys, sells, last_price_sol, \
             last_price_at, opened_at, closed_at, updated_at, peak_price_sol) \
         VALUES (7, 'mint', 40.0, 0.25, 0.375, 0.0625, 0.125, 3, 1, 0.00625, \
             '2026-01-02T00:00:00Z', '2026-01-01T00:00:00Z', NULL, '2026-01-02T00:00:00Z', 0.0075);
         CREATE TABLE copy_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO copy_metadata VALUES ('schema_version', '7');
         CREATE TABLE copy_decisions (
             id INTEGER PRIMARY KEY AUTOINCREMENT, task_id INTEGER NOT NULL, signature TEXT NOT NULL,
             mint TEXT, outcome_json TEXT NOT NULL, decided_at TEXT NOT NULL,
             UNIQUE(task_id, signature), FOREIGN KEY(task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
         );
         CREATE TABLE copy_activity (
             id INTEGER PRIMARY KEY AUTOINCREMENT, task_id INTEGER NOT NULL, kind TEXT NOT NULL,
             details_json TEXT NOT NULL, created_at TEXT NOT NULL,
             FOREIGN KEY(task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
         );",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) \
         VALUES (7, 'legacy-paper', 'mint', ?1, '2026-01-02T00:00:00Z'), \
                (7, 'live-history', 'mint', ?2, '2026-01-02T00:00:01Z')",
        rusqlite::params![LEGACY, LIVE],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO copy_activity (task_id, kind, details_json, created_at) \
         VALUES (7, 'paper_filled', ?1, '2026-01-02T00:00:00Z'), \
                (7, 'live_submitted', ?2, '2026-01-02T00:00:01Z')",
        rusqlite::params![LEGACY, LIVE],
    )
    .unwrap();
}

/// A store of `version` (8, 10, 11 or 12) holding a quote-model copy paper book: built
/// from the released store by the current opener, then put back into the shape that
/// version wrote, with a quote-model round, its paper spend and a per-task run counter.
fn released_at(path: &Path, version: i64) {
    released_v7(path);
    drop(CopyDatabase::open(path, ChainId::Solana).unwrap());
    let conn = Connection::open(path).unwrap();
    conn.execute_batch("ALTER TABLE copy_tasks DROP COLUMN paper_started_at;")
        .unwrap();
    if version < 12 {
        conn.execute_batch(
            "ALTER TABLE copy_paper_positions DROP COLUMN round_partial_exits;
             ALTER TABLE copy_paper_positions DROP COLUMN take_profit_level;",
        )
        .unwrap();
    }
    if version == 8 {
        conn.execute_batch("DROP TABLE store_schema;").unwrap();
    } else {
        conn.execute(
            "UPDATE store_schema SET version = ?1, min_reader_version = 10 WHERE id = 1",
            [version],
        )
        .unwrap();
    }
    conn.execute(
        "UPDATE copy_metadata SET value = ?1 WHERE key = 'schema_version'",
        [version.to_string()],
    )
    .unwrap();
    conn.execute_batch(
        "INSERT INTO copy_paper_positions (task_id, mint, token_amount, cost_basis_native, \
             invested_native, buys, sells, opened_at, updated_at, token_amount_raw, token_decimals) \
         VALUES (7, 'quote-mint', 40.0, 0.25, 0.25, 1, 0, '2026-10-01T00:00:00Z', \
             '2026-10-01T00:00:00Z', '40000000', 6);
         INSERT INTO copy_spend (task_id, mode, mint, spent_native, buy_count, updated_at) \
         VALUES (7, 'paper', 'quote-mint', 0.25, 1, '2026-10-01T00:00:00Z');
         INSERT INTO copy_metadata VALUES ('paper_epoch:7', '2');",
    )
    .unwrap();
}

fn schema(conn: &Connection) -> Vec<(String, String)> {
    conn.prepare("SELECT name, COALESCE(sql, '') FROM sqlite_master ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn spend(conn: &Connection) -> Vec<(String, String, f64, i64)> {
    conn.prepare(
        "SELECT mode, mint, spent_native, buy_count FROM copy_spend \
         WHERE task_id = 7 ORDER BY mode, mint",
    )
    .unwrap()
    .query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn history(conn: &Connection) -> (Vec<String>, Vec<String>) {
    let read = |sql: &str| -> Vec<String> {
        conn.prepare(sql)
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    (
        read("SELECT outcome_json FROM copy_decisions ORDER BY id"),
        read("SELECT details_json FROM copy_activity ORDER BY id"),
    )
}

fn archive_rows(conn: &Connection) -> Vec<(String, f64)> {
    conn.prepare("SELECT mint, token_amount FROM copy_paper_positions ORDER BY mint")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn paper_started_at(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT paper_started_at FROM copy_tasks WHERE id = 7",
        [],
        |row| row.get(0),
    )
    .unwrap()
}

/// What every upgraded store holds, whatever shape it came from.
fn assert_moved_to_the_shared_book(conn: &Connection) {
    assert!(
        paper_started_at(conn).is_some(),
        "the task starts its paper run on the open that moved it"
    );
    assert!(
        spend(conn).iter().all(|(mode, ..)| mode != "paper"),
        "no paper spend is left outside the archive: {:?}",
        spend(conn)
    );
    let counters: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM copy_metadata WHERE key LIKE 'paper_epoch:%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(counters, 0, "the per-task run counters retire");
    assert_eq!(
        history(conn),
        (
            vec![LEGACY.to_owned(), LIVE.to_owned()],
            vec![LEGACY.to_owned(), LIVE.to_owned()]
        ),
        "decisions and activity are never rewritten"
    );
    let ledger: (i64, i64) = conn
        .query_row(
            "SELECT version, min_reader_version FROM store_schema WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(ledger, (13, 13));
    let stamped: String = conn
        .query_row(
            "SELECT value FROM copy_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stamped, "13");
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let fk_violations = conn
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query_map([], |_| Ok(()))
        .unwrap()
        .count();
    assert_eq!(fk_violations, 0);
}

/// Opens the store twice and proves the second open changes nothing.
fn open_twice(path: &Path) -> Connection {
    drop(CopyDatabase::open(path, ChainId::Solana).unwrap());
    let conn = Connection::open(path).unwrap();
    let first = (
        schema(&conn),
        spend(&conn),
        archive_rows(&conn),
        paper_started_at(&conn),
        history(&conn),
    );
    drop(conn);
    drop(CopyDatabase::open(path, ChainId::Solana).unwrap());
    let conn = Connection::open(path).unwrap();
    assert_eq!(
        (
            schema(&conn),
            spend(&conn),
            archive_rows(&conn),
            paper_started_at(&conn),
            history(&conn),
        ),
        first,
        "a second open is a no-op"
    );
    conn
}

#[test]
fn a_v7_store_archives_its_estimates_and_starts_a_paper_run() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released_v7(&path);
    let conn = open_twice(&path);
    assert_moved_to_the_shared_book(&conn);
    assert_eq!(
        spend(&conn),
        vec![("paper_legacy".to_owned(), "mint".to_owned(), 0.375, 3)],
        "estimate-model spend stays with the legacy estimates"
    );
    assert!(
        archive_rows(&conn).is_empty(),
        "the estimate-model book moved to the legacy archive at v8"
    );
}

#[test]
fn every_quote_model_store_keeps_its_copy_paper_book_as_history() {
    for version in [8, 10, 11, 12] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copy_trading.db");
        released_at(&path, version);
        let conn = open_twice(&path);
        assert_moved_to_the_shared_book(&conn);
        assert_eq!(
            archive_rows(&conn),
            vec![("quote-mint".to_owned(), 40.0)],
            "v{version}: the copy paper book is kept, never dropped or rewritten"
        );
        assert_eq!(
            spend(&conn),
            vec![
                ("paper_archive".to_owned(), "quote-mint".to_owned(), 0.25, 1),
                ("paper_legacy".to_owned(), "mint".to_owned(), 0.375, 3),
            ],
            "v{version}: the quote-model spend moves to the archive"
        );
    }
}

/// A core from before v13 that opened the store again stamps it back and writes paper
/// spend into the copy book; the next open merges that spend into the archive and
/// keeps the task's paper run start.
#[test]
fn a_rerun_after_an_older_build_merges_its_paper_spend_into_the_archive() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released_at(&path, 12);
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    let conn = Connection::open(&path).unwrap();
    let started = paper_started_at(&conn);
    conn.execute_batch(
        "UPDATE copy_metadata SET value = '12' WHERE key = 'schema_version';
         INSERT INTO copy_spend (task_id, mode, mint, spent_native, buy_count, updated_at) \
         VALUES (7, 'paper', 'quote-mint', 0.5, 2, '2026-10-02T00:00:00Z'), \
                (7, 'paper', 'new-mint', 0.125, 1, '2026-10-02T00:00:00Z');",
    )
    .unwrap();
    drop(conn);

    let conn = open_twice(&path);
    assert_moved_to_the_shared_book(&conn);
    assert_eq!(
        spend(&conn),
        vec![
            ("paper_archive".to_owned(), "new-mint".to_owned(), 0.125, 1),
            ("paper_archive".to_owned(), "quote-mint".to_owned(), 0.75, 3),
            ("paper_legacy".to_owned(), "mint".to_owned(), 0.375, 3),
        ]
    );
    assert_eq!(
        paper_started_at(&conn),
        started,
        "a run already started is never moved by the archive"
    );
}

#[tokio::test]
async fn the_archive_reads_as_earlier_results_and_never_as_current_spend() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released_at(&path, 12);
    let db = CopyDatabase::open(&path, ChainId::Solana).unwrap();
    assert_eq!(db.archived_paper_spent(7).await.unwrap(), 0.25);
    assert_eq!(db.live_total_spent(7).await.unwrap(), 0.0);
    let archived = db.archived_paper_positions(7).await.unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].mint, "quote-mint");
    let task = db.get_task(7).await.unwrap().unwrap();
    assert!(task.paper_started_at.is_some());
}
