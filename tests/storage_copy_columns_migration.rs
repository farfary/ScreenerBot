// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Released copy-trading storage through the production opener: unit-neutral column names.

use crate::common;

use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::trader::copy::CopyDatabase;
use std::path::Path;

const RENAMED: &[(&str, &str, &str)] = &[
    ("copy_tasks", "max_sol_per_trade", "max_native_per_trade"),
    ("copy_tasks", "max_sol_per_token", "max_native_per_token"),
    ("copy_tasks", "total_budget_sol", "total_budget_native"),
    (
        "copy_tasks",
        "min_target_trade_sol",
        "min_target_trade_native",
    ),
    (
        "copy_tasks",
        "max_target_trade_sol",
        "max_target_trade_native",
    ),
    ("copy_spend", "spent_sol", "spent_native"),
    (
        "copy_paper_positions",
        "cost_basis_sol",
        "cost_basis_native",
    ),
    ("copy_paper_positions", "invested_sol", "invested_native"),
    (
        "copy_paper_positions",
        "realized_proceeds_sol",
        "realized_proceeds_native",
    ),
    (
        "copy_paper_positions",
        "realized_cost_sol",
        "realized_cost_native",
    ),
    (
        "copy_paper_positions",
        "last_price_sol",
        "last_price_native",
    ),
    (
        "copy_paper_positions",
        "peak_price_sol",
        "peak_price_native",
    ),
];

fn columns(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn schema(conn: &Connection) -> Vec<(String, String)> {
    conn.prepare("SELECT name, COALESCE(sql, '') FROM sqlite_master ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn released(path: &Path) {
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
             '2026-01-02T00:00:00Z', '2026-01-01T00:00:00Z', NULL, '2026-01-02T00:00:00Z', 0.0075);",
    )
    .unwrap();
}

fn released_with_history(path: &Path) -> (&'static str, &'static str) {
    const LEGACY: &str =
        r#"{"outcome":"paper_filled","fill":{"model":"legacy"},"marker":"legacy"}"#;
    const LIVE: &str = r#"{"outcome":"live_submitted","marker":"live"}"#;
    released(path);
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE copy_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
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
         VALUES (7, 'legacy-paper', 'mint', ?1, '2026-01-02T00:00:00Z')",
        [LEGACY],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO copy_decisions (task_id, signature, mint, outcome_json, decided_at) \
         VALUES (7, 'live-history', 'mint', ?1, '2026-01-02T00:00:01Z')",
        [LIVE],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO copy_activity (task_id, kind, details_json, created_at) \
         VALUES (7, 'paper_filled', ?1, '2026-01-02T00:00:00Z'), \
                (7, 'live_submitted', ?2, '2026-01-02T00:00:01Z')",
        rusqlite::params![LEGACY, LIVE],
    )
    .unwrap();
    (LEGACY, LIVE)
}

