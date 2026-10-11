// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the store opener: each class recovers a damaged, foreign or too-new file as the decision table declares, the vault is never moved without a verified snapshot, a refused open leaves the file byte-identical, and only ledger and vault stores are checked at open.

use super::*;
use crate::database::catalog::{SalvageRule, POSITIONS, RPC_STATS, TRANSACTIONS, WALLETS};
use crate::database::schema::{ledger, StoreSchema};

const KEPT: &[SalvageRule] = &[SalvageRule {
    table: "kept",
    columns: None,
    filter: None,
}];

fn create(tx: &rusqlite::Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS kept (v INTEGER PRIMARY KEY)")
        .map_err(|error| DatabaseError::classify_sqlite_failure("create kept", error))
}

fn refuse(tx: &rusqlite::Transaction<'_>, context: &SchemaContext) -> Result<(), DatabaseError> {
    create(tx, context)?;
    tx.execute_batch(
        "CREATE TABLE written_before_refusal (v INTEGER); INSERT INTO kept (v) VALUES (100);",
    )
    .map_err(|error| DatabaseError::classify_sqlite_failure("write", error))?;
    Err(DatabaseError::Migration {
        store: "test.db".to_owned(),
        detail: "unrecognized column".to_owned(),
    })
}

const fn schema(step: crate::database::schema::SchemaStep) -> StoreSchema {
    StoreSchema {
        version: 1,
        min_reader_version: 1,
        baseline: 1,
        renames: None,
        step,
        legacy_stamp: None,
    }
}

static CACHE: StoreSpec = StoreSpec {
    schema: schema(create),
    salvage: KEPT,
    ..RPC_STATS
};
static DERIVED: StoreSpec = StoreSpec {
    schema: schema(create),
    salvage: &[],
    ..TRANSACTIONS
};
static LEDGER: StoreSpec = StoreSpec {
    schema: schema(create),
    salvage: KEPT,
    ..POSITIONS
};
static VAULT: StoreSpec = StoreSpec {
    schema: schema(create),
    verify_snapshot: None,
    ..WALLETS
};

/// `spec` at version 2 with `step`, so its open writes the ledger.
fn raised(
    spec: &'static StoreSpec,
    step: crate::database::schema::SchemaStep,
) -> &'static StoreSpec {
    Box::leak(Box::new(StoreSpec {
        schema: StoreSchema {
            version: 2,
            ..schema(step)
        },
        ..*spec
    }))
}

fn refusing(spec: &'static StoreSpec) -> &'static StoreSpec {
    raised(spec, refuse)
}

fn chains() -> Vec<ChainId> {
    vec![ChainId::Solana]
}

/// A store of `spec` at `name` holding rows 1 to 3 in `kept` and a table no step reads,
/// upgraded on its only connection and closed, so every page is in the main file.
fn fixture(directory: &Path, name: &str, spec: &'static StoreSpec) -> PathBuf {
    let path = directory.join(name);
    let mut connection = Connection::open(&path).unwrap();
    configure_connection(&connection, spec.config).unwrap();
    upgrade_connection(&mut connection, spec, chains()).expect("upgrade fixture");
    connection
        .execute_batch(
            "INSERT INTO kept (v) VALUES (1), (2), (3);
             CREATE TABLE unread (v INTEGER); INSERT INTO unread (v) VALUES (1);",
        )
        .unwrap();
    drop(connection);
    assert!(
        !path.with_extension("db-wal").exists(),
        "fixture left a WAL"
    );
    path
}

fn kept_rows(pool: &SqlitePool) -> i64 {
    pool.get()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM kept", [], |row| row.get(0))
        .unwrap()
}

fn quarantined(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory.join(crate::paths::QUARANTINE_DIRECTORY_NAME))
        .map(|entries| entries.map(|entry| entry.unwrap().path()).collect())
        .unwrap_or_default()
}

fn health_of(path: &Path) -> StoreHealth {
    health::status(path)
        .expect("the open published its health")
        .health
}

