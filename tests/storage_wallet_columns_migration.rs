// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Unit-neutral wallet-monitor column and table names through the production initializers.

mod common;

use rusqlite::Connection;
use std::path::Path;
use std::process::{Command, Output};

const CHILD: &str = "SCREENERBOT_WALLET_COLUMNS_CHILD";

fn open(dir: &Path, file: &str) -> Connection {
    Connection::open(dir.join("data").join(file)).unwrap()
}

fn run_initializers(dir: &Path) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("wallet_initializers_child")
        .env(CHILD, "1")
        .env("SCREENERBOT_DATA_DIR", dir)
        .output()
        .unwrap()
}

fn boot(dir: &Path) {
    let output = run_initializers(dir);
    assert!(
        output.status.success(),
        "wallet initializer failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn wallet_initializers_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    screenerbot::paths::ensure_all_directories().unwrap();
    screenerbot::config::load_config().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        screenerbot::wallets::initialize().await.unwrap();
        screenerbot::wallet::initialize_wallet_database()
            .await
            .unwrap();
    });
}

fn seed_main_wallet(dir: &Path) {
    let wallets = common::seed_store(
        &dir.join("data/wallets.db"),
        screenerbot::database::WALLETS_DB,
        include_str!("fixtures/v0.2.13-wallets.sql"),
    );
    wallets
        .execute(
            "INSERT INTO wallets (id, name, address, encrypted_key, nonce, role) VALUES (17, 'main', 'address', 'ciphertext', 'nonce', 'main')",
            [],
        )
        .unwrap();
}

fn seed_legacy_monitor(dir: &Path) {
    let monitor = common::seed_store(
        &dir.join("data/wallet.db"),
        screenerbot::database::WALLET_MONITOR_DB,
        include_str!("fixtures/v0.2.13-wallet-monitor.sql"),
    );
    monitor
        .execute(
            "INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_equity_sol, total_tokens_count) VALUES (23, 'address', '2025-01-02T00:00:00+00:00', 1.5, 1500000000, 4.5, 2)",
            [],
        )
        .unwrap();
    monitor
        .execute(
            "INSERT INTO sol_flow_cache (wallet_address, signature, timestamp, sol_delta) VALUES ('address', 'signature', '2025-01-02', -1.25)",
            [],
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

fn has_table(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |r| r.get::<_, i64>(0),
    )
    .unwrap()
        == 1
}

fn assert_canonical_names(conn: &Connection) {
    let snapshot_columns = columns(conn, "wallet_snapshots");
    for name in [
        "native_balance",
        "native_balance_raw",
        "total_equity_native",
    ] {
        assert!(snapshot_columns.iter().any(|c| c == name), "missing {name}");
    }
    for name in ["sol_balance", "sol_balance_lamports", "total_equity_sol"] {
        assert!(!snapshot_columns.iter().any(|c| c == name), "kept {name}");
    }
    assert!(has_table(conn, "native_flow_cache"));
    assert!(!has_table(conn, "sol_flow_cache"));
    let flow_columns = columns(conn, "native_flow_cache");
    assert!(flow_columns.iter().any(|c| c == "native_delta"));
    assert!(!flow_columns.iter().any(|c| c == "sol_delta"));
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn legacy_wallet_monitor_names_migrate_to_unit_neutral_names() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    seed_main_wallet(dir.path());
    seed_legacy_monitor(dir.path());

    boot(dir.path());
    let monitor = open(dir.path(), "wallet.db");
    assert_canonical_names(&monitor);
    assert_eq!(
        monitor
            .query_row(
                "SELECT native_balance, native_balance_raw, total_equity_native FROM wallet_snapshots WHERE id = 23",
                [],
                |r| Ok((r.get::<_, f64>(0)?, r.get::<_, i64>(1)?, r.get::<_, f64>(2)?)),
            )
            .unwrap(),
        (1.5, 1_500_000_000, 4.5)
    );
    assert_eq!(
        monitor
            .query_row(
                "SELECT signature, native_delta FROM native_flow_cache",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)),
            )
            .unwrap(),
        ("signature".to_owned(), -1.25)
    );
    let first = schema(&monitor);
    drop(monitor);

    boot(dir.path());
    let monitor = open(dir.path(), "wallet.db");
    assert_eq!(schema(&monitor), first, "second open changed the schema");
    assert_canonical_names(&monitor);
    assert_eq!(
        monitor
            .query_row(
                "SELECT COUNT(*), SUM(native_balance) FROM wallet_snapshots",
                [],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?)),
            )
            .unwrap(),
        (1, 1.5)
    );
}

