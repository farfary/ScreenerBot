// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the All Tokens page read: a page reads its mints in order and then only their rows, and it must list exactly the rows, in exactly the order, of the whole sorted listing.

use rusqlite::params;

use super::TokenDatabase;
use crate::chains::ChainId;

fn store() -> (tempfile::TempDir, TokenDatabase) {
    // Row assembly reads the preferred market data source from the config.
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("tokens store");
    (directory, db)
}

/// Tokens with ties, gaps and a GeckoTerminal-only value: `(mint, symbol, dex liquidity,
/// gecko liquidity, rugcheck score)`. `None` for a liquidity leaves that market row out.
const ROWS: [(&str, &str, Option<f64>, Option<f64>, Option<i32>); 8] = [
    ("MintA", "ALPHA", Some(500.0), None, Some(10)),
    ("MintB", "BRAVO", Some(900.0), None, None),
    ("MintC", "CHARLIE", Some(500.0), None, Some(4_000)),
    ("MintD", "DELTA", None, Some(700.0), Some(10)),
    ("MintE", "ECHO", None, None, None),
    ("MintF", "FOXTROT", Some(50.0), Some(9_999.0), Some(1)),
    ("MintG", "GOLF", None, None, Some(250)),
    ("MintH", "HOTEL", Some(900.0), None, None),
];

fn seed(db: &TokenDatabase) {
    let conn = db.conn().expect("connection");
    for (mint, symbol, dex, gecko, score) in ROWS {
        conn.execute(
            "INSERT INTO tokens (chain_id, mint, symbol, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES ('solana', ?1, ?2, 1, 1, 1)",
            params![mint, symbol],
        )
        .expect("insert token");
        if let Some(liquidity) = dex {
            conn.execute(
                "INSERT INTO market_dexscreener (chain_id, mint, liquidity_usd, market_data_last_fetched_at, market_data_first_fetched_at)
                 VALUES ('solana', ?1, ?2, 1, 1)",
                params![mint, liquidity],
            )
            .expect("insert dex row");
        }
        if let Some(liquidity) = gecko {
            conn.execute(
                "INSERT INTO market_geckoterminal (chain_id, mint, liquidity_usd, market_data_last_fetched_at, market_data_first_fetched_at)
                 VALUES ('solana', ?1, ?2, 1, 1)",
                params![mint, liquidity],
            )
            .expect("insert gecko row");
        }
        if let Some(score) = score {
            conn.execute(
                "INSERT INTO security_rugcheck (chain_id, mint, score, security_data_last_fetched_at, security_data_first_fetched_at)
                 VALUES ('solana', ?1, ?2, 1, 1)",
                params![mint, score],
            )
            .expect("insert security row");
        }
    }
}

fn mints(db: &TokenDatabase, limit: usize, offset: usize, sort: &str, dir: &str) -> Vec<String> {
    db.get_all_tokens_optional_market(limit, offset, Some(sort), Some(dir), false)
        .expect("token page")
        .into_iter()
        .map(|token| token.mint)
        .collect()
}

#[test]
fn pages_list_the_whole_sorted_listing_in_order() {
    let (_directory, db) = store();
    seed(&db);

    for sort in ["liquidity_usd", "risk_score", "symbol", "mint", "updated"] {
        for dir in ["asc", "desc"] {
            let whole = mints(&db, 0, 0, sort, dir);
            assert_eq!(whole.len(), ROWS.len(), "{sort} {dir}");
            let paged: Vec<String> = (0..ROWS.len())
                .step_by(3)
                .flat_map(|offset| mints(&db, 3, offset, sort, dir))
                .collect();
            assert_eq!(paged, whole, "{sort} {dir}");
        }
    }
    assert!(mints(&db, 3, ROWS.len(), "liquidity_usd", "desc").is_empty());
}

#[test]
fn a_liquidity_page_orders_by_value_then_mint_with_unpriced_tokens_last() {
    let (_directory, db) = store();
    seed(&db);

    // DexScreener first, GeckoTerminal where DexScreener has no row; ties by mint.
    assert_eq!(
        mints(&db, 5, 0, "liquidity_usd", "desc"),
        ["MintB", "MintH", "MintD", "MintA", "MintC"]
    );
    assert_eq!(
        mints(&db, 5, 5, "liquidity_usd", "desc"),
        ["MintF", "MintE", "MintG"]
    );
}

#[test]
fn every_paged_row_carries_its_own_data() {
    let (_directory, db) = store();
    seed(&db);

    let page = db
        .get_all_tokens_optional_market(4, 2, Some("risk_score"), Some("desc"), false)
        .expect("token page");
    assert_eq!(page.len(), 4);
    for token in page {
        let (_, symbol, _, _, score) = ROWS
            .iter()
            .find(|row| row.0 == token.mint)
            .expect("seeded mint");
        assert_eq!(token.symbol, *symbol, "{}", token.mint);
        assert_eq!(token.security_score, *score, "{}", token.mint);
    }
}
