// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the open-time upgrade: the ledger decides what runs, a newer store is refused before any write, the store is copied before a breaking change or a raised legacy stamp, and a failed step leaves nothing behind.

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use super::*;
use crate::chains::ChainId;
use crate::database::catalog::POSITIONS;
use crate::database::configure::POSITIONS_DB;
use crate::database::schema::ledger::LedgerRow;

const STAMP: LegacyStamp = LegacyStamp {
    table: "meta",
    key: "schema_version",
    version: 6,
};

fn create_step(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS stepped (v INTEGER)")
        .map_err(|error| DatabaseError::classify_sqlite_failure("step", error))
}

fn failing_step(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    create_step(tx, &context())?;
    tx.execute("INSERT INTO stepped (v) VALUES (1)", [])
        .map_err(|error| DatabaseError::classify_sqlite_failure("step", error))?;
    Err(DatabaseError::Migration {
        store: "positions.db".to_owned(),
        detail: "unrecognized column".to_owned(),
    })
}

const fn spec(
    version: i64,
    min_reader_version: i64,
    step: crate::database::schema::SchemaStep,
    legacy_stamp: Option<LegacyStamp>,
) -> StoreSpec {
    StoreSpec {
        schema: crate::database::schema::StoreSchema {
            version,
            min_reader_version,
            baseline: 1,
            renames: None,
            step,
            legacy_stamp,
        },
        ..POSITIONS
    }
}

fn context() -> SchemaContext {
    SchemaContext {
        chains: vec![ChainId::Solana],
    }
}

fn store(directory: &Path) -> (PathBuf, Connection) {
    let path = directory.join("positions.db");
    let conn = Connection::open(&path).unwrap();
    configure_connection(&conn);
    (path, conn)
}

fn configure_connection(conn: &Connection) {
    crate::database::configure_connection(conn, POSITIONS_DB).unwrap();
}

fn seed(conn: &Connection, ledger: Option<(i64, i64)>, stamp: Option<i64>) {
    conn.execute_batch("CREATE TABLE IF NOT EXISTS kept (v INTEGER); INSERT INTO kept VALUES (7);")
        .unwrap();
    if let Some((version, min_reader_version)) = ledger {
        let schema = spec(version, min_reader_version, create_step, None).schema;
        ledger::write(conn, &schema).unwrap();
    }
    if let Some(stamp) = stamp {
        conn.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT)")
            .unwrap();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)",
            [stamp.to_string()],
        )
        .unwrap();
    }
}

fn run(
    conn: &mut Connection,
    path: &Path,
    spec: &StoreSpec,
    fence: bool,
) -> Result<Upgrade, DatabaseError> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    let upgrade = apply_with_fence(&tx, spec, path, &context(), fence)?;
    tx.commit().unwrap();
    Ok(upgrade)
}

fn stamp(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'schema_version'",
        [],
        |row| row.get(0),
    )
    .optional()
    .unwrap()
}

fn has_table(conn: &Connection, table: &str) -> bool {
    crate::database::schema::table_exists(conn, table).unwrap()
}

#[test]
fn a_fresh_store_runs_its_step_and_records_its_version() {
    let directory = tempfile::tempdir().unwrap();
    let (path, mut conn) = store(directory.path());
    let upgrade = run(&mut conn, &path, &spec(2, 1, create_step, None), false).unwrap();
    assert_eq!(upgrade.assessment, Assessment::Unrecorded);
    assert_eq!(upgrade.backup, None, "an empty file is never copied");
    assert!(has_table(&conn, "stepped"));
    assert_eq!(
        ledger::read(&conn)
            .unwrap()
            .map(|row| (row.version, row.min_reader_version)),
        Some((2, 1))
    );
}

#[test]
fn a_store_too_new_is_refused_before_its_step_runs() {
    let directory = tempfile::tempdir().unwrap();
    let (path, mut conn) = store(directory.path());
    seed(&conn, Some((9, 9)), Some(9));

    let error = run(
        &mut conn,
        &path,
        &spec(2, 1, create_step, Some(STAMP)),
        true,
    )
    .unwrap_err();
    assert!(
        matches!(
            &error,
            DatabaseError::StoreTooNew { store, version: 9, min_reader_version: 9, supported_version: 2, .. }
                if store == "positions.db"
        ),
        "{error:?}"
    );
    let text = error.to_string();
    for part in ["positions.db", ledger::WRITTEN_BY, "9", "2"] {
        assert!(text.contains(part), "{text}");
    }
    assert!(!has_table(&conn, "stepped"));
    assert_eq!(stamp(&conn).as_deref(), Some("9"));
    assert!(!directory.path().join("backups").exists());
}

