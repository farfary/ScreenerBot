// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Startup with stores that cannot open: the real boot runs in a child process on a seeded data directory, and the dashboard, the update check and the store report stay up while the process keeps running.

use crate::common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::database::catalog::{StoreSpec, POSITIONS, STORES, STRATEGIES};
use screenerbot::database::health::{self, StoreHealth};
use screenerbot::database::recovery::FailureKind;

const CHILD_ENV: &str = "SCREENERBOT_STORAGE_UNAVAILABLE_CHILD";
/// The longest a boot may take to bring up the webserver and the update check.
const STARTUP_BUDGET: Duration = Duration::from_secs(90);
/// The longest the graceful shutdown after the checks may take.
const SHUTDOWN_BUDGET: Duration = Duration::from_secs(60);
/// The services that start in every storage state.
const ALWAYS_STARTED: [&str; 2] = ["webserver", "update_check"];

/// Every catalog store is a directory where its file belongs, so no store can open. A
/// boot in Explore Mode still serves the dashboard and runs the update check, reports
/// every store unavailable on `/api/status`, and keeps running until it is stopped.
#[test]
fn startup_with_every_store_unavailable_keeps_the_dashboard_and_update_check() {
    let test = "startup_with_every_store_unavailable_keeps_the_dashboard_and_update_check";
    if std::env::var_os(CHILD_ENV).is_some() {
        boot_with_every_store_unavailable();
        return;
    }
    let directory = tempfile::tempdir().expect("create isolated data directory");
    let data = directory.path().join("data");
    for store in STORES {
        for path in store.paths() {
            std::fs::create_dir_all(under(directory.path(), &path))
                .expect("put a directory where the store belongs");
        }
    }
    // A configuration without a wallet boots Explore Mode, the tier the status route
    // answers in before setup is complete.
    std::fs::create_dir_all(&data).expect("create data directory");
    std::fs::write(data.join("config.toml"), "").expect("write empty configuration");

    run_child(test, directory.path());
}

/// A ledger store stamped by a newer core is isolated: it is left byte-identical, its
/// health records it unavailable and kept in place, and startup continues instead of
/// stopping on a storage screen. The strategies store is opened by the dashboard's own
/// persistence in every boot mode, so it stopped every boot before.
#[test]
fn a_ledger_store_too_new_is_isolated_and_startup_continues() {
    let test = "a_ledger_store_too_new_is_isolated_and_startup_continues";
    if std::env::var_os(CHILD_ENV).is_some() {
        boot_with_ledger_stores_too_new();
        return;
    }
    let directory = tempfile::tempdir().expect("create isolated data directory");
    let stamped: Vec<(PathBuf, Vec<u8>)> = [&POSITIONS, &STRATEGIES]
        .into_iter()
        .map(|store| {
            let path = data_path(directory.path(), store);
            stamp_too_new(&path);
            let bytes = std::fs::read(&path).expect("read stamped store");
            (path, bytes)
        })
        .collect();

    run_child(test, directory.path());

    for (path, before) in stamped {
        assert_eq!(
            std::fs::read(&path).expect("read isolated store"),
            before,
            "{} changed although it was isolated",
            path.display()
        );
    }
}

fn run_child(test: &str, base: &Path) {
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg(common::test_name(module_path!(), test))
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env("SCREENERBOT_DATA_DIR", base)
        .status()
        .expect("run isolated startup child");
    assert!(status.success(), "{test} failed in its child process");
}

/// The Solana file of `store` under the data root `base`, as the child resolves it.
fn data_path(base: &Path, store: &StoreSpec) -> PathBuf {
    under(base, &store.path(ChainId::Solana))
}

/// `path`, a file under this process's data root, moved under the data root `base`.
fn under(base: &Path, path: &Path) -> PathBuf {
    base.join(
        path.strip_prefix(screenerbot::paths::get_base_directory())
            .expect("store path under the data root"),
    )
}

