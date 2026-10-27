// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Realized auto-trader performance over a window, plus open exposure, of one book. One
//! aggregation over one row set, shared by the dashboard Stats tab and the agent
//! tools, so no surface can show a figure the other never computed. The live and the
//! paper book are aggregated apart and never mixed.
//!
//! The realized part is cached per book, wallet, window and UTC day behind the
//! fingerprint of the window's closed rounds: a poll of an unchanged book costs one
//! indexed aggregate statement, and the closed rows are read and aggregated again only
//! when that fingerprint moves. Open exposure is read on every call.

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use crate::config::with_config;
use crate::positions::{self, Book, ClosedFingerprint, Position};

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
    fn since(self, now: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Self::Days(days) => now - Duration::days(i64::from(days)),
            Self::All => DateTime::UNIX_EPOCH,
        }
    }
}

/// The longest span, in days, the P&L series shows one point per day for. A longer
/// span shows one point per ISO week, so a long history stays a readable chart.
const DAILY_SERIES_MAX_DAYS: i64 = 90;

/// Realized performance over a closed window, plus the live exposure that window
/// does not cover.
///
/// Every derived figure is `Option` and is `None` when the window holds nothing to
/// derive it from. A fresh install has no win rate and no best trade, and `0.0`
/// there is a fabricated claim (a green `+0.0%` "best trade") rather than an empty
/// state, so the absence travels to the dashboard instead of a zero.
#[derive(Debug, Serialize)]
pub struct TraderStats {
    // Live exposure (not window-bound).
    pub open_positions_count: usize,
    pub max_open_positions: usize,
    pub locked_native: f64,

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

    // Quality.
    pub win_rate_pct: Option<f64>,
    pub avg_win_pct: Option<f64>,
    pub avg_loss_pct: Option<f64>,
    pub avg_hold_time_hours: Option<f64>,
    pub median_hold_time_hours: Option<f64>,
    pub best_trade_pct: Option<f64>,
    pub best_trade_token: Option<String>,
    pub worst_trade_pct: Option<f64>,
    pub worst_trade_token: Option<String>,

    /// One entry per day of the window, oldest first, or one per ISO week when the
    /// window spans more than [`DAILY_SERIES_MAX_DAYS`] days, including buckets with no
    /// trades so the curve keeps a true time axis. Empty for the whole history of a book
    /// that never closed a round.
    pub daily_pnl: Vec<DailyPnlPoint>,
    pub exit_breakdown: Vec<ExitBreakdown>,
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

#[derive(Debug, Clone, Serialize)]
pub struct ExitBreakdown {
    pub exit_type: String,
    pub count: usize,
    pub avg_profit_pct: f64,
    /// Realized SOL attributable to this exit reason.
    pub net_pnl_native: f64,
}

/// Aggregate the closed rounds of `book` in `window` with its currently open exposure.
pub async fn trader_stats(book: Book, window: StatsWindow) -> TraderStats {
    // Open exposure. Not window-bound: what is at risk right now is independent of
    // how far back the realized window reaches.
    let open_positions = positions::books::open_positions(book).await;
    let max_open_positions = with_config(|cfg| cfg.trader.max_open_positions);
    let realized = cached_realized(&REALIZED_CACHE, book, window, Utc::now()).await;
    realized.with_exposure(&open_positions, max_open_positions)
}

/// The realized part of `book` in `window` at `now`, from `cache` while the window's
/// closed rounds keep their fingerprint. A store that cannot be read yields the empty
/// window and is never cached, so the next poll reads it again.
async fn cached_realized(
    cache: &RealizedCache,
    book: Book,
    window: StatsWindow,
    now: DateTime<Utc>,
) -> Arc<RealizedStats> {
    let since = window.since(now);
    let (Ok(wallet), Some(fingerprint)) = (
        crate::utils::get_wallet_address(),
        closed_fingerprint(book, since).await,
    ) else {
        return Arc::new(RealizedStats::of(window, Vec::new(), now));
    };
    let key = StatsKey {
        book,
        wallet,
        window,
        day: now.date_naive(),
    };
    if let Some(realized) = cache.get(&key, &fingerprint) {
        return realized;
    }
    let Some(closed) = closed_since(book, since).await else {
        return Arc::new(RealizedStats::of(window, Vec::new(), now));
    };
    let realized = Arc::new(RealizedStats::of(window, closed, now));
    cache.insert(key, fingerprint, Arc::clone(&realized));
    realized
}

/// The fingerprint of the rounds of `book` closed since `since`; `None` when its store is
/// not open or cannot be read.
async fn closed_fingerprint(book: Book, since: DateTime<Utc>) -> Option<ClosedFingerprint> {
    match book {
        Book::Live => {
            let db_arc = positions::db::get_positions_database().await.ok()?;
            let db_guard = db_arc.lock().await;
            db_guard
                .as_ref()?
                .closed_fingerprint_since(since)
                .await
                .ok()
        }
        Book::Paper => positions::paper::book()?
            .closed_fingerprint_since(since)
            .await
            .ok(),
    }
}

/// The rounds of `book` closed since `since`, newest first; `None` when its store is not
/// open or cannot be read.
async fn closed_since(book: Book, since: DateTime<Utc>) -> Option<Vec<Position>> {
    match book {
        Book::Live => {
            let db_arc = positions::db::get_positions_database().await.ok()?;
            let db_guard = db_arc.lock().await;
            db_guard
                .as_ref()?
                .get_closed_positions_since(since)
                .await
                .ok()
        }
        Book::Paper => positions::paper::book()?.closed_since(since).await.ok(),
    }
}

/// What one cached realized part is of. The day is part of the key because the P&L
/// series and a day window's start move with the UTC date even when no round closes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StatsKey {
    book: Book,
    wallet: String,
    window: StatsWindow,
    day: NaiveDate,
}