#[test]
fn released_copy_columns_open_under_unit_neutral_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let (legacy_json, live_json) = released_with_history(&path);

    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    let conn = Connection::open(&path).unwrap();
    for (table, old, new) in RENAMED {
        let names = columns(&conn, table);
        assert!(names.iter().any(|name| name == new), "{table}.{new}");
        assert!(!names.iter().any(|name| name == old), "{table}.{old}");
    }
    let task: (f64, f64, f64, Option<f64>, Option<f64>) = conn
        .query_row(
            "SELECT max_native_per_trade, max_native_per_token, total_budget_native, \
             min_target_trade_native, max_target_trade_native FROM copy_tasks WHERE id = 7",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(task, (0.125, 0.5, 2.75, Some(0.01), Some(9.5)));
    let spent: f64 = conn
        .query_row(
            "SELECT spent_native FROM copy_spend WHERE task_id = 7 AND mode = 'paper_legacy'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(spent, 0.375);
    let book: (f64, f64, f64, f64, Option<f64>, Option<f64>) = conn
        .query_row(
            "SELECT cost_basis_native, invested_native, realized_proceeds_native, \
             realized_cost_native, last_price_native, peak_price_native \
             FROM copy_paper_legacy_positions WHERE task_id = 7 AND mint = 'mint'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        book,
        (0.25, 0.375, 0.0625, 0.125, Some(0.00625), Some(0.0075))
    );
    let current_positions: i64 = conn
        .query_row("SELECT COUNT(*) FROM copy_paper_positions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(current_positions, 0);
    let active_columns = columns(&conn, "copy_paper_positions");
    assert!(active_columns.iter().any(|name| name == "token_amount_raw"));
    assert!(active_columns.iter().any(|name| name == "token_decimals"));
    let archived_columns = columns(&conn, "copy_paper_legacy_positions");
    assert!(!archived_columns
        .iter()
        .any(|name| name == "token_amount_raw"));
    assert!(!archived_columns.iter().any(|name| name == "token_decimals"));
    let history: (String, String, i64, i64) = conn
        .query_row(
            "SELECT (SELECT outcome_json FROM copy_decisions WHERE signature='legacy-paper'), \
             (SELECT outcome_json FROM copy_decisions WHERE signature='live-history'), \
             (SELECT COUNT(*) FROM copy_activity WHERE kind='paper_filled'), \
             (SELECT COUNT(*) FROM copy_activity WHERE kind='live_submitted')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        history,
        (legacy_json.to_owned(), live_json.to_owned(), 1, 1)
    );
    let archive_fk: String = conn
        .query_row(
            "SELECT \"table\" FROM pragma_foreign_key_list('copy_paper_legacy_positions') LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(archive_fk, "copy_tasks");
    let fk_violations = conn
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query_map([], |_| Ok(()))
        .unwrap()
        .count();
    assert_eq!(fk_violations, 0);

    let migrated = schema(&conn);
    drop(conn);
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    let conn = Connection::open(&path).unwrap();
    assert_eq!(schema(&conn), migrated, "a second open is a no-op");
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
}

#[test]
fn a_table_with_both_column_names_refuses_and_keeps_storage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released(&path);
    Connection::open(&path)
        .unwrap()
        .execute("ALTER TABLE copy_spend ADD COLUMN spent_native REAL", [])
        .unwrap();
    let before = schema(&Connection::open(&path).unwrap());

    let error = CopyDatabase::open(&path, ChainId::Solana)
        .err()
        .expect("both names refuse the open");
    assert!(
        matches!(
            &error,
            screenerbot::trader::Error::Database(
                screenerbot::errors::DatabaseError::SchemaConflict { store, object }
            ) if store == "copy_trading.db" && object == "copy_spend.spent_sol and copy_spend.spent_native"
        ),
        "{error:?}"
    );
    assert_eq!(schema(&Connection::open(&path).unwrap()), before);
}

#[test]
fn a_conflicting_legacy_archive_rolls_back_the_whole_upgrade() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released(&path);
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE copy_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         INSERT INTO copy_metadata VALUES ('schema_version', '7');
         CREATE TABLE copy_paper_legacy_positions (
             task_id INTEGER NOT NULL, mint TEXT NOT NULL,
             token_amount REAL NOT NULL DEFAULT 0, cost_basis_native REAL NOT NULL DEFAULT 0,
             invested_native REAL NOT NULL DEFAULT 0, realized_proceeds_native REAL NOT NULL DEFAULT 0,
             realized_cost_native REAL NOT NULL DEFAULT 0, buys INTEGER NOT NULL DEFAULT 0,
             sells INTEGER NOT NULL DEFAULT 0, last_price_native REAL, last_price_at TEXT,
             opened_at TEXT NOT NULL, closed_at TEXT, updated_at TEXT NOT NULL,
             peak_price_native REAL, PRIMARY KEY(task_id, mint),
             FOREIGN KEY(task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
         );
         INSERT INTO copy_paper_legacy_positions (task_id, mint, opened_at, updated_at, token_amount) \
         VALUES (7, 'mint', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z', 1);",
    )
    .unwrap();
    let before = schema(&conn);
    drop(conn);

    assert!(CopyDatabase::open(&path, ChainId::Solana).is_err());

    let conn = Connection::open(&path).unwrap();
    assert_eq!(schema(&conn), before);
    let version: String = conn
        .query_row(
            "SELECT value FROM copy_metadata WHERE key='schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, "7");
    let current_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM copy_paper_positions", [], |row| {
            row.get(0)
        })
        .unwrap();
    let archive_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM copy_paper_legacy_positions",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!((current_rows, archive_rows), (1, 1));
    let spend_mode: String = conn
        .query_row("SELECT mode FROM copy_spend WHERE task_id=7", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(spend_mode, "paper");
}

#[test]
fn malformed_schema_version_rolls_back_archive_renames_and_history_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    let (legacy_json, live_json) = released_with_history(&path);
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "UPDATE copy_metadata SET value='not-a-version' WHERE key='schema_version'",
        [],
    )
    .unwrap();
    let before = schema(&conn);
    drop(conn);

    assert!(CopyDatabase::open(&path, ChainId::Solana).is_err());

    let conn = Connection::open(&path).unwrap();
    assert_eq!(schema(&conn), before);
    let history: (String, String, i64, i64, String) = conn
        .query_row(
            "SELECT (SELECT outcome_json FROM copy_decisions WHERE signature='legacy-paper'), \
             (SELECT outcome_json FROM copy_decisions WHERE signature='live-history'), \
             (SELECT COUNT(*) FROM copy_activity WHERE kind='paper_filled'), \
             (SELECT COUNT(*) FROM copy_activity WHERE kind='live_submitted'), \
             (SELECT mode FROM copy_spend WHERE task_id=7 AND mint='mint')",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        history,
        (
            legacy_json.to_owned(),
            live_json.to_owned(),
            1,
            1,
            "paper".to_owned()
        )
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM copy_paper_positions WHERE task_id=7",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='copy_paper_legacy_positions'", [], |row| row.get::<_, i64>(0)).unwrap(),
        0
    );
}

