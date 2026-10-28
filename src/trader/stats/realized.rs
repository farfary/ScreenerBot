// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The realized figures of one window: which closed rounds count as trades, their totals,
//! the P&L series and drawdown, hold times and the open exposure beside them.

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use std::collections::HashMap;

use super::breakdowns;
use super::types::{
    DailyPnlPoint, Outcome, PnlBucket, RealizedStats, RoundTotals, StatsWindow, TraderStats,
    DAILY_SERIES_MAX_DAYS,
};
use crate::positions::{self, Position};

/// True for a round a paper run reset closed at its mark: no exit decision closed it, so
/// it is not a trade.
fn is_paper_reset_close(position: &Position) -> bool {
    position.closed_reason.as_deref()
        == Some(format!("{:?}", crate::trader::TradeReason::PaperReset).as_str())
}

impl Outcome {
    /// The outcome of a closed round by the sign of its return; `None` for a flat round
    /// and for a round whose return is unknown.
    pub(crate) fn of(position: &Position) -> Option<Self> {
        let pnl_percent = position.pnl_percent?;
        if pnl_percent > 0.0 {
            Some(Self::Win)
        } else if pnl_percent < 0.0 {
            Some(Self::Loss)
        } else {
            None
        }
    }
}

impl RoundTotals {
    /// The totals of the `counted` rounds, with `excluded_untrusted` left out before.
    ///
    /// Realized P&L is the `pnl` each round booked at close, fee-aware and DCA-aware.
    /// Deriving it as `native_received - entry_size_native` counted every DCA add as pure
    /// profit, because `entry_size_native` is only the first buy and never grows.
    fn of(counted: &[&Position], excluded_untrusted: usize) -> Self {
        let closed = counted.len();
        let outcome_count = |outcome: Outcome| {
            counted
                .iter()
                .filter(|p| Outcome::of(p) == Some(outcome))
                .count()
        };
        let winners = outcome_count(Outcome::Win);
        let losers = outcome_count(Outcome::Loss);
        let decided = counted.iter().filter(|p| p.pnl_percent.is_some()).count();
        Self {
            closed,
            winners,
            losers,
            excluded_untrusted,
            realized_native: counted.iter().filter_map(|p| p.pnl).sum(),
            win_rate_pct: (decided > 0).then(|| (winners as f64 / decided as f64) * 100.0),
        }
    }
}

/// The totals of the closed `rounds`, counted as [`counted_rounds`] counts them.
pub fn round_totals<'a>(rounds: impl IntoIterator<Item = &'a Position>) -> RoundTotals {
    let (counted, excluded_untrusted) = counted_rounds(rounds);
    RoundTotals::of(&counted, excluded_untrusted)
}

/// The closed `rounds` counted as trades, in their order, and how many were left out for
/// an incomplete basis or history. A round a paper run reset closed is not a trade and is
/// neither counted nor left out. A round whose basis or history is incomplete has no
/// honest P&L; Overview already refuses to price those, and counting them is what let the
/// Stats tab disagree with Home on the same wallet.
fn counted_rounds<'a>(
    rounds: impl IntoIterator<Item = &'a Position>,
) -> (Vec<&'a Position>, usize) {
    let mut excluded_untrusted = 0;
    let counted = rounds
        .into_iter()
        .filter(|p| !is_paper_reset_close(p))
        .filter(|p| {
            let trusted = p.has_trustworthy_pnl();
            excluded_untrusted += usize::from(!trusted);
            trusted
        })
        .collect();
    (counted, excluded_untrusted)
}

/// The hold of a closed round in hours; `None` for a round with no exit time.
fn hold_hours(position: &Position) -> Option<f64> {
    position
        .exit_time
        .map(|exit| (exit - position.entry_time).num_seconds() as f64 / 3600.0)
}

/// The median of `values`; `None` when there are none.
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

/// The mean of `values`; `None` when there are none.
fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

