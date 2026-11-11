// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! A token-list row carries the live state its columns and row actions read: whether the
//! pool service prices the token, whether a position in it is open and whether candles are
//! stored, under the names the token detail uses, and the risk band that tones its score.
//! Rows that carried only the stored token left the Status column empty and offered Buy for
//! a token already held.

use std::collections::HashMap;

use chrono::Utc;
use serde_json::{json, Value};

use super::types::{build_token_list_response, TokenListItem, TokenRowState};
use crate::filtering::{FilteringQueryResult, FilteringView};
use crate::tokens::security::SecurityRiskBand;
use crate::tokens::types::Token;

fn token(mint: &str) -> Token {
    let now = Utc::now();
    serde_json::from_value(json!({
        "mint": mint,
        "symbol": "ROW",
        "name": "Row Token",
        "data_source": "DexScreener",
        "first_discovered_at": now,
        "metadata_last_fetched_at": now,
        "decimals_last_fetched_at": now,
        "market_data_last_fetched_at": now,
        "pool_price_last_calculated_at": now,
        "price_usd": 0.0,
        "price_sol": 0.0,
        "price_native": "0",
        "websites": [],
        "socials": [],
        "is_rugged": false,
        "security_risks": [],
        "top_holders": [],
        "is_blacklisted": false,
        "priority": "PoolTracked",
    }))
    .expect("token fixture")
}

/// The listed rows of a page holding `priced`, `held` and `bare`, as JSON.
fn listed_rows() -> Vec<Value> {
    let mints = ["PricedMint", "HeldMint", "BareMint"];
    let result = FilteringQueryResult {
        items: mints.iter().map(|mint| token(mint)).collect(),
        page: 1,
        page_size: 50,
        total: mints.len(),
        total_pages: 1,
        timestamp: Utc::now(),
        priced_total: 2,
        positions_total: 1,
        blacklisted_total: 0,
        priced_mints: vec!["PricedMint".to_owned(), "HeldMint".to_owned()],
        open_position_mints: vec!["HeldMint".to_owned()],
        ohlcv_mints: vec!["PricedMint".to_owned()],
        rejection_reasons: HashMap::new(),
        available_rejection_reasons: Vec::new(),
        blacklist_reasons: HashMap::new(),
        market_checks: HashMap::new(),
    };
    let response = build_token_list_response(result, FilteringView::All);
    let body = serde_json::to_value(&response).expect("list response serializes");
    body["items"].as_array().expect("items array").clone()
}

fn state_of(row: &Value) -> (bool, bool, bool) {
    let flag = |name: &str| {
        row[name]
            .as_bool()
            .unwrap_or_else(|| panic!("{name} is sent"))
    };
    (
        flag("has_pool_price"),
        flag("has_open_position"),
        flag("has_ohlcv"),
    )
}

#[test]
fn each_listed_row_carries_the_live_state_of_its_own_token() {
    let rows = listed_rows();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["mint"], "PricedMint");
    assert_eq!(state_of(&rows[0]), (true, false, true));
    assert_eq!(rows[1]["mint"], "HeldMint");
    assert_eq!(state_of(&rows[1]), (true, true, false));
    assert_eq!(rows[2]["mint"], "BareMint");
    assert_eq!(state_of(&rows[2]), (false, false, false));
}

#[test]
fn a_listed_row_keeps_the_stored_token_fields_the_columns_read() {
    let rows = listed_rows();
    for field in ["is_blacklisted", "image_url", "security_score"] {
        assert!(
            rows[0]
                .as_object()
                .is_some_and(|row| row.contains_key(field)),
            "{field} is sent"
        );
    }
}

#[test]
fn a_listed_row_carries_the_risk_band_of_its_normalised_score() {
    let mut scored = token("ScoredMint");
    scored.security_score_normalised = Some(SecurityRiskBand::MEDIUM_MAX + 1);
    let row = serde_json::to_value(TokenListItem::new(scored, TokenRowState::default()))
        .expect("row serializes");
    assert_eq!(row["security_band"], "high");

    let unscored = TokenListItem::new(token("BareMint"), TokenRowState::default());
    let row = serde_json::to_value(unscored).expect("row serializes");
    assert!(
        row["security_band"].is_null(),
        "an unreported token has no band"
    );
}