/// Overwrite the b-tree page type of `table`'s root page, so any read of the table and
/// `quick_check` report the file as damaged.
fn damage_root_page(path: &Path, table: &str) {
    let (page_size, root): (u64, u64) = {
        let connection = Connection::open(path).unwrap();
        (
            connection
                .query_row("PRAGMA page_size", [], |row| row.get(0))
                .unwrap(),
            connection
                .query_row(
                    "SELECT rootpage FROM sqlite_master WHERE name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap(),
        )
    };
    let mut bytes = fs::read(path).unwrap();
    bytes[usize::try_from((root - 1) * page_size).unwrap()] = 0xFF;
    fs::write(path, bytes).unwrap();
}

fn garbage() -> Vec<u8> {
    b"this file holds no SQLite header".repeat(128)
}

#[test]
fn a_file_that_is_not_a_database_is_recovered_by_its_class() {
    let directory = tempfile::tempdir().unwrap();
    for (name, spec) in [
        ("cache.db", &CACHE),
        ("derived.db", &DERIVED),
        ("ledger.db", &LEDGER),
    ] {
        let path = directory.path().join(name);
        fs::write(&path, garbage()).unwrap();
        let pool = pool_at(spec, &path, chains()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(kept_rows(&pool), 0, "{name} was created afresh");
        let StoreHealth::Recovered { quarantine } = health_of(&path) else {
            panic!("{name}: {:?}", health_of(&path));
        };
        assert_eq!(fs::read(quarantine.join(name)).unwrap(), garbage());
    }
    assert_eq!(quarantined(directory.path()).len(), 3);
}

/// The vault without a verified snapshot is reported, never moved, recreated or
/// replaced by an empty store, whatever made its open fail.
#[test]
fn a_failed_vault_without_a_snapshot_is_left_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let foreign = directory.path().join("foreign.db");
    fs::write(&foreign, garbage()).unwrap();
    let damaged = fixture(directory.path(), "damaged.db", &VAULT);
    damage_root_page(&damaged, "kept");
    let too_new = fixture(directory.path(), "too_new.db", &VAULT);
    ledger::write(
        &Connection::open(&too_new).unwrap(),
        &StoreSchema {
            version: 9,
            min_reader_version: 9,
            ..schema(create)
        },
    )
    .unwrap();
    let refused = fixture(directory.path(), "refused.db", &VAULT);

    for (path, spec) in [
        (&foreign, &VAULT),
        (&damaged, &VAULT),
        (&too_new, &VAULT),
        (&refused, refusing(&VAULT)),
    ] {
        let before = fs::read(path).unwrap();
        let error = pool_at(spec, path, chains()).expect_err("the vault open fails");
        assert_eq!(fs::read(path).unwrap(), before, "{error}");
        assert!(
            matches!(
                health_of(path),
                StoreHealth::Unavailable { isolated: true, .. }
            ),
            "{error}: {:?}",
            health_of(path)
        );
    }
    assert!(quarantined(directory.path()).is_empty());
}

/// A damaged ledger or vault store is moved to quarantine, never deleted, and replaced
/// by a copy of its latest verified snapshot.
#[test]
fn a_damaged_ledger_or_vault_store_is_restored_from_its_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    for (name, spec) in [("ledger.db", &LEDGER), ("vault.db", &VAULT)] {
        let path = fixture(directory.path(), name, spec);
        let snapshot = snapshot::take(spec, &path, snapshot::SnapshotKind::Boot)
            .unwrap()
            .expect("snapshotted");
        damage_root_page(&path, "kept");
        let damaged = fs::read(&path).unwrap();

        let pool = pool_at(spec, &path, chains()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(kept_rows(&pool), 3);
        let StoreHealth::Restored { quarantine, .. } = health_of(&path) else {
            panic!("{name}: {:?}", health_of(&path));
        };
        assert_eq!(fs::read(quarantine.join(name)).unwrap(), damaged);
        assert!(snapshot.exists());
    }
}

/// A newer store is refused byte-identical when it holds records; a rebuildable store is
/// moved aside whole and created afresh.
#[test]
fn a_store_too_new_is_refused_unchanged_unless_rebuildable() {
    let directory = tempfile::tempdir().unwrap();
    let newer = StoreSchema {
        version: 9,
        min_reader_version: 9,
        ..schema(create)
    };
    for (name, spec, rebuilt) in [("ledger.db", &LEDGER, false), ("cache.db", &CACHE, true)] {
        let path = fixture(directory.path(), name, spec);
        ledger::write(&Connection::open(&path).unwrap(), &newer).unwrap();
        let before = fs::read(&path).unwrap();
        let opened = pool_at(spec, &path, chains());
        if rebuilt {
            assert_eq!(kept_rows(&opened.unwrap()), 0);
            let StoreHealth::Recovered { quarantine } = health_of(&path) else {
                panic!("{name}: {:?}", health_of(&path));
            };
            assert_eq!(fs::read(quarantine.join(name)).unwrap(), before);
        } else {
            let error = opened.expect_err("a newer ledger store is refused");
            assert!(
                matches!(error, DatabaseError::StoreTooNew { .. }),
                "{error}"
            );
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }
    assert_eq!(quarantined(directory.path()).len(), 1);
}

/// Every statement of a refused open rolls back with it: the file is byte-identical and
/// nothing is moved. A cache store whose step fails is rebuilt once and then reported.
#[test]
fn a_refused_open_changes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    for (name, spec) in [("ledger.db", &LEDGER), ("derived.db", &DERIVED)] {
        let path = fixture(directory.path(), name, spec);
        let before = fs::read(&path).unwrap();
        let error = pool_at(refusing(spec), &path, chains()).expect_err("the step refuses");
        assert!(matches!(error, DatabaseError::Migration { .. }), "{error}");
        if spec.class == StoreClass::Ledger {
            assert_eq!(fs::read(&path).unwrap(), before);
        }
    }
    let ledger_path = directory.path().join("ledger.db");
    let reopened = pool_at(&LEDGER, &ledger_path, chains()).unwrap();
    assert_eq!(kept_rows(&reopened), 3);
    assert!(!crate::database::schema::table_exists(
        &reopened.get().unwrap(),
        "written_before_refusal"
    )
    .unwrap());
    assert_eq!(
        quarantined(directory.path()).len(),
        1,
        "only the derived store is moved, once"
    );
}

/// Only ledger and vault stores are checked when they open: a cache store whose unread
/// table is damaged opens in place, a ledger store with the same damage does not.
#[test]
fn only_ledger_and_vault_stores_are_checked_at_open() {
    let directory = tempfile::tempdir().unwrap();
    let cache = fixture(directory.path(), "cache.db", &CACHE);
    damage_root_page(&cache, "unread");
    pool_at(&CACHE, &cache, chains()).expect("the cache store opens in place");
    assert_eq!(health_of(&cache), StoreHealth::Healthy);

    let ledger = fixture(directory.path(), "ledger.db", &LEDGER);
    damage_root_page(&ledger, "unread");
    pool_at(&LEDGER, &ledger, chains()).expect("the ledger store is rebuilt");
    assert!(matches!(health_of(&ledger), StoreHealth::Recovered { .. }));
    assert!(
        !include_str!("open.rs").contains(concat!("integrity", "_check")),
        "a full integrity check never runs at open"
    );
}

/// A file another program holds locked, or one that cannot be written, is reported and
/// never moved.
#[test]
fn environment_failures_never_move_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let busy = fixture(directory.path(), "busy.db", &CACHE);
    let mut holder = Connection::open(&busy).unwrap();
    let lock = holder
        .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)
        .unwrap();
    let error = pool_at(&CACHE, &busy, chains()).expect_err("the store is locked");
    assert!(
        matches!(FailureKind::of(&error), FailureKind::Busy),
        "{error}"
    );
    drop(lock);
    drop(holder);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let locked = fixture(directory.path(), "read_only.db", &CACHE);
        let before = fs::read(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o444)).unwrap();
        let error =
            pool_at(raised(&CACHE, create), &locked, chains()).expect_err("the store is read-only");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(FailureKind::of(&error), FailureKind::ReadOnly, "{error}");
        assert_eq!(fs::read(&locked).unwrap(), before);
    }
    assert!(quarantined(directory.path()).is_empty());
}

