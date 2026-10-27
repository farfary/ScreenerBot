// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Tests for a token's creation moment and holder count: the earliest valid on-chain or
//! pool time wins, discovery is the fallback but never a displayed age, and an uncounted
//! holder total is unknown.

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde_json::json;

use super::types::{CreationSource, Token};

/// A token discovered at `discovered` with the given on-chain and pool creation times.
fn token(
    discovered: DateTime<Utc>,
    on_chain: Option<DateTime<Utc>>,
    pool: Option<DateTime<Utc>>,
) -> Token {
    serde_json::from_value(json!({
        "mint": "CreationMint11111111111111111111111111111111",
        "symbol": "AGE",
        "name": "Age Token",
        "data_source": "DexScreener",
        "first_discovered_at": discovered,
        "blockchain_created_at": on_chain,
        "pool_created_at": pool,
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

fn at(year: i32, month: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, 1, 0, 0, 0).unwrap()
}

#[test]
fn each_source_answers_alone() {
    let discovered = Utc::now() - Duration::seconds(16);

    let on_chain = token(discovered, Some(at(2023, 5)), None).created_at();
    assert_eq!(
        (on_chain.at, on_chain.source),
        (at(2023, 5), CreationSource::OnChain)
    );

    let pool = token(discovered, None, Some(at(2021, 3))).created_at();
    assert_eq!((pool.at, pool.source), (at(2021, 3), CreationSource::Pool));

    let discovery = token(discovered, None, None).created_at();
    assert_eq!(
        (discovery.at, discovery.source),
        (discovered, CreationSource::Discovery)
    );
}

#[test]
fn the_earliest_known_moment_wins() {
    let discovered = Utc::now();

    let older_pool = token(discovered, Some(at(2024, 1)), Some(at(2022, 6))).created_at();
    assert_eq!(
        (older_pool.at, older_pool.source),
        (at(2022, 6), CreationSource::Pool)
    );

    let older_mint = token(discovered, Some(at(2022, 6)), Some(at(2024, 1))).created_at();
    assert_eq!(
        (older_mint.at, older_mint.source),
        (at(2022, 6), CreationSource::OnChain)
    );

    let tie = token(discovered, Some(at(2022, 6)), Some(at(2022, 6))).created_at();
    assert_eq!(tie.source, CreationSource::OnChain);
}

#[test]
fn invalid_timestamps_are_ignored() {
    let discovered = Utc::now();
    let epoch = DateTime::UNIX_EPOCH;
    let before_epoch = epoch - Duration::days(1);

    for invalid in [epoch, before_epoch] {
        let pool_only = token(discovered, Some(invalid), Some(at(2021, 3))).created_at();
        assert_eq!(pool_only.source, CreationSource::Pool, "{invalid}");

        let on_chain_only = token(discovered, Some(at(2023, 5)), Some(invalid)).created_at();
        assert_eq!(on_chain_only.source, CreationSource::OnChain, "{invalid}");

        let neither = token(discovered, Some(invalid), Some(invalid)).created_at();
        assert_eq!(
            (neither.at, neither.source),
            (discovered, CreationSource::Discovery),
            "{invalid}"
        );
    }
}

#[test]
fn the_on_chain_creation_time_is_valid_or_absent() {
    let discovered = Utc::now();
    assert_eq!(
        token(discovered, Some(at(2023, 5)), Some(at(2021, 3))).on_chain_created_at(),
        Some(at(2023, 5))
    );
    assert_eq!(
        token(discovered, None, Some(at(2021, 3))).on_chain_created_at(),
        None
    );
    let epoch = DateTime::UNIX_EPOCH;
    for invalid in [epoch, epoch - Duration::days(1)] {
        assert_eq!(
            token(discovered, Some(invalid), Some(at(2021, 3))).on_chain_created_at(),
            None,
            "{invalid}"
        );
    }
}

#[test]
fn discovery_is_never_a_displayed_age() {
    let discovered = Utc::now() - Duration::seconds(16);
    assert_eq!(
        token(discovered, None, None).created_at().age_origin(),
        None
    );
    assert_eq!(
        token(discovered, None, Some(at(2021, 3)))
            .created_at()
            .age_origin(),
        Some(at(2021, 3))
    );
    assert_eq!(
        token(discovered, Some(at(2023, 5)), None)
            .created_at()
            .age_origin(),
        Some(at(2023, 5))
    );
}

#[test]
fn an_uncounted_holder_total_is_unknown() {
    let mut token = token(Utc::now(), None, None);
    for (reported, known) in [
        (None, None),
        (Some(0), None),
        (Some(-3), None),
        (Some(1), Some(1)),
        (Some(1_234_567), Some(1_234_567)),
    ] {
        token.total_holders = reported;
        assert_eq!(token.holder_count(), known, "{reported:?}");
    }
}
