// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-readiness evidence of a Paper strategy: the shared paper checks composed over the
//! paper rounds the strategy took part in during the current paper run. The arm dialog and
//! the strategy overview read one evidence builder, so the two never disagree.

use serde::Serialize;

use crate::config::with_config;
use crate::i18n::{ids, UiText};
use crate::positions::paper::PaperStrategyEvidence;
use crate::positions::Position;
use crate::strategies::types::{Strategy, StrategyType};
use crate::trader::paper::readiness::{self, CheckText, Readiness};
use crate::trader::LiveBlock;

/// The paper evidence of every strategy in the current paper run, read on the blocking
/// pool; empty without a loaded paper book or a wallet. Never starts a paper run.
pub async fn paper_strategy_evidence() -> crate::Result<PaperStrategyEvidence> {
    let Some(book) = crate::positions::paper::book() else {
        return Ok(PaperStrategyEvidence::default());
    };
    let Ok(wallet_address) = crate::utils::get_wallet_address() else {
        return Ok(PaperStrategyEvidence::default());
    };
    Ok(tokio::task::spawn_blocking(move || book.strategy_evidence(&wallet_address)).await??)
}

/// The live readiness of `strategy` from the paper `evidence`, against the configured
/// minimum of closed rounds, with the live block in force now.
pub fn readiness_of(strategy: &Strategy, evidence: &PaperStrategyEvidence) -> Readiness {
    readiness_over(
        strategy,
        &evidence.rounds_of(&strategy.id),
        min_closed_rounds(),
        evidence,
    )
}

/// [`strategy_readiness`] of `strategy` over its paper `rounds`, against `needed` closed
/// rounds.
fn readiness_over(
    strategy: &Strategy,
    rounds: &[Position],
    needed: usize,
    evidence: &PaperStrategyEvidence,
) -> Readiness {
    strategy_readiness(
        strategy.strategy_type,
        rounds,
        needed,
        evidence.failed_exit_streak_of(&strategy.id),
        crate::trader::live_block(),
    )
}

/// How far a paper strategy is from arming live: its closed paper rounds against the
/// number needed, whether every check passes, and the first check that fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessProgress {
    pub closed: usize,
    pub needed: usize,
    pub ready: bool,
    /// The id of the first failing check, in the order the arm dialog lists them.
    pub failing: Option<&'static str>,
}

/// The readiness progress of `strategy`, from the same composition as [`readiness_of`].
pub fn readiness_progress(
    strategy: &Strategy,
    evidence: &PaperStrategyEvidence,
) -> ReadinessProgress {
    let rounds = evidence.rounds_of(&strategy.id);
    let needed = min_closed_rounds();
    let readiness = readiness_over(strategy, &rounds, needed, evidence);
    ReadinessProgress {
        closed: closed_rounds(&rounds).len(),
        needed,
        ready: readiness.ready,
        failing: readiness
            .checks
            .iter()
            .find(|check| !check.passed)
            .map(|check| check.id),
    }
}

/// The closed paper rounds a strategy needs before it may arm live.
fn min_closed_rounds() -> usize {
    with_config(|config| config.trader.paper.readiness_min_closed_rounds)
}

/// The closed rounds of `rounds`: those with an exit time.
fn closed_rounds(rounds: &[Position]) -> Vec<&Position> {
    rounds
        .iter()
        .filter(|round| round.exit_time.is_some())
        .collect()
}

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
    let closed = closed_rounds(rounds);
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
