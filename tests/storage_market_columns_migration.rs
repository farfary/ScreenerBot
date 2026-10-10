// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Unit-neutral token, pool and OHLCV column names through the production initializers.

mod common;

use rusqlite::Connection;
use screenerbot::database;
use screenerbot::tokens::schema::SCHEMA_VERSION as TOKENS_SCHEMA_VERSION;
use std::path::Path;
use std::process::{Command, Output};

const CHILD: &str = "SCREENERBOT_MARKET_COLUMNS_CHILD";

fn open(dir: &Path, file: &str) -> Connection {
    Connection::open(dir.join("data").join(file)).unwrap()
}

fn run_initializers(dir: &Path) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("market_initializers_child")
        .env(CHILD, "1")
        .env("SCREENERBOT_DATA_DIR", dir)
        .output()
        .unwrap()
}

fn boot(dir: &Path) {
    let output = run_initializers(dir);
    assert!(
        output.status.success(),
        "market initializer failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn market_initializers_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    screenerbot::paths::ensure_all_directories().unwrap();
    screenerbot::config::load_config().unwrap();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut pools =
            screenerbot::pools::database::PoolsDatabase::new(screenerbot::chains::ChainId::Solana);
        pools.initialize().await.unwrap();
        screenerbot::ohlcvs::OhlcvService::initialize(screenerbot::chains::ChainScope::One(
            screenerbot::chains::ChainId::Solana,
        ))
        .await
        .unwrap();
    });
    let token_path = screenerbot::chains::get_tokens_db_path();
    screenerbot::tokens::database::TokenDatabase::new(
        &token_path.to_string_lossy(),
        screenerbot::chains::ChainId::Solana,
    )
    .unwrap();
}

fn new_data_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("data")).unwrap();
    dir
}

/// The candle-data version a fresh OHLCV store records, so the legacy fixture
/// is stamped current and any candle wipe would come from the migration.
fn current_ohlcv_data_version() -> i64 {
    let dir = new_data_dir();
    boot(dir.path());
    open(dir.path(), "ohlcvs.db")
        .query_row(
            "SELECT version FROM ohlcv_data_versions WHERE chain_id = 'solana'",
            [],
            |r| r.get(0),
        )
        .unwrap()
}

fn seed_legacy_tokens(dir: &Path) {
    let tokens = common::seed_store(
        &dir.join("data").join("tokens.db"),
        database::TOKENS_DB,
        include_str!("fixtures/v0.2.13-tokens.sql"),
    );
    tokens
        .execute(
            "INSERT INTO tokens (chain_id, mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at) VALUES ('solana', 'MINT', 'TOK', 'Token', 9, 1, 1, 1)",
            [],
        )
        .unwrap();
    tokens
        .execute(
            "INSERT INTO token_pools (chain_id, mint, pool_address, base_mint, quote_mint, is_sol_pair, liquidity_sol, price_sol, price_native, pool_data_last_fetched_at, pool_data_first_seen_at) VALUES ('solana', 'MINT', 'POOL', 'MINT', 'QUOTE', 1, 12.5, 0.25, '0.5', 1, 1)",
            [],
        )
        .unwrap();
    tokens
        .pragma_update(None, "user_version", TOKENS_SCHEMA_VERSION)
        .unwrap();
}

fn seed_legacy_pools(dir: &Path) {
    let pools = common::seed_store(
        &dir.join("data").join("pools.db"),
        database::POOLS_DB,
        include_str!("fixtures/v0.2.13-pools.sql"),
    );
    pools
        .execute(
            "INSERT INTO price_history (id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, created_at) VALUES (31, 'solana', 'MINT', 'POOL', 2.0, 0.01, 1.0, 7, 60, 3.25, 400.0, '2026-01-01T00:01:00Z')",
            [],
        )
        .unwrap();
    pools.pragma_update(None, "user_version", 1).unwrap();
}

