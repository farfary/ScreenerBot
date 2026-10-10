// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for token eviction: which tokens a sweep removes and keeps, that every row a removed token owns goes with it, the bounded transactions and the index-only walk.

use std::cell::Cell;
use std::collections::HashSet;

use rusqlite::{params, Connection};

use super::retention::{
    discovery_counts, evict_tokens_unused_before, CANDIDATES_SQL, MAX_ROWS_PER_TRANSACTION,
};
use crate::chains::ChainId;

const CUTOFF: i64 = 1_000;
const STALE: i64 = 100;
const FRESH: i64 = 5_000;
/// Rows `insert_full_token` writes for one token.
const ROWS_PER_FULL_TOKEN: u64 = 8;

fn store() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().expect("temp dir");
    let conn = Connection::open(directory.path().join("tokens.db")).expect("open tokens store");
    crate::tokens::schema::initialize_schema(&conn).expect("tokens schema");
    (directory, conn)
}

fn insert_token(conn: &Connection, chain: &str, mint: &str, discovered: i64, fetched: i64) {
    conn.execute(
        "INSERT INTO tokens (chain_id, mint, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![chain, mint, discovered, fetched],
    )
    .expect("insert token");
}

/// A token with a row in every table that holds token data, all last touched at
/// `fetched`.
fn insert_full_token(conn: &Connection, mint: &str, discovered: i64, fetched: i64) {
    insert_token(conn, "solana", mint, discovered, fetched);
    for table in ["market_dexscreener", "market_geckoterminal"] {
        conn.execute(
            &format!(
                "INSERT INTO {table} (chain_id, mint, market_data_last_fetched_at, market_data_first_fetched_at)
                 VALUES ('solana', ?1, ?2, ?2)"
            ),
            params![mint, fetched],
        )
        .expect("insert market row");
    }
    conn.execute(
        "INSERT INTO security_rugcheck (chain_id, mint, security_data_last_fetched_at, security_data_first_fetched_at)
         VALUES ('solana', ?1, ?2, ?2)",
        params![mint, fetched],
    )
    .expect("insert security row");
    conn.execute(
        "INSERT INTO update_tracking (chain_id, mint, market_data_last_updated_at)
         VALUES ('solana', ?1, ?2)",
        params![mint, fetched],
    )
    .expect("insert tracking row");
    conn.execute(
        "INSERT INTO token_media (chain_id, mint, media_last_fetched_at, media_next_fetch_at)
         VALUES ('solana', ?1, ?2, ?2)",
        params![mint, fetched],
    )
    .expect("insert media row");
    conn.execute(
        "INSERT INTO token_pools (chain_id, mint, pool_address, base_mint, quote_mint, is_native_pair,
                                  pool_data_last_fetched_at, pool_data_first_seen_at)
         VALUES ('solana', ?1, ?2, ?1, 'native', 1, ?3, ?3)",
        params![mint, format!("pool-{mint}"), fetched],
    )
    .expect("insert pool row");
    conn.execute(
        "INSERT INTO rejection_history (chain_id, mint, reason, source, rejected_at)
         VALUES ('solana', ?1, 'low_liquidity', 'filtering', ?2)",
        params![mint, fetched],
    )
    .expect("insert rejection row");
}

fn mints(conn: &Connection, chain: &str) -> HashSet<String> {
    let mut statement = conn
        .prepare("SELECT mint FROM tokens WHERE chain_id = ?1")
        .expect("prepare mints");
    statement
        .query_map(params![chain], |row| row.get(0))
        .expect("list mints")
        .collect::<rusqlite::Result<_>>()
        .expect("read mints")
}

fn rows_of(conn: &Connection, table: &str, mint: &str) -> i64 {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE mint = ?1"),
        params![mint],
        |row| row.get(0),
    )
    .expect("count rows")
}

fn never_stop() -> bool {
    false
}

