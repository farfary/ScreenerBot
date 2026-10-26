// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token detail answers a token's creation time: on-chain creation when known, else discovery.

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde_json::json;

use super::detail::creation_timestamp;
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
fn creation_time_is_the_on_chain_creation_when_one_is_known() {
    let discovered = Utc::now() - Duration::minutes(5);
    let created = Utc.with_ymd_and_hms(2025, 7, 15, 12, 0, 0).unwrap();
    assert_eq!(
        creation_timestamp(&token(discovered, Some(created))),
        created.timestamp()
    );
}

#[test]
fn creation_time_falls_back_to_discovery_without_a_valid_creation() {
    let discovered = Utc::now() - Duration::minutes(5);
    for created in [None, Some(DateTime::UNIX_EPOCH)] {
        assert_eq!(
            creation_timestamp(&token(discovered, created)),
            discovered.timestamp(),
            "{created:?}"
        );
    }
}
