// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The breakdowns of the counted rounds of one window: by exit group, by entry source, by
//! return, the win and loss streaks, and the newest closes.

use std::collections::HashMap;

use super::types::{
    ExitBreakdown, Outcome, OutcomeBucket, RecentRound, SourceBreakdown, Streak, OUTCOME_EDGES_PCT,
    RECENT_ROUNDS_SHOWN, SOURCE_ORDER,
};
use crate::i18n::{ids, UiText};
use crate::positions::Position;
use crate::trader::{closed_reason_group_text, closed_reason_key, closed_reason_text};

/// The counted rounds of one group, accumulated one round at a time. A round whose P&L is
/// unknown counts toward the group size only: it carries no outcome, return or SOL.
#[derive(Default)]
struct GroupTotals {
    count: usize,
    /// Rounds with a known return, the denominator of the win rate.
    decided: usize,
    winners: usize,
    returns: Vec<f64>,
    net_pnl_native: Option<f64>,
}

impl GroupTotals {
    fn add(&mut self, position: &Position) {
        self.count += 1;
        if let Some(pnl_percent) = position.pnl_percent {
            self.decided += 1;
            self.returns.push(pnl_percent);
        }
        self.winners += usize::from(Outcome::of(position) == Some(Outcome::Win));
        if let Some(pnl) = position.pnl {
            *self.net_pnl_native.get_or_insert(0.0) += pnl;
        }
    }

    /// Winners per round with a known return, in percent; `None` when no round has one.
    fn win_rate_pct(&self) -> Option<f64> {
        (self.decided > 0).then(|| self.winners as f64 / self.decided as f64 * 100.0)
    }

    /// Mean return of the rounds that carry one; `None` when none does.
    fn avg_profit_pct(&self) -> Option<f64> {
        (!self.returns.is_empty())
            .then(|| self.returns.iter().sum::<f64>() / self.returns.len() as f64)
    }
}

/// The counted rounds grouped by `trader::closed_reason_key`, carrying the SOL each group
/// returned — a reason can be the most frequent exit and still be the one losing the
/// money. Ordered by count, the key breaking a tie, so the order never changes between
/// polls.
pub(super) fn exit_breakdown(closed: &[&Position]) -> Vec<ExitBreakdown> {
    let mut groups: HashMap<String, GroupTotals> = HashMap::new();
    for position in closed {
        groups
            .entry(closed_reason_key(position.closed_reason.as_deref()))
            .or_default()
            .add(position);
    }
    let mut exits: Vec<ExitBreakdown> = groups
        .into_iter()
        .map(|(exit_type, totals)| ExitBreakdown {
            label: closed_reason_group_text(&exit_type),
            count: totals.count,
            winners: totals.winners,
            win_rate_pct: totals.win_rate_pct(),
            avg_profit_pct: totals.avg_profit_pct(),
            net_pnl_native: totals.net_pnl_native,
            exit_type,
        })
        .collect();
    exits.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.exit_type.cmp(&b.exit_type))
    });
    exits
}

/// The counted rounds grouped by entry source, in [`SOURCE_ORDER`]; a source with no round
/// is left out.
pub(super) fn source_breakdown(closed: &[&Position]) -> Vec<SourceBreakdown> {
    SOURCE_ORDER
        .iter()
        .filter_map(|source| {
            let mut totals = GroupTotals::default();
            for position in closed.iter().filter(|p| p.origin.kind() == *source) {
                totals.add(position);
            }
            (totals.count > 0).then(|| SourceBreakdown {
                source: (*source).to_owned(),
                count: totals.count,
                winners: totals.winners,
                win_rate_pct: totals.win_rate_pct(),
                avg_profit_pct: totals.avg_profit_pct(),
                net_pnl_native: totals.net_pnl_native,
            })
        })
        .collect()
}

/// The counted rounds by return, one bucket per range of [`OUTCOME_EDGES_PCT`]; empty with
/// no counted round. A round whose return is unknown sits in no bucket: placing it at 0%
/// would state a result the round does not have.
pub(super) fn outcome_distribution(closed: &[&Position]) -> Vec<OutcomeBucket> {
    if closed.is_empty() {
        return Vec::new();
    }
    let mut buckets: Vec<OutcomeBucket> = (0..=OUTCOME_EDGES_PCT.len())
        .map(|index| OutcomeBucket {
            lower_pct: index.checked_sub(1).map(|lower| OUTCOME_EDGES_PCT[lower]),
            upper_pct: OUTCOME_EDGES_PCT.get(index).copied(),
            count: 0,
            net_pnl_native: 0.0,
        })
        .collect();
    for position in closed {
        let Some(pnl_percent) = position.pnl_percent else {
            continue;
        };
        let index = OUTCOME_EDGES_PCT
            .iter()
            .filter(|edge| pnl_percent >= **edge)
            .count();
        buckets[index].count += 1;
        buckets[index].net_pnl_native += position.pnl.unwrap_or_default();
    }
    buckets
}

/// The win and loss streaks of the counted rounds.
pub(super) struct Streaks {
    pub current: Option<Streak>,
    pub longest_win: usize,
    pub longest_loss: usize,
}

/// The streaks of the `closed` rounds (newest first). The current streak runs back from
/// the newest round while the outcome holds; the longest runs are taken in exit order. A
/// flat round ends a run.
pub(super) fn streaks(closed: &[&Position]) -> Streaks {
    let current = closed
        .first()
        .and_then(|newest| Outcome::of(newest))
        .map(|outcome| Streak {
            outcome,
            length: closed
                .iter()
                .take_while(|p| Outcome::of(p) == Some(outcome))
                .count(),
        });
    let (mut longest_win, mut longest_loss) = (0, 0);
    let mut run: Option<Streak> = None;
    for position in closed.iter().rev() {
        run = match (Outcome::of(position), run) {
            (None, _) => None,
            (Some(outcome), Some(run)) if run.outcome == outcome => Some(Streak {
                outcome,
                length: run.length + 1,
            }),
            (Some(outcome), _) => Some(Streak { outcome, length: 1 }),
        };
        if let Some(run) = run {
            let longest = match run.outcome {
                Outcome::Win => &mut longest_win,
                Outcome::Loss => &mut longest_loss,
            };
            *longest = (*longest).max(run.length);
        }
    }
    Streaks {
        current,
        longest_win,
        longest_loss,
    }
}

/// The newest [`RECENT_ROUNDS_SHOWN`] of the `closed` rounds (newest first). A round
/// closed without a recorded reason reads "Not recorded".
pub(super) fn recent_rounds(closed: &[&Position]) -> Vec<RecentRound> {
    closed
        .iter()
        .filter_map(|position| {
            let exit = position.exit_time?;
            Some(RecentRound {
                position_id: position.id,
                mint: position.mint.clone(),
                symbol: position.symbol.clone(),
                name: position.name.clone(),
                source: position.origin.kind().to_owned(),
                exit_time: exit.timestamp(),
                hold_secs: (exit - position.entry_time).num_seconds(),
                pnl_native: position.pnl,
                pnl_percent: position.pnl_percent,
                reason: position
                    .closed_reason
                    .as_deref()
                    .filter(|reason| !reason.trim().is_empty())
                    .map(closed_reason_text)
                    .unwrap_or_else(|| UiText::new(ids::TRADE_REASON_UNKNOWN)),
            })
        })
        .take(RECENT_ROUNDS_SHOWN)
        .collect()
}

#[cfg(test)]
#[path = "breakdowns_tests.rs"]
mod tests;
