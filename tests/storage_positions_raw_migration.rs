// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Released position storage through the production initializer in fresh processes.

mod common;

use rusqlite::{params, types::Value, Connection};
use std::{path::Path, process::Command};

const CHILD: &str = "SCREENERBOT_POSITION_AMOUNT_CHILD";
const CASES: [u64; 4] = [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX];

/// Positions storage as v0.2.13 leaves it, by the release that first created the file.
/// The column layout of `positions` depends on that release, not on the one installed.
const CREATED_BY: [(&str, &str); 4] = [
    (
        "v0.1.116",
        include_str!("fixtures/v0.2.13-positions-created-v0.1.116.sql"),
    ),
    (
        "v0.1.117",
        include_str!("fixtures/v0.2.13-positions-created-v0.1.117.sql"),
    ),
    (
        "v0.1.121",
        include_str!("fixtures/v0.2.13-positions-created-v0.1.121.sql"),
    ),
    ("v0.2.0", include_str!("fixtures/v0.2.13-positions.sql")),
];

/// The indexes releases before chain identity created on `positions`. A pre-v0.2.0 build
/// reopening an upgraded file creates them again.
const LEGACY_INDEXES: &str = "
CREATE INDEX IF NOT EXISTS idx_positions_wallet ON positions(wallet_address);
CREATE INDEX IF NOT EXISTS idx_positions_mint ON positions(mint);
CREATE INDEX IF NOT EXISTS idx_positions_entry_signature ON positions(entry_transaction_signature);
CREATE INDEX IF NOT EXISTS idx_positions_exit_signature ON positions(exit_transaction_signature);
";

const WITHOUT_LEGACY_INDEXES: &str = "
DROP INDEX IF EXISTS idx_positions_wallet;
DROP INDEX IF EXISTS idx_positions_mint;
DROP INDEX IF EXISTS idx_positions_entry_signature;
DROP INDEX IF EXISTS idx_positions_exit_signature;
";

fn open(dir: &Path) -> Connection {
    Connection::open(dir.join("data/positions.db")).unwrap()
}

/// The store's bytes with no sidecar left: an open that changed nothing leaves the main
/// file byte for byte as it was.
fn store_file(dir: &Path) -> Vec<u8> {
    let data = dir.join("data");
    assert!(
        !data.join("positions.db-wal").exists(),
        "a WAL sidecar remains"
    );
    std::fs::read(data.join("positions.db")).unwrap()
}

fn has_column(conn: &Connection, table: &str, name: &str) -> bool {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .any(|column| column.unwrap() == name)
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

fn seed(dir: &Path, sql: &str) -> Connection {
    common::seed_store(
        &dir.join("data/positions.db"),
        screenerbot::database::POSITIONS_DB,
        sql,
    )
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), include_str!("fixtures/v0.2.13-positions.sql"));
    dir
}

