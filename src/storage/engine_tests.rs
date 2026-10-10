// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the housekeeping engine. Each runs in a child process whose data directory is a fresh temporary directory, so no test reaches a real install.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use super::engine::{carry_out, run_pass};
use super::types::{Action, Plan};
use crate::database::catalog::{self, Compaction, StoreSpec, STORES};
use crate::database::handles::StoreHandle;
use crate::database::{configure_connection, EVENTS_WRITE_DB};
use crate::paths;

const CHILD_ENV: &str = "SCREENERBOT_STORAGE_ENGINE_CHILD";
const RAN_MARKER: &str = "child-ran";
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Run `body` in a child process of this test binary with its own data directory.
/// The child leaves a marker, so a filter that matches no test cannot pass silently.
fn isolated(test: &str, body: impl FnOnce()) {
    if std::env::var_os(CHILD_ENV).is_some() {
        body();
        fs::write(paths::get_base_directory().join(RAN_MARKER), b"").unwrap();
        return;
    }
    let base = tempfile::tempdir().unwrap();
    let temporary = base.path().join("tmp");
    fs::create_dir_all(&temporary).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(format!("storage::engine_tests::{test}"))
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env("SCREENERBOT_DATA_DIR", base.path())
        .env("TMPDIR", &temporary)
        .status()
        .unwrap();
    assert!(status.success(), "isolated child {test} failed");
    assert!(base.path().join(RAN_MARKER).exists(), "{test} did not run");
}

fn pass(compaction_due: bool) -> Plan {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_pass(compaction_due))
        .unwrap()
}

fn write(path: &Path, bytes: &[u8], age: Duration) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - age)
        .unwrap();
}

fn deletions(plan: &Plan) -> usize {
    plan.actions
        .iter()
        .filter(|action| matches!(action, Action::Delete { .. }))
        .count()
}

#[test]
fn one_pass_keeps_what_must_stay_and_a_second_pass_deletes_nothing() {
    isolated(
        "one_pass_keeps_what_must_stay_and_a_second_pass_deletes_nothing",
        || {
            let data = paths::get_data_directory();
            let updates = paths::get_updates_directory();
            let logs = paths::get_logs_directory();
            let kept = [
                paths::get_config_path(),
                paths::get_account_store_path(),
                data.join("wallets.db"),
                data.join("notes.txt"),
                paths::get_quarantine_directory()
                    .join("tokens-20261010T000000Z")
                    .join("tokens.db"),
                paths::get_wallet_reset_directory("20261010-000000").join("positions.db"),
                paths::get_base_directory()
                    .join("analysis-exports")
                    .join("report.csv"),
                updates.join("unrelated.bin"),
                logs.join("screenerbot_2026-10-10_10-00-00.log"),
                logs.join("screenerbot_2026-10-10_11-00-00.log"),
            ];
            for path in &kept {
                write(path, b"kept", DAY * 3);
            }
            write(
                &logs.join("screenerbot_2026-10-10_10-00-00.log"),
                b"recent",
                Duration::from_secs(60),
            );
            write(
                &logs.join("screenerbot_2026-10-10_11-00-00.log"),
                b"recent",
                Duration::from_secs(30),
            );
            let removed = [
                data.join("mcp.json"),
                data.join("rpc_stats.json"),
                data.join("screenerbot.db"),
                updates.join("ScreenerBot-v0.0.1-macOS-arm64.dmg"),
                updates.join("ScreenerBot-v0.0.1-macOS-arm64.dmg.part"),
                logs.join("screenerbot_2026-10-01_00-00-00.log"),
                logs.join("screenerbot_2026-10-02_00-00-00.log"),
            ];
            for path in &removed {
                let bytes: &[u8] = if path.ends_with("screenerbot.db") {
                    b""
                } else {
                    b"stale"
                };
                write(path, bytes, DAY * 3);
            }
            fs::create_dir_all(data.join("cache_pool")).unwrap();

            let first = pass(true);

            for path in &kept {
                assert!(path.exists(), "{} was deleted", path.display());
            }
            for path in &removed {
                assert!(!path.exists(), "{} survived", path.display());
            }
            assert!(!data.join("cache_pool").exists());
            assert!(deletions(&first) > removed.len());
            assert_eq!(deletions(&pass(true)), 0);
        },
    );
}

