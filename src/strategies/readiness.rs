// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-readiness evidence of a Paper strategy: the shared paper checks composed over the
//! paper rounds the strategy took part in during its evidence window, the current paper run
//! from the last change of its rules on. The arm dialog and the strategy overview read one
//! evidence builder, so the two never disagree.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::config::with_config;
use crate::i18n::{ids, UiText};
use crate::positions::paper::PaperStrategyEvidence;
use crate::positions::Position;
use crate::strategies::types::{Strategy, StrategyType};
use crate::trader::paper::readiness::{self, CheckText, HistoryText, Readiness};
use crate::trader::LiveBlock;

/// The paper evidence of every strategy in the current paper run, read on the blocking
/// pool; empty without a loaded paper book or a wallet. Never starts a paper run. The
/// wallet address is awaited: the synchronous read refuses on a current-thread runtime,
/// which would read as no evidence at all.
pub async fn paper_strategy_evidence() -> crate::Result<PaperStrategyEvidence> {
    let Some(book) = crate::positions::paper::book() else {
        return Ok(PaperStrategyEvidence::default());
    };
    let Ok(wallet_address) = crate::config::get_wallet_pubkey_string_async().await else {
        return Ok(PaperStrategyEvidence::default());
    };
    Ok(tokio::task::spawn_blocking(move || book.strategy_evidence(&wallet_address)).await??)
}

/// The span of the paper evidence a strategy is judged on: the current paper run, from the
/// last change of the strategy's rules on. Rounds entered under earlier rules say nothing
/// about the rules the strategy would arm live with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceWindow {
    /// Where the window starts: the later of the run start and the rules change; none
    /// before the first paper run.
    pub since: Option<DateTime<Utc>>,
    /// The rules change the window starts at, when the rules changed after the run
    /// started; none when the window is the whole run.
    pub rules_changed_at: Option<DateTime<Utc>>,
}

impl EvidenceWindow {
    /// The window of `strategy` in the paper run that started at `run_started_at`.
    pub fn of(strategy: &Strategy, run_started_at: Option<DateTime<Utc>>) -> Self {
        let rules_changed_at = run_started_at
            .filter(|started_at| strategy.rules_changed_at > *started_at)
            .map(|_| strategy.rules_changed_at);
        Self {
            since: rules_changed_at.or(run_started_at),
            rules_changed_at,
        }
    }

    /// `round` counts as evidence: it was entered at or after the rules change. The run
    /// start is applied by the store read of the run's rounds.
    pub fn admits(&self, round: &Position) -> bool {
        self.rules_changed_at
            .is_none_or(|changed_at| round.entry_time >= changed_at)
    }
}

/// The paper rounds of `strategy` inside its evidence window, open and closed, in entry
/// order, with the window. The arm dialog, the readiness progress and the overview's paper
/// figures all count these rounds.
pub fn counted_rounds(
    strategy: &Strategy,
    evidence: &PaperStrategyEvidence,
) -> (Vec<Position>, EvidenceWindow) {
    let window = EvidenceWindow::of(strategy, evidence.run_started_at());
    let rounds = evidence
        .rounds_of(&strategy.id)
        .into_iter()
        .filter(|round| window.admits(round))
        .collect();
    (rounds, window)
}

/// The live readiness of `strategy` from the paper `evidence`, against the configured
/// minimum of closed rounds, with the live block in force now.
pub fn readiness_of(strategy: &Strategy, evidence: &PaperStrategyEvidence) -> Readiness {
    let (rounds, window) = counted_rounds(strategy, evidence);
    readiness_over(strategy, &rounds, window, min_closed_rounds(), evidence)
}

/// [`strategy_readiness`] of `strategy` over its counted paper `rounds` in `window`,
/// against `needed` closed rounds. The failed-exit streak is read over the whole run: it
/// measures whether exits fill, and it ends at the next filled exit.
fn readiness_over(
    strategy: &Strategy,
    rounds: &[Position],
    window: EvidenceWindow,
    needed: usize,
    evidence: &PaperStrategyEvidence,
) -> Readiness {
    strategy_readiness(
        strategy.strategy_type,
        rounds,
        window.rules_changed_at,
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
    /// The rules change the counted rounds start at, when the rules changed after the
    /// paper run started.
    pub since_rules_changed: Option<DateTime<Utc>>,
}

/// The readiness progress of `strategy`, from the same composition as [`readiness_of`].
pub fn readiness_progress(
    strategy: &Strategy,
    evidence: &PaperStrategyEvidence,
) -> ReadinessProgress {
    let (rounds, window) = counted_rounds(strategy, evidence);
    let needed = min_closed_rounds();
    let readiness = readiness_over(strategy, &rounds, window, needed, evidence);
    ReadinessProgress {
        closed: closed_rounds(&rounds).len(),
        needed,
        ready: readiness.ready,
        failing: readiness
            .checks
            .iter()
            .find(|check| !check.passed)
            .map(|check| check.id),
        since_rules_changed: window.rules_changed_at,
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
/// closed, against `min_closed_rounds`; `rules_changed_at` is the rules change the rounds
/// are counted from, when it falls inside the run; `failed_exits_in_a_row` counts the latest paper
/// exits of its rounds that found no fill, and `block` is why live trading is unavailable
/// right now, if it is. Profit is judged for entry strategies only: an exit strategy
/// closes rounds another strategy chose, so their profit is not its own. An open round is
/// priced once a mark was booked after its entry.
pub fn strategy_readiness(
    strategy_type: StrategyType,
    rounds: &[Position],
    rules_changed_at: Option<DateTime<Utc>>,
    min_closed_rounds: usize,
    failed_exits_in_a_row: u32,
    block: Option<LiveBlock>,
) -> Readiness {
    let closed = closed_rounds(rounds);
    let unpriced = rounds
        .iter()
        .filter(|round| round.exit_time.is_none() && !marked_since_entry(round))
        .count();
    let history_text = match rules_changed_at {
        Some(changed) => HistoryText {
            met: ids::STRATEGIES_READINESS_HISTORY_SINCE_MET,
            short: ids::STRATEGIES_READINESS_HISTORY_SINCE_SHORT,
            since: Some(changed),
        },
        None => HistoryText::RUN,
    };
    let mut checks = vec![readiness::history(
        closed.len(),
        min_closed_rounds,
        history_text,
    )];
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
