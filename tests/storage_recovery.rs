// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Fault injection for the storage phase: every catalog store, damaged in each way a store file fails, opens as its class declares in a fresh process, and an upgrade interrupted at any point leaves its store unchanged and fully upgraded by the next open.

use crate::common;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use screenerbot::chains::ChainId;
use screenerbot::database::backup::snapshot::{self, SnapshotKind};
use screenerbot::database::catalog::{self, StoreClass, StoreSpec, STORES};
use screenerbot::database::schema::{ledger, upgrade, SchemaContext};

const CHILD_ENV: &str = "SCREENERBOT_STORAGE_RECOVERY_CHILD";
const SEED: &str = "seed";
const PHASE: &str = "phase";
const REPORT: &str = "storage-recovery-report.json";
const SENTINEL: &str = "__storage_recovery_sentinel";

/// One way a store file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    ZeroLength,
    GarbageHeader,
    TruncatedLastPage,
    DamagedPage,
    NewerStore,
    UnknownTable,
    ReadOnly,
    StaleSharedMemory,
    Locked,
}

const FAULTS: [Fault; 9] = [
    Fault::ZeroLength,
    Fault::GarbageHeader,
    Fault::TruncatedLastPage,
    Fault::DamagedPage,
    Fault::NewerStore,
    Fault::UnknownTable,
    Fault::ReadOnly,
    Fault::StaleSharedMemory,
    Fault::Locked,
];

/// Stores held locked by another process for [`Fault::Locked`], one per class that
/// retries: each one costs the whole busy retry budget.
const LOCKED_STORES: [&str; 2] = ["copy_trading", "rpc_stats"];

/// The child half of the fault matrix: seed every store, or run the storage phase
/// and report every store's health.
#[test]
fn storage_recovery_child() {
    let Ok(mode) = std::env::var(CHILD_ENV) else {
        return;
    };
    screenerbot::paths::ensure_all_directories().expect("create recovery directories");
    screenerbot::config::load_config().expect("load isolated configuration");
    screenerbot::database::open::open_all();
    if mode == SEED {
        seed_every_store();
    }
    let report =
        serde_json::to_vec_pretty(&screenerbot::database::health::all()).expect("encode report");
    std::fs::write(
        screenerbot::paths::get_base_directory().join(REPORT),
        report,
    )
    .expect("write recovery report");
}

fn seed_every_store() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create runtime")
        .block_on(async {
            common::configure_trading_wallet();
            screenerbot::wallets::initialize()
                .await
                .expect("initialize wallets");
            assert!(
                screenerbot::wallets::has_main_wallet().await,
                "the configured trading wallet becomes the main wallet"
            );
        });
    for store in STORES {
        let path = store.path(ChainId::Solana);
        Connection::open(&path)
            .expect("open seeded store")
            .execute_batch(&format!(
                "CREATE TABLE {SENTINEL} (v INTEGER); INSERT INTO {SENTINEL} (v) VALUES (1);"
            ))
            .expect("write sentinel");
        snapshot::take(store, &path, SnapshotKind::Boot).expect("snapshot store");
    }
}

fn launch(base: &Path, mode: &str) -> Vec<serde_json::Value> {
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg(common::test_name(module_path!(), "storage_recovery_child"))
        .arg("--nocapture")
        .env(CHILD_ENV, mode)
        .env("SCREENERBOT_DATA_DIR", base)
        .status()
        .expect("run storage recovery child");
    assert!(status.success(), "{mode} child failed");
    let report = std::fs::read(base.join(REPORT)).expect("read recovery report");
    std::fs::remove_file(base.join(REPORT)).expect("clear recovery report");
    serde_json::from_slice(&report).expect("decode recovery report")
}

fn store_path(base: &Path, store: &StoreSpec) -> PathBuf {
    let name = store
        .path(ChainId::Solana)
        .file_name()
        .expect("store file name")
        .to_owned();
    base.join("data").join(name)
}

/// Fold every store's write-ahead log into its file, so a fault reaches its pages.
fn checkpoint(path: &Path) {
    Connection::open(path)
        .expect("open store to checkpoint")
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint store");
    assert!(
        !path.with_extension("db-wal").exists(),
        "{} kept its WAL",
        path.display()
    );
}