#[test]
fn invalid_raw_row_in_v8_refuses_open_without_modifying_storage() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released(&path);
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    let conn = Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO copy_paper_positions (task_id, mint, token_amount, token_amount_raw, token_decimals, \
         opened_at, updated_at) VALUES (7, 'invalid', 1.0, '01', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [],
    )
    .unwrap();
    let before = schema(&conn);
    let row_before: (String, i64) = conn
        .query_row("SELECT token_amount_raw, token_decimals FROM copy_paper_positions WHERE mint='invalid'", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    drop(conn);

    assert!(CopyDatabase::open(&path, ChainId::Solana).is_err());

    let conn = Connection::open(&path).unwrap();
    assert_eq!(schema(&conn), before);
    let row_after: (String, i64) = conn
        .query_row("SELECT token_amount_raw, token_decimals FROM copy_paper_positions WHERE mint='invalid'", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    assert_eq!(row_after, row_before);
}

/// An older build re-adds `peak_price_sol` to a migrated store and writes only that
/// name. The next open keeps every peak either build wrote, the canonical one where both
/// did, and drops the legacy column; a further open writes nothing.
#[test]
fn paper_peaks_from_both_builds_merge_on_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released(&path);
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "ALTER TABLE copy_paper_positions ADD COLUMN peak_price_sol REAL;
             INSERT INTO copy_paper_positions (task_id, mint, token_amount, token_amount_raw, token_decimals, \
                 opened_at, updated_at, peak_price_sol) \
             VALUES (7, 'legacy', 1.0, '1', 0, '2026-01-03T00:00:00Z', '2026-01-03T00:00:00Z', 0.02);
             INSERT INTO copy_paper_positions (task_id, mint, token_amount, token_amount_raw, token_decimals, \
                 opened_at, updated_at, peak_price_native, peak_price_sol) \
             VALUES (7, 'both', 1.0, '1', 0, '2026-01-03T00:00:00Z', '2026-01-03T00:00:00Z', 0.03, 0.01);
             INSERT INTO copy_paper_positions (task_id, mint, token_amount, token_amount_raw, token_decimals, \
                 opened_at, updated_at) \
             VALUES (7, 'neither', 1.0, '1', 0, '2026-01-03T00:00:00Z', '2026-01-03T00:00:00Z');",
        )
        .unwrap();

    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());

    let conn = Connection::open(&path).unwrap();
    assert!(!columns(&conn, "copy_paper_positions")
        .iter()
        .any(|name| name == "peak_price_sol"));
    let peaks: Vec<(String, Option<f64>)> = conn
        .prepare("SELECT mint, peak_price_native FROM copy_paper_positions ORDER BY mint")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        peaks,
        vec![
            ("both".to_owned(), Some(0.03)),
            ("legacy".to_owned(), Some(0.02)),
            ("neither".to_owned(), None),
        ]
    );
    let merged = schema(&conn);
    drop(conn);
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    assert_eq!(schema(&Connection::open(&path).unwrap()), merged);
}

#[test]
fn a_fresh_copy_database_opens_cleanly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    drop(CopyDatabase::open(&path, ChainId::Solana).unwrap());
    let conn = Connection::open(&path).unwrap();
    for (table, old, new) in RENAMED {
        let names = columns(&conn, table);
        assert!(names.iter().any(|name| name == new), "{table}.{new}");
        assert!(!names.iter().any(|name| name == old), "{table}.{old}");
    }
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
}