/// The realized parts last computed, each with the fingerprint of the closed rounds it
/// was computed from.
#[derive(Default)]
struct RealizedCache {
    entries: Mutex<HashMap<StatsKey, (ClosedFingerprint, Arc<RealizedStats>)>>,
}

impl RealizedCache {
    /// The cached part of `key`, while it was computed from rounds of `fingerprint`.
    fn get(&self, key: &StatsKey, fingerprint: &ClosedFingerprint) -> Option<Arc<RealizedStats>> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries
            .get(key)
            .filter(|(cached, _)| cached == fingerprint)
            .map(|(_, realized)| Arc::clone(realized))
    }

    /// Cache `realized` for `key`, dropping every entry of an earlier day.
    fn insert(&self, key: StatsKey, fingerprint: ClosedFingerprint, realized: Arc<RealizedStats>) {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.retain(|cached, _| cached.day == key.day);
        entries.insert(key, (fingerprint, realized));
    }
}

static REALIZED_CACHE: LazyLock<RealizedCache> = LazyLock::new(RealizedCache::default);

/// True for a round a paper run reset closed at its mark: no exit decision closed it, so
/// it is not a trade.
fn is_paper_reset_close(position: &Position) -> bool {
    position.closed_reason.as_deref()
        == Some(format!("{:?}", crate::trader::types::TradeReason::PaperReset).as_str())
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
    /// `None` while no round is counted.
    pub win_rate_pct: Option<f64>,
}