static REFUSALS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static FAILING_STORE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

fn failing_store_path() -> PathBuf {
    FAILING_STORE.get().expect("path set by the test").clone()
}

fn counted_refuse(
    tx: &rusqlite::Transaction<'_>,
    context: &SchemaContext,
) -> Result<(), DatabaseError> {
    REFUSALS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    refuse(tx, context)
}

/// A store file whose open failed is opened once: every later caller receives the same
/// error from the failure cache and the file is never probed or checked again.
#[test]
fn a_failed_store_open_is_attempted_once_per_process() {
    let directory = tempfile::tempdir().unwrap();
    let path = fixture(directory.path(), "ledger.db", &LEDGER);
    FAILING_STORE.set(path.clone()).unwrap();
    let spec: &'static StoreSpec = Box::leak(Box::new(StoreSpec {
        location: StoreLocation::Shared(failing_store_path),
        ..*raised(&LEDGER, counted_refuse)
    }));
    assert!(is_store_file(spec, &path));

    let first = pool_at(spec, &path, chains()).expect_err("the open is refused");
    let state = health_of(&path);
    assert!(matches!(
        state,
        StoreHealth::Unavailable { isolated: true, .. }
    ));
    for _ in 0..3 {
        let again = pool_at(spec, &path, chains()).expect_err("the failure is cached");
        assert_eq!(again.to_string(), first.to_string());
        assert_eq!(FailureKind::of(&again), FailureKind::of(&first));
    }
    assert_eq!(REFUSALS.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(health_of(&path), state);
}
