//! Promo generator for the trader statistics tab.

use std::collections::HashMap;

use chrono::Utc;

use crate::webserver::routes::trader::types::{DailyPnlPoint, ExitBreakdown, TraderStatsResponse};

use super::aggregates;

/// Promo window, matching the Stats tab's default selection.
const PROMO_PERIOD_DAYS: u32 = 30;

/// Generate promo trader stats response, fully derived from the closed/open arrays.
pub fn get_promo_trader_stats() -> TraderStatsResponse {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let mut trades = aggregates::closed_trades(now);
    let realized = aggregates::period_over(trades.iter());
    let (best, worst) = aggregates::best_worst(&trades);

    let mut exit_pnl: HashMap<String, f64> = HashMap::new();
    for t in &trades {
        *exit_pnl.entry(t.reason.to_owned()).or_insert(0.0) += t.pnl_sol;
    }

    let exit_breakdown = aggregates::reason_breakdown(&trades)
        .into_iter()
        .map(|r| ExitBreakdown {
            net_pnl_sol: exit_pnl.get(&r.reason).copied().unwrap_or_default(),
            exit_type: r.reason,
            count: r.count,
            avg_profit_pct: r.avg_profit_pct,
        })
        .collect();

    // Walk exits oldest-first so the equity curve and the daily buckets come from the
    // same pass the live handler uses.
    trades.sort_by_key(|t| t.exit_time);
    let mut per_day: HashMap<String, (f64, usize)> = HashMap::new();
    let mut equity = 0.0f64;
    let mut peak = 0.0f64;
    let mut max_drawdown_sol = 0.0f64;
    for t in &trades {
        equity += t.pnl_sol;
        peak = peak.max(equity);
        max_drawdown_sol = max_drawdown_sol.max(peak - equity);
        let entry = per_day
            .entry(t.exit_time.format("%Y-%m-%d").to_string())
            .or_insert((0.0, 0));
        entry.0 += t.pnl_sol;
        entry.1 += 1;
    }

    let today = now.date_naive();
    let mut daily_pnl = Vec::new();
    let mut day = (now - chrono::Duration::days(i64::from(PROMO_PERIOD_DAYS))).date_naive();
    while day <= today {
        let date = day.format("%Y-%m-%d").to_string();
        let (net_pnl_sol, count) = per_day.get(&date).copied().unwrap_or((0.0, 0));
        daily_pnl.push(DailyPnlPoint {
            date,
            net_pnl_sol,
            trades: count,
        });
        day = match day.succ_opt() {
            Some(next) => next,
            None => break,
        };
    }

    let total_trades = trades.len();
    let winners = realized.wins as usize;
    let losers = total_trades.saturating_sub(winners);

    let win_pcts: Vec<f64> = trades
        .iter()
        .filter(|t| t.pnl_percent > 0.0)
        .map(|t| t.pnl_percent)
        .collect();
    let loss_pcts: Vec<f64> = trades
        .iter()
        .filter(|t| t.pnl_percent < 0.0)
        .map(|t| t.pnl_percent)
        .collect();

    let mut holds: Vec<f64> = trades
        .iter()
        .map(|t| t.hold_minutes as f64 / 60.0)
        .collect();
    holds.sort_by(f64::total_cmp);
    let median_hold_time_hours = (!holds.is_empty()).then(|| {
        let mid = holds.len() / 2;
        if holds.len() % 2 == 0 {
            (holds[mid - 1] + holds[mid]) / 2.0
        } else {
            holds[mid]
        }
    });

    TraderStatsResponse {
        period_days: PROMO_PERIOD_DAYS,
        open_positions_count: open.count,
        max_open_positions: open.count.max(1),
        locked_sol: open.invested_sol,
        total_trades,
        winners,
        losers,
        excluded_untrusted: 0,
        // Trader stats reports REALIZED P&L (closed trades) like the live handler.
        total_pnl_sol: realized.net_pnl_sol,
        gross_profit_sol: realized.profit_sol,
        gross_loss_sol: realized.loss_sol,
        profit_factor: (realized.loss_sol > 0.0).then(|| realized.profit_sol / realized.loss_sol),
        expectancy_sol: (total_trades > 0).then(|| realized.net_pnl_sol / total_trades as f64),
        max_drawdown_sol,
        win_rate_pct: (total_trades > 0).then_some(realized.win_rate),
        avg_win_pct: (!win_pcts.is_empty())
            .then(|| win_pcts.iter().sum::<f64>() / win_pcts.len() as f64),
        avg_loss_pct: (!loss_pcts.is_empty())
            .then(|| loss_pcts.iter().sum::<f64>() / loss_pcts.len() as f64),
        avg_hold_time_hours: (total_trades > 0).then(|| aggregates::avg_hold_hours(&trades)),
        median_hold_time_hours,
        best_trade_pct: Some(best.pnl_percent),
        best_trade_token: Some(best.symbol.to_owned()),
        worst_trade_pct: Some(worst.pnl_percent),
        worst_trade_token: Some(worst.symbol.to_owned()),
        daily_pnl,
        exit_breakdown,
    }
}