#[test]
fn a_sweep_removes_only_unused_tokens_and_everything_they_own() {
    let (_directory, mut conn) = store();
    insert_full_token(&conn, "stale", STALE, STALE);
    insert_full_token(&conn, "refreshed", STALE, FRESH);
    insert_full_token(&conn, "recent", FRESH, FRESH);
    insert_full_token(&conn, "favorite", STALE, STALE);
    insert_full_token(&conn, "blocked", STALE, STALE);
    insert_full_token(&conn, "filtered", STALE, STALE);
    insert_full_token(&conn, "in-use", STALE, STALE);
    insert_token(&conn, "solana", "bare", STALE, STALE);
    insert_token(&conn, "other-chain", "elsewhere", STALE, STALE);
    conn.execute(
        "INSERT INTO token_favorites (chain_id, mint) VALUES ('solana', 'favorite')",
        [],
    )
    .expect("insert favorite");
    conn.execute_batch(
        "INSERT INTO blacklist (chain_id, mint, reason, source, added_at)
             VALUES ('solana', 'blocked', 'scam', 'manual', 1);
         INSERT INTO blacklist (chain_id, mint, reason, source, added_at)
             VALUES ('solana', 'filtered', 'rug', 'filtering', 1);",
    )
    .expect("insert blacklist");
    let protected: HashSet<String> = ["in-use".to_owned()].into();

    let eviction =
        evict_tokens_unused_before(&mut conn, ChainId::Solana, CUTOFF, &protected, &never_stop)
            .expect("evict");

    assert_eq!(
        mints(&conn, "solana"),
        ["refreshed", "recent", "favorite", "blocked", "in-use"]
            .map(str::to_owned)
            .into()
    );
    assert_eq!(mints(&conn, "other-chain"), ["elsewhere".to_owned()].into());
    assert_eq!(eviction.tokens, 3);
    assert_eq!(eviction.rows, 2 * ROWS_PER_FULL_TOKEN + 1);
    assert!(!eviction.interrupted);
    for mint in ["stale", "filtered"] {
        for table in [
            "market_dexscreener",
            "market_geckoterminal",
            "security_rugcheck",
            "update_tracking",
            "token_media",
            "token_pools",
            "rejection_history",
        ] {
            assert_eq!(rows_of(&conn, table, mint), 0, "{table} keeps {mint}");
        }
    }
    assert_eq!(
        rows_of(&conn, "blacklist", "filtered"),
        1,
        "a blacklist decision outlives the token's data"
    );
    let violations: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("foreign key check");
    assert_eq!(violations, 0);
}

#[test]
fn each_transaction_deletes_a_bounded_number_of_rows_and_a_resumed_sweep_finishes() {
    let (_directory, mut conn) = store();
    let tokens = 600u64;
    for index in 0..tokens {
        // Many tokens share a discovery time, so the keyset walk must page through ties.
        insert_full_token(
            &conn,
            &format!("mint-{index:04}"),
            STALE + (index % 3) as i64,
            STALE,
        );
    }
    let checks = Cell::new(0);
    let stop_after_one_transaction = || {
        checks.set(checks.get() + 1);
        checks.get() > 1
    };
    let first = evict_tokens_unused_before(
        &mut conn,
        ChainId::Solana,
        CUTOFF,
        &HashSet::new(),
        &stop_after_one_transaction,
    )
    .expect("first transaction");
    assert!(first.interrupted);
    assert!(first.rows >= MAX_ROWS_PER_TRANSACTION);
    assert!(first.rows < MAX_ROWS_PER_TRANSACTION + ROWS_PER_FULL_TOKEN);
    assert_eq!(first.rows, first.tokens * ROWS_PER_FULL_TOKEN);

    let rest = evict_tokens_unused_before(
        &mut conn,
        ChainId::Solana,
        CUTOFF,
        &HashSet::new(),
        &never_stop,
    )
    .expect("rest");
    assert!(!rest.interrupted);
    assert_eq!(first.tokens + rest.tokens, tokens);
    assert!(mints(&conn, "solana").is_empty());
}

#[test]
fn discovery_counts_count_tokens_before_each_cutoff() {
    let (_directory, conn) = store();
    for (mint, discovered) in [("a", 10), ("b", 20), ("c", 30), ("d", 40)] {
        insert_token(&conn, "solana", mint, discovered, discovered);
    }
    insert_token(&conn, "other-chain", "e", 10, 10);
    let (total, before) = discovery_counts(&conn, ChainId::Solana, &[15, 30, 100]).expect("counts");
    assert_eq!(total, 4);
    assert_eq!(before, [1, 2, 4]);
}

#[test]
fn the_candidate_walk_reads_indexes_only_and_never_sorts() {
    let (_directory, conn) = store();
    let mut statement = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {CANDIDATES_SQL}"))
        .expect("explain candidates");
    let details: Vec<String> = statement
        .query_map(params!["solana", CUTOFF, 0, 500], |row| row.get(3))
        .expect("query plan")
        .collect::<rusqlite::Result<_>>()
        .expect("read plan");
    assert!(!details.is_empty());
    for detail in &details {
        assert!(!detail.starts_with("SCAN"), "full scan in {details:?}");
        assert!(!detail.contains("TEMP B-TREE"), "sort in {details:?}");
    }
}
