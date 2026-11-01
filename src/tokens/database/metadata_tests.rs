// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the decimals preload: the capped selection keeps the tokens with a pool first and then the most recently refreshed, skips invalid decimals and other chains, and returns the most important mint last.

use rusqlite::params;

use super::TokenDatabase;
use crate::chains::ChainId;

fn store() -> (tempfile::TempDir, TokenDatabase) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("tokens store");
    (directory, db)
}

fn insert_token(
    db: &TokenDatabase,
    chain: &str,
    mint: &str,
    decimals: Option<i64>,
    refreshed: i64,
) {
    db.conn()
        .expect("connection")
        .execute(
            "INSERT INTO tokens (chain_id, mint, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES (?1, ?2, ?3, 1, ?4, 1)",
            params![chain, mint, decimals, refreshed],
        )
        .expect("insert token");
}

fn insert_pool(db: &TokenDatabase, chain: &str, mint: &str, pool: &str) {
    db.conn()
        .expect("connection")
        .execute(
            "INSERT INTO token_pools (chain_id, mint, pool_address, base_mint, quote_mint, is_native_pair,
                                      pool_data_last_fetched_at, pool_data_first_seen_at)
             VALUES (?1, ?2, ?3, ?2, 'native', 1, 1, 1)",
            params![chain, mint, pool],
        )
        .expect("insert pool");
}

/// Five valid tokens, two of them with pools (one with two pools), plus rows the preload
/// must never return: zero, missing and out-of-range decimals, a pooled token with zero
/// decimals, and a pooled token of another chain refreshed after all of them.
fn seeded_store() -> (tempfile::TempDir, TokenDatabase) {
    let (directory, db) = store();
    insert_token(&db, "solana", "pooled-old", Some(6), 10);
    insert_token(&db, "solana", "pooled-new", Some(9), 30);
    insert_token(&db, "solana", "plain-old", Some(6), 5);
    insert_token(&db, "solana", "plain-mid", Some(6), 20);
    insert_token(&db, "solana", "plain-new", Some(8), 40);
    insert_token(&db, "solana", "zero", Some(0), 50);
    insert_token(&db, "solana", "missing", None, 60);
    insert_token(&db, "solana", "junk", Some(255), 70);
    insert_token(&db, "solana", "pooled-zero", Some(0), 80);
    insert_token(&db, "foreign", "pooled-foreign", Some(9), 90);
    insert_pool(&db, "solana", "pooled-old", "pool-a");
    insert_pool(&db, "solana", "pooled-new", "pool-b");
    insert_pool(&db, "solana", "pooled-new", "pool-c");
    insert_pool(&db, "solana", "pooled-zero", "pool-d");
    insert_pool(&db, "foreign", "pooled-foreign", "pool-e");
    (directory, db)
}

fn preloaded_mints(db: &TokenDatabase, limit: usize) -> Vec<String> {
    db.get_tokens_with_decimals_for_preload(limit)
        .expect("preload")
        .into_iter()
        .map(|(mint, _)| mint)
        .collect()
}

#[test]
fn an_uncapped_preload_returns_every_valid_token_with_the_pooled_ones_last() {
    let (_directory, db) = seeded_store();

    assert_eq!(
        preloaded_mints(&db, 100),
        [
            "plain-old",
            "plain-mid",
            "plain-new",
            "pooled-old",
            "pooled-new"
        ]
    );
}

#[test]
fn the_cap_keeps_the_pooled_tokens_and_then_the_most_recently_refreshed() {
    let (_directory, db) = seeded_store();

    assert_eq!(
        preloaded_mints(&db, 4),
        ["plain-mid", "plain-new", "pooled-old", "pooled-new"]
    );
}

#[test]
fn a_cap_below_the_pooled_count_keeps_only_the_newest_pooled_tokens() {
    let (_directory, db) = seeded_store();

    assert_eq!(preloaded_mints(&db, 1), ["pooled-new"]);
}

#[test]
fn each_preloaded_mint_carries_its_stored_decimals() {
    let (_directory, db) = seeded_store();

    let decimals = db
        .get_tokens_with_decimals_for_preload(100)
        .expect("preload");

    assert!(decimals.contains(&("pooled-new".to_owned(), 9)));
    assert!(decimals.contains(&("plain-new".to_owned(), 8)));
}
