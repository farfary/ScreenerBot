// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-readiness evidence of a Paper strategy: the shared paper checks composed over the
//! paper rounds the strategy took part in during the current paper run.

use crate::i18n::{ids, UiText};
use crate::positions::Position;
use crate::strategies::types::StrategyType;
use crate::trader::paper::readiness::{self, CheckText, Readiness};
use crate::trader::LiveBlock;

/// The wording of why live trading is unavailable.
pub fn live_block_text(block: LiveBlock) -> UiText {
    UiText::new(match block {
        LiveBlock::SetupIncomplete => ids::PAPER_LIVE_BLOCK_SETUP_INCOMPLETE,
        LiveBlock::ForceStop => ids::PAPER_LIVE_BLOCK_FORCE_STOP,
    })
}

/// The readiness of a strategy of `strategy_type` from its paper `rounds`, open and
/// closed, against `min_closed_rounds`; `failed_exits_in_a_row` counts the latest paper
/// exits of its rounds that found no fill, and `block` is why live trading is unavailable
/// right now, if it is. Profit is judged for entry strategies only: an exit strategy
/// closes rounds another strategy chose, so their profit is not its own. An open round is
/// priced once a mark was booked after its entry.
pub fn strategy_readiness(
    strategy_type: StrategyType,
    rounds: &[Position],
    min_closed_rounds: usize,
    failed_exits_in_a_row: u32,
    block: Option<LiveBlock>,
) -> Readiness {
    let closed: Vec<&Position> = rounds
        .iter()
        .filter(|round| round.exit_time.is_some())
        .collect();
    let unpriced = rounds
        .iter()
        .filter(|round| round.exit_time.is_none() && !marked_since_entry(round))
        .count();
    let mut checks = vec![readiness::history(closed.len(), min_closed_rounds)];
    if strategy_type == StrategyType::Entry {
        let realized: f64 = closed
            .iter()
            .map(|round| round.pnl.unwrap_or_default())
            .sum();
        let wins = closed
            .iter()
            .filter(|round| round.pnl.is_some_and(|pnl| pnl > 0.0))
            .count();
        checks.push(readiness::profit(realized, closed.len(), wins));
    }
    checks.push(readiness::priced(
        unpriced,
        CheckText {
            title: ids::STRATEGIES_READINESS_PRICED,
            passed: ids::STRATEGIES_READINESS_PRICED_OK,
            failed: ids::STRATEGIES_READINESS_PRICED_MISSING,
        },
    ));
    checks.push(readiness::exits_filled(failed_exits_in_a_row));
    checks.push(readiness::runtime(
        block.map(live_block_text),
        ids::STRATEGIES_READINESS_RUNTIME,
        ids::STRATEGIES_READINESS_RUNTIME_OK,
    ));
    Readiness::from_checks(checks)
}

/// A mark was booked on `round` after its entry: the entry price it opened with is not a
/// market reading of the round.
fn marked_since_entry(round: &Position) -> bool {
    round.current_price.is_some()
        && round
            .current_price_updated
            .is_some_and(|marked_at| marked_at > round.entry_time)
}

#[cfg(test)]
#[path = "readiness_tests.rs"]
mod tests;
