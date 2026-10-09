// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The token-details source list names only the dialog-wide sources.

use super::source_status::build_source_status;

#[tokio::test]
async fn source_status_lists_market_and_security_sources_only() {
    let sources: Vec<String> = build_source_status(true, true, true)
        .await
        .into_iter()
        .map(|status| status.source)
        .collect();
    assert_eq!(sources, ["dexscreener", "geckoterminal", "rugcheck"]);
}