fn damage_root_page(path: &Path, table: &str) {
    let (page_size, root): (u64, u64) = {
        let connection = Connection::open(path).expect("open store to damage");
        (
            connection
                .query_row("PRAGMA page_size", [], |row| row.get(0))
                .expect("read page size"),
            connection
                .query_row(
                    "SELECT rootpage FROM sqlite_master WHERE name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("read root page"),
        )
    };
    let mut bytes = std::fs::read(path).expect("read store");
    bytes[usize::try_from((root - 1) * page_size).expect("page offset")] = 0xFF;
    std::fs::write(path, bytes).expect("write damaged store");
}

fn inject(fault: Fault, path: &Path) {
    let execute = |sql: &str| {
        Connection::open(path)
            .expect("open store to change")
            .execute_batch(sql)
            .expect("change store");
    };
    match fault {
        Fault::ZeroLength => std::fs::write(path, b"").expect("empty store"),
        Fault::GarbageHeader => {
            let mut bytes = std::fs::read(path).expect("read store");
            bytes[..16].copy_from_slice(b"not an sqlite db");
            std::fs::write(path, bytes).expect("write store");
        }
        Fault::TruncatedLastPage => {
            let length = std::fs::metadata(path).expect("store size").len();
            std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .and_then(|file| file.set_len(length - 512))
                .expect("truncate store");
        }
        Fault::DamagedPage => damage_root_page(path, SENTINEL),
        Fault::NewerStore => execute(
            "CREATE TABLE IF NOT EXISTS store_schema (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 version INTEGER NOT NULL,
                 min_reader_version INTEGER NOT NULL,
                 written_by TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             INSERT OR REPLACE INTO store_schema VALUES (1, 999, 999, '99.0.0', '2030-01-01');",
        ),
        Fault::UnknownTable => execute(
            "CREATE TABLE __unknown_extra (v INTEGER); INSERT INTO __unknown_extra VALUES (1);",
        ),
        Fault::ReadOnly => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444))
                    .expect("make store read-only");
            }
        }
        Fault::StaleSharedMemory => {
            std::fs::write(path.with_extension("db-shm"), vec![0xA5_u8; 32_768])
                .expect("write stale shared memory")
        }
        Fault::Locked => {}
    }
}

fn state(report: &[serde_json::Value], store: &str) -> serde_json::Value {
    report
        .iter()
        .find(|entry| entry["store"] == store)
        .unwrap_or_else(|| panic!("{store} is missing from the health report"))["health"]
        .clone()
}

fn sentinel_rows(path: &Path) -> Option<i64> {
    Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .ok()?
        .query_row(&format!("SELECT COUNT(*) FROM {SENTINEL}"), [], |row| {
            row.get(0)
        })
        .ok()
}

fn quarantined_copy(base: &Path, health: &serde_json::Value, path: &Path) -> Vec<u8> {
    let directory = PathBuf::from(health["quarantine"].as_str().expect("quarantine path"));
    assert!(directory.starts_with(base), "{}", directory.display());
    std::fs::read(directory.join(path.file_name().expect("store file name")))
        .expect("read quarantined store")
}

