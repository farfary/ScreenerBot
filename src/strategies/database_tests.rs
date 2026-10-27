// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the strategies store schema: the paper or live mode a store gains, and the
//! refusal an older core meets on a store that carries it.

use rusqlite::{params, Connection};

use super::*;
use crate::chains::ChainId;
use crate::database::schema::ledger::{self, Assessment};
use crate::database::{catalog, open};

/// Opens `path` as the strategies store, as the storage phase does.
fn open_store(path: &std::path::Path) {
    open::pool_at(&catalog::STRATEGIES, path, vec![ChainId::Solana]).unwrap();
}

fn insert_row(conn: &Connection, id: &str, kind: &str, enabled: bool) {
    conn.execute(
        "INSERT INTO strategies (id, name, type, enabled, rules_json, created_at, updated_at)
         VALUES (?1, ?1, ?2, ?3, '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        params![id, kind, enabled],
    )
    .unwrap();
}

/// A store as a version 3 core left it: no mode column, ledger at version 3.
fn version_3_store(path: &std::path::Path) {
    open_store(path);
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "ALTER TABLE strategies DROP COLUMN mode;
         UPDATE store_schema SET version = 3, min_reader_version = 2 WHERE id = 1;",
    )
    .unwrap();
    insert_row(&conn, "entry-on", "ENTRY", true);
    insert_row(&conn, "exit-off", "EXIT", false);
}

#[test]
fn a_version_3_store_gains_the_live_mode_and_survives_a_second_open() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    version_3_store(&path);

    open_store(&path);
    open_store(&path);

    let conn = Connection::open(&path).unwrap();
    let rows: Vec<(String, bool, String)> = conn
        .prepare("SELECT id, enabled, mode FROM strategies ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows,
        [
            ("entry-on".to_owned(), true, "live".to_owned()),
            ("exit-off".to_owned(), false, "live".to_owned()),
        ],
        "every strategy of an older store keeps trading live"
    );
    let row = ledger::read(&conn).unwrap().unwrap();
    assert_eq!(
        (row.version, row.min_reader_version),
        (STORE_VERSION, MIN_READER_VERSION)
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
    assert!(
        conn.execute(
            "UPDATE strategies SET mode = 'shadow' WHERE id = 'entry-on'",
            []
        )
        .is_err(),
        "the mode is paper or live only"
    );
}

/// The mode changes how a strategy trades, so it is a breaking change: a core that
/// ignored it would trade a paper strategy live.
#[test]
fn a_store_with_the_mode_is_too_new_for_a_core_without_it() {
    assert_eq!(MIN_READER_VERSION, STORE_VERSION);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("strategies.db");
    open_store(&path);
    let row = ledger::read(&Connection::open(&path).unwrap())
        .unwrap()
        .unwrap();
    let older_core = StoreSchema {
        version: STORE_VERSION - 1,
        ..SCHEMA
    };
    assert_eq!(ledger::assess(Some(&row), &older_core), Assessment::TooNew);
    assert_eq!(ledger::assess(Some(&row), &SCHEMA), Assessment::Current);
}
