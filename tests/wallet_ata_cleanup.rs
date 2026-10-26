// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The empty-account cleanup store.
//!
//! The failed-ATA cache must not park an empty account forever: `close_ata` records
//! every close failure, including a transient RPC error or the race where the account
//! was already gone, and `scan_closeable_atas` then hides every cached row from later
//! cleanup cycles. A cached transient failure therefore has to carry the time it
//! becomes eligible again, or the rent of that account is never reclaimed by the
//! periodic cleanup.
//!
//! The store once also held the tables of the removed tools area and of the manual
//! cleanup sessions. A store written by the last release that had them must open, drop
//! exactly those tables, keep every failed-close row and stay consistent across the
//! next launch.

use crate::common;

use chrono::{Duration, Utc};
use rusqlite::Connection;
use screenerbot::database;
use screenerbot::wallets::ata_cleanup::database::{
    get_excluded_atas_for_wallet, get_failed_atas_for_wallet, init_ata_cleanup_db,
    upsert_failed_ata,
};

const WALLET: &str = "Wallet1111111111111111111111111111111111111";
const ATA: &str = "Ata11111111111111111111111111111111111111111";
const PERMANENT_ATA: &str = "PermAta111111111111111111111111111111111111";
const MINT: &str = "TokenMint111111111111111111111111111111111";

const UPGRADE_CHILD_ENV: &str = "SCREENERBOT_ATA_CLEANUP_UPGRADE_CHILD";

/// The tables of the removed tools area and of the manual cleanup sessions that the
/// store's upgrade drops.
const RETIRED_TABLES: [&str; 7] = [
    "ata_closures",
    "ata_sessions",
    "mw_sessions",
    "mw_wallet_ops",
    "schema_version",
    "tool_favorites",
    "watched_tokens",
];

#[test]
fn a_transient_close_failure_is_cached_with_a_retry_time() {
    common::run_isolated(
        module_path!(),
        "a_transient_close_failure_is_cached_with_a_retry_time",
        || async {
            let _dir = common::isolated_env();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            init_ata_cleanup_db().expect("open the ATA cleanup store");

            upsert_failed_ata(
                ATA,
                Some(MINT),
                WALLET,
                "rpc request timed out while sending the close transaction",
                false,
            )
            .expect("record the failed close");

            let rows = get_failed_atas_for_wallet(WALLET).expect("read the failed ATAs");
            assert_eq!(rows.len(), 1, "the failed ATA is cached once");
            let row = &rows[0];
            assert!(
                !row.is_permanent_failure,
                "a transient failure is not permanent"
            );
            assert!(
                row.next_retry_at.is_some(),
                "a non-permanent failure must record when the ATA is retried, \
                 otherwise the scan skips it for good: {row:?}"
            );
        },
    );
}

#[test]
fn a_cached_failure_is_excluded_only_until_its_retry_time() {
    common::run_isolated(
        module_path!(),
        "a_cached_failure_is_excluded_only_until_its_retry_time",
        || async {
            let _dir = common::isolated_env();
            screenerbot::paths::ensure_all_directories().expect("create data directories");
            init_ata_cleanup_db().expect("open the ATA cleanup store");

            upsert_failed_ata(ATA, Some(MINT), WALLET, "rpc request timed out", false)
                .expect("record the transient failure");
            upsert_failed_ata(PERMANENT_ATA, Some(MINT), WALLET, "owner mismatch", true)
                .expect("record the permanent failure");

            let now = Utc::now();
            let parked = get_excluded_atas_for_wallet(WALLET, now).expect("read exclusions");
            assert!(
                parked.contains(ATA),
                "a transient failure is parked until its retry time"
            );
            assert!(parked.contains(PERMANENT_ATA));

            let later = get_excluded_atas_for_wallet(WALLET, now + Duration::hours(25))
                .expect("read exclusions after the longest backoff");
            assert!(
                !later.contains(ATA),
                "a transient failure whose retry time has passed is attempted again"
            );
            assert!(
                later.contains(PERMANENT_ATA),
                "a permanent failure is never attempted again"
            );
        },
    );
}

/// Open the store in a fresh process on `directory`: the store memoises its handle in
/// a process global, so each launch of the upgrade path needs its own process.
fn launch_on(directory: &std::path::Path) {
    let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg(common::test_name(
            module_path!(),
            "a_store_from_the_tools_release_drops_only_the_retired_tables",
        ))
        .arg("--nocapture")
        .env(UPGRADE_CHILD_ENV, "1")
        .env("SCREENERBOT_DATA_DIR", directory)
        .status()
        .expect("run the ATA cleanup store child");
    assert!(
        status.success(),
        "the ATA cleanup store failed to open on {}",
        directory.display()
    );
}

fn tables(connection: &Connection) -> Vec<String> {
    connection
        .prepare(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .expect("list tables")
        .query_map([], |row| row.get(0))
        .expect("read tables")
        .collect::<Result<_, _>>()
        .expect("collect tables")
}

#[test]
fn a_store_from_the_tools_release_drops_only_the_retired_tables() {
    if std::env::var_os(UPGRADE_CHILD_ENV).is_some() {
        screenerbot::paths::ensure_all_directories().expect("create data directories");
        screenerbot::config::load_config().expect("load isolated configuration");
        init_ata_cleanup_db().expect("open the ATA cleanup store");
        return;
    }

    let directory = tempfile::tempdir().expect("create data directory");
    let path = directory.path().join("data").join("tools.db");
    drop(common::seed_store(
        &path,
        database::ATA_CLEANUP_DB,
        include_str!("fixtures/v0.2.20-tools.sql"),
    ));

    // The upgrade, then the next launch on the upgraded store.
    launch_on(directory.path());
    launch_on(directory.path());

    let connection = Connection::open(&path).expect("open the upgraded store");
    let remaining = tables(&connection);
    for retired in RETIRED_TABLES {
        assert!(
            !remaining.iter().any(|table| table == retired),
            "{retired} must be dropped, found {remaining:?}"
        );
    }
    assert_eq!(
        remaining,
        ["ata_failed_cache", "store_schema"],
        "the store keeps exactly the failed-close cache and its ledger"
    );
    let rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM ata_failed_cache", [], |row| {
            row.get(0)
        })
        .expect("count ata_failed_cache");
    assert_eq!(rows, 1, "ata_failed_cache keeps its sentinel row");
    let (version, min_reader_version): (i64, i64) = connection
        .query_row(
            "SELECT version, min_reader_version FROM store_schema WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read the store ledger");
    assert_eq!((version, min_reader_version), (3, 3));
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .expect("run the integrity check");
    assert_eq!(integrity, "ok");
    let foreign_key_errors: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("run the foreign-key check");
    assert_eq!(foreign_key_errors, 0, "foreign keys stay satisfied");
}