fn seed_legacy_ohlcv(dir: &Path, data_version: i64) {
    let ohlcv = common::seed_store(
        &dir.join("data").join("ohlcvs.db"),
        database::OHLCVS_DB,
        include_str!("fixtures/v0.2.13-ohlcv.sql"),
    );
    ohlcv
        .execute(
            "INSERT INTO ohlcv_pools (id, chain_id, mint, pool_address, dex, liquidity, is_default, is_sol_pair) VALUES (41, 'solana', 'MINT', 'POOL', 'dex', 5.0, 1, 0)",
            [],
        )
        .unwrap();
    ohlcv
        .execute(
            "INSERT INTO ohlcv_candles (chain_id, mint, pool_address, timeframe, timestamp, open, high, low, close, volume) VALUES ('solana', 'MINT', 'POOL', '1m', 60, 1, 2, 0.5, 1.5, 10)",
            [],
        )
        .unwrap();
    ohlcv
        .execute(
            "INSERT INTO ohlcv_monitor_config (chain_id, mint, priority, last_activity) VALUES ('solana', 'MINT', 'high', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    ohlcv
        .execute(
            "INSERT INTO ohlcv_data_versions (chain_id, version) VALUES ('solana', ?1)",
            [data_version],
        )
        .unwrap();
}

fn seed_legacy_stores(dir: &Path, data_version: i64) {
    seed_legacy_tokens(dir);
    seed_legacy_pools(dir);
    seed_legacy_ohlcv(dir, data_version);
}

fn schema(conn: &Connection) -> Vec<(String, String, Option<String>)> {
    conn.prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

fn columns(conn: &Connection, table: &str) -> Vec<String> {
    conn.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
}

fn assert_columns(conn: &Connection, table: &str, present: &[&str], absent: &[&str]) {
    let live = columns(conn, table);
    for name in present {
        assert!(live.iter().any(|c| c == name), "{table} missing {name}");
    }
    for name in absent {
        assert!(!live.iter().any(|c| c == name), "{table} kept {name}");
    }
}

fn assert_integrity(conn: &Connection) {
    assert_eq!(
        conn.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

fn assert_canonical_names(dir: &Path) {
    let tokens = open(dir, "tokens.db");
    assert_columns(
        &tokens,
        "token_pools",
        &[
            "is_native_pair",
            "liquidity_native",
            "price_sol",
            "price_native",
        ],
        &["is_sol_pair", "liquidity_sol"],
    );
    assert_integrity(&tokens);

    let pools = open(dir, "pools.db");
    assert_columns(
        &pools,
        "price_history",
        &["native_reserves", "price_sol"],
        &["sol_reserves"],
    );
    assert_integrity(&pools);

    let ohlcv = open(dir, "ohlcvs.db");
    assert_columns(&ohlcv, "ohlcv_pools", &["is_native_pair"], &["is_sol_pair"]);
    assert_integrity(&ohlcv);
}

fn assert_sentinels(dir: &Path) {
    assert_eq!(
        open(dir, "tokens.db")
            .query_row(
                "SELECT is_native_pair, liquidity_native, price_sol, price_native FROM token_pools WHERE pool_address = 'POOL'",
                [],
                |r| Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, String>(3)?
                )),
            )
            .unwrap(),
        (1, 12.5, 0.25, "0.5".to_owned())
    );
    assert_eq!(
        open(dir, "pools.db")
            .query_row(
                "SELECT price_sol, native_reserves, token_reserves FROM price_history WHERE id = 31",
                [],
                |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?)),
            )
            .unwrap(),
        (0.01, 3.25, 400.0)
    );
    let ohlcv = open(dir, "ohlcvs.db");
    assert_eq!(
        ohlcv
            .query_row(
                "SELECT pool_address, is_native_pair FROM ohlcv_pools WHERE id = 41",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .unwrap(),
        ("POOL".to_owned(), 0)
    );
    for (table, expected) in [("ohlcv_candles", 1), ("ohlcv_monitor_config", 1)] {
        assert_eq!(
            ohlcv
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            expected,
            "{table} rows changed"
        );
    }
}

#[test]
fn legacy_market_names_migrate_to_unit_neutral_names() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let data_version = current_ohlcv_data_version();
    let dir = new_data_dir();
    seed_legacy_stores(dir.path(), data_version);

    boot(dir.path());
    assert_canonical_names(dir.path());
    assert_sentinels(dir.path());
    let first: Vec<_> = ["tokens.db", "pools.db", "ohlcvs.db"]
        .into_iter()
        .map(|file| schema(&open(dir.path(), file)))
        .collect();

    boot(dir.path());
    let second: Vec<_> = ["tokens.db", "pools.db", "ohlcvs.db"]
        .into_iter()
        .map(|file| schema(&open(dir.path(), file)))
        .collect();
    assert_eq!(second, first, "second open changed a schema");
    assert_canonical_names(dir.path());
    assert_sentinels(dir.path());
}

/// No released build re-adds the legacy token or price-history names, so a store
/// holding both names has unknown provenance and its open is refused unchanged.
#[test]
fn market_store_with_both_names_refuses_to_open() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let data_version = current_ohlcv_data_version();
    for (file, conflict, expected) in [
        (
            "tokens.db",
            "ALTER TABLE token_pools ADD COLUMN is_native_pair INTEGER",
            "token_pools.is_sol_pair and token_pools.is_native_pair",
        ),
        (
            "pools.db",
            "ALTER TABLE price_history ADD COLUMN native_reserves REAL",
            "price_history.sol_reserves and price_history.native_reserves",
        ),
    ] {
        let dir = new_data_dir();
        seed_legacy_stores(dir.path(), data_version);
        let store = open(dir.path(), file);
        store.execute_batch(conflict).unwrap();
        let before = schema(&store);
        drop(store);

        let output = run_initializers(dir.path());
        assert!(
            !output.status.success(),
            "{file}: conflicting names were accepted"
        );
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains("SchemaConflict") && text.contains(expected) && text.contains(file),
            "{file}: unexpected failure: {text}"
        );

        let store = open(dir.path(), file);
        assert_eq!(
            schema(&store),
            before,
            "{file}: refused open changed the schema"
        );
    }
}