/// A newer store this core can still read keeps its ledger: only renames are
/// reconciled, and the version is never lowered.
#[test]
fn a_compatible_newer_store_is_opened_without_its_step() {
    let directory = tempfile::tempdir().unwrap();
    let (path, mut conn) = store(directory.path());
    seed(&conn, Some((3, 2)), None);
    let upgrade = run(&mut conn, &path, &spec(2, 1, create_step, None), false).unwrap();
    assert_eq!(upgrade.assessment, Assessment::CompatibleNewer);
    assert!(!has_table(&conn, "stepped"));
    assert_eq!(
        ledger::read(&conn).unwrap(),
        Some(LedgerRow {
            version: 3,
            min_reader_version: 2,
            written_by: ledger::WRITTEN_BY.to_owned(),
        })
    );
}

#[test]
fn only_a_breaking_change_copies_a_recorded_store_first() {
    let directory = tempfile::tempdir().unwrap();
    let (path, mut conn) = store(directory.path());
    seed(&conn, Some((1, 1)), None);
    let upgrade = run(&mut conn, &path, &spec(2, 1, create_step, None), false).unwrap();
    assert_eq!(
        upgrade.assessment,
        Assessment::Older {
            from: 1,
            breaking: false
        }
    );
    assert_eq!(upgrade.backup, None);

    let upgrade = run(&mut conn, &path, &spec(3, 3, create_step, None), false).unwrap();
    let backup = upgrade.backup.expect("a breaking change copies the store");
    let copy = Connection::open(&backup).unwrap();
    assert_eq!(
        ledger::read(&copy).unwrap().map(|row| row.version),
        Some(2),
        "the copy holds the store as the upgrade found it"
    );
    assert_eq!(ledger::read(&conn).unwrap().map(|row| row.version), Some(3));
}

/// The legacy stamp older cores read rises to the schema's stamp, or to the fence while
/// a breaking ledger change is declared, and the store is copied whenever it rises.
#[test]
fn a_rising_legacy_stamp_copies_the_store_and_is_never_lowered() {
    let directory = tempfile::tempdir().unwrap();
    let (path, mut conn) = store(directory.path());
    seed(&conn, None, Some(5));

    let upgrade = run(
        &mut conn,
        &path,
        &spec(2, 1, create_step, Some(STAMP)),
        false,
    )
    .unwrap();
    assert!(upgrade.backup.is_some());
    assert_eq!(stamp(&conn).as_deref(), Some("6"));

    let upgrade = run(
        &mut conn,
        &path,
        &spec(2, 1, create_step, Some(STAMP)),
        false,
    )
    .unwrap();
    assert_eq!(
        upgrade.backup, None,
        "a stamp already at its target is not copied"
    );

    let upgrade = run(
        &mut conn,
        &path,
        &spec(2, 1, create_step, Some(STAMP)),
        true,
    )
    .unwrap();
    assert!(upgrade.backup.is_some());
    assert_eq!(
        stamp(&conn).as_deref(),
        Some(POSITIONS_FENCE.to_string().as_str())
    );

    let upgrade = run(
        &mut conn,
        &path,
        &spec(2, 1, create_step, Some(STAMP)),
        false,
    )
    .unwrap();
    assert_eq!(upgrade.backup, None);
    assert_eq!(
        stamp(&conn).as_deref(),
        Some(POSITIONS_FENCE.to_string().as_str()),
        "the stamp is never lowered"
    );
}

/// Every write of an open belongs to its one transaction: a step that fails after
/// writing leaves no table, row, ledger or stamp behind.
#[test]
fn a_failed_step_leaves_the_store_as_it_was() {
    let directory = tempfile::tempdir().unwrap();
    let (path, mut conn) = store(directory.path());
    seed(&conn, None, Some(5));

    let error = run(
        &mut conn,
        &path,
        &spec(2, 1, failing_step, Some(STAMP)),
        false,
    )
    .unwrap_err();
    assert!(
        matches!(error, DatabaseError::Migration { .. }),
        "{error:?}"
    );
    assert!(!has_table(&conn, "stepped"));
    assert!(!has_table(&conn, ledger::TABLE));
    assert_eq!(stamp(&conn).as_deref(), Some("5"));
    let kept: i64 = conn
        .query_row("SELECT COUNT(*) FROM kept", [], |row| row.get(0))
        .unwrap();
    assert_eq!(kept, 1);
}

#[test]
fn the_fence_rises_only_for_a_breaking_ledger_or_vault_change() {
    static PLAIN: StoreSpec = spec(4, 1, create_step, None);
    static BREAKING: StoreSpec = spec(4, 2, create_step, None);
    static BREAKING_CACHE: StoreSpec = StoreSpec {
        class: StoreClass::Cache,
        ..spec(4, 2, create_step, None)
    };
    assert!(!fence_raised(&[&PLAIN]));
    assert!(!fence_raised(&[&BREAKING_CACHE]));
    assert!(fence_raised(&[&PLAIN, &BREAKING]));
}
