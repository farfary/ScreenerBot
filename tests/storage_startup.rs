// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Regression coverage for the SQLite stores opened during desktop startup.

use crate::common;

use std::collections::BTreeSet;
use std::process::Command;

use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::database::catalog::STORES;
use screenerbot::paths::{chain_db_path, DbKind};
use screenerbot::storage::types::Classification;

const CHILD_ENV: &str = "SCREENERBOT_STORAGE_STARTUP_CHILD";
const CACHE_CHILD_ENV: &str = "SCREENERBOT_CACHE_RECOVERY_CHILD";
/// The longest the storage phase may take to open every store, the owner's real stores
/// included.
const STORAGE_PHASE_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);
/// The cache stores whose owners open through the recovery path.
const RECOVERED_CACHE_FILES: &[&str] = &["ohlcvs.db", "pools.db", "rpc_stats.db"];
/// Tables whose rows are still pruned on their owner's own schedule and therefore carry
/// no catalog retention rule. The list only shrinks.
const OWNER_PRUNED_TABLES: &[(&str, &str)] = &[
    ("actions", "action_steps"),
    ("actions", "actions"),
    ("ai", "ai_decision_history"),
    ("ai_chat", "ai_task_runs"),
    ("events", "events"),
    ("pools", "price_history"),
    ("rpc_stats", "calls"),
    ("rpc_stats", "minute_buckets"),
    ("tokens", "rejection_history"),
    ("tokens", "rejection_stats"),
    ("tools", "ata_failed_cache"),
    ("wallet", "wallet_dashboard_metrics"),
    ("wallet", "wallet_snapshots"),
];

