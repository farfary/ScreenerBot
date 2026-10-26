// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token-details source list names only the dialog-wide sources, and calls a market
//! source "not listed" only when the source answered so.

use super::source_status::{build_source_status, market_status, MarketKnowledge};

#[tokio::test]
async fn source_status_lists_market_and_security_sources_only() {
    let sources: Vec<String> =
        build_source_status(MarketKnowledge::Listed, MarketKnowledge::Listed, true)
            .await
            .into_iter()
            .map(|status| status.source)
            .collect();
    assert_eq!(sources, ["dexscreener", "geckoterminal", "rugcheck"]);
}

#[test]
fn only_an_answer_makes_a_source_not_listed() {
    let state = |knowledge, unhealthy| {
        market_status("dexscreener", "DexScreener", knowledge, unhealthy).state
    };
    for unhealthy in [false, true] {
        assert_eq!(state(MarketKnowledge::Listed, unhealthy), "ok");
        assert_eq!(
            state(MarketKnowledge::AnsweredNotListed, unhealthy),
            "no_data"
        );
    }
    assert_eq!(state(MarketKnowledge::Unanswered, false), "pending");
    assert_eq!(state(MarketKnowledge::Unanswered, true), "unavailable");
}
