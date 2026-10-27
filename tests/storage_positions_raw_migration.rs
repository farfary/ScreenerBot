// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Released position storage through the production initializer in fresh processes.

use crate::common;

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

/// A released-layout position row: id, mint, exit price, exit time, native received, token
/// amount, remaining amount, exited amount and P&L.
const INSERT_RELEASED_POSITION: &str = "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest, sol_received, entry_fee_lamports, exit_fee_lamports, token_amount, remaining_token_amount, total_exited_amount, pnl, archived) VALUES (?1, 'solana', 'wallet', ?2, 'SYM', 'Token', 0.5, '2025-01-01T00:00:00Z', ?3, ?4, 'buy', 1.5, 2.5, 0.75, 0.25, ?5, 11, 13, ?6, ?7, ?8, ?9, 0)";

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
        .arg(common::test_name(
            module_path!(),
            "released_position_amounts_survive_initialization",
        ))
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
    let insert = INSERT_RELEASED_POSITION;
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
            // The closed row took two adds; the others never did.
            conn.execute(
                "UPDATE positions SET dca_count = 2, average_entry_price = 0.4 WHERE id = 1",
                [],
            )
            .unwrap();
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
            // The closed row keeps its maximum; the open rows hold a released average of 0,
            // never their first fill, so their peak starts at that average.
            assert_eq!(
                assert_basis_peak_backfilled(&conn, &case),
                [0.75, 0.0, 0.0],
                "{case}"
            );
            assert_eq!(take_profit_levels(&conn), [0, 0, 0], "{case}");
            assert_eq!(
                entry_liquidities(&conn),
                [(None, None), (None, None), (None, None)],
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

/// Every row's basis peak, as the open that added the column seeded it, read against the
/// columns the seed reads (none of which the later upgrade steps change on these fixtures):
/// the running maximum of a closed row and of an open row whose average is still its first
/// fill without adds, the average entry of any other open row. Returns the basis peaks in
/// row order.
fn assert_basis_peak_backfilled(conn: &Connection, case: &str) -> Vec<f64> {
    let rows: Vec<(i64, bool, i64, f64, f64, f64, f64)> = conn
        .prepare("SELECT id, exit_time IS NOT NULL, dca_count, COALESCE(effective_entry_price, entry_price), price_highest, average_entry_price, basis_peak_price FROM positions ORDER BY id")
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    let mut basis_peaks = Vec::with_capacity(rows.len());
    for (id, closed, dca_count, first_fill, price_highest, average_entry_price, basis_peak_price) in
        rows
    {
        let expected = if closed || (dca_count == 0 && average_entry_price == first_fill) {
            price_highest
        } else {
            average_entry_price
        };
        assert_eq!(basis_peak_price, expected, "{case}: row {id}");
        basis_peaks.push(basis_peak_price);
    }
    basis_peaks
}

/// The positions store as the release before the basis peak leaves it: the current layout
/// without `basis_peak_price` and the later `take_profit_level`, `entry_liquidity_native`
/// and `entry_liquidity_pool`, its ledger and legacy stamp at version 6.
const BEFORE_BASIS_PEAK: &str = "
    ALTER TABLE positions DROP COLUMN entry_liquidity_pool;
    ALTER TABLE positions DROP COLUMN entry_liquidity_native;
    ALTER TABLE positions DROP COLUMN take_profit_level;
    ALTER TABLE positions DROP COLUMN basis_peak_price;
    UPDATE store_schema SET version = 6, min_reader_version = 6 WHERE id = 1;
    UPDATE position_metadata SET value = '6' WHERE key = 'schema_version';
";

/// Rows of a version 6 store: an open row never averaged into, a closed row that took two
/// adds, an open manual row, and an open row a buy made elsewhere averaged down without an
/// add.
const INSERT_VERSION_6_POSITION: &str = "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time, position_type, entry_size_native, total_size_native, average_entry_price, price_highest, price_lowest, dca_count) VALUES (?1, 'solana', 'wallet', ?2, 'SYM', 'Token', 1.0, '2026-01-01T00:00:00Z', ?3, ?4, 'buy', 1.0, ?5, ?6, ?7, 0.5, ?8)";

/// The columns a version 6 store held, as the upgrade must keep them.
fn version_6_columns(conn: &Connection) -> Vec<Vec<Value>> {
    let mut stmt = conn
        .prepare("SELECT id, mint, exit_price, exit_time, total_size_native, average_entry_price, price_highest, price_lowest, dca_count, origin_kind, management FROM positions ORDER BY id")
        .unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |row| {
        (0..width)
            .map(|i| row.get::<_, Value>(i))
            .collect::<rusqlite::Result<Vec<_>>>()
    })
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}

