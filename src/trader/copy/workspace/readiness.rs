// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-readiness evidence from a copy task's rounds of the shared paper book: the
//! shared paper checks composed with the copy latency check.

use crate::config::with_config;
use crate::i18n::{ids, UiArg, UiText};
use crate::trader::copy::control::{CopyLiveBlock, CopyTaskSummary};
use crate::trader::copy::CopyRound;
use crate::trader::paper::readiness::{self, CheckText, HistoryText, Readiness, ReadinessCheck};

/// Seconds with one decimal, the figure the latency wording shows.
fn tenths_of_seconds(ms: u64) -> String {
    format!("{:.1}", ms as f64 / 1000.0)
}

/// The wording of why a copy task cannot run live.
pub(super) fn live_block_text(block: CopyLiveBlock) -> UiText {
    match block {
        CopyLiveBlock::Trader(block) => crate::strategies::readiness::live_block_text(block),
        CopyLiveBlock::CopyTradingDisabled => {
            UiText::new(ids::COPY_LIVE_BLOCK_COPY_TRADING_DISABLED)
        }
    }
}

/// `rounds` are the task's closed paper rounds of its current run, `unpriced` its open
/// rounds without a price, `failed_exits` the paper exits of its rounds that found no
/// fill in a row, and `block` why live execution is unavailable right now, if it is.
pub(super) fn readiness(
    summary: &CopyTaskSummary,
    rounds: &[CopyRound],
    unpriced: usize,
    failed_exits: u32,
    block: Option<CopyLiveBlock>,
) -> Readiness {
    let (min_rounds, max_arrival_ms) = with_config(|config| {
        (
            config.copy_trading.readiness_min_closed_rounds,
            config.copy_trading.max_arrival_distance_ms,
        )
    });
    let realized: f64 = rounds.iter().map(|round| round.pnl_native).sum();
    let wins = rounds.iter().filter(|round| round.pnl_native > 0.0).count();
    let p95 = summary.stats.arrival_distance.p95_ms;
    let checks = vec![
        readiness::history(rounds.len(), min_rounds, HistoryText::RUN),
        readiness::profit(realized, rounds.len(), wins),
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
        readiness::priced(
            unpriced,
            CheckText {
                title: ids::COPY_READINESS_PRICED,
                passed: ids::COPY_READINESS_MARKED_OK,
                failed: ids::COPY_READINESS_MARKED_MISSING,
            },
        ),
        readiness::exits_filled(failed_exits),
        readiness::runtime(
            block.map(live_block_text),
            ids::COPY_READINESS_RUNTIME,
            ids::COPY_READINESS_RUNTIME_OK,
        ),
    ];
    Readiness::from_checks(checks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::LanguageIdentifier;

    #[test]
    fn live_block_reasons_render_their_wording() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        for (block, english) in [
            (
                CopyLiveBlock::Trader(crate::trader::LiveBlock::SetupIncomplete),
                "Finish wallet and RPC setup first",
            ),
            (
                CopyLiveBlock::Trader(crate::trader::LiveBlock::ForceStop),
                "The emergency stop is engaged",
            ),
            (
                CopyLiveBlock::CopyTradingDisabled,
                "Copy processing is paused globally",
            ),
        ] {
            assert_eq!(live_block_text(block).render_plain(&en), english);
        }
    }

    #[test]
    fn copy_priced_check_keeps_its_holding_wording() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let missing = readiness::priced(
            3,
            CheckText {
                title: ids::COPY_READINESS_PRICED,
                passed: ids::COPY_READINESS_MARKED_OK,
                failed: ids::COPY_READINESS_MARKED_MISSING,
            },
        );
        assert!(!missing.passed);
        assert_eq!(
            missing.detail.render_plain(&en),
            "3 open holdings without a price"
        );
    }

    #[test]
    fn a_run_of_unfilled_paper_exits_fails_the_copy_readiness() {
        crate::config::utils::install_default_config();
        let summary = crate::trader::copy::workspace::tests::summary_fixture();
        let check = |failed_exits| {
            readiness(&summary, &[], 0, failed_exits, None)
                .checks
                .into_iter()
                .find(|check| check.id == "exits")
                .expect("copy readiness carries the exits check")
        };
        assert!(check(0).passed);
        assert!(check(readiness::FAILED_EXIT_LIMIT - 1).passed);
        assert!(!check(readiness::FAILED_EXIT_LIMIT).passed);
    }
}
