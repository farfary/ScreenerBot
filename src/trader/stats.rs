//! Realized auto-trader performance over a window, plus live exposure. One
//! aggregation over one row set, shared by the dashboard Stats tab and the agent
//! tools, so no surface can show a figure the other never computed.

use serde::Serialize;
use std::collections::HashMap;

use crate::config::with_config;
use crate::positions;

/// Realized performance over a closed window, plus the live exposure that window
/// does not cover.
///
/// Every derived figure is `Option` and is `None` when the window holds nothing to
/// derive it from. A fresh install has no win rate and no best trade, and `0.0`
/// there is a fabricated claim (a green `+0.0%` "best trade") rather than an empty
/// state, so the absence travels to the dashboard instead of a zero.
#[derive(Debug, Serialize)]
pub struct TraderStats {
    /// Length of the realized window, echoed back so the UI labels what it shows.
    pub period_days: u32,

    // Live exposure (not window-bound).
    pub open_positions_count: usize,
    pub max_open_positions: usize,
    pub locked_sol: f64,

    // Trade counts.
    pub total_trades: usize,
    pub winners: usize,
    pub losers: usize,
    /// Closed rounds excluded because their cost basis or history is incomplete, so
    /// no honest P&L exists for them. Surfaced rather than silently dropped.
    pub excluded_untrusted: usize,

    // Realized money, in SOL — the single monetary unit.
    pub total_pnl_sol: f64,
    pub gross_profit_sol: f64,
    pub gross_loss_sol: f64,
    pub profit_factor: Option<f64>,
    pub expectancy_sol: Option<f64>,
    pub max_drawdown_sol: f64,

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

    /// One entry per day in the window, oldest first, including days with no trades
    /// so the curve keeps a true time axis.
    pub daily_pnl: Vec<DailyPnlPoint>,
    pub exit_breakdown: Vec<ExitBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct DailyPnlPoint {
    /// UTC calendar day, `YYYY-MM-DD`.
    pub date: String,
    pub net_pnl_sol: f64,
    pub trades: usize,
}

#[derive(Debug, Serialize)]
pub struct ExitBreakdown {
    pub exit_type: String,
    pub count: usize,
    pub avg_profit_pct: f64,
    /// Realized SOL attributable to this exit reason.
    pub net_pnl_sol: f64,
}

/// Aggregate closed rounds in the last `period_days` (clamped to 1..=365) with
/// the currently open exposure.
pub async fn trader_stats(period_days: u32) -> TraderStats {
    // Live exposure. Not window-bound: what is at risk right now is independent of
    // how far back the realized window reaches.
    let open_positions = positions::get_open_positions().await;
    let open_positions_count = open_positions.len();
    let locked_sol: f64 = open_positions.iter().map(|p| p.total_size_sol).sum();
    let max_open_positions = with_config(|cfg| cfg.trader.max_open_positions);

    let window_start = chrono::Utc::now() - chrono::Duration::days(i64::from(period_days));
    let recent_closed = {
        let db_ref = positions::db::get_positions_database().await.ok();
        if let Some(db_arc) = db_ref {
            let db_guard = db_arc.lock().await;
            if let Some(db) = db_guard.as_ref() {
                db.get_closed_positions_since(window_start)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };

    // A round whose basis or history is incomplete has no honest P&L. Overview
    // already refuses to price those; counting them here is what let this tab
    // disagree with Home on the same wallet.
    let fetched = recent_closed.len();
    let closed: Vec<_> = recent_closed
        .into_iter()
        .filter(|p| p.has_trustworthy_pnl())
        .collect();
    let excluded_untrusted = fetched - closed.len();
    let total_trades = closed.len();

    let winners = closed
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or_default() > 0.0)
        .count();
    let losers = closed
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or_default() < 0.0)
        .count();
    let win_rate_pct = (total_trades > 0).then(|| (winners as f64 / total_trades as f64) * 100.0);

    // Hold time. The median is reported alongside the mean because one forgotten
    // bag drags the mean far away from what a typical round actually looked like.
    let mut hold_hours: Vec<f64> = closed
        .iter()
        .filter_map(|p| {
            p.exit_time
                .map(|exit| (exit - p.entry_time).num_seconds() as f64 / 3600.0)
        })
        .collect();
    let avg_hold_time_hours =
        (!hold_hours.is_empty()).then(|| hold_hours.iter().sum::<f64>() / hold_hours.len() as f64);
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

    // Realized P&L in SOL across the window.
    //
    // Uses the `pnl` the position booked at close — fee-aware and DCA-aware. Deriving it
    // as `sol_received - entry_size_sol` counted every DCA add as pure profit, because
    // `entry_size_sol` is only the FIRST buy and never grows.
    let total_pnl_sol: f64 = closed.iter().filter_map(|p| p.pnl).sum();
    let gross_profit_sol: f64 = closed
        .iter()
        .filter_map(|p| p.pnl)
        .filter(|v| *v > 0.0)
        .sum();
    let gross_loss_sol: f64 = closed
        .iter()
        .filter_map(|p| p.pnl)
        .filter(|v| *v < 0.0)
        .map(f64::abs)
        .sum();
    // Profit factor is undefined without a loss to divide by — a losing streak with
    // no wins is 0.0, but a clean run with no losses is "no answer yet", not infinity.
    let profit_factor = (gross_loss_sol > 0.0).then(|| gross_profit_sol / gross_loss_sol);
    let expectancy_sol = (total_trades > 0).then(|| total_pnl_sol / total_trades as f64);

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

    // Daily buckets and the drawdown share one pass over the window in exit order.
    // `get_closed_positions_since` returns newest first, so walk it in reverse.
    let mut per_day: HashMap<String, (f64, usize)> = HashMap::new();
    let mut equity = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut max_drawdown_sol = 0.0_f64;

    for pos in closed.iter().rev() {
        let pnl = pos.pnl.unwrap_or_default();
        equity += pnl;
        peak = peak.max(equity);
        max_drawdown_sol = max_drawdown_sol.max(peak - equity);

        if let Some(exit) = pos.exit_time {
            let entry = per_day
                .entry(exit.format("%Y-%m-%d").to_string())
                .or_insert((0.0, 0));
            entry.0 += pnl;
            entry.1 += 1;
        }
    }

    // Emit every calendar day in the window, including the empty ones, so the curve
    // keeps a true time axis instead of compressing quiet stretches away.
    let today = chrono::Utc::now().date_naive();
    let first_day = window_start.date_naive();
    let mut daily_pnl = Vec::new();
    let mut day = first_day;
    while day <= today {
        let key = day.format("%Y-%m-%d").to_string();
        let (net_pnl_sol, trades) = per_day.get(&key).copied().unwrap_or((0.0, 0));
        daily_pnl.push(DailyPnlPoint {
            date: key,
            net_pnl_sol,
            trades,
        });
        day = match day.succ_opt() {
            Some(next) => next,
            None => break,
        };
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
        .map(|(exit_type, (count, profits, net_pnl_sol))| ExitBreakdown {
            exit_type,
            count,
            avg_profit_pct: if profits.is_empty() {
                0.0
            } else {
                profits.iter().sum::<f64>() / profits.len() as f64
            },
            net_pnl_sol,
        })
        .collect();
    exit_breakdown.sort_by(|a, b| b.count.cmp(&a.count));

    TraderStats {
        period_days,
        open_positions_count,
        max_open_positions,
        locked_sol,
        total_trades,
        winners,
        losers,
        excluded_untrusted,
        total_pnl_sol,
        gross_profit_sol,
        gross_loss_sol,
        profit_factor,
        expectancy_sol,
        max_drawdown_sol,
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
