// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The figures of the Stats aggregation: the window, the realized figures of one book,
//! its open exposure, and the breakdowns of its closed rounds.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use crate::i18n::UiText;

/// The realized window of the Stats figures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatsWindow {
    /// The last `n` days up to now.
    Days(u32),
    /// The whole history of the book, from its first closed round.
    All,
}

impl StatsWindow {
    /// The earliest exit the window reads from at `now`. All reads every closed round.
    pub(crate) fn since(self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Days(days) => now - Duration::days(i64::from(days)),
            Self::All => DateTime::UNIX_EPOCH,
        }
    }
}

/// The longest span, in days, the P&L series shows one point per day for. A longer
/// span shows one point per ISO week, so a long history stays a readable chart.
pub(super) const DAILY_SERIES_MAX_DAYS: i64 = 90;

/// The edges, in percent, of the return buckets of the outcome distribution. A bucket
/// holds its lower edge and not its upper one; the first is open below, the last above.
pub(super) const OUTCOME_EDGES_PCT: [f64; 8] = [-50.0, -20.0, -5.0, 0.0, 5.0, 20.0, 50.0, 100.0];

/// How many of the newest counted rounds the recent list carries.
pub(super) const RECENT_ROUNDS_SHOWN: usize = 10;

/// The entry sources of the source breakdown, in the order it lists them.
pub(super) const SOURCE_ORDER: [&str; 4] = ["auto", "copy", "manual", "external"];

/// Realized performance over a closed window, plus the live exposure that window
/// does not cover.
///
/// Every derived figure is `Option` and is `None` when the window holds nothing to
/// derive it from. A fresh install has no win rate and no best trade, and `0.0`
/// there is a fabricated claim (a green `+0.0%` "best trade") rather than an empty
/// state, so the absence travels to the dashboard instead of a zero.
#[derive(Debug, Serialize)]
pub struct TraderStats {
    // Live exposure (not window-bound), over the rounds that take a position slot.
    pub open_positions_count: usize,
    pub max_open_positions: usize,
    pub locked_native: f64,
    /// Unrealized P&L of the priced slot rounds at their pool prices; `None` while no
    /// round is priced.
    pub unrealized_native: Option<f64>,
    /// Slot rounds with no unrealized P&L yet, left out of `unrealized_native`.
    pub open_unpriced: usize,

    #[serde(flatten)]
    pub realized: RealizedStats,
}

/// The window-bound part of [`TraderStats`]: every figure realized by the closed rounds
/// of the window.
#[derive(Debug, Clone, Serialize)]
pub struct RealizedStats {
    /// Length of the realized window in days, echoed back so the UI labels what it
    /// shows; `None` for the whole history.
    pub period_days: Option<u32>,

    // Trade counts.
    pub total_trades: usize,
    pub winners: usize,
    pub losers: usize,
    /// Closed rounds excluded because their cost basis or history is incomplete, so
    /// no honest P&L exists for them. Surfaced rather than silently dropped.
    pub excluded_untrusted: usize,

    // Realized money, in SOL — the single monetary unit.
    pub total_pnl_native: f64,
    pub gross_profit_native: f64,
    pub gross_loss_native: f64,
    pub profit_factor: Option<f64>,
    pub expectancy_native: Option<f64>,
    pub max_drawdown_native: f64,
    /// Network fees of every buy and sale of the counted rounds, already inside their
    /// P&L; `None` with no counted round.
    pub fees_native: Option<f64>,
    /// Average SOL won per winning round over average SOL lost per losing round; `None`
    /// without a win and a loss.
    pub payoff_ratio: Option<f64>,

    // Quality.
    pub win_rate_pct: Option<f64>,
    pub avg_win_pct: Option<f64>,
    pub avg_loss_pct: Option<f64>,
    pub avg_hold_time_hours: Option<f64>,
    pub median_hold_time_hours: Option<f64>,
    pub median_win_hold_hours: Option<f64>,
    pub median_loss_hold_hours: Option<f64>,
    pub best_trade_pct: Option<f64>,
    pub best_trade_token: Option<String>,
    pub worst_trade_pct: Option<f64>,
    pub worst_trade_token: Option<String>,

    // Streaks, in exit order. A flat round ends a run.
    /// The run the newest counted round belongs to; `None` with no round or when the
    /// newest round is flat.
    pub current_streak: Option<Streak>,
    pub longest_win_streak: usize,
    pub longest_loss_streak: usize,

