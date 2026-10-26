// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Per-source availability status for the token-details dialog.

use super::types::SourceStatus;
use crate::chains::ChainId;
use crate::i18n::{ids, UiArg, UiText};
use crate::tokens::DataSource;

/// What the dialog knows about one market source for a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MarketKnowledge {
    /// Market data from the source is stored.
    Listed,
    /// The source answered and lists no pair for the token.
    AnsweredNotListed,
    /// No data and no answer yet.
    Unanswered,
}

impl MarketKnowledge {
    /// What is known about `source` for `mint`, given whether its data is stored.
    pub(super) fn of(chain: ChainId, source: DataSource, mint: &str, has_data: bool) -> Self {
        if has_data {
            Self::Listed
        } else if crate::tokens::answered_not_listed(chain, source, mint) {
            Self::AnsweredNotListed
        } else {
            Self::Unanswered
        }
    }
}

/// The dialog status of one market source. A source is "not listed" only when it
/// answered so; without data or an answer it is unavailable while its endpoint is
/// unhealthy and pending otherwise.
pub(super) fn market_status(
    endpoint: &str,
    label: &str,
    knowledge: MarketKnowledge,
    endpoint_unhealthy: bool,
) -> SourceStatus {
    let label_arg = || UiArg::Text(label.to_owned());
    let (state, text) = match knowledge {
        MarketKnowledge::Listed => ("ok", UiText::new(ids::TOKENS_RESULT_SOURCE_LIVE)),
        MarketKnowledge::AnsweredNotListed => (
            "no_data",
            UiText::new(ids::TOKENS_RESULT_SOURCE_NOT_LISTED).arg("label", label_arg()),
        ),
        MarketKnowledge::Unanswered if endpoint_unhealthy => (
            "unavailable",
            UiText::new(ids::TOKENS_RESULT_SOURCE_UNAVAILABLE).arg("label", label_arg()),
        ),
        MarketKnowledge::Unanswered => (
            "pending",
            UiText::new(ids::TOKENS_RESULT_SOURCE_PENDING).arg("label", label_arg()),
        ),
    };
    SourceStatus {
        source: endpoint.to_owned(),
        label: label.to_owned(),
        state: state.to_owned(),
        text,
    }
}

async fn is_endpoint_unhealthy(endpoint: &str) -> bool {
    crate::connectivity::get_endpoint_health(endpoint)
        .await
        .map(|h| h.is_unhealthy())
        .unwrap_or(false)
}

/// Build the per-source status list for the token-details dialog: each market source
/// with what is known about it ([`market_status`]), then the security report, which is
/// unavailable while its endpoint is unhealthy and missing otherwise.
pub(super) async fn build_source_status(
    dexscreener: MarketKnowledge,
    geckoterminal: MarketKnowledge,
    has_rugcheck: bool,
) -> Vec<SourceStatus> {
    let (dex_unhealthy, gecko_unhealthy) = tokio::join!(
        is_endpoint_unhealthy("dexscreener"),
        is_endpoint_unhealthy("geckoterminal"),
    );
    let dex = market_status("dexscreener", "DexScreener", dexscreener, dex_unhealthy);
    let gecko = market_status(
        "geckoterminal",
        "GeckoTerminal",
        geckoterminal,
        gecko_unhealthy,
    );

    let rug = {
        let (state, text) = if has_rugcheck {
            ("ok", UiText::new(ids::TOKENS_RESULT_SECURITY_AVAILABLE))
        } else if is_endpoint_unhealthy("rugcheck").await {
            (
                "unavailable",
                UiText::new(ids::TOKENS_RESULT_SOURCE_UNAVAILABLE)
                    .arg("label", UiArg::Text("Rugcheck".to_owned())),
            )
        } else {
            ("no_data", UiText::new(ids::TOKENS_RESULT_SECURITY_MISSING))
        };
        SourceStatus {
            source: "rugcheck".to_owned(),
            label: "Rugcheck".to_owned(),
            state: state.to_owned(),
            text,
        }
    };

    vec![dex, gecko, rug]
}