/// A store whose ledger row names a schema no released core reads. It is written in WAL
/// mode, the journal every open sets, so opening it changes no byte of the file.
fn stamp_too_new(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("store directory"))
        .expect("create store directory");
    let connection = Connection::open(path).expect("create stamped store");
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE store_schema (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 version INTEGER NOT NULL,
                 min_reader_version INTEGER NOT NULL,
                 written_by TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             INSERT INTO store_schema VALUES (1, 999, 999, '99.0.0', '2030-01-01');
             CREATE TABLE written_by_a_newer_core (value TEXT NOT NULL);
             INSERT INTO written_by_a_newer_core VALUES ('kept');",
        )
        .expect("stamp store too new");
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint stamped store");
}

fn boot_with_every_store_unavailable() {
    let port = free_port();
    boot(port, |port| async move {
        let response: serde_json::Value =
            reqwest::get(format!("http://127.0.0.1:{port}/api/status"))
                .await
                .expect("request /api/status")
                .error_for_status()
                .expect("/api/status answers")
                .json()
                .await
                .expect("decode /api/status");
        let stores = response["stores"]
            .as_array()
            .unwrap_or_else(|| panic!("/api/status reports no stores: {response}"));
        for store in STORES {
            let reported: Vec<&serde_json::Value> = stores
                .iter()
                .filter(|entry| entry["store"] == store.id)
                .collect();
            assert!(
                !reported.is_empty(),
                "{} is missing from /api/status",
                store.id
            );
            for entry in reported {
                assert_eq!(entry["state"], "unavailable", "{entry}");
                assert_eq!(entry["isolated"], false, "{entry}");
                assert!(entry["reconciled"].is_boolean(), "{entry}");
            }
        }
    });
}

fn boot_with_ledger_stores_too_new() {
    let port = free_port();
    boot(port, |_| async {
        for store in [&POSITIONS, &STRATEGIES] {
            let status = health::status(&store.path(ChainId::Solana))
                .unwrap_or_else(|| panic!("{} has no recorded health", store.id));
            match status.health {
                StoreHealth::Unavailable {
                    failure, isolated, ..
                } => {
                    assert_eq!(failure, FailureKind::StoreTooNew, "{}", store.id);
                    assert!(isolated, "{} must be isolated", store.id);
                }
                other => panic!("{} must be unavailable, got {other:?}", store.id),
            }
        }
    });
}

/// Run the real boot with the webserver on `port`, wait until the always-started
/// services run, call `inspect`, then stop the process gracefully. The boot must still
/// be running when the checks are made and must end without an error.
fn boot<F, Fut>(port: u16, inspect: F)
where
    F: FnOnce(u16) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    screenerbot::arguments::set_cmd_args(vec![
        "screenerbot".to_owned(),
        "--port".to_owned(),
        port.to_string(),
    ]);
    screenerbot::paths::ensure_all_directories().expect("create startup directories");
    screenerbot::config::load_config().expect("load isolated configuration");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create multi-thread runtime");
    runtime.block_on(async {
        let run = tokio::spawn(screenerbot::run::run_bot());
        let started = Instant::now();
        loop {
            assert!(
                !run.is_finished(),
                "the process stopped during startup: {:?}",
                run.await
            );
            if running_services()
                .await
                .is_some_and(|running| ALWAYS_STARTED.iter().all(|name| running.contains(name)))
            {
                break;
            }
            assert!(
                started.elapsed() < STARTUP_BUDGET,
                "{ALWAYS_STARTED:?} did not start within {STARTUP_BUDGET:?}"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        inspect(port).await;
        assert!(!run.is_finished(), "the process stopped after startup");

        screenerbot::global::request_restart();
        tokio::time::timeout(SHUTDOWN_BUDGET, run)
            .await
            .expect("graceful shutdown finished in time")
            .expect("run task joined")
            .expect("the process ended without an error");
    });
}

/// The services with running tasks, once startup has handed the manager back.
async fn running_services() -> Option<Vec<&'static str>> {
    let manager = screenerbot::services::get_service_manager().await?;
    let guard = manager.read().await;
    guard
        .as_ref()
        .map(|manager| manager.get_running_service_names())
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("reserve a loopback port")
        .port()
}