#[test]
fn a_store_that_is_not_open_is_never_recreated() {
    isolated("a_store_that_is_not_open_is_never_recreated", || {
        let moved = paths::get_quarantine_directory()
            .join("tokens-20261010T000000Z")
            .join("tokens.db");
        write(&moved, b"quarantined", DAY);
        pass(true);
        for store in STORES {
            for path in store.paths() {
                assert!(!path.exists(), "{} was created", path.display());
            }
        }
        assert_eq!(fs::read(&moved).unwrap(), b"quarantined");
    });
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_in_a_family_directory_is_left_with_its_target() {
    isolated(
        "a_symbolic_link_in_a_family_directory_is_left_with_its_target",
        || {
            let target = paths::get_base_directory().join("elsewhere.log");
            write(&target, b"outside", DAY * 30);
            let logs = paths::get_logs_directory();
            fs::create_dir_all(&logs).unwrap();
            let link = logs.join("screenerbot_2020-01-01_00-00-00.log");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            let outside_dir = paths::get_base_directory().join("empty-elsewhere");
            fs::create_dir_all(&outside_dir).unwrap();
            fs::create_dir_all(paths::get_data_directory()).unwrap();
            let dir_link = paths::get_data_directory().join("cache_pool");
            std::os::unix::fs::symlink(&outside_dir, &dir_link).unwrap();

            pass(false);

            assert!(fs::symlink_metadata(&link).is_ok());
            assert_eq!(fs::read(&target).unwrap(), b"outside");
            assert!(fs::symlink_metadata(&dir_link).is_ok());
            assert!(outside_dir.exists());
        },
    );
}

#[cfg(unix)]
#[test]
fn a_deletion_that_fails_is_retried_by_the_next_pass() {
    isolated("a_deletion_that_fails_is_retried_by_the_next_pass", || {
        use std::os::unix::fs::PermissionsExt;
        let data = paths::get_data_directory();
        let legacy = data.join("mcp.json");
        let old_log = paths::get_logs_directory().join("screenerbot_2026-10-01_00-00-00.log");
        write(&legacy, b"stale", DAY);
        write(&old_log, b"stale", DAY * 3);
        fs::set_permissions(&data, fs::Permissions::from_mode(0o555)).unwrap();

        pass(false);
        assert!(legacy.exists());
        assert!(!old_log.exists());

        fs::set_permissions(&data, fs::Permissions::from_mode(0o755)).unwrap();
        pass(false);
        assert!(!legacy.exists());
    });
}

/// A store file holding `rows` rows of which the first `keep` survive, so the rest
/// are free pages, open in a pool configured like the store's owner.
fn fragmented_store(spec: &'static StoreSpec, path: &Path, rows: i64, keep: i64) -> StoreHandle {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    {
        let conn = Connection::open(path).unwrap();
        configure_connection(&conn, spec.config).unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, v BLOB)")
            .unwrap();
        for id in 0..rows {
            conn.execute("INSERT INTO t (id, v) VALUES (?1, zeroblob(4000))", [id])
                .unwrap();
        }
        conn.execute("DELETE FROM t WHERE id >= ?1", [keep])
            .unwrap();
        conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")
            .unwrap();
    }
    let config = spec.config;
    let manager = SqliteConnectionManager::file(path)
        .with_init(move |conn| configure_connection(conn, config));
    StoreHandle {
        spec,
        path: path.to_path_buf(),
        pool: Pool::builder().max_size(1).build(manager).unwrap(),
    }
}

fn pragma(handle: &StoreHandle, name: &str) -> i64 {
    handle
        .pool
        .get()
        .unwrap()
        .pragma_query_value(None, name, |row| row.get(0))
        .unwrap()
}

fn store_actions(store: &'static str, path: &Path) -> [Action; 3] {
    let path = path.to_path_buf();
    [
        Action::Checkpoint {
            store,
            path: path.clone(),
        },
        Action::ConvertToIncremental {
            store,
            path: path.clone(),
        },
        Action::Reclaim { store, path },
    ]
}

