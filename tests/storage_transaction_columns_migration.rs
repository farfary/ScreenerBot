// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Unit-neutral transaction column names through the production initializer.
//!
//! Each open runs in a child process: `paths::get_data_directory()` memoises its
//! base directory per process, so every fixture needs its own process to open its
//! own data directory.

mod common;

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CHILD: &str = "SCREENERBOT_TRANSACTION_COLUMNS_CHILD";

const LEGACY_SUBJECT_DELTAS: &str = include_str!("fixtures/v0.2.13-subject-deltas.sql");

const LEGACY_RAW_TRANSACTIONS: &str = "
CREATE TABLE raw_transactions (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    slot INTEGER,
    block_time INTEGER,
    timestamp TEXT NOT NULL,
    status TEXT NOT NULL,
    success BOOLEAN NOT NULL DEFAULT false,
    error_message TEXT,
    fee_lamports INTEGER,
    compute_units_consumed INTEGER,
    instructions_count INTEGER NOT NULL DEFAULT 0,
    accounts_count INTEGER NOT NULL DEFAULT 0,
    raw_transaction_data TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, signature, wallet_address)
);";

const LEGACY_PROCESSED_TRANSACTIONS: &str = "
CREATE TABLE processed_transactions (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    transaction_type TEXT NOT NULL,
    type_kind TEXT NOT NULL DEFAULT 'unknown',
    direction TEXT NOT NULL,
    sol_balance_change TEXT,
    token_balance_changes TEXT,
    token_swap_info TEXT,
    swap_pnl_info TEXT,
    ata_operations TEXT,
    token_transfers TEXT,
    instruction_info TEXT,
    analysis_duration_ms INTEGER,
    cached_analysis TEXT,
    analysis_version INTEGER NOT NULL DEFAULT 2,
    fee_sol REAL NOT NULL DEFAULT 0,
    sol_delta REAL,
    processed_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, signature, wallet_address),
    FOREIGN KEY (chain_id, signature, wallet_address) REFERENCES raw_transactions(chain_id, signature, wallet_address) ON DELETE CASCADE
);";

const RENAMED: [(&str, &str, &str); 5] = [
    ("raw_transactions", "fee_lamports", "fee_raw"),
    (
        "processed_transactions",
        "sol_balance_change",
        "native_balance_change",
    ),
    ("processed_transactions", "fee_sol", "fee_native"),
    ("processed_transactions", "sol_delta", "native_delta"),
    ("subject_asset_deltas", "fee_lamports", "fee_raw"),
];

#[tokio::test(flavor = "multi_thread")]
async fn transaction_initializer_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    common::ensure_config();
    common::configure_own_wallet();
    screenerbot::transactions::TransactionDatabase::new(screenerbot::chains::ChainId::Solana)
        .await
        .unwrap();
}

fn database_path(dir: &Path) -> PathBuf {
    dir.join("data").join("transactions.db")
}

fn open(dir: &Path) -> Connection {
    Connection::open(database_path(dir)).unwrap()
}

fn run_initializer(dir: &Path) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("transaction_initializer_child")
        .env(CHILD, "1")
        .env("SCREENERBOT_DATA_DIR", dir)
        .output()
        .unwrap()
}

fn boot(dir: &Path) {
    let output = run_initializer(dir);
    assert!(
        output.status.success(),
        "transaction initializer failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn data_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    dir
}

fn seed_legacy(dir: &Path) {
    let conn = open(dir);
    conn.execute_batch(LEGACY_SUBJECT_DELTAS).unwrap();
    conn.execute_batch(LEGACY_RAW_TRANSACTIONS).unwrap();
    conn.execute_batch(LEGACY_PROCESSED_TRANSACTIONS).unwrap();
    conn.execute_batch(
        "INSERT INTO db_metadata (key, value) VALUES ('schema_version', '7');
         INSERT INTO raw_transactions (signature, wallet_address, timestamp, status, success, fee_lamports)
         VALUES ('SIG_SENTINEL', 'wallet-a', '2026-01-01T00:00:00Z', 'Finalized', 1, 5000);
         INSERT INTO processed_transactions (signature, wallet_address, transaction_type, type_kind, direction, sol_balance_change, fee_sol, sol_delta)
         VALUES ('SIG_SENTINEL', 'wallet-a', '\"Unknown\"', 'unknown', 'Outgoing', '{\"sentinel\":true}', 0.000005, -0.25);
         INSERT INTO subject_asset_deltas (wallet_address, signature, mint, slot, block_time, tx_index, delta_raw, before_raw, after_raw, decimals, kind, venue, fee_lamports, success)
         VALUES ('wallet-a', 'SIG_SENTINEL', 'native', 7, 8, 0, -250000000, 1000000000, 750000000, 9, 'transfer', NULL, 5000, 1);",
    )
    .unwrap();
}

fn schema(conn: &Connection) -> Vec<(String, String, Option<String>)> {
    conn.prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

fn columns(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

fn assert_canonical_names(conn: &Connection) {
    for (table, old, new) in RENAMED {
        let present = columns(conn, table);
        assert!(present.iter().any(|c| c == new), "missing {table}.{new}");
        assert!(!present.iter().any(|c| c == old), "kept {table}.{old}");
    }
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn legacy_transaction_columns_migrate_to_unit_neutral_names() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = data_dir();
    seed_legacy(dir.path());

    boot(dir.path());
    let conn = open(dir.path());
    assert_canonical_names(&conn);
    assert_eq!(
        conn.query_row(
            "SELECT fee_raw FROM raw_transactions WHERE signature = 'SIG_SENTINEL'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        5000
    );
    assert_eq!(
        conn.query_row(
            "SELECT native_balance_change, fee_native, native_delta FROM processed_transactions WHERE signature = 'SIG_SENTINEL'",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?)),
        )
        .unwrap(),
        ("{\"sentinel\":true}".to_owned(), 0.000005, -0.25)
    );
    assert_eq!(
        conn.query_row(
            "SELECT fee_raw, delta_raw FROM subject_asset_deltas WHERE signature = 'SIG_SENTINEL'",
            [],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
        )
        .unwrap(),
        (5000, "-250000000".to_owned())
    );
    let first = schema(&conn);
    drop(conn);

    boot(dir.path());
    let conn = open(dir.path());
    assert_eq!(schema(&conn), first, "second open changed the schema");
    assert_canonical_names(&conn);
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*), SUM(native_delta) FROM processed_transactions",
            [],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)),
        )
        .unwrap(),
        (1, -0.25)
    );
}

#[test]
fn transaction_store_with_both_names_refuses_to_open() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = data_dir();
    seed_legacy(dir.path());
    let conn = open(dir.path());
    conn.execute_batch("ALTER TABLE processed_transactions ADD COLUMN fee_native REAL")
        .unwrap();
    let before = schema(&conn);
    drop(conn);

    let output = run_initializer(dir.path());
    assert!(!output.status.success(), "conflicting names were accepted");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains(
            "both processed_transactions.fee_sol and processed_transactions.fee_native exist"
        ),
        "unexpected failure: {text}"
    );

    let conn = open(dir.path());
    assert_eq!(schema(&conn), before, "refused open changed the schema");
    assert_eq!(
        conn.query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "7"
    );
}

#[test]
fn fresh_transaction_store_opens_with_unit_neutral_names() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = data_dir();

    boot(dir.path());
    let conn = open(dir.path());
    assert_canonical_names(&conn);
    let first = schema(&conn);
    drop(conn);

    boot(dir.path());
    let conn = open(dir.path());
    assert_eq!(schema(&conn), first, "second open changed the schema");
}