    /// One entry per day of the window, oldest first, or one per ISO week when the
    /// window spans more than [`DAILY_SERIES_MAX_DAYS`] days, including buckets with no
    /// trades so the curve keeps a true time axis. Empty for the whole history of a book
    /// that never closed a round.
    pub daily_pnl: Vec<DailyPnlPoint>,
    pub exit_breakdown: Vec<ExitBreakdown>,
    pub source_breakdown: Vec<SourceBreakdown>,
    /// The counted rounds by return, one bucket per range of [`OUTCOME_EDGES_PCT`]; empty
    /// with no counted round.
    pub outcome_distribution: Vec<OutcomeBucket>,
    /// The newest [`RECENT_ROUNDS_SHOWN`] counted rounds, newest first.
    pub recent_rounds: Vec<RecentRound>,
}

/// The counted closed rounds of a set: how many, how many won and lost, what they realized
/// and their win rate. The one counting rule shared by the Stats tab and the strategy
/// overview.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RoundTotals {
    /// Closed rounds counted as trades.
    pub closed: usize,
    pub winners: usize,
    pub losers: usize,
    /// Closed rounds left out because their cost basis or history is incomplete, so no
    /// honest P&L exists for them. Surfaced rather than silently dropped.
    pub excluded_untrusted: usize,
    /// Realized P&L of the counted rounds in the native asset.
    pub realized_native: f64,
    /// Winners per counted round with a known return; `None` while no such round exists.
    /// A round whose P&L is unknown is neither a win nor a loss and stays out of the rate.
    pub win_rate_pct: Option<f64>,
}

/// Whether a closed round won or lost, by the sign of its return. A flat round is
/// neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Win,
    Loss,
}

/// A run of closed rounds with the same outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Streak {
    pub outcome: Outcome,
    pub length: usize,
}

/// The span one P&L point covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PnlBucket {
    /// One UTC calendar day.
    Day,
    /// One ISO week, Monday to Sunday, UTC.
    Week,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyPnlPoint {
    /// First UTC day of the bucket, `YYYY-MM-DD`: the day itself, or the Monday of the
    /// week.
    pub date: String,
    pub bucket: PnlBucket,
    pub net_pnl_native: f64,
    pub trades: usize,
}

/// The counted rounds of one exit group.
#[derive(Debug, Clone, Serialize)]
pub struct ExitBreakdown {
    /// The group key from `trader::closed_reason_key`.
    pub exit_type: String,
    pub label: UiText,
    pub count: usize,
    pub winners: usize,
    /// Winners per round with a known return; `None` when no round in the group has one.
    pub win_rate_pct: Option<f64>,
    /// Mean return of the rounds that carry one; `None` when none does.
    pub avg_profit_pct: Option<f64>,
    /// Realized SOL attributable to this exit reason; `None` when no round in the group
    /// has a known P&L.
    pub net_pnl_native: Option<f64>,
}

/// The counted rounds of one entry source (`PositionOrigin::kind`).
#[derive(Debug, Clone, Serialize)]
pub struct SourceBreakdown {
    pub source: String,
    pub count: usize,
    pub winners: usize,
    /// Winners per round with a known return; `None` when no round of the source has one.
    pub win_rate_pct: Option<f64>,
    /// Mean return of the rounds that carry one; `None` when none does.
    pub avg_profit_pct: Option<f64>,
    /// `None` when no round of the source has a known P&L.
    pub net_pnl_native: Option<f64>,
}

/// The counted rounds whose return falls in `[lower_pct, upper_pct)`. A round with an
/// unknown return is in no bucket.
#[derive(Debug, Clone, Serialize)]
pub struct OutcomeBucket {
    /// `None` for the first bucket, open below.
    pub lower_pct: Option<f64>,
    /// `None` for the last bucket, open above.
    pub upper_pct: Option<f64>,
    pub count: usize,
    pub net_pnl_native: f64,
}

/// One recently closed counted round.
#[derive(Debug, Clone, Serialize)]
pub struct RecentRound {
    pub position_id: Option<i64>,
    pub mint: String,
    pub symbol: String,
    pub name: String,
    /// The entry source, `PositionOrigin::kind`.
    pub source: String,
    /// Unix seconds.
    pub exit_time: i64,
    pub hold_secs: i64,
    pub pnl_native: Option<f64>,
    pub pnl_percent: Option<f64>,
    pub reason: UiText,
}