impl RealizedStats {
    /// The realized figures of `window` at `now` from the rounds it closed. The rounds are
    /// put in exit order here (newest first, the id breaking a tie), so no figure depends on
    /// the order they arrive in. A round a paper run reset closed is not a trade and is left
    /// out of every figure. A day window starts `n` days before `now`; the whole history
    /// starts at the book's first closed round.
    pub(crate) fn of(
        window: StatsWindow,
        mut recent_closed: Vec<Position>,
        now: DateTime<Utc>,
    ) -> Self {
        recent_closed.sort_by(|a, b| b.exit_time.cmp(&a.exit_time).then(b.id.cmp(&a.id)));
        let (period_days, window_start) = match window {
            StatsWindow::Days(days) => (Some(days), Some(window.since(now))),
            StatsWindow::All => (None, recent_closed.iter().filter_map(|p| p.exit_time).min()),
        };

        let (closed, excluded_untrusted) = counted_rounds(&recent_closed);
        let RoundTotals {
            closed: total_trades,
            winners,
            losers,
            realized_native: total_pnl_native,
            win_rate_pct,
            ..
        } = RoundTotals::of(&closed, excluded_untrusted);

        // Hold time. The median is reported alongside the mean because one forgotten
        // bag drags the mean far away from what a typical round actually looked like.
        let hold_hours_of = |outcome: Option<Outcome>| -> Vec<f64> {
            closed
                .iter()
                .filter(|p| outcome.is_none() || Outcome::of(p) == outcome)
                .filter_map(|p| hold_hours(p))
                .collect()
        };
        let hold_hours_all = hold_hours_of(None);
        let avg_hold_time_hours = mean(&hold_hours_all);
        let median_hold_time_hours = median(hold_hours_all);
        let median_win_hold_hours = median(hold_hours_of(Some(Outcome::Win)));
        let median_loss_hold_hours = median(hold_hours_of(Some(Outcome::Loss)));

        // Best/worst by percent, keeping the token so the card can name it.
        let best_trade = closed
            .iter()
            .filter(|p| p.pnl_percent.is_some())
            .max_by(|a, b| {
                a.pnl_percent
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&b.pnl_percent.unwrap_or(f64::NEG_INFINITY))
            });
        let worst_trade = closed
            .iter()
            .filter(|p| p.pnl_percent.is_some())
            .min_by(|a, b| {
                a.pnl_percent
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&b.pnl_percent.unwrap_or(f64::INFINITY))
            });

        let best_trade_pct = best_trade.and_then(|p| p.pnl_percent);
        let best_trade_token = best_trade.map(|p| p.symbol.clone());
        let worst_trade_pct = worst_trade.and_then(|p| p.pnl_percent);
        let worst_trade_token = worst_trade.map(|p| p.symbol.clone());

        // Gross realized P&L in SOL across the window, from the same booked `pnl` the total
        // is taken on.
        let profits: Vec<f64> = closed
            .iter()
            .filter_map(|p| p.pnl)
            .filter(|v| *v > 0.0)
            .collect();
        let losses: Vec<f64> = closed
            .iter()
            .filter_map(|p| p.pnl)
            .filter(|v| *v < 0.0)
            .map(f64::abs)
            .collect();
        let gross_profit_native: f64 = profits.iter().sum();
        let gross_loss_native: f64 = losses.iter().sum();
        // Profit factor is undefined without a loss to divide by — a losing streak with
        // no wins is 0.0, but a clean run with no losses is "no answer yet", not infinity.
        let profit_factor =
            (gross_loss_native > 0.0).then(|| gross_profit_native / gross_loss_native);
        let expectancy_native = (total_trades > 0).then(|| total_pnl_native / total_trades as f64);
        let payoff_ratio = (!profits.is_empty() && gross_loss_native > 0.0).then(|| {
            (gross_profit_native / profits.len() as f64) / (gross_loss_native / losses.len() as f64)
        });
        let fees_native = (!closed.is_empty()).then(|| {
            closed
                .iter()
                .map(|p| positions::calculate_position_total_fees(p))
                .sum()
        });

        let avg_win_pct = {
            let wins: Vec<f64> = closed
                .iter()
                .filter_map(|p| p.pnl_percent)
                .filter(|v| *v > 0.0)
                .collect();
            mean(&wins)
        };
        let avg_loss_pct = {
            let losses: Vec<f64> = closed
                .iter()
                .filter_map(|p| p.pnl_percent)
                .filter(|v| *v < 0.0)
                .collect();
            mean(&losses)
        };

        let (daily_pnl, max_drawdown_native) = pnl_series(&closed, window_start, now);
        let streaks = breakdowns::streaks(&closed);

        Self {
            period_days,
            total_trades,
            winners,
            losers,
            excluded_untrusted,
            total_pnl_native,
            gross_profit_native,
            gross_loss_native,
            profit_factor,
            expectancy_native,
            max_drawdown_native,
            fees_native,
            payoff_ratio,
            win_rate_pct,
            avg_win_pct,
            avg_loss_pct,
            avg_hold_time_hours,
            median_hold_time_hours,
            median_win_hold_hours,
            median_loss_hold_hours,
            best_trade_pct,
            best_trade_token,
            worst_trade_pct,
            worst_trade_token,
            current_streak: streaks.current,
            longest_win_streak: streaks.longest_win,
            longest_loss_streak: streaks.longest_loss,
            daily_pnl,
            exit_breakdown: breakdowns::exit_breakdown(&closed),
            source_breakdown: breakdowns::source_breakdown(&closed),
            outcome_distribution: breakdowns::outcome_distribution(&closed),
            recent_rounds: breakdowns::recent_rounds(&closed),
        }
    }

    /// These figures with the book's open rounds that take a slot (`slot_positions`) and
    /// its slot limit.
    pub(crate) fn with_exposure(
        &self,
        slot_positions: &[Position],
        max_open_positions: usize,
    ) -> TraderStats {
        let unrealized = positions::open_unrealized(slot_positions);
        TraderStats {
            open_positions_count: slot_positions.len(),
            max_open_positions,
            locked_native: slot_positions.iter().map(|p| p.total_size_native).sum(),
            unrealized_native: unrealized.native,
            open_unpriced: unrealized.unpriced,
            realized: self.clone(),
        }
    }
}

