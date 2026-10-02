// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-readiness evidence from the paper book. Every check carries catalog
//! text for its title and its detail; the figures are arguments.

use serde::Serialize;

use super::PaperHolding;
use crate::config::with_config;
use crate::i18n::{ids, UiArg, UiText};
use crate::trader::copy::control::CopyTaskSummary;
use crate::trader::copy::CopyRound;

#[derive(Debug, Serialize)]
pub struct ReadinessCheck {
    pub id: &'static str,
    pub text: UiText,
    pub passed: bool,
    pub detail: UiText,
}

/// Advisory evidence from the paper book before arming live. Arming still
/// needs the explicit confirmation; this is what the confirmation is about.
#[derive(Debug, Serialize)]
pub struct Readiness {
    pub ready: bool,
    pub checks: Vec<ReadinessCheck>,
}

/// A signed SOL figure as the dashboard writes one, with a true minus sign.
fn signed_sol(value: f64) -> String {
    let sign = if value > 0.0 {
        "+"
    } else if value < 0.0 {
        "\u{2212}"
    } else {
        ""
    };
    format!("{sign}{:.4}", value.abs())
}

/// Seconds with one decimal, the figure the latency wording shows.
fn tenths_of_seconds(ms: u64) -> String {
    format!("{:.1}", ms as f64 / 1000.0)
}

pub(super) fn live_block_text(reason: &str) -> UiText {
    UiText::new(match reason {
        "setup_incomplete" => ids::COPY_LIVE_BLOCK_SETUP_INCOMPLETE,
        "force_stop" => ids::COPY_LIVE_BLOCK_FORCE_STOP,
        "copy_trading_disabled" => ids::COPY_LIVE_BLOCK_COPY_TRADING_DISABLED,
        _ => ids::COPY_LIVE_BLOCK_UNAVAILABLE,
    })
}

fn count(value: usize) -> UiArg {
    UiArg::Count(i64::try_from(value).unwrap_or(i64::MAX))
}

/// `block` is why live execution is unavailable right now, if it is.
pub(super) fn readiness(
    summary: &CopyTaskSummary,
    rounds: &[CopyRound],
    holdings: &[PaperHolding],
    block: Option<&'static str>,
) -> Readiness {
    let (min_rounds, max_arrival_ms) = with_config(|config| {
        (
            config.copy_trading.readiness_min_closed_rounds,
            config.copy_trading.max_arrival_distance_ms,
        )
    });
    let realized: f64 = rounds.iter().map(|round| round.pnl_sol).sum();
    let wins = rounds.iter().filter(|round| round.pnl_sol > 0.0).count();
    let p95 = summary.stats.arrival_distance.p95_ms;
    let unpriced = holdings
        .iter()
        .filter(|holding| holding.open && holding.mark_price_sol.is_none())
        .count();
    let checks = vec![
        ReadinessCheck {
            id: "history",
            text: UiText::new(ids::COPY_READINESS_HISTORY),
            passed: rounds.len() >= min_rounds,
            detail: UiText::new(if rounds.len() >= min_rounds {
                ids::COPY_READINESS_HISTORY_MET
            } else {
                ids::COPY_READINESS_HISTORY_SHORT
            })
            .arg("count", count(rounds.len()))
            .arg("needed", count(min_rounds)),
        },
        ReadinessCheck {
            id: "profit",
            text: UiText::new(ids::COPY_READINESS_PROFIT),
            passed: realized > 0.0,
            detail: UiText::new(ids::COPY_READINESS_PROFIT_DETAIL)
                .arg("realized", UiArg::Text(signed_sol(realized)))
                .arg("count", count(rounds.len()))
                .arg("wins", count(wins)),
        },
        ReadinessCheck {
            id: "latency",
            text: UiText::new(ids::COPY_READINESS_LATENCY),
            passed: p95.is_some_and(|p95| p95 <= max_arrival_ms),
            detail: match p95 {
                Some(p95) => UiText::new(ids::COPY_READINESS_LATENCY_DETAIL)
                    .arg("p95", UiArg::Text(tenths_of_seconds(p95)))
                    .arg("limit", UiArg::Text(tenths_of_seconds(max_arrival_ms))),
                None => UiText::new(ids::COPY_READINESS_LATENCY_NONE),
            },
        },
        ReadinessCheck {
            id: "priced",
            text: UiText::new(ids::COPY_READINESS_PRICED),
            passed: unpriced == 0,
            detail: if unpriced == 0 {
                UiText::new(ids::COPY_READINESS_PRICED_OK)
            } else {
                UiText::new(ids::COPY_READINESS_PRICED_MISSING).arg("count", count(unpriced))
            },
        },
        ReadinessCheck {
            id: "runtime",
            text: UiText::new(ids::COPY_READINESS_RUNTIME),
            passed: block.is_none(),
            detail: block
                .map(live_block_text)
                .unwrap_or_else(|| UiText::new(ids::COPY_READINESS_RUNTIME_OK)),
        },
    ];
    Readiness {
        ready: checks.iter().all(|check| check.passed),
        checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::LanguageIdentifier;

    #[test]
    fn live_block_reasons_render_their_wording() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        for (reason, english) in [
            ("setup_incomplete", "Finish wallet and RPC setup first"),
            ("force_stop", "The emergency stop is engaged"),
            (
                "copy_trading_disabled",
                "Copy processing is paused globally",
            ),
            ("anything_else", "Live execution is unavailable"),
        ] {
            assert_eq!(live_block_text(reason).render_plain(&en), english);
        }
    }

    #[test]
    fn readiness_details_keep_their_figures_and_plurals() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let profit = UiText::new(ids::COPY_READINESS_PROFIT_DETAIL)
            .arg("realized", UiArg::Text(signed_sol(-0.12345)))
            .arg("count", count(1))
            .arg("wins", count(0));
        assert_eq!(
            profit.render_plain(&en),
            "\u{2212}0.1235 SOL realized over 1 round, 0 won"
        );
        let short = UiText::new(ids::COPY_READINESS_HISTORY_SHORT)
            .arg("count", count(2))
            .arg("needed", count(5));
        assert_eq!(short.render_plain(&en), "2 of 5 closed paper rounds");
        let met = UiText::new(ids::COPY_READINESS_HISTORY_MET)
            .arg("count", count(1))
            .arg("needed", count(1));
        assert_eq!(met.render_plain(&en), "1 closed paper round, 1 needed");
        let missing = UiText::new(ids::COPY_READINESS_PRICED_MISSING).arg("count", count(3));
        assert_eq!(
            missing.render_plain(&en),
            "3 open holdings without a pool price"
        );
    }
}
