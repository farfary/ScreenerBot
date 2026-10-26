// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token detail answers the moment a token's age counts from: its on-chain or pool
//! creation, never its discovery; it answers only counted holders; and the wrapped native
//! coin carries only the figures that describe the coin.

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde_json::json;

use super::detail::{age_origin_timestamp, MarketFigures};
use crate::tokens::types::Token;

/// A token discovered at `discovered` whose provider reported `created` as its creation.
fn token(discovered: DateTime<Utc>, created: Option<DateTime<Utc>>) -> Token {
    serde_json::from_value(json!({
        "mint": "DetailCreationMint1111111111111111111111111",
        "symbol": "AGE",
        "name": "Age Token",
        "data_source": "DexScreener",
        "first_discovered_at": discovered,
        "blockchain_created_at": created,
        "metadata_last_fetched_at": discovered,
        "decimals_last_fetched_at": discovered,
        "market_data_last_fetched_at": discovered,
        "pool_price_last_calculated_at": discovered,
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

#[test]
fn the_age_counts_from_the_on_chain_creation_when_one_is_known() {
    let discovered = Utc::now() - Duration::minutes(5);
    let created = Utc.with_ymd_and_hms(2025, 7, 15, 12, 0, 0).unwrap();
    assert_eq!(
        age_origin_timestamp(&token(discovered, Some(created))),
        Some(created.timestamp())
    );
}

#[test]
fn the_age_counts_from_the_oldest_pool_without_an_on_chain_creation() {
    let discovered = Utc::now() - Duration::minutes(5);
    let pool = Utc.with_ymd_and_hms(2021, 3, 1, 0, 0, 0).unwrap();
    let mut token = token(discovered, None);
    token.pool_created_at = Some(pool);
    assert_eq!(age_origin_timestamp(&token), Some(pool.timestamp()));
}

#[test]
fn discovery_alone_is_no_age() {
    let discovered = Utc::now() - Duration::minutes(5);
    for created in [None, Some(DateTime::UNIX_EPOCH)] {
        assert_eq!(
            age_origin_timestamp(&token(discovered, created)),
            None,
            "{created:?}"
        );
    }
}

#[test]
fn an_uncounted_holder_total_is_unknown_and_a_count_stays_a_count() {
    let now = Utc::now();
    let mut token = token(now, None);
    for (reported, shown) in [(None, None), (Some(0), None), (Some(12_345), Some(12_345))] {
        token.total_holders = reported;
        let figures = MarketFigures::for_analysis(false, Some(187.25), &token, 0.5, 93.6);
        assert_eq!(figures.total_holders, shown, "{reported:?}");
    }
}

#[test]
fn the_wrapped_native_coin_is_priced_at_the_shared_native_figure() {
    let figures = MarketFigures::of_native_asset(Some(187.25));
    assert_eq!(figures.price_sol, Some(1.0));
    assert_eq!(figures.price_usd, Some(187.25));
    assert_eq!(MarketFigures::of_native_asset(None).price_usd, None);
}

#[test]
fn the_wrapped_native_coin_shows_no_wrapper_supply_holders_or_pooled_reserves() {
    let figures = MarketFigures::of_native_asset(Some(187.25));
    assert_eq!(figures.market_cap, None);
    assert_eq!(figures.fdv, None);
    assert_eq!(figures.liquidity_usd, None);
    assert_eq!(figures.total_holders, None);
}

#[test]
fn the_wrapped_native_mint_is_the_native_asset() {
    let adapter = crate::chains::adapter_for(crate::chains::ChainId::Solana);
    assert!(adapter.is_native_asset(crate::chains::solana::constants::SOL_MINT));
}

#[test]
fn the_analysis_reports_the_native_coin_figures_for_the_wrapped_native_coin() {
    let now = Utc::now();
    let mut wrapped = token(now, None);
    wrapped.market_cap = Some(90_000_000_000.0);
    wrapped.fdv = Some(95_000_000_000.0);
    wrapped.liquidity_usd = Some(4_000_000_000.0);
    wrapped.total_holders = Some(12);

    assert_eq!(
        MarketFigures::for_analysis(true, Some(187.25), &wrapped, 0.98, 183.5),
        MarketFigures::of_native_asset(Some(187.25))
    );

    let quoted = MarketFigures::for_analysis(false, Some(187.25), &wrapped, 0.5, 93.6);
    assert_eq!(quoted.price_sol, Some(0.5));
    assert_eq!(quoted.price_usd, Some(93.6));
    assert_eq!(quoted.market_cap, wrapped.market_cap);
    assert_eq!(quoted.fdv, wrapped.fdv);
    assert_eq!(quoted.total_holders, wrapped.total_holders);

    let unpriced = MarketFigures::for_analysis(false, Some(187.25), &wrapped, 0.0, 0.0);
    assert_eq!((unpriced.price_sol, unpriced.price_usd), (None, None));
}