/// Every catalog store, damaged in each way a store file fails, ends as its class
/// declares: a rebuildable store is moved aside whole and created afresh, a ledger or
/// vault store is restored from its snapshot with the damaged file kept, a store too
/// new for this core or blocked by its surroundings is left byte-identical, and no
/// store ever loses its file.
#[test]
fn every_store_recovers_every_fault_as_its_class_declares() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    for fault in FAULTS {
        let directory = tempfile::tempdir().expect("create isolated data directory");
        let base = directory.path();
        launch(base, SEED);

        let mut before = BTreeMap::new();
        let mut holders = Vec::new();
        for store in STORES {
            let path = store_path(base, store);
            checkpoint(&path);
            if fault != Fault::Locked {
                inject(fault, &path);
            }
            // Closing any descriptor of a file drops every POSIX lock this process holds
            // on it, so the store is read before it is locked.
            before.insert(store.id, std::fs::read(&path).expect("read faulted store"));
            if fault == Fault::Locked && LOCKED_STORES.contains(&store.id) {
                let connection = Connection::open(&path).expect("open store to lock");
                connection
                    .execute_batch("BEGIN EXCLUSIVE")
                    .expect("lock store");
                holders.push(connection);
            }
        }

        let report = launch(base, PHASE);
        drop(holders);

        for store in STORES {
            let path = store_path(base, store);
            if fault == Fault::ReadOnly {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
                        .expect("restore store permissions");
                }
            }
            let health = state(&report, store.id);
            let outcome = health["state"].as_str().expect("health state").to_owned();
            let context = format!("{fault:?} on {}: {health}", store.id);
            let faulted = &before[store.id];
            let rebuildable = matches!(store.class, StoreClass::Cache | StoreClass::Derived);

            match outcome.as_str() {
                "healthy" => {
                    assert!(
                        !matches!(
                            fault,
                            Fault::ZeroLength
                                | Fault::GarbageHeader
                                | Fault::TruncatedLastPage
                                | Fault::NewerStore
                        ),
                        "{context}"
                    );
                    if fault != Fault::DamagedPage {
                        assert_eq!(sentinel_rows(&path), Some(1), "{context}");
                    }
                    if fault == Fault::UnknownTable {
                        let kept: i64 = Connection::open(&path)
                            .expect("open store")
                            .query_row("SELECT COUNT(*) FROM __unknown_extra", [], |row| row.get(0))
                            .expect("read unknown table");
                        assert_eq!(kept, 1, "{context}");
                    }
                }
                "recovered" => {
                    assert!(rebuildable, "{context}");
                    assert_eq!(
                        &quarantined_copy(base, &health, &path),
                        faulted,
                        "{context}"
                    );
                }
                "restored" => {
                    assert!(!rebuildable, "{context}");
                    assert_eq!(
                        &quarantined_copy(base, &health, &path),
                        faulted,
                        "{context}"
                    );
                    assert_eq!(sentinel_rows(&path), Some(1), "{context}");
                }
                "unavailable" => {
                    assert_eq!(
                        &std::fs::read(&path).expect("read store"),
                        faulted,
                        "{context}"
                    );
                    assert!(
                        matches!(fault, Fault::NewerStore | Fault::ReadOnly | Fault::Locked)
                            || store.class == StoreClass::Ledger,
                        "{context}"
                    );
                }
                other => panic!("{context}: unexpected state {other}"),
            }

            let expected = match (fault, rebuildable) {
                (Fault::ZeroLength | Fault::GarbageHeader | Fault::TruncatedLastPage, true)
                | (Fault::NewerStore, true) => Some("recovered"),
                (Fault::ZeroLength | Fault::GarbageHeader | Fault::TruncatedLastPage, false)
                | (Fault::DamagedPage, false) => Some("restored"),
                (Fault::NewerStore, false) => Some("unavailable"),
                (Fault::Locked, _) if LOCKED_STORES.contains(&store.id) => Some("unavailable"),
                (Fault::Locked | Fault::StaleSharedMemory | Fault::UnknownTable, _) => {
                    Some("healthy")
                }
                (Fault::DamagedPage | Fault::ReadOnly, _) => None,
            };
            if let Some(expected) = expected {
                assert_eq!(outcome, expected, "{context}");
            }
        }
    }
}

/// The released schema of each store, with the catalog entry that upgrades it.
const FIXTURES: &[(&str, &StoreSpec)] = &[
    ("v0.2.13-actions.sql", &catalog::ACTIONS),
    ("v0.2.13-ai.sql", &catalog::AI),
    ("v0.2.13-copy.sql", &catalog::COPY_TRADING),
    ("v0.2.13-events.sql", &catalog::EVENTS),
    ("v0.2.13-ohlcv.sql", &catalog::OHLCVS),
    ("v0.2.13-pools.sql", &catalog::POOLS),
    ("v0.2.13-positions.sql", &catalog::POSITIONS),
    (
        "v0.2.13-positions-created-v0.1.116.sql",
        &catalog::POSITIONS,
    ),
    (
        "v0.2.13-positions-created-v0.1.117.sql",
        &catalog::POSITIONS,
    ),
    (
        "v0.2.13-positions-created-v0.1.121.sql",
        &catalog::POSITIONS,
    ),
    ("v0.2.13-rpc_stats.sql", &catalog::RPC_STATS),
    ("v0.2.13-strategies.sql", &catalog::STRATEGIES),
    ("v0.2.13-tokens.sql", &catalog::TOKENS),
    ("v0.2.13-tools.sql", &catalog::ATA_CLEANUP),
    ("v0.2.13-transactions.sql", &catalog::TRANSACTIONS),
    ("v0.2.13-wallet-monitor.sql", &catalog::WALLET),
    ("v0.2.13-wallets.sql", &catalog::WALLETS),
];

/// Interruption points per fixture: the upgrade's virtual-machine instructions are
/// split into this many equal runs, and it is interrupted after each one.
const INTERRUPTION_POINTS: usize = 48;

/// Every table's name, definition and row count.
fn contents(path: &Path) -> Vec<(String, Option<String>, i64)> {
    let connection = Connection::open(path).expect("open store to read");
    let objects: Vec<(String, String, Option<String>)> = connection
        .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .expect("list objects")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("read objects")
        .collect::<Result<_, _>>()
        .expect("collect objects");
    objects
        .into_iter()
        .map(|(kind, name, sql)| {
            let rows = if kind == "table" {
                connection
                    .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |row| {
                        row.get(0)
                    })
                    .expect("count rows")
            } else {
                0
            };
            (name, sql, rows)
        })
        .collect()
}