#[cfg(unix)]
#[test]
fn a_checkpoint_only_store_is_never_rewritten_whatever_it_is_asked() {
    isolated(
        "a_checkpoint_only_store_is_never_rewritten_whatever_it_is_asked",
        || {
            use std::os::unix::fs::MetadataExt;
            for spec in STORES
                .into_iter()
                .filter(|spec| spec.compaction == Compaction::CheckpointOnly)
            {
                let path = paths::get_base_directory()
                    .join("stores")
                    .join(spec.paths()[0].file_name().unwrap());
                let store = fragmented_store(spec, &path, 400, 10);
                let inode = fs::metadata(&path).unwrap().ino();
                let pages = pragma(&store, "page_count");
                let free = pragma(&store, "freelist_count");
                let mode = pragma(&store, "auto_vacuum");
                assert!(free > 0);

                let outcome =
                    carry_out(&store_actions(spec.id, &path), std::slice::from_ref(&store));

                assert_eq!(outcome.failures, 0);
                assert_eq!(outcome.store_operations, 1, "{}", spec.id);
                assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
                assert_eq!(pragma(&store, "page_count"), pages);
                assert_eq!(pragma(&store, "freelist_count"), free);
                assert_eq!(pragma(&store, "auto_vacuum"), mode);
            }
            assert!(catalog::WALLETS.compaction == Compaction::CheckpointOnly);
        },
    );
}

#[test]
fn a_checkpoint_empties_the_write_ahead_log() {
    isolated("a_checkpoint_empties_the_write_ahead_log", || {
        let path = paths::get_base_directory().join("stores").join("events.db");
        let events = fragmented_store(&catalog::EVENTS, &path, 10, 10);
        assert_eq!(events.spec.config.cache_size, EVENTS_WRITE_DB.cache_size);
        events
            .pool
            .get()
            .unwrap()
            .execute("INSERT INTO t (id, v) VALUES (1000, zeroblob(4000))", [])
            .unwrap();
        let wal = PathBuf::from(format!("{}-wal", path.display()));
        assert!(fs::metadata(&wal).unwrap().len() > 0);

        let [checkpoint, _, _] = store_actions("events", &path);
        let outcome = carry_out(&[checkpoint], std::slice::from_ref(&events));

        assert_eq!((outcome.store_operations, outcome.failures), (1, 0));
        assert_eq!(fs::metadata(&wal).unwrap().len(), 0);
    });
}

#[test]
fn reclaiming_empties_the_freelist_whatever_its_share_of_the_file() {
    isolated(
        "reclaiming_empties_the_freelist_whatever_its_share_of_the_file",
        || {
            // Mostly free (rebuilt), partly free (incremental), and nothing free.
            for (rows, keep) in [(400, 10), (400, 300), (20, 20)] {
                let path = paths::get_base_directory()
                    .join(format!("stores-{rows}-{keep}"))
                    .join("events.db");
                let events = fragmented_store(&catalog::EVENTS, &path, rows, keep);
                let pages = pragma(&events, "page_count");
                let [_, _, reclaim] = store_actions("events", &path);

                let outcome = carry_out(&[reclaim], std::slice::from_ref(&events));

                assert_eq!((outcome.store_operations, outcome.failures), (1, 0));
                assert_eq!(pragma(&events, "freelist_count"), 0, "{rows}/{keep}");
                assert!(pragma(&events, "page_count") <= pages);
                let kept: i64 = events
                    .pool
                    .get()
                    .unwrap()
                    .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(kept, keep);
            }
        },
    );
}

#[test]
fn a_store_in_none_auto_vacuum_mode_is_converted_once() {
    isolated("a_store_in_none_auto_vacuum_mode_is_converted_once", || {
        let path = paths::get_base_directory()
            .join("stores")
            .join("copy_trading.db");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch("PRAGMA auto_vacuum = NONE; CREATE TABLE t (id INTEGER PRIMARY KEY);")
            .unwrap();
        let config = catalog::COPY_TRADING.config;
        let manager = SqliteConnectionManager::file(&path)
            .with_init(move |conn| configure_connection(conn, config));
        let store = StoreHandle {
            spec: &catalog::COPY_TRADING,
            path: path.clone(),
            pool: Pool::builder().max_size(1).build(manager).unwrap(),
        };
        assert_eq!(pragma(&store, "auto_vacuum"), 0);
        let [_, convert, _] = store_actions("copy_trading", &path);

        for _ in 0..2 {
            let outcome = carry_out(std::slice::from_ref(&convert), std::slice::from_ref(&store));
            assert_eq!(outcome.failures, 0);
            assert_eq!(pragma(&store, "auto_vacuum"), 2);
        }
    });
}

#[test]
fn a_store_operation_for_a_store_not_open_is_skipped() {
    isolated("a_store_operation_for_a_store_not_open_is_skipped", || {
        let path = paths::get_data_directory().join("events.db");
        let outcome = carry_out(&store_actions("events", &path), &[]);
        assert_eq!((outcome.store_operations, outcome.failures), (0, 0));
        assert!(!path.exists());
    });
}