/// Every stored object. `ALTER TABLE ... RENAME` stores the new table name quoted; the
/// quotes are removed so a rebuilt table compares equal to a freshly created one.
fn schema(conn: &Connection) -> Vec<(String, String, Option<String>)> {
    conn.prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .unwrap()
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let sql: Option<String> = row.get(2)?;
            let sql = sql.map(|sql| {
                sql.replacen(
                    &format!("CREATE TABLE \"{name}\""),
                    &format!("CREATE TABLE {name}"),
                    1,
                )
            });
            Ok((row.get(0)?, name, sql))
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn sequences(conn: &Connection) -> Vec<(String, i64)> {
    conn.prepare("SELECT name, seq FROM sqlite_sequence ORDER BY name")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

/// Every row of every table, in rowid order.
fn contents(conn: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
                .unwrap();
            let width = stmt.column_count();
            let rows = stmt
                .query_map([], |row| {
                    (0..width)
                        .map(|i| row.get::<_, Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}

fn assert_checks_pass(conn: &Connection, case: &str) {
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "ok",
        "{case}"
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        0,
        "{case}"
    );
}

/// Synthetic rows every released layout holds: a closed auto-trader position, an open one,
/// a manual one, an entry and an exit, and a deleted last position id.
fn seed_released_rows(conn: &Connection, legacy_manual_flag: bool) {
    let insert = "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, sol_received, entry_fee_lamports, exit_fee_lamports, token_amount, remaining_token_amount, total_exited_amount, pnl, archived) VALUES (?1, 'solana', 'wallet', ?2, 'SYM', 'Token', 0.5, '2025-01-01T00:00:00Z', ?3, ?4, 'buy', 1.5, 2.5, 0.75, 0.25, ?5, 11, 13, ?6, ?7, ?8, ?9, 0)";
    conn.execute(
        insert,
        params![
            1,
            "mint-closed",
            0.75,
            "2025-01-02T00:00:00Z",
            3.25,
            CASES[3] as i64,
            CASES[0] as i64,
            CASES[3] as i64,
            1.75
        ],
    )
    .unwrap();
    conn.execute(
        insert,
        params![
            2,
            "mint-open",
            None::<f64>,
            None::<String>,
            None::<f64>,
            CASES[1] as i64,
            CASES[1] as i64,
            CASES[0] as i64,
            None::<f64>
        ],
    )
    .unwrap();
    conn.execute(
        insert,
        params![
            3,
            "mint-manual",
            None::<f64>,
            None::<String>,
            None::<f64>,
            CASES[2] as i64,
            CASES[2] as i64,
            CASES[0] as i64,
            None::<f64>
        ],
    )
    .unwrap();
    if legacy_manual_flag {
        conn.execute(
            "UPDATE positions SET manual_management = 1 WHERE id = 3",
            [],
        )
        .unwrap();
    } else {
        conn.execute(
            "UPDATE positions SET origin_kind = 'manual', management = 'user_only' WHERE id = 3",
            [],
        )
        .unwrap();
    }
    conn.execute(
        insert,
        params![
            4,
            "mint-deleted",
            None::<f64>,
            None::<String>,
            None::<f64>,
            1,
            1,
            0,
            None::<f64>
        ],
    )
    .unwrap();
    conn.execute("DELETE FROM positions WHERE id = 4", [])
        .unwrap();
    conn.execute("INSERT INTO position_entries (id, position_id, wallet_address, timestamp, amount, price, sol_spent, transaction_signature, is_dca, fees_lamports) VALUES (1, 1, 'wallet', '2025-01-01T00:00:00Z', ?1, 0.5, 1.5, 'entry-1', 0, 11)", [CASES[3] as i64]).unwrap();
    conn.execute("INSERT INTO position_exits (id, position_id, wallet_address, timestamp, amount, price, sol_received, transaction_signature, is_partial, percentage, fees_lamports) VALUES (1, 1, 'wallet', '2025-01-02T00:00:00Z', ?1, 0.75, 3.25, 'exit-1', 0, 100.0, 13)", [CASES[2] as i64]).unwrap();
    conn.execute("INSERT INTO position_states (id, position_id, state, reason) VALUES (1, 2, 'Open', 'kept')", []).unwrap();
    conn.execute(
        "INSERT INTO position_metadata (key, value) VALUES ('schema_version', '5')",
        [],
    )
    .unwrap();
}

#[test]
fn every_released_layout_upgrades_to_a_fresh_install_and_reopens_unchanged() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let fresh = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(fresh.path().join("data")).unwrap();
    boot(fresh.path(), true);
    let fresh_schema = schema(&open(fresh.path()));

    for (created_by, sql) in CREATED_BY {
        for legacy_indexes in [false, true] {
            let case = format!("created by {created_by}, legacy indexes {legacy_indexes}");
            let dir = tempfile::tempdir().unwrap();
            let conn = seed(dir.path(), sql);
            conn.execute_batch(if legacy_indexes {
                LEGACY_INDEXES
            } else {
                WITHOUT_LEGACY_INDEXES
            })
            .unwrap();
            let legacy_manual_flag = has_column(&conn, "positions", "manual_management");
            seed_released_rows(&conn, legacy_manual_flag);
            let released_sequences = sequences(&conn);
            assert_eq!(
                released_sequences,
                [
                    ("position_entries".to_owned(), 1),
                    ("position_exits".to_owned(), 1),
                    ("position_states".to_owned(), 1),
                    ("positions".to_owned(), 4),
                ],
                "{case}"
            );
            drop(conn);

            boot(dir.path(), true);
            let conn = open(dir.path());
            assert_eq!(schema(&conn), fresh_schema, "{case}");
            assert_eq!(sequences(&conn), released_sequences, "{case}");
            assert_checks_pass(&conn, &case);
            let positions: Vec<(i64, String, String, String, String, String, f64, i64)> = conn
                .prepare("SELECT id, token_amount, remaining_token_amount, total_exited_amount, origin_kind, management, native_received, entry_fee_raw FROM positions ORDER BY id")
                .unwrap()
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
                        row.get(7)?,
                    ))
                })
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            let amount = |i: usize| CASES[i].to_string();
            assert_eq!(
                positions,
                [
                    (
                        1,
                        amount(3),
                        amount(0),
                        amount(3),
                        "auto".into(),
                        "auto_trader".into(),
                        3.25,
                        11
                    ),
                    (
                        2,
                        amount(1),
                        amount(1),
                        amount(0),
                        "auto".into(),
                        "auto_trader".into(),
                        0.0,
                        11
                    ),
                    (
                        3,
                        amount(2),
                        amount(2),
                        amount(0),
                        "manual".into(),
                        "user_only".into(),
                        0.0,
                        11
                    ),
                ],
                "{case}"
            );
            assert_eq!(
                conn.query_row(
                    "SELECT e.amount, e.native_spent, e.fees_raw, x.amount, x.native_received, x.fees_raw FROM position_entries e, position_exits x WHERE e.id = 1 AND x.id = 1",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?, row.get::<_, f64>(4)?, row.get::<_, i64>(5)?)),
                )
                .unwrap(),
                (amount(3), 1.5, 11, amount(2), 3.25, 13),
                "{case}"
            );

            let (backup, manifest) = screenerbot::database::backup::upgrade_backup_paths(
                &dir.path().join("data/positions.db"),
            )
            .unwrap();
            let manifest: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(manifest).unwrap()).unwrap();
            assert_eq!(manifest["fromSchemaVersion"], "5", "{case}");
            let backup = Connection::open(backup).unwrap();
            assert!(has_column(&backup, "positions", "entry_size_sol"), "{case}");
            assert_eq!(sequences(&backup), released_sequences, "{case}");

            let upgraded = (schema(&conn), contents(&conn));
            drop(conn);
            boot(dir.path(), true);
            let conn = open(dir.path());
            assert_eq!(
                (schema(&conn), contents(&conn)),
                upgraded,
                "{case}: second launch"
            );
        }
    }
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
        conn.execute("INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, sol_received, entry_fee_lamports, exit_fee_lamports, token_amount, remaining_token_amount, total_exited_amount, origin_kind, management, round_key, basis_complete, history_complete, holding_state, pnl, archived, created_at, updated_at) VALUES (?1, 'solana', 'wallet', ?2, 'S', 'Name', 1.25, '2025-01-01', 'buy', 2.5, 3.5, 4.5, 0.5, 7.5, 11, 13, ?3, ?3, ?3, 'copy', 'copy_task', ?4, 0, 1, 'frozen', 6.5, 1, '2025-01-01', '2025-01-02')", params![id, format!("mint-{i}"), *amount as i64, format!("round-{i}")]).unwrap();
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
    let pending: Vec<serde_json::Value> = CASES
        .iter()
        .enumerate()
        .map(|(i, amount)| {
            serde_json::json!({
                "signature": format!("pending-{i}"),
                "mint": format!("mint-{i}"),
                "position_id": 21 + i as i64,
                "expected_exit_amount": amount,
                "requested_exit_percentage": 25.0,
                "expiry_height": null,
                "created_at": "2025-01-02T00:00:00Z"
            })
        })
        .collect();
    conn.execute(
        "INSERT INTO position_metadata (key, value) VALUES ('pending_partial_exits', ?1)",
        [serde_json::Value::Array(pending).to_string()],
    )
    .unwrap();
    drop(conn);

    boot(dir.path(), true);
    boot(dir.path(), true);
    let conn = open(dir.path());
    for (table, old, new) in [
        ("positions", "entry_size_sol", "entry_size_native"),
        ("positions", "total_size_sol", "total_size_native"),
        ("positions", "sol_received", "native_received"),
        ("positions", "entry_fee_lamports", "entry_fee_raw"),
        ("positions", "exit_fee_lamports", "exit_fee_raw"),
        ("position_exits", "sol_received", "native_received"),
        ("position_exits", "fees_lamports", "fees_raw"),
        ("position_entries", "sol_spent", "native_spent"),
        ("position_entries", "fees_lamports", "fees_raw"),
    ] {
        assert!(has_column(&conn, table, new), "missing {table}.{new}");
        assert!(
            !has_column(&conn, table, old),
            "legacy column remains: {table}.{old}"
        );
    }
    assert_eq!(
        conn.query_row(
            "SELECT entry_size_native, total_size_native, native_received, entry_fee_raw, exit_fee_raw FROM positions WHERE id = 21",
            [],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))
        )
        .unwrap(),
        (2.5, 3.5, 7.5, 11, 13)
    );
    assert_eq!(
        conn.query_row(
            "SELECT native_spent, fees_raw FROM position_entries WHERE id = 31",
            [],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?))
        )
        .unwrap(),
        (2.5, 99)
    );
    assert_eq!(
        conn.query_row(
            "SELECT native_received, fees_raw FROM position_exits WHERE id = 41",
            [],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?))
        )
        .unwrap(),
        (3.25, 88)
    );
    assert_eq!(
        conn.query_row(
            "SELECT price_sol FROM token_snapshots WHERE id = 63",
            [],
            |row| row.get::<_, f64>(0)
        )
        .unwrap(),
        1.5
    );
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
    assert_eq!(index_count, 21);
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_positions_open_round'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        1,
        "the one-open-position index is missing after initialization"
    );
    assert_eq!(
        conn.query_row(
            "SELECT value FROM position_metadata WHERE key = 'sentinel'",
            [],
            |row| row.get::<_, String>(0)
        )
        .unwrap(),
        "kept"
    );
    let stored: String = conn
        .query_row(
            "SELECT value FROM position_metadata WHERE key = 'pending_partial_exits'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let migrated: serde_json::Value = serde_json::from_str(&stored).unwrap();
    let rehydrated: Vec<screenerbot::positions::PendingPartialExit> =
        serde_json::from_str(&stored).unwrap();
    assert_eq!(rehydrated.len(), CASES.len());
    for (i, amount) in CASES.iter().enumerate() {
        assert_eq!(
            migrated[i]["expected_exit_amount"],
            serde_json::Value::String(amount.to_string())
        );
        assert_eq!(
            rehydrated[i].expected_exit_amount,
            screenerbot::chains::RawAmount::from(*amount)
        );
    }
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
        "CREATE TRIGGER unknown_entry_trigger AFTER INSERT ON position_entries BEGIN SELECT 1; END",
        "ALTER TABLE position_exits ADD COLUMN unknown_note TEXT",
    ] {
        let dir = fixture();
        let conn = open(dir.path());
        conn.execute("INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, token_amount) VALUES (21, 'solana', 'wallet', 'mint', 'S', 'Name', 1, '2025-01-01', 'buy', 1, 1, 1, 1, -1)", []).unwrap();
        conn.execute_batch(object).unwrap();
        let before: Vec<(String, String)> = conn.prepare("SELECT type, sql FROM sqlite_master WHERE tbl_name = 'positions' ORDER BY type, name").unwrap().query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        drop(conn);
        let file = store_file(dir.path());
        boot(dir.path(), false);
        assert_eq!(
            store_file(dir.path()),
            file,
            "{object}: the refused file changed"
        );
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
    let file = store_file(dir.path());
    boot(dir.path(), false);
    assert_eq!(store_file(dir.path()), file, "the refused file changed");
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
