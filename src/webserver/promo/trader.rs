// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Promo generators for the Auto Trader page: the trader statistics tab, the
//! running state, the monitor controls and the period loss limit panel.

use std::collections::HashMap;

use chrono::{Duration, Utc};

use crate::trader::stats::{DailyPnlPoint, ExitBreakdown, TraderStats};
use crate::trader::{LossLimitSnapshot, MonitorState, MonitorsStatus, TraderStatus};

use super::aggregates;

/// Promo window, matching the Stats tab's default selection.
const PROMO_PERIOD_DAYS: u32 = 30;

/// Configured period loss limit, in SOL.
const PROMO_LOSS_LIMIT_NATIVE: f64 = 0.5;

/// Length of one loss limit period, the daily default of the live setting.
const PROMO_LOSS_LIMIT_PERIOD_HOURS: i64 = 24;

/// How far into its period the promo session is (19h 20m), so the panel shows a
/// countdown rather than a freshly reset period.
const PROMO_LOSS_LIMIT_ELAPSED_MINUTES: i64 = 19 * 60 + 20;

/// Generate the promo trader status: available and running, the state the promo
/// header and status bar report.
pub fn get_promo_trader_status() -> TraderStatus {
    TraderStatus {
        enabled: true,
        running: true,
        available: true,
        unavailable_reason: None,
    }
}

/// Generate the promo monitor status: both monitors enabled and running under a
/// running trader.
pub fn get_promo_monitors_status() -> MonitorsStatus {
    let trader = get_promo_trader_status();
    let monitor = MonitorState {
        enabled: true,
        running: trader.running,
    };
    MonitorsStatus {
        entry_monitor: monitor.clone(),
        exit_monitor: monitor,
        master_enabled: trader.running,
        force_stopped: false,
        available: trader.available,
    }
}

/// Generate the promo period loss limit status. The period loss is the realized
/// loss of the promo closed trades that exited since the period started, counted
/// the way the live limiter reads the books, so it never contradicts the trade
/// history shown elsewhere.
pub fn get_promo_loss_limit_status() -> LossLimitSnapshot {
    let now = Utc::now();
    let period_start = now - Duration::minutes(PROMO_LOSS_LIMIT_ELAPSED_MINUTES);
    let period_end = period_start + Duration::hours(PROMO_LOSS_LIMIT_PERIOD_HOURS);
    let trades = aggregates::closed_trades(now);
    let current_loss_native =
        aggregates::period_over(trades.iter().filter(|t| t.exit_time >= period_start)).loss_sol;
    let is_limited = current_loss_native >= PROMO_LOSS_LIMIT_NATIVE;

    LossLimitSnapshot {
        enabled: true,
        limit_native: PROMO_LOSS_LIMIT_NATIVE,
        current_loss_native,
        is_limited,
        limited_at: is_limited.then_some(now),
        period_start,
        period_remaining_secs: (period_end - now).num_seconds().max(0),
        progress_percent: (current_loss_native / PROMO_LOSS_LIMIT_NATIVE * 100.0).min(100.0),
    }
}

/// Generate promo trader stats response, fully derived from the closed/open arrays.
pub fn get_promo_trader_stats() -> TraderStats {
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
            net_pnl_native: exit_pnl.get(&r.reason).copied().unwrap_or_default(),
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
            net_pnl_native: net_pnl_sol,
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

    TraderStats {
        period_days: PROMO_PERIOD_DAYS,
        open_positions_count: open.count,
        max_open_positions: open.count.max(1),
        locked_native: open.invested_sol,
        total_trades,
        winners,
        losers,
        excluded_untrusted: 0,
        // Trader stats reports REALIZED P&L (closed trades) like the live handler.
        total_pnl_native: realized.net_pnl_sol,
        gross_profit_native: realized.profit_sol,
        gross_loss_native: realized.loss_sol,
        profit_factor: (realized.loss_sol > 0.0).then(|| realized.profit_sol / realized.loss_sol),
        expectancy_native: (total_trades > 0).then(|| realized.net_pnl_sol / total_trades as f64),
        max_drawdown_native: max_drawdown_sol,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::promo::get_promo_header_metrics;
    use crate::webserver::routes::header::TraderHeaderState;

    #[test]
    fn trader_status_and_monitors_agree_with_the_header() {
        crate::config::utils::install_default_config();
        let header = get_promo_header_metrics().trader;
        let header_running = matches!(header.state, TraderHeaderState::Running);

        let status = get_promo_trader_status();
        assert!(status.available);
        assert_eq!(status.enabled, header.enabled);
        assert_eq!(status.running, header_running);
        assert!(status.running);

        let monitors = get_promo_monitors_status();
        assert!(monitors.available);
        assert!(!monitors.force_stopped);
        assert_eq!(monitors.master_enabled, status.running);
        for monitor in [&monitors.entry_monitor, &monitors.exit_monitor] {
            assert!(monitor.enabled);
            assert_eq!(monitor.running, status.running);
        }
    }

    #[test]
    fn loss_limit_is_configured_and_untripped() {
        let limit = get_promo_loss_limit_status();
        assert!(limit.enabled);
        assert!(limit.limit_native > 0.0);
        assert!(!limit.is_limited);
        assert_eq!(limit.limited_at, None);
        assert!(limit.current_loss_native >= 0.0);
        assert!(limit.current_loss_native < limit.limit_native);
        // Below the panel's warning band, so the bar never reads as a near miss.
        assert!(limit.progress_percent < 75.0);
        assert!(limit.period_remaining_secs > 0);
        assert!(limit.period_remaining_secs < PROMO_LOSS_LIMIT_PERIOD_HOURS * 3600);
    }

    #[test]
    fn loss_limit_counts_the_closed_trades_of_the_period() {
        let limit = get_promo_loss_limit_status();
        let trades = aggregates::closed_trades(Utc::now());
        let expected: f64 = trades
            .iter()
            .filter(|t| t.exit_time >= limit.period_start && t.pnl_sol < 0.0)
            .map(|t| -t.pnl_sol)
            .sum();
        assert!((limit.current_loss_native - expected).abs() < 1e-9);
    }
}
