//! Per-source availability status for the token-details dialog.

use super::types::SourceStatus;
use crate::i18n::{ids, UiArg, UiText};

/// Build the per-source status list for the token-details dialog.
///
/// `has_*` reflect whether we hold data for each source. When we don't, we
/// distinguish "the source genuinely doesn't list this token" (`no_data`) from
/// "the source is currently unreachable/rate-limited" (`unavailable`) using the
/// connectivity health monitor, so the UI can say "retrying" instead of a flat
/// "no data" when a provider is merely throttled.
pub(super) async fn build_source_status(
    has_dexscreener: bool,
    has_geckoterminal: bool,
    has_rugcheck: bool,
    has_ohlcv: bool,
) -> Vec<SourceStatus> {
    async fn market_state(endpoint: &str, label: &str, has_data: bool) -> SourceStatus {
        let (state, text) = if has_data {
            ("ok", UiText::new(ids::TOKENS_RESULT_SOURCE_LIVE))
        } else if crate::connectivity::get_endpoint_health(endpoint)
            .await
            .map(|h| h.is_unhealthy())
            .unwrap_or(false)
        {
            (
                "unavailable",
                UiText::new(ids::TOKENS_RESULT_SOURCE_UNAVAILABLE)
                    .arg("label", UiArg::Text(label.to_owned())),
            )
        } else {
            (
                "no_data",
                UiText::new(ids::TOKENS_RESULT_SOURCE_NOT_LISTED)
                    .arg("label", UiArg::Text(label.to_owned())),
            )
        };
        SourceStatus {
            source: endpoint.to_owned(),
            label: label.to_owned(),
            state: state.to_owned(),
            text,
        }
    }

    let (dex, gecko) = tokio::join!(
        market_state("dexscreener", "DexScreener", has_dexscreener),
        market_state("geckoterminal", "GeckoTerminal", has_geckoterminal),
    );

    let rug = {
        let (state, text) = if has_rugcheck {
            ("ok", UiText::new(ids::TOKENS_RESULT_SECURITY_AVAILABLE))
        } else if crate::connectivity::get_endpoint_health("rugcheck")
            .await
            .map(|h| h.is_unhealthy())
            .unwrap_or(false)
        {
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

    let ohlcv = SourceStatus {
        source: "ohlcv".to_owned(),
        label: "Chart".to_owned(),
        state: if has_ohlcv { "ok" } else { "no_data" }.to_owned(),
        text: UiText::new(if has_ohlcv {
            ids::TOKENS_RESULT_CHART_AVAILABLE
        } else {
            ids::TOKENS_RESULT_CHART_MISSING
        }),
    };

    vec![dex, gecko, rug, ohlcv]
}