#[test]
fn wallet_monitor_with_both_names_refuses_to_open() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    for (conflict, expected) in [
        (
            "CREATE TABLE native_flow_cache (signature TEXT PRIMARY KEY, native_delta REAL NOT NULL DEFAULT 0)",
            "sol_flow_cache and native_flow_cache",
        ),
        (
            "ALTER TABLE wallet_snapshots ADD COLUMN native_balance REAL",
            "wallet_snapshots.sol_balance and wallet_snapshots.native_balance",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("data")).unwrap();
        seed_main_wallet(dir.path());
        seed_legacy_monitor(dir.path());
        let monitor = open(dir.path(), "wallet.db");
        monitor.execute_batch(conflict).unwrap();
        let before = schema(&monitor);
        drop(monitor);

        let output = run_initializers(dir.path());
        assert!(!output.status.success(), "conflicting names were accepted");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains("SchemaConflict") && text.contains("wallet.db") && text.contains(expected),
            "unexpected failure: {text}"
        );

        let monitor = open(dir.path(), "wallet.db");
        assert_eq!(schema(&monitor), before, "refused open changed the schema");
    }
}

/// A released build reopening a renamed store re-adds `total_equity_sol` and re-creates
/// `sol_flow_cache` in whichever shape it knows; each writer's rows then sit under the
/// name it used, and the next open folds them into the canonical names.
#[test]
fn flows_and_equity_from_both_builds_merge_on_open() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    for legacy_flow_table in [
        "CREATE TABLE IF NOT EXISTS sol_flow_cache (chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL DEFAULT '', signature TEXT NOT NULL, timestamp TEXT NOT NULL, sol_delta REAL NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY (chain_id, wallet_address, signature)); CREATE INDEX IF NOT EXISTS idx_flow_cache_chain_wallet_timestamp ON sol_flow_cache(chain_id, wallet_address, timestamp DESC);",
        "CREATE TABLE IF NOT EXISTS sol_flow_cache (signature TEXT PRIMARY KEY, timestamp TEXT NOT NULL, sol_delta REAL NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')));",
    ] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("data")).unwrap();
        seed_main_wallet(dir.path());
        boot(dir.path());

        let monitor = open(dir.path(), "wallet.db");
        monitor
            .execute_batch(
                "INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, native_balance, native_balance_raw, total_equity_native) VALUES (1, 'address', '2025-01-01T00:00:00+00:00', 1.0, 1000000000, 4.5);
                 INSERT INTO native_flow_cache (wallet_address, signature, timestamp, native_delta) VALUES ('address', 'shared', '2025-01-01', -1.0);
                 INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('current_wallet', 'address');
                 ALTER TABLE wallet_snapshots ADD COLUMN total_equity_sol REAL;",
            )
            .unwrap();
        monitor.execute_batch(legacy_flow_table).unwrap();
        monitor
            .execute_batch(
                "INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, native_balance, native_balance_raw, total_equity_sol) VALUES (2, 'address', '2025-01-02T00:00:00+00:00', 2.0, 2000000000, 7.5);
                 INSERT INTO sol_flow_cache (signature, timestamp, sol_delta) VALUES ('shared', '2025-01-01', -9.0);
                 INSERT INTO sol_flow_cache (signature, timestamp, sol_delta) VALUES ('legacy', '2025-01-02', 2.25);",
            )
            .unwrap();
        drop(monitor);

        boot(dir.path());
        let monitor = open(dir.path(), "wallet.db");
        assert_canonical_names(&monitor);
        let equity: Vec<(i64, Option<f64>)> = monitor
            .prepare("SELECT id, total_equity_native FROM wallet_snapshots ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(equity, vec![(1, Some(4.5)), (2, Some(7.5))]);
        let flows: Vec<(String, String, f64)> = monitor
            .prepare("SELECT wallet_address, signature, native_delta FROM native_flow_cache WHERE chain_id = 'solana' ORDER BY signature, wallet_address")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let flow = |address: &str, signature: &str, delta: f64| {
            (address.to_owned(), signature.to_owned(), delta)
        };
        // The keyed shape keeps its rows under the empty address its writer stored, so the
        // shared signature is a distinct key; the pre-chain shape takes the recorded wallet
        // and the canonical row wins the shared key.
        let expected = if legacy_flow_table.contains("chain_id") {
            vec![
                flow("", "legacy", 2.25),
                flow("", "shared", -9.0),
                flow("address", "shared", -1.0),
            ]
        } else {
            vec![flow("address", "legacy", 2.25), flow("address", "shared", -1.0)]
        };
        assert_eq!(flows, expected);
        assert_eq!(
            monitor
                .query_row(
                    "SELECT tbl_name FROM sqlite_master WHERE type = 'index' AND name = 'idx_flow_cache_chain_wallet_timestamp'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap(),
            "native_flow_cache"
        );
        let first = schema(&monitor);
        drop(monitor);

        boot(dir.path());
        let monitor = open(dir.path(), "wallet.db");
        assert_eq!(schema(&monitor), first, "a further open changed the schema");
    }
}

#[test]
fn fresh_wallet_monitor_opens_with_unit_neutral_names() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    seed_main_wallet(dir.path());

    boot(dir.path());
    let monitor = open(dir.path(), "wallet.db");
    assert_canonical_names(&monitor);
    let first = schema(&monitor);
    drop(monitor);

    boot(dir.path());
    let monitor = open(dir.path(), "wallet.db");
    assert_eq!(schema(&monitor), first, "second open changed the schema");
}