#[test]
fn a_store_before_the_basis_peak_backfills_it_and_reopens_unchanged() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let fresh = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(fresh.path().join("data")).unwrap();
    boot(fresh.path(), true);
    let fresh_schema = schema(&open(fresh.path()));

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    boot(dir.path(), true);
    let conn = open(dir.path());
    conn.execute_batch(BEFORE_BASIS_PEAK).unwrap();
    assert!(!has_column(&conn, "positions", "basis_peak_price"));
    conn.execute(
        INSERT_VERSION_6_POSITION,
        params![
            1,
            "mint-open",
            None::<f64>,
            None::<String>,
            1.0,
            1.0,
            1.5,
            0
        ],
    )
    .unwrap();
    conn.execute(
        INSERT_VERSION_6_POSITION,
        params![
            2,
            "mint-averaged",
            0.9,
            "2026-01-02T00:00:00Z",
            2.0,
            0.8,
            1.6,
            2
        ],
    )
    .unwrap();
    conn.execute(
        INSERT_VERSION_6_POSITION,
        params![
            3,
            "mint-manual",
            None::<f64>,
            None::<String>,
            1.0,
            1.0,
            1.25,
            0
        ],
    )
    .unwrap();
    conn.execute(
        "UPDATE positions SET origin_kind = 'manual', management = 'user_only' WHERE id = 3",
        [],
    )
    .unwrap();
    conn.execute(
        INSERT_VERSION_6_POSITION,
        params![
            4,
            "mint-grown",
            None::<f64>,
            None::<String>,
            2.0,
            0.75,
            1.4,
            0
        ],
    )
    .unwrap();
    let before = version_6_columns(&conn);
    drop(conn);

    let case = "version 6 store";
    boot(dir.path(), true);
    let conn = open(dir.path());
    assert_eq!(schema(&conn), fresh_schema, "{case}");
    assert_checks_pass(&conn, case);
    assert_eq!(version_6_columns(&conn), before, "{case}: stored data");
    assert_eq!(
        assert_basis_peak_backfilled(&conn, case),
        [1.5, 1.6, 1.25, 0.75],
        "{case}"
    );
    assert_eq!(take_profit_levels(&conn), [0, 0, 0, 0], "{case}");
    assert_eq!(entry_liquidities(&conn), vec![(None, None); 4], "{case}");

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

/// Every row's reserved take-profit steps, in row order.
fn take_profit_levels(conn: &Connection) -> Vec<i64> {
    conn.prepare("SELECT take_profit_level FROM positions ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

/// The positions store as the release before the take-profit ladder leaves it: the
/// current layout without `take_profit_level` and the later `entry_liquidity_native` and
/// `entry_liquidity_pool`, its ledger and legacy stamp at version 7.
const BEFORE_TAKE_PROFIT_LADDER: &str = "
    ALTER TABLE positions DROP COLUMN entry_liquidity_pool;
    ALTER TABLE positions DROP COLUMN entry_liquidity_native;
    ALTER TABLE positions DROP COLUMN take_profit_level;
    UPDATE store_schema SET version = 7, min_reader_version = 7 WHERE id = 1;
    UPDATE position_metadata SET value = '7' WHERE key = 'schema_version';
";

/// Rows of a version 7 store: an open row that took a partial exit, a closed row, and an
/// open manual row.
const INSERT_VERSION_7_POSITION: &str = "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time, position_type, entry_size_native, total_size_native, average_entry_price, price_highest, price_lowest, basis_peak_price, partial_exit_count, remaining_token_amount, total_exited_amount, origin_kind, management) VALUES (?1, 'solana', 'wallet', ?2, 'SYM', 'Token', 1.0, '2026-01-01T00:00:00Z', ?3, ?4, 'buy', 1.0, 1.0, 1.0, ?5, 0.5, ?5, ?6, ?7, ?8, ?9, ?10)";

/// Every column a version 7 store held, as the upgrade must keep them.
fn version_7_columns(conn: &Connection) -> Vec<Vec<Value>> {
    let columns = conn
        .prepare("SELECT name FROM pragma_table_info('positions') WHERE name NOT IN ('take_profit_level', 'entry_liquidity_native', 'entry_liquidity_pool') ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {} FROM positions ORDER BY id",
            columns.join(", ")
        ))
        .unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |row| {
        (0..width)
            .map(|i| row.get::<_, Value>(i))
            .collect::<rusqlite::Result<Vec<_>>>()
    })
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}

