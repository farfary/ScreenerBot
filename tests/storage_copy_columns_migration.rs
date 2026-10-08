// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Released copy-trading storage through the production opener: unit-neutral column names.

mod common;

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

#[test]
fn released_copy_columns_open_under_unit_neutral_names() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("copy_trading.db");
    released(&path);

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
            "SELECT spent_native FROM copy_spend WHERE task_id = 7 AND mode = 'paper'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(spent, 0.375);
    let book: (f64, f64, f64, f64, Option<f64>, Option<f64>) = conn
        .query_row(
            "SELECT cost_basis_native, invested_native, realized_proceeds_native, \
             realized_cost_native, last_price_native, peak_price_native \
             FROM copy_paper_positions WHERE task_id = 7 AND mint = 'mint'",
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

    assert!(CopyDatabase::open(&path, ChainId::Solana).is_err());
    assert_eq!(schema(&Connection::open(&path).unwrap()), before);
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
