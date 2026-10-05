// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Released position storage through the production initializer in fresh processes.

use rusqlite::{params, Connection};
use std::{path::Path, process::Command};

const CHILD: &str = "SCREENERBOT_POSITION_AMOUNT_CHILD";
const CASES: [u64; 4] = [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];

fn open(dir: &Path) -> Connection {
    Connection::open(dir.join("data/positions.db")).unwrap()
}

fn boot(dir: &Path, succeeds: bool) {
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("released_position_amounts_survive_initialization")
        .env(CHILD, "1")
        .env("SCREENERBOT_DATA_DIR", dir)
        .status()
        .unwrap();
    assert_eq!(status.success(), succeeds);
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    open(dir.path())
        .execute_batch(include_str!("fixtures/v0.2.13-positions.sql"))
        .unwrap();
    dir
}

#[test]
fn released_position_amounts_survive_initialization() {
    if std::env::var_os(CHILD).is_some() {
        screenerbot::paths::ensure_all_directories().unwrap();
        screenerbot::config::load_config().unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(screenerbot::positions::initialize_positions_database());
        if result.is_err() {
            panic!("position initializer failed: {result:?}");
        }
        return;
    }
    let dir = fixture();
    let conn = open(dir.path());
    for (i, amount) in CASES.iter().enumerate() {
        let id = 21 + i as i64;
        conn.execute("INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount, remaining_token_amount, total_exited_amount, origin_kind, management, round_key, basis_complete, history_complete, holding_state, pnl, archived, created_at, updated_at) VALUES (?1, 'solana', 'wallet', ?2, 'S', 'Name', 1.25, '2025-01-01', 'buy', 2.5, 3.5, 4.5, 0.5, ?3, ?3, ?3, 'copy', 'copy_task', ?4, 0, 1, 'frozen', 6.5, 1, '2025-01-01', '2025-01-02')", params![id, format!("mint-{i}"), *amount as i64, format!("round-{i}")]).unwrap();
        conn.execute("INSERT INTO position_entries (id, position_id, wallet_address, timestamp, amount, price, sol_spent, transaction_signature, is_dca, fees_lamports) VALUES (?1, ?2, 'wallet', '2025-01-01', ?3, 1.25, 2.5, ?4, 1, 99)", params![31 + i as i64, id, *amount as i64, format!("entry-{i}")]).unwrap();
        conn.execute("INSERT INTO position_exits (id, position_id, wallet_address, timestamp, amount, price, sol_received, transaction_signature, is_partial, percentage, fees_lamports) VALUES (?1, ?2, 'wallet', '2025-01-02', ?3, 1.5, 3.25, ?4, 1, 25.0, 88)", params![41 + i as i64, id, *amount as i64, format!("exit-{i}")]).unwrap();
    }
    conn.execute("INSERT INTO position_states (id, position_id, state, reason) VALUES (61, 21, 'Open', 'kept')", []).unwrap();
    conn.execute("INSERT INTO position_tracking (id, position_id, price, price_source, pool_address) VALUES (62, 21, 1.5, 'pool', 'pool-kept')", []).unwrap();
    conn.execute("INSERT INTO token_snapshots (id, position_id, snapshot_type, mint, symbol, price_sol, volume_h24) VALUES (63, 21, 'opening', 'mint-0', 'S', 1.5, 42.0)", []).unwrap();
    conn.execute(
        "INSERT INTO position_metadata (key, value) VALUES ('sentinel', 'kept')",
        [],
    )
    .unwrap();
    drop(conn);

    boot(dir.path(), true);
    boot(dir.path(), true);
    let conn = open(dir.path());
    for (i, amount) in CASES.iter().enumerate() {
        for (table, id, column) in [
            ("positions", 21 + i as i64, "token_amount"),
            ("positions", 21 + i as i64, "remaining_token_amount"),
            ("positions", 21 + i as i64, "total_exited_amount"),
            ("position_entries", 31 + i as i64, "amount"),
            ("position_exits", 41 + i as i64, "amount"),
        ] {
            let (value, kind): (String, String) = conn
                .query_row(
                    &format!("SELECT {column}, typeof({column}) FROM {table} WHERE id = ?1"),
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!((value, kind), (amount.to_string(), "text".to_owned()));
        }
    }
    assert_eq!(conn.query_row("SELECT origin_kind, management, round_key, basis_complete, history_complete, holding_state, pnl, archived, created_at, updated_at FROM positions WHERE id = 21", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, String>(5)?, row.get::<_, f64>(6)?, row.get::<_, i64>(7)?, row.get::<_, String>(8)?, row.get::<_, String>(9)?))).unwrap(), ("copy".into(), "copy_task".into(), "round-0".into(), 0, 1, "frozen".into(), 6.5, 1, "2025-01-01".into(), "2025-01-02".into()));
    for (table, id) in [
        ("position_states", 61),
        ("position_tracking", 62),
        ("token_snapshots", 63),
    ] {
        assert_eq!(
            conn.query_row(
                &format!("SELECT position_id FROM {table} WHERE id = ?1"),
                [id],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            21
        );
    }
    for (table, count) in [
        ("positions", 4),
        ("position_entries", 4),
        ("position_exits", 4),
    ] {
        assert_eq!(
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            count
        );
    }
    for (table, id, column, expected) in [
        ("position_entries", 31, "transaction_signature", "entry-0"),
        ("position_exits", 41, "transaction_signature", "exit-0"),
        ("position_tracking", 62, "pool_address", "pool-kept"),
    ] {
        assert_eq!(
            conn.query_row(
                &format!("SELECT {column} FROM {table} WHERE id = ?1"),
                [id],
                |row| row.get::<_, String>(0)
            )
            .unwrap(),
            expected
        );
    }
    let index_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_position%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(index_count, 20);
    assert_eq!(
        conn.query_row(
            "SELECT value FROM position_metadata WHERE key = 'sentinel'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "kept"
    );
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
    conn.execute(
        "UPDATE positions SET token_amount = '18446744073709551616' WHERE id = 21",
        [],
    )
    .unwrap();
    drop(conn);
    boot(dir.path(), true);
    let conn = open(dir.path());
    assert_eq!(
        conn.query_row(
            "SELECT token_amount, typeof(token_amount) FROM positions WHERE id = 21",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        )
        .unwrap(),
        ("18446744073709551616".into(), "text".into())
    );
}

#[test]
fn unknown_position_objects_preserve_original_storage() {
    for object in [
        "CREATE INDEX unknown_amount_index ON positions(token_amount)",
        "CREATE TRIGGER unknown_amount_trigger AFTER INSERT ON positions BEGIN SELECT 1; END",
        "ALTER TABLE positions ADD COLUMN unknown_amount INTEGER",
    ] {
        let dir = fixture();
        let conn = open(dir.path());
        conn.execute("INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount) VALUES (21, 'solana', 'wallet', 'mint', 'S', 'Name', 1, '2025-01-01', 'buy', 1, 1, 1, 1, -1)", []).unwrap();
        conn.execute_batch(object).unwrap();
        let before: Vec<(String, String)> = conn.prepare("SELECT type, sql FROM sqlite_master WHERE tbl_name = 'positions' ORDER BY type, name").unwrap().query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        drop(conn);
        boot(dir.path(), false);
        let conn = open(dir.path());
        let after: Vec<(String, String)> = conn.prepare("SELECT type, sql FROM sqlite_master WHERE tbl_name = 'positions' ORDER BY type, name").unwrap().query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        assert_eq!(after, before);
        assert_eq!(
            conn.query_row(
                "SELECT token_amount FROM positions WHERE id = 21",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            -1
        );
    }
}

#[test]
fn malformed_legacy_amount_preserves_original_storage() {
    let dir = fixture();
    let conn = open(dir.path());
    conn.execute("INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount) VALUES (21, 'solana', 'wallet', 'mint', 'S', 'Name', 1, '2025-01-01', 'buy', 1, 1, 1, 1, 'malformed')", []).unwrap();
    drop(conn);
    boot(dir.path(), false);
    let conn = open(dir.path());
    assert_eq!(
        conn.query_row(
            "SELECT token_amount, typeof(token_amount) FROM positions WHERE id = 21",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        )
        .unwrap(),
        ("malformed".into(), "text".into())
    );
    assert_eq!(
        conn.query_row(
            "SELECT type FROM pragma_table_info('positions') WHERE name = 'token_amount'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "INTEGER"
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
}