/// An OHLCV store holding both pair-flag names in a shape no release writes is a cache
/// of unknown provenance: the service moves it to the quarantine directory unchanged,
/// rebuilds the store and keeps the manually monitored tokens.
#[test]
fn ohlcv_store_of_unknown_provenance_is_quarantined_and_rebuilt() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let data_version = current_ohlcv_data_version();
    let dir = new_data_dir();
    seed_legacy_stores(dir.path(), data_version);
    let store = open(dir.path(), "ohlcvs.db");
    store
        .execute_batch("ALTER TABLE ohlcv_pools ADD COLUMN is_native_pair INTEGER")
        .unwrap();
    let before = schema(&store);
    drop(store);

    boot(dir.path());

    let quarantined: Vec<_> = std::fs::read_dir(dir.path().join("data").join("quarantine"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(quarantined.len(), 1, "one quarantined store");
    let moved = &quarantined[0];
    assert!(moved
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("ohlcvs-"));
    assert!(moved.join("manifest.json").exists());
    let original = Connection::open(moved.join("ohlcvs.db")).unwrap();
    assert_eq!(
        schema(&original),
        before,
        "the quarantined store is unchanged"
    );
    drop(original);

    let rebuilt = open(dir.path(), "ohlcvs.db");
    assert_columns(
        &rebuilt,
        "ohlcv_pools",
        &["is_native_pair"],
        &["is_sol_pair"],
    );
    assert_integrity(&rebuilt);
    assert_eq!(
        rebuilt
            .query_row("SELECT COUNT(*) FROM ohlcv_pools", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0,
        "cached pools are not carried over"
    );
    assert_eq!(
        rebuilt
            .query_row(
                "SELECT mint, priority, source FROM ohlcv_monitor_config",
                [],
                |r| Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?
                )),
            )
            .unwrap(),
        ("MINT".to_owned(), "high".to_owned(), "manual".to_owned()),
        "the manual monitor is salvaged"
    );
}

/// An older build re-adds `is_sol_pair` with `DEFAULT 1` to a migrated store and writes
/// only that name. The next open keeps every flag either build wrote and drops the
/// legacy column; a further open writes nothing.
#[test]
fn ohlcv_pair_flags_from_both_builds_merge_on_open() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = new_data_dir();
    boot(dir.path());
    let store = open(dir.path(), "ohlcvs.db");
    store
        .execute_batch(
            "ALTER TABLE ohlcv_pools ADD COLUMN is_sol_pair INTEGER NOT NULL DEFAULT 1;
             INSERT INTO ohlcv_pools (chain_id, mint, pool_address, dex, is_native_pair) VALUES ('solana', 'M', 'canonical-non-native', 'dex', 0);
             INSERT INTO ohlcv_pools (chain_id, mint, pool_address, dex, is_sol_pair) VALUES ('solana', 'M', 'legacy-non-native', 'dex', 0);
             INSERT INTO ohlcv_pools (chain_id, mint, pool_address, dex) VALUES ('solana', 'M', 'native', 'dex');",
        )
        .unwrap();
    drop(store);

    boot(dir.path());

    let store = open(dir.path(), "ohlcvs.db");
    assert_columns(&store, "ohlcv_pools", &["is_native_pair"], &["is_sol_pair"]);
    let flags: Vec<(String, i64)> = store
        .prepare("SELECT pool_address, is_native_pair FROM ohlcv_pools ORDER BY pool_address")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        flags,
        vec![
            ("canonical-non-native".to_owned(), 0),
            ("legacy-non-native".to_owned(), 0),
            ("native".to_owned(), 1),
        ]
    );
    assert_integrity(&store);
    assert!(!dir.path().join("data").join("quarantine").exists());
    let merged = schema(&store);
    drop(store);

    boot(dir.path());
    assert_eq!(schema(&open(dir.path(), "ohlcvs.db")), merged);
}

#[test]
fn fresh_market_stores_open_with_unit_neutral_names() {
    if std::env::var_os(CHILD).is_some() {
        return;
    }
    let dir = new_data_dir();
    boot(dir.path());
    assert_canonical_names(dir.path());
    let first: Vec<_> = ["tokens.db", "pools.db", "ohlcvs.db"]
        .into_iter()
        .map(|file| schema(&open(dir.path(), file)))
        .collect();

    boot(dir.path());
    let second: Vec<_> = ["tokens.db", "pools.db", "ohlcvs.db"]
        .into_iter()
        .map(|file| schema(&open(dir.path(), file)))
        .collect();
    assert_eq!(second, first, "second open changed a schema");
}