/// The P&L series of the window and the realized drawdown, from one pass over the
/// `closed` rounds (newest first) in exit order. Every bucket from the window's first day
/// to `now` is emitted, the empty ones included, so the curve keeps a true time axis
/// instead of compressing quiet stretches away.
fn pnl_series(
    closed: &[&Position],
    window_start: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> (Vec<DailyPnlPoint>, f64) {
    let today = now.date_naive();
    let first_day = window_start.map(|start| start.date_naive());
    let bucket = match first_day {
        Some(first) if (today - first).num_days() + 1 > DAILY_SERIES_MAX_DAYS => PnlBucket::Week,
        _ => PnlBucket::Day,
    };
    let mut per_bucket: HashMap<NaiveDate, (f64, usize)> = HashMap::new();
    let mut equity = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut max_drawdown_native = 0.0_f64;

    for pos in closed.iter().rev() {
        let pnl = pos.pnl.unwrap_or_default();
        equity += pnl;
        peak = peak.max(equity);
        max_drawdown_native = max_drawdown_native.max(peak - equity);

        if let Some(exit) = pos.exit_time {
            let entry = per_bucket
                .entry(bucket_start(bucket, exit.date_naive()))
                .or_insert((0.0, 0));
            entry.0 += pnl;
            entry.1 += 1;
        }
    }

    let mut daily_pnl = Vec::new();
    if let Some(first) = first_day {
        let step = match bucket {
            PnlBucket::Day => Duration::days(1),
            PnlBucket::Week => Duration::weeks(1),
        };
        let mut start = bucket_start(bucket, first);
        while start <= today {
            let (net_pnl_native, trades) = per_bucket.get(&start).copied().unwrap_or((0.0, 0));
            daily_pnl.push(DailyPnlPoint {
                date: start.format("%Y-%m-%d").to_string(),
                bucket,
                net_pnl_native,
                trades,
            });
            start = match start.checked_add_signed(step) {
                Some(next) => next,
                None => break,
            };
        }
    }
    (daily_pnl, max_drawdown_native)
}

/// The first day of the `bucket` holding `day`: the day itself, or the Monday of its ISO
/// week.
fn bucket_start(bucket: PnlBucket, day: NaiveDate) -> NaiveDate {
    match bucket {
        PnlBucket::Day => day,
        PnlBucket::Week => day - Duration::days(i64::from(day.weekday().num_days_from_monday())),
    }
}