/// Run one open's upgrade of `spec` on `path`, the progress handler interrupting it at
/// its `interrupt_at`-th call. Returns whether it committed and how often the handler ran.
fn upgrade_once(
    path: &Path,
    spec: &'static StoreSpec,
    period: i32,
    interrupt_at: Option<usize>,
) -> (bool, usize) {
    let mut connection = Connection::open(path).expect("open fixture store");
    screenerbot::database::configure_connection(&connection, spec.config)
        .expect("configure fixture store");
    connection
        .pragma_update(None, "foreign_keys", "OFF")
        .expect("foreign keys off");
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&calls);
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("begin upgrade");
    transaction.progress_handler(
        period,
        Some(move || {
            let call = counter.fetch_add(1, Ordering::SeqCst) + 1;
            interrupt_at.is_some_and(|at| call >= at)
        }),
    );
    let context = SchemaContext {
        chains: vec![ChainId::Solana],
    };
    let upgraded = upgrade::apply(&transaction, spec, path, &context).is_ok();
    transaction.progress_handler(period, None::<fn() -> bool>);
    let committed = upgraded && transaction.commit().is_ok();
    (committed, calls.load(Ordering::SeqCst))
}

/// An upgrade interrupted after any statement leaves its store exactly as it was, and
/// the next open upgrades it in full with every check clean and no recovery action.
#[test]
fn an_upgrade_interrupted_at_any_point_leaves_the_store_to_the_next_open() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for (fixture, spec) in FIXTURES {
        let directory = tempfile::tempdir().expect("create fixture directory");
        let template = directory.path().join("template.db");
        {
            let connection = Connection::open(&template).expect("create fixture store");
            screenerbot::database::configure_connection(&connection, spec.config)
                .expect("configure fixture store");
            connection
                .execute_batch(
                    &std::fs::read_to_string(fixtures.join(fixture)).expect("read fixture"),
                )
                .expect("load fixture");
        }
        checkpoint(&template);
        let original = contents(&template);
        // Each run upgrades its own copy, so no write-ahead log or shared memory of an
        // earlier run is ever read beside it.
        let fresh = |run: usize| {
            let run_directory = directory.path().join(run.to_string());
            std::fs::create_dir(&run_directory).expect("create run directory");
            let path = run_directory.join(format!("{}.db", spec.id));
            std::fs::copy(&template, &path).expect("copy fixture store");
            path
        };

        let (committed, instructions) = upgrade_once(&fresh(0), spec, 1, None);
        assert!(committed, "{fixture}: the uninterrupted upgrade fails");
        let period =
            i32::try_from((instructions / INTERRUPTION_POINTS).max(1)).expect("progress period");

        let mut interruption = 1;
        loop {
            let path = fresh(interruption);
            let (committed, _) = upgrade_once(&path, spec, period, Some(interruption));
            if committed {
                break;
            }
            assert_eq!(
                contents(&path),
                original,
                "{fixture}: interrupted at call {interruption} changed the store"
            );

            let pool = screenerbot::database::open::pool_at(spec, &path, vec![ChainId::Solana])
                .unwrap_or_else(|error| panic!("{fixture} at call {interruption}: {error}"));
            let connection = pool.get().expect("pooled connection");
            assert_eq!(
                ledger::read(&connection)
                    .expect("read ledger")
                    .map(|row| row.version),
                Some(spec.schema.version),
                "{fixture} at call {interruption}"
            );
            let integrity: String = connection
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .expect("integrity check");
            assert_eq!(integrity, "ok", "{fixture} at call {interruption}");
            let foreign_key_failure = connection
                .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
                .optional()
                .expect("foreign key check");
            assert!(
                foreign_key_failure.is_none(),
                "{fixture} at call {interruption}"
            );
            assert_eq!(
                screenerbot::database::health::status(&path)
                    .expect("recorded health")
                    .health,
                screenerbot::database::health::StoreHealth::Healthy,
                "{fixture} at call {interruption}"
            );
            drop(connection);
            drop(pool);
            interruption += 1;
            assert!(
                interruption <= INTERRUPTION_POINTS * 2 + 2,
                "{fixture}: the upgrade never completed"
            );
        }
        assert!(
            interruption > 1,
            "{fixture}: no interruption point was tested"
        );
    }
}
