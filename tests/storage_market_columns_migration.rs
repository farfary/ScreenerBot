// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Unit-neutral token, pool and OHLCV column names through the production initializers.

use rusqlite::Connection;
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
    let tokens = open(dir, "tokens.db");
    tokens
        .execute_batch(include_str!("fixtures/v0.2.13-tokens.sql"))
        .unwrap();
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
    let pools = open(dir, "pools.db");
    pools
        .execute_batch(include_str!("fixtures/v0.2.13-pools.sql"))
        .unwrap();
    pools
        .execute(
            "INSERT INTO price_history (id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, created_at) VALUES (31, 'solana', 'MINT', 'POOL', 2.0, 0.01, 1.0, 7, 60, 3.25, 400.0, '2026-01-01T00:01:00Z')",
            [],
        )
        .unwrap();
    pools.pragma_update(None, "user_version", 1).unwrap();
}

fn seed_legacy_ohlcv(dir: &Path, data_version: i64) {
    let ohlcv = open(dir, "ohlcvs.db");
    ohlcv
        .execute_batch(include_str!("fixtures/v0.2.13-ohlcv.sql"))
        .unwrap();
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
            "both token_pools.is_sol_pair and token_pools.is_native_pair exist",
        ),
        (
            "pools.db",
            "ALTER TABLE price_history ADD COLUMN native_reserves REAL",
            "both price_history.sol_reserves and price_history.native_reserves exist",
        ),
        (
            "ohlcvs.db",
            "ALTER TABLE ohlcv_pools ADD COLUMN is_native_pair INTEGER",
            "both ohlcv_pools.is_sol_pair and ohlcv_pools.is_native_pair exist",
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
            text.contains(expected),
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