/// The Solana file name of every catalog store: the startup matrix covers exactly the
/// stores the catalog declares.
fn database_files() -> Vec<String> {
    STORES
        .into_iter()
        .map(|store| {
            store
                .path(ChainId::Solana)
                .file_name()
                .expect("catalog store file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn database_filename_inventory_matches_production_paths() {
    let mut production = database_filenames(include_str!("../src/paths/database.rs"));
    production.extend(database_filenames(include_str!(
        "../src/rpc/stats/database.rs"
    )));

    // The four chain-scoped stores resolve through `paths::chain_db_path`,
    // which carries no literal `join("...")` filename: fold their Solana
    // file names in from the resolver itself, and pin that (a) Solana keeps
    // the historical un-suffixed names and (b) the `<stem>-<chain>.db`
    // pattern a later chain variant takes builds on the same stems.
    for (kind, solana_name) in [
        (DbKind::Tokens, "tokens.db"),
        (DbKind::Pools, "pools.db"),
        (DbKind::Ohlcvs, "ohlcvs.db"),
        (DbKind::RpcStats, "rpc_stats.db"),
    ] {
        let path = chain_db_path(kind, ChainId::Solana);
        let name = path
            .file_name()
            .expect("chain-scoped store file name")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            name, solana_name,
            "Solana keeps its historical file name for {kind:?}"
        );
        assert_eq!(
            solana_name,
            format!("{}.db", kind.file_stem()),
            "the <stem>-<chain>.db pattern must build on the legacy stems"
        );
        production.insert(name);
    }

    let catalog = database_files();
    let expected: BTreeSet<String> = catalog.iter().cloned().collect();
    assert_eq!(
        expected.len(),
        catalog.len(),
        "two catalog stores share a file"
    );
    assert_eq!(
        production, expected,
        "every production SQLite filename is a catalog store"
    );
}

#[test]
fn startup_storage_matrix_survives_two_fresh_process_launches() {
    if std::env::var_os(CHILD_ENV).is_some() {
        initialize_startup_stores();
        return;
    }

    let directory = tempfile::tempdir().expect("create isolated startup-storage directory");
    assert_two_launches_keep_every_store(directory.path());
}

/// The owner's real stores, cloned through the backup API from read-only connections,
/// open in two fresh processes with every row-level check clean. Whatever release created
/// a store, the startup initializers must accept it.
#[test]
#[ignore = "requires a local ScreenerBot data directory; runs only against a throwaway clone"]
fn real_stores_survive_two_fresh_process_launches_on_a_clone() {
    let files = database_files();
    let names: Vec<&str> = files.iter().map(String::as_str).collect();
    let Some(clone) = common::clone_real_stores(&names) else {
        return;
    };
    assert_two_launches_keep_every_store(clone.path());
}

/// A released build reopening the stores between two launches re-creates the legacy names
/// it knows and writes under them. The next launch folds every such object into its
/// canonical name, keeps the canonical indexes and every row-level check clean, and a
/// further launch changes nothing.
#[test]
fn startup_storage_matrix_survives_a_released_build_between_launches() {
    if std::env::var_os(CHILD_ENV).is_some() {
        return;
    }
    let directory = tempfile::tempdir().expect("create isolated startup-storage directory");
    let data = directory.path().join("data");
    launch_startup_stores(directory.path(), 1);

    let mut indexes = Vec::new();
    for reopen in common::released_reopen::RELEASED_REOPENS {
        let connection = Connection::open(data.join(reopen.file)).expect("open launched store");
        indexes.push(index_names(&connection));
        reopen.apply(&connection);
    }

    launch_startup_stores(directory.path(), 2);
    let mut schemas = Vec::new();
    for (reopen, before) in common::released_reopen::RELEASED_REOPENS
        .iter()
        .zip(indexes)
    {
        let connection = Connection::open(data.join(reopen.file)).expect("open folded store");
        reopen.assert_legacy_objects_gone(&connection);
        reopen.assert_merged(&connection);
        assert_eq!(
            index_names(&connection),
            before,
            "{} lost or gained an index",
            reopen.file
        );
        schemas.push(schema(&connection));
    }
    assert_every_store_is_clean(directory.path(), "2");

    launch_startup_stores(directory.path(), 3);
    for (reopen, before) in common::released_reopen::RELEASED_REOPENS
        .iter()
        .zip(schemas)
    {
        let connection = Connection::open(data.join(reopen.file)).expect("open relaunched store");
        assert_eq!(
            schema(&connection),
            before,
            "{} changed on a further launch",
            reopen.file
        );
    }
}

/// A cache store whose file is not a database moves whole into the quarantine directory
/// with a manifest, and its owner opens a rebuilt store in its place; the next launch
/// opens the rebuilt store without quarantining it again.
#[test]
fn unreadable_cache_stores_are_quarantined_and_rebuilt() {
    if std::env::var_os(CACHE_CHILD_ENV).is_some() {
        open_cache_stores_through_recovery();
        return;
    }
    let directory = tempfile::tempdir().expect("create isolated cache-recovery directory");
    let data = directory.path().join("data");
    std::fs::create_dir_all(&data).expect("create data directory");
    let damaged = b"this file holds no SQLite header".repeat(128);
    for file in RECOVERED_CACHE_FILES {
        std::fs::write(data.join(file), &damaged).expect("write damaged cache store");
    }

    for launch in 1..=2 {
        let status = Command::new(std::env::current_exe().expect("test executable"))
            .arg("--exact")
            .arg(common::test_name(
                module_path!(),
                "unreadable_cache_stores_are_quarantined_and_rebuilt",
            ))
            .arg("--nocapture")
            .env(CACHE_CHILD_ENV, "1")
            .env("SCREENERBOT_DATA_DIR", directory.path())
            .status()
            .expect("run isolated cache-recovery child");
        assert!(status.success(), "cache-recovery launch {launch} failed");
    }

    let quarantine = data.join("quarantine");
    for file in RECOVERED_CACHE_FILES {
        let stem = file.trim_end_matches(".db");
        let moved: Vec<std::path::PathBuf> = std::fs::read_dir(&quarantine)
            .expect("list quarantine directory")
            .map(|entry| entry.expect("quarantine entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.strip_prefix(stem))
                    .is_some_and(|rest| rest.starts_with('-') && rest[1..].starts_with("20"))
            })
            .collect();
        assert_eq!(
            moved.len(),
            1,
            "{file} was quarantined {} times",
            moved.len()
        );
        assert_eq!(
            std::fs::read(moved[0].join(file)).expect("read quarantined store"),
            damaged,
            "{file} changed on its way into quarantine"
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(moved[0].join("manifest.json")).expect("read quarantine manifest"),
        )
        .expect("decode quarantine manifest");
        assert_eq!(manifest["store"], stem);
        assert_eq!(manifest["class"], "cache");
        assert!(
            manifest["reason"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty()),
            "{file} manifest names no reason"
        );

        let rebuilt = Connection::open(data.join(file)).expect("open rebuilt store");
        assert_eq!(
            rebuilt
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .expect("run integrity check"),
            "ok",
            "rebuilt {file} failed its integrity check"
        );
        assert!(
            rebuilt
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
                    [],
                    |row| { row.get::<_, i64>(0) }
                )
                .expect("count rebuilt tables")
                > 0,
            "{file} was not rebuilt"
        );
    }
}

fn assert_two_launches_keep_every_store(directory: &std::path::Path) {
    for launch in 1..=2 {
        launch_startup_stores(directory, launch);
    }
    assert_every_store_is_clean(directory, "2");
    assert_every_table_has_a_retention_rule(directory);
}

/// Every table a store creates has exactly one retention rule in the catalog, or is
/// still pruned by its owner, so a new table cannot grow without a rule. A rule never
/// names a table its store does not create.
fn assert_every_table_has_a_retention_rule(directory: &std::path::Path) {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for store in STORES {
        let file = store
            .path(ChainId::Solana)
            .file_name()
            .expect("catalog store file name")
            .to_owned();
        let connection =
            Connection::open(directory.join("data").join(file)).expect("open launched store");
        let tables: BTreeSet<String> = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' \
                 AND name NOT LIKE 'sqlite_%' AND name != '__startup_storage_test_sentinel'",
            )
            .expect("prepare table listing")
            .query_map([], |row| row.get(0))
            .expect("list tables")
            .collect::<rusqlite::Result<_>>()
            .expect("read tables");
        for table in &tables {
            seen.insert((store.id, table.clone()));
            let declared = store.retention.rule(table).is_some();
            let owner_pruned = OWNER_PRUNED_TABLES.contains(&(store.id, table.as_str()));
            if declared == owner_pruned {
                problems.push(format!(
                    "{}.{table}: declared {declared}, owner-pruned {owner_pruned}",
                    store.id
                ));
            }
        }
        for rule in store.retention.tables {
            if !tables.contains(rule.table()) {
                problems.push(format!(
                    "{}.{}: rule names a table the store does not create",
                    store.id,
                    rule.table()
                ));
            }
        }
    }
    for (store, table) in OWNER_PRUNED_TABLES {
        if !seen.contains(&(*store, (*table).to_owned())) {
            problems.push(format!(
                "{store}.{table}: listed as owner-pruned but not created"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "tables without exactly one retention rule: {problems:#?}"
    );
}

fn launch_startup_stores(directory: &std::path::Path, launch: usize) {
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg(common::test_name(
            module_path!(),
            "startup_storage_matrix_survives_two_fresh_process_launches",
        ))
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .env("SCREENERBOT_DATA_DIR", directory)
        .status()
        .expect("run isolated startup-storage child");
    assert!(status.success(), "startup-storage launch {launch} failed");
}

fn index_names(connection: &Connection) -> BTreeSet<String> {
    connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND sql IS NOT NULL")
        .expect("prepare index listing")
        .query_map([], |row| row.get(0))
        .expect("list indexes")
        .collect::<rusqlite::Result<_>>()
        .expect("read indexes")
}

fn schema(connection: &Connection) -> Vec<(String, String, Option<String>)> {
    connection
        .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .expect("prepare schema listing")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("list schema")
        .collect::<rusqlite::Result<_>>()
        .expect("read schema")
}

fn assert_every_store_is_clean(directory: &std::path::Path, launches: &str) {
    for database in database_files() {
        let path = directory.join("data").join(&database);
        assert!(
            path.exists(),
            "production initializer did not create {database}"
        );
        let connection = Connection::open(&path).expect("reopen initialized database");
        let sentinel: String = connection
            .query_row(
                "SELECT value FROM __startup_storage_test_sentinel WHERE key = 'launches'",
                [],
                |row| row.get(0),
            )
            .expect("preserve startup-storage sentinel across relaunch");
        assert_eq!(
            sentinel, launches,
            "startup-storage sentinel was not preserved for {database}"
        );
        assert_eq!(
            connection
                .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                .expect("run integrity check"),
            "ok",
            "integrity check failed for {database}"
        );
        let foreign_key_errors: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("run foreign-key check");
        assert_eq!(
            foreign_key_errors, 0,
            "foreign-key check failed for {database}"
        );
    }
}

fn database_filenames(source: &str) -> BTreeSet<String> {
    source
        .split("join(\"")
        .skip(1)
        .filter_map(|tail| tail.split('"').next())
        .filter(|name| name.ends_with(".db"))
        .map(str::to_owned)
        .collect()
}

fn initialize_startup_stores() {
    screenerbot::paths::ensure_all_directories().expect("create startup-storage directories");
    screenerbot::config::load_config().expect("load isolated configuration");
    let elapsed = screenerbot::database::open::open_all();
    assert!(
        elapsed < STORAGE_PHASE_BUDGET,
        "the storage phase took {elapsed:?}"
    );
    assert_every_store_opened_by_the_phase();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create multi-thread runtime");
    runtime.block_on(async {
        screenerbot::wallets::initialize()
            .await
            .expect("initialize wallets database");
        if !screenerbot::wallets::has_main_wallet().await {
            screenerbot::wallets::create_wallet(screenerbot::wallets::CreateWalletRequest {
                name: "startup-storage".to_owned(),
                notes: None,
                set_as_main: true,
            })
            .await
            .expect("create isolated main wallet");
        }

        screenerbot::actions::init_database()
            .await
            .expect("initialize actions database");
        screenerbot::events::database::EventsDatabase::new()
            .await
            .expect("initialize events database");
        screenerbot::positions::initialize_positions_database()
            .await
            .expect("initialize positions database");
        screenerbot::transactions::init_transaction_database()
            .await
            .expect("initialize transactions database");
        screenerbot::wallet::initialize_wallet_database()
            .await
            .expect("initialize wallet-monitor database");

        let mut pools = screenerbot::pools::database::PoolsDatabase::new(ChainId::Solana);
        pools.initialize().await.expect("initialize pools database");
        screenerbot::ohlcvs::OhlcvService::initialize(screenerbot::chains::ChainScope::One(
            ChainId::Solana,
        ))
        .await
        .expect("initialize OHLCV database");
        screenerbot::strategies::init_strategy_system(
            screenerbot::strategies::engine::EngineConfig::default(),
        )
        .await
        .expect("initialize strategies database");
    });

    let token_path = screenerbot::chains::get_tokens_db_path();
    screenerbot::tokens::database::TokenDatabase::new(
        &token_path.to_string_lossy(),
        ChainId::Solana,
    )
    .expect("initialize tokens database");
    screenerbot::trader::copy::CopyDatabase::new(ChainId::Solana)
        .expect("initialize copy-trading database");
    screenerbot::wallets::watch::WatchDatabase::new(ChainId::Solana)
        .expect("initialize wallet-watch tables");
    screenerbot::tools::init_tools_db().expect("initialize tools database");
    screenerbot::llm_analysis::init_analysis_database().expect("initialize analysis database");
    screenerbot::assistant::init_chat_db().expect("initialize chat database");
    screenerbot::agent_control::init_store().expect("initialize agent-control database");
    screenerbot::rpc::RpcStatsDatabase::new().expect("initialize RPC statistics database");

    for database in database_files() {
        let path = screenerbot::paths::get_data_directory().join(database);
        let connection = Connection::open(&path).expect("open initialized database for sentinel");
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS __startup_storage_test_sentinel (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )
            .expect("create startup-storage sentinel");
        connection
            .execute(
                "INSERT INTO __startup_storage_test_sentinel (key, value) VALUES ('launches', '1') \
                 ON CONFLICT(key) DO UPDATE SET value = CAST(value AS INTEGER) + 1",
                [],
            )
            .expect("advance startup-storage sentinel");
    }
    assert_every_written_entry_is_declared();
}

/// Every catalog store is opened by the storage phase before any owner asks for it, and
/// every one of them opened.
fn assert_every_store_opened_by_the_phase() {
    let open: Vec<std::path::PathBuf> = screenerbot::database::handles::open_stores()
        .into_iter()
        .map(|handle| handle.path)
        .collect();
    for store in STORES {
        let path = store.path(ChainId::Solana);
        assert!(open.contains(&path), "{} was not opened", store.id);
        let status = screenerbot::database::health::status(&path)
            .unwrap_or_else(|| panic!("{} has no recorded health", store.id));
        assert_eq!(
            status.health,
            screenerbot::database::health::StoreHealth::Healthy,
            "{}",
            store.id
        );
    }
    assert_eq!(open.len(), STORES.len());
}

/// Everything a launch leaves under the data root is a store, a declared family or a
/// layout directory, so housekeeping has a rule for it.
fn assert_every_written_entry_is_declared() {
    let mut pending = vec![screenerbot::paths::get_base_directory()];
    let mut undeclared = Vec::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("list data root") {
            let path = entry.expect("read data root entry").path();
            match screenerbot::storage::classify(&path) {
                Classification::LayoutDirectory => pending.push(path),
                Classification::Store(_) | Classification::Family(_) => {}
                Classification::Unknown => undeclared.push(path),
            }
        }
    }
    assert!(
        undeclared.is_empty(),
        "entries with no store or family declaration: {undeclared:?}"
    );
}

fn open_cache_stores_through_recovery() {
    screenerbot::paths::ensure_all_directories().expect("create cache-recovery directories");
    screenerbot::config::load_config().expect("load isolated configuration");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("create multi-thread runtime");
    runtime.block_on(async {
        screenerbot::ohlcvs::OhlcvService::initialize(screenerbot::chains::ChainScope::One(
            ChainId::Solana,
        ))
        .await
        .expect("open OHLCV store through recovery");
        screenerbot::pools::database::initialize_database(ChainId::Solana)
            .await
            .expect("open pools store through recovery");
        screenerbot::rpc::StatsManager::new(true)
            .await
            .expect("open RPC statistics store through recovery");
    });
}