#[test]
fn a_store_before_the_take_profit_ladder_gains_it_and_reopens_unchanged() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let fresh = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(fresh.path().join("data")).unwrap();
    boot(fresh.path(), true);
    let fresh_schema = schema(&open(fresh.path()));

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    boot(dir.path(), true);
    let conn = open(dir.path());
    conn.execute_batch(BEFORE_TAKE_PROFIT_LADDER).unwrap();
    assert!(!has_column(&conn, "positions", "take_profit_level"));
    for (id, mint, exit_price, exit_time, peak, partials, remaining, exited, origin, management) in [
        (
            1,
            "mint-partial",
            None,
            None,
            1.4,
            1,
            "600",
            "400",
            "auto",
            "auto_trader",
        ),
        (
            2,
            "mint-closed",
            Some(1.2),
            Some("2026-01-02T00:00:00Z"),
            1.3,
            0,
            "0",
            "1000",
            "auto",
            "auto_trader",
        ),
        (
            3,
            "mint-manual",
            None,
            None,
            1.1,
            0,
            "1000",
            "0",
            "manual",
            "user_only",
        ),
    ] {
        conn.execute(
            INSERT_VERSION_7_POSITION,
            params![
                id, mint, exit_price, exit_time, peak, partials, remaining, exited, origin,
                management
            ],
        )
        .unwrap();
    }
    let before = version_7_columns(&conn);
    drop(conn);

    let case = "version 7 store";
    boot(dir.path(), true);
    let conn = open(dir.path());
    assert_eq!(schema(&conn), fresh_schema, "{case}");
    assert_checks_pass(&conn, case);
    assert_eq!(version_7_columns(&conn), before, "{case}: stored data");
    assert_eq!(take_profit_levels(&conn), [0, 0, 0], "{case}");
    assert_eq!(
        entry_liquidities(&conn),
        [(None, None), (None, None), (None, None)],
        "{case}"
    );
    assert_eq!(
        conn.query_row(
            "SELECT version, min_reader_version FROM store_schema WHERE id = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap(),
        (9, 7),
        "{case}: a version 7 core still reads the store"
    );

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

/// Every row's pool reserves at entry and the pool they were read from, in row order.
fn entry_liquidities(conn: &Connection) -> Vec<(Option<f64>, Option<String>)> {
    conn.prepare("SELECT entry_liquidity_native, entry_liquidity_pool FROM positions ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

/// The positions store as the release before the entry liquidity reading leaves it: the
/// current layout without `entry_liquidity_native` and `entry_liquidity_pool`, its ledger
/// and legacy stamp at version 8.
const BEFORE_ENTRY_LIQUIDITY: &str = "
    ALTER TABLE positions DROP COLUMN entry_liquidity_pool;
    ALTER TABLE positions DROP COLUMN entry_liquidity_native;
    UPDATE store_schema SET version = 8, min_reader_version = 7 WHERE id = 1;
    UPDATE position_metadata SET value = '8' WHERE key = 'schema_version';
";

/// Rows of a version 8 store: an open bot row past its first take-profit step, a closed
/// row, and an open manual row.
const INSERT_VERSION_8_POSITION: &str = "INSERT INTO positions (id, chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, exit_price, exit_time, position_type, entry_size_native, total_size_native, average_entry_price, price_highest, price_lowest, basis_peak_price, take_profit_level, origin_kind, management) VALUES (?1, 'solana', 'wallet', ?2, 'SYM', 'Token', 1.0, '2026-01-01T00:00:00Z', ?3, ?4, 'buy', 1.0, 1.0, 1.0, ?5, 0.5, ?5, ?6, ?7, ?8)";

/// Every column a version 8 store held, as the upgrade must keep them.
fn version_8_columns(conn: &Connection) -> Vec<Vec<Value>> {
    let columns = conn
        .prepare("SELECT name FROM pragma_table_info('positions') WHERE name NOT IN ('entry_liquidity_native', 'entry_liquidity_pool') ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {} FROM positions ORDER BY id",
            columns.join(", ")
        ))
        .unwrap();
    let width = stmt.column_count();
    stmt.query_map([], |row| {
        (0..width)
            .map(|i| row.get::<_, Value>(i))
            .collect::<rusqlite::Result<Vec<_>>>()
    })
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}

/// A store from before the entry liquidity reading gains its columns with no reading on
/// any row: no row measured its pool at entry, so a liquidity rule reads each as unknown.
#[test]
fn a_store_before_the_entry_liquidity_gains_it_and_reopens_unchanged() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let fresh = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(fresh.path().join("data")).unwrap();
    boot(fresh.path(), true);
    let fresh_schema = schema(&open(fresh.path()));

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    boot(dir.path(), true);
    let conn = open(dir.path());
    conn.execute_batch(BEFORE_ENTRY_LIQUIDITY).unwrap();
    assert!(!has_column(&conn, "positions", "entry_liquidity_native"));
    assert!(!has_column(&conn, "positions", "entry_liquidity_pool"));
    for (id, mint, exit_price, exit_time, peak, level, origin, management) in [
        (1, "mint-open", None, None, 1.4, 1, "auto", "auto_trader"),
        (
            2,
            "mint-closed",
            Some(1.2),
            Some("2026-01-02T00:00:00Z"),
            1.3,
            0,
            "auto",
            "auto_trader",
        ),
        (3, "mint-manual", None, None, 1.1, 0, "manual", "user_only"),
    ] {
        conn.execute(
            INSERT_VERSION_8_POSITION,
            params![id, mint, exit_price, exit_time, peak, level, origin, management],
        )
        .unwrap();
    }
    let before = version_8_columns(&conn);
    drop(conn);

    let case = "version 8 store";
    boot(dir.path(), true);
    let conn = open(dir.path());
    assert_eq!(schema(&conn), fresh_schema, "{case}");
    assert_checks_pass(&conn, case);
    assert_eq!(version_8_columns(&conn), before, "{case}: stored data");
    assert_eq!(
        entry_liquidities(&conn),
        [(None, None), (None, None), (None, None)],
        "{case}"
    );
    assert_eq!(
        conn.query_row(
            "SELECT version, min_reader_version FROM store_schema WHERE id = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap(),
        (9, 7),
        "{case}: a version 7 core still reads the store"
    );

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

/// One position row for the leg counter repair: its mint, whether it is open, its origin, the
/// counters it carries, and the records it holds as (entries flagged as adds, other entries,
/// partial exits, recorded closing sales).
struct CounterCase {
    id: i64,
    open: bool,
    origin: &'static str,
    dca_count: i64,
    partial_exit_count: i64,
    adds: i64,
    first_entries: i64,
    partials: i64,
    closing_sales: i64,
}

fn seed_counter_case(conn: &Connection, case: &CounterCase) {
    let exit_time = (!case.open).then_some("2025-01-02T00:00:00Z");
    conn.execute(
        INSERT_RELEASED_POSITION,
        params![
            case.id,
            format!("mint-{}", case.id),
            None::<f64>,
            exit_time,
            None::<f64>,
            1_000,
            1_000,
            0,
            None::<f64>
        ],
    )
    .unwrap();
    // The row's own entry is the first of its first-entry records; an add or a handover
    // record carries another swap's signature.
    conn.execute(
        "UPDATE positions SET origin_kind = ?2, dca_count = ?3, partial_exit_count = ?4, entry_transaction_signature = ?5 WHERE id = ?1",
        params![
            case.id,
            case.origin,
            case.dca_count,
            case.partial_exit_count,
            format!("entry-{}-1", case.id)
        ],
    )
    .unwrap();
    let mut record = 0;
    for (count, is_dca, kind) in [(case.first_entries, 0, "entry"), (case.adds, 1, "add")] {
        for _ in 0..count {
            record += 1;
            conn.execute(
                "INSERT INTO position_entries (position_id, wallet_address, timestamp, amount, price, sol_spent, transaction_signature, is_dca, fees_lamports) VALUES (?1, 'wallet', '2025-01-01T00:00:00Z', 10, 0.5, 1.5, ?2, ?3, 11)",
                params![case.id, format!("{kind}-{}-{record}", case.id), is_dca],
            )
            .unwrap();
        }
    }
    let mut exit = 0;
    for (count, is_partial) in [(case.partials, 1), (case.closing_sales, 0)] {
        for _ in 0..count {
            exit += 1;
            conn.execute(
                "INSERT INTO position_exits (position_id, wallet_address, timestamp, amount, price, sol_received, transaction_signature, is_partial, percentage, fees_lamports) VALUES (?1, 'wallet', '2025-01-02T00:00:00Z', 5, 0.75, 1.0, ?2, ?3, 25.0, 13)",
                params![case.id, format!("exit-{}-{exit}", case.id), is_partial],
            )
            .unwrap();
        }
    }
}

/// Open bot rows whose counters a release before the ledger stopped writing them left above
/// their records are lowered to those records by the production initializer, once; every
/// other row, and the records themselves, are kept as stored.
#[test]
fn inflated_leg_counters_are_lowered_by_the_initializer_once() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    // (counters expected after the repair) per row.
    let cases = [
        // Inflated by outside buys and a wallet-history close: lowered to the records.
        (
            CounterCase {
                id: 11,
                open: true,
                origin: "auto",
                dca_count: 3,
                partial_exit_count: 2,
                adds: 1,
                first_entries: 1,
                partials: 1,
                closing_sales: 0,
            },
            (1, 1),
        ),
        // Inflated adds only: the partial counter already equals its records.
        (
            CounterCase {
                id: 12,
                open: true,
                origin: "manual",
                dca_count: 4,
                partial_exit_count: 1,
                adds: 0,
                first_entries: 1,
                partials: 1,
                closing_sales: 0,
            },
            (0, 1),
        ),
        // Counters equal to their records.
        (
            CounterCase {
                id: 13,
                open: true,
                origin: "auto",
                dca_count: 1,
                partial_exit_count: 1,
                adds: 1,
                first_entries: 1,
                partials: 1,
                closing_sales: 0,
            },
            (1, 1),
        ),
        // Counters below their records are never raised.
        (
            CounterCase {
                id: 14,
                open: true,
                origin: "copy",
                dca_count: 0,
                partial_exit_count: 0,
                adds: 2,
                first_entries: 1,
                partials: 2,
                closing_sales: 0,
            },
            (0, 0),
        ),
        // A wallet-derived row counts the round's acquisitions, not records.
        (
            CounterCase {
                id: 15,
                open: true,
                origin: "external",
                dca_count: 5,
                partial_exit_count: 3,
                adds: 0,
                first_entries: 1,
                partials: 0,
                closing_sales: 0,
            },
            (5, 3),
        ),
        // A closed row is settled.
        (
            CounterCase {
                id: 16,
                open: false,
                origin: "auto",
                dca_count: 6,
                partial_exit_count: 6,
                adds: 1,
                first_entries: 1,
                partials: 1,
                closing_sales: 0,
            },
            (6, 6),
        ),
        // A row a later buy reopened counts its recorded closing sale as a partial exit.
        (
            CounterCase {
                id: 18,
                open: true,
                origin: "auto",
                dca_count: 1,
                partial_exit_count: 4,
                adds: 1,
                first_entries: 1,
                partials: 1,
                closing_sales: 1,
            },
            (1, 2),
        ),
        (
            CounterCase {
                id: 19,
                open: true,
                origin: "auto",
                dca_count: 1,
                partial_exit_count: 2,
                adds: 1,
                first_entries: 1,
                partials: 1,
                closing_sales: 1,
            },
            (1, 2),
        ),
        // A row with no entry record has nothing to count against.
        (
            CounterCase {
                id: 17,
                open: true,
                origin: "auto",
                dca_count: 2,
                partial_exit_count: 2,
                adds: 0,
                first_entries: 0,
                partials: 0,
                closing_sales: 0,
            },
            (2, 2),
        ),
        // Records of other swaps alone, such as a handover, are not the row's own: its
        // adds may be missing from them, so its counters are kept.
        (
            CounterCase {
                id: 20,
                open: true,
                origin: "auto",
                dca_count: 3,
                partial_exit_count: 0,
                adds: 1,
                first_entries: 0,
                partials: 0,
                closing_sales: 0,
            },
            (3, 0),
        ),
    ];

    for (created_by, sql) in CREATED_BY {
        let dir = tempfile::tempdir().unwrap();
        let conn = seed(dir.path(), sql);
        for (case, _) in &cases {
            seed_counter_case(&conn, case);
        }
        let records = |conn: &Connection| -> (i64, i64) {
            conn.query_row(
                "SELECT (SELECT COUNT(*) FROM position_entries), (SELECT COUNT(*) FROM position_exits)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
        };
        let released_records = records(&conn);
        drop(conn);

        boot(dir.path(), true);
        let conn = open(dir.path());
        assert_checks_pass(&conn, created_by);
        let counters = |conn: &Connection| -> Vec<(i64, i64, i64)> {
            conn.prepare("SELECT id, dca_count, partial_exit_count FROM positions ORDER BY id")
                .unwrap()
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap()
        };
        let mut expected: Vec<(i64, i64, i64)> = cases
            .iter()
            .map(|(case, (dca, partial))| (case.id, *dca, *partial))
            .collect();
        expected.sort_unstable();
        assert_eq!(counters(&conn), expected, "{created_by}: first launch");
        assert_eq!(
            records(&conn),
            released_records,
            "{created_by}: records kept"
        );

        let repaired = (schema(&conn), contents(&conn));
        drop(conn);
        boot(dir.path(), true);
        let conn = open(dir.path());
        assert_checks_pass(&conn, created_by);
        assert_eq!(
            (schema(&conn), contents(&conn)),
            repaired,
            "{created_by}: second launch"
        );
    }
}