impl RoundTotals {
    /// The totals of the `counted` rounds, with `excluded_untrusted` left out before.
    ///
    /// Realized P&L is the `pnl` each round booked at close, fee-aware and DCA-aware.
    /// Deriving it as `native_received - entry_size_native` counted every DCA add as pure
    /// profit, because `entry_size_native` is only the first buy and never grows.
    fn of(counted: &[&Position], excluded_untrusted: usize) -> Self {
        let closed = counted.len();
        let winners = counted
            .iter()
            .filter(|p| p.pnl_percent.unwrap_or_default() > 0.0)
            .count();
        let losers = counted
            .iter()
            .filter(|p| p.pnl_percent.unwrap_or_default() < 0.0)
            .count();
        Self {
            closed,
            winners,
            losers,
            excluded_untrusted,
            realized_native: counted.iter().filter_map(|p| p.pnl).sum(),
            win_rate_pct: (closed > 0).then(|| (winners as f64 / closed as f64) * 100.0),
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

impl RealizedStats {
    /// The realized figures of `window` at `now` from the rounds it closed
    /// (`recent_closed`, newest first). A round a paper run reset closed is not a trade
    /// and is left out of every figure. A day window starts `n` days before `now`; the
    /// whole history starts at the book's first closed round.
    fn of(window: StatsWindow, recent_closed: Vec<Position>, now: DateTime<Utc>) -> Self {
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
        let mut hold_hours: Vec<f64> = closed
            .iter()
            .filter_map(|p| {
                p.exit_time
                    .map(|exit| (exit - p.entry_time).num_seconds() as f64 / 3600.0)
            })
            .collect();
        let avg_hold_time_hours = (!hold_hours.is_empty())
            .then(|| hold_hours.iter().sum::<f64>() / hold_hours.len() as f64);
        hold_hours.sort_by(f64::total_cmp);
        let median_hold_time_hours = (!hold_hours.is_empty()).then(|| {
            let mid = hold_hours.len() / 2;
            if hold_hours.len() % 2 == 0 {
                (hold_hours[mid - 1] + hold_hours[mid]) / 2.0
            } else {
                hold_hours[mid]
            }
        });

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
        let gross_profit_native: f64 = closed
            .iter()
            .filter_map(|p| p.pnl)
            .filter(|v| *v > 0.0)
            .sum();
        let gross_loss_native: f64 = closed
            .iter()
            .filter_map(|p| p.pnl)
            .filter(|v| *v < 0.0)
            .map(f64::abs)
            .sum();
        // Profit factor is undefined without a loss to divide by — a losing streak with
        // no wins is 0.0, but a clean run with no losses is "no answer yet", not infinity.
        let profit_factor =
            (gross_loss_native > 0.0).then(|| gross_profit_native / gross_loss_native);
        let expectancy_native = (total_trades > 0).then(|| total_pnl_native / total_trades as f64);

        let avg_win_pct = {
            let wins: Vec<f64> = closed
                .iter()
                .filter_map(|p| p.pnl_percent)
                .filter(|v| *v > 0.0)
                .collect();
            (!wins.is_empty()).then(|| wins.iter().sum::<f64>() / wins.len() as f64)
        };
        let avg_loss_pct = {
            let losses: Vec<f64> = closed
                .iter()
                .filter_map(|p| p.pnl_percent)
                .filter(|v| *v < 0.0)
                .collect();
            (!losses.is_empty()).then(|| losses.iter().sum::<f64>() / losses.len() as f64)
        };

        // The P&L buckets and the drawdown share one pass over the window in exit order.
        // `get_closed_positions_since` returns newest first, so walk it in reverse.
        let today = now.date_naive();
        let first_day = window_start.map(|start| start.date_naive());
        let bucket = match first_day {
            Some(first) if (today - first).num_days() + 1 > DAILY_SERIES_MAX_DAYS => {
                PnlBucket::Week
            }
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

        // Emit every bucket of the window, including the empty ones, so the curve keeps a
        // true time axis instead of compressing quiet stretches away.
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

        // Exit breakdown from closed_reason, carrying the SOL each reason actually
        // returned — a reason can be the most frequent exit and still be the one losing
        // the money.
        let mut exit_stats: HashMap<String, (usize, Vec<f64>, f64)> = HashMap::new();
        for pos in &closed {
            let exit_type = pos
                .closed_reason
                .clone()
                .unwrap_or_else(|| "unknown".to_owned());
            let entry = exit_stats.entry(exit_type).or_insert((0, Vec::new(), 0.0));
            entry.0 += 1;
            if let Some(pnl_pct) = pos.pnl_percent {
                entry.1.push(pnl_pct);
            }
            entry.2 += pos.pnl.unwrap_or_default();
        }

        let mut exit_breakdown: Vec<ExitBreakdown> = exit_stats
            .into_iter()
            .map(
                |(exit_type, (count, profits, net_pnl_native))| ExitBreakdown {
                    exit_type,
                    count,
                    avg_profit_pct: if profits.is_empty() {
                        0.0
                    } else {
                        profits.iter().sum::<f64>() / profits.len() as f64
                    },
                    net_pnl_native,
                },
            )
            .collect();
        exit_breakdown.sort_by(|a, b| b.count.cmp(&a.count));

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
            win_rate_pct,
            avg_win_pct,
            avg_loss_pct,
            avg_hold_time_hours,
            median_hold_time_hours,
            best_trade_pct,
            best_trade_token,
            worst_trade_pct,
            worst_trade_token,
            daily_pnl,
            exit_breakdown,
        }
    }

    /// These figures with the book's current `open_positions` and its slot limit.
    fn with_exposure(&self, open_positions: &[Position], max_open_positions: usize) -> TraderStats {
        TraderStats {
            open_positions_count: open_positions.len(),
            max_open_positions,
            locked_native: open_positions.iter().map(|p| p.total_size_native).sum(),
            realized: self.clone(),
        }
    }
}

/// The first day of the `bucket` holding `day`: the day itself, or the Monday of its ISO
/// week.
fn bucket_start(bucket: PnlBucket, day: NaiveDate) -> NaiveDate {
    match bucket {
        PnlBucket::Day => day,
        PnlBucket::Week => day - Duration::days(i64::from(day.weekday().num_days_from_monday())),
    }
}

#[cfg(test)]
#[path = "stats_tests.rs"]
mod tests;
