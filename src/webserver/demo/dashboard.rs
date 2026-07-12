//! Demo generators for the dashboard home, overview and portfolio calendar.

use std::collections::HashMap;

use chrono::{NaiveDate, Utc};

use crate::webserver::routes::dashboard::{
    BlacklistInfo, CalendarDay, DashboardOverview, HomeDashboardResponse, MonitoringInfo,
    OpenPositionDetail, PortfolioCalendarResponse, PositionPerformer, PositionsSnapshot,
    PositionsSummary, RpcInfo, ServiceStatus, SystemInfo, SystemMetrics, TokenStatistics,
    TraderAnalytics, TraderStatusInfo, TradingPeriodStats, WalletAnalytics, WalletInfo,
};

use super::aggregates::{self, PeriodAgg};
use super::data::*;

/// Demo uptime shared by every "system" block (3d 7h 23m 45s).
const DEMO_UPTIME_SECS: u64 = 3 * 24 * 3600 + 7 * 3600 + 23 * 60 + 45;
const DEMO_UPTIME_STR: &str = "3d 7h 23m 45s";

/// Build a `TradingPeriodStats` from a realized-P&L bucket plus buys that
/// occurred in the same window (round-trip sells + still-open entries).
fn period_stats(agg: &PeriodAgg, extra_open_buys: i64) -> TradingPeriodStats {
    TradingPeriodStats {
        buys: agg.sells + extra_open_buys,
        sells: agg.sells,
        profit_sol: agg.profit_sol,
        loss_sol: agg.loss_sol,
        net_pnl_sol: agg.net_pnl_sol,
        drawdown_percent: agg.drawdown_percent,
        win_rate: agg.win_rate,
    }
}

/// Generate demo home dashboard response.
pub fn get_demo_home_dashboard() -> HomeDashboardResponse {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let trades = aggregates::closed_trades(now);
    let open_buys = open.count as i64;

    // All open positions were entered in the last few hours, so their buys land in
    // every period that includes "now" (today/week/month/all_time) but not yesterday.
    let trader = TraderAnalytics {
        today: period_stats(&aggregates::within_hours(&trades, now, 24), open_buys),
        yesterday: period_stats(&aggregates::between_hours(&trades, now, 24, 48), 0),
        this_week: period_stats(&aggregates::within_hours(&trades, now, 24 * 7), open_buys),
        this_month: period_stats(&aggregates::within_hours(&trades, now, 24 * 30), open_buys),
        all_time: period_stats(&aggregates::period_over(trades.iter()), open_buys),
    };

    let demo_equity = DEMO_SOL_BALANCE + open.current_value_sol;
    let wallet = WalletAnalytics {
        current_balance_sol: DEMO_SOL_BALANCE,
        token_count: open.count,
        tokens_worth_sol: open.current_value_sol,
        total_equity_sol: demo_equity,
        unpriced_token_count: 0,
        start_of_day_balance_sol: DEMO_START_BALANCE,
        change_sol: demo_equity - DEMO_START_BALANCE,
        change_percent: (demo_equity - DEMO_START_BALANCE) / DEMO_START_BALANCE * 100.0,
        sol_price_usd: 180.0,
        balance_history: vec![
            DEMO_START_BALANCE,
            DEMO_START_BALANCE * 1.01,
            DEMO_START_BALANCE * 0.99,
            DEMO_START_BALANCE * 1.03,
            demo_equity,
        ],
    };

    let positions = PositionsSnapshot {
        open_count: open.count as i64,
        total_invested_sol: open.invested_sol,
        unrealized_pnl_sol: open.unrealized_pnl_sol,
        unrealized_pnl_percent: open.unrealized_pnl_percent,
        avg_position_size_sol: if open.count > 0 {
            open.invested_sol / open.count as f64
        } else {
            0.0
        },
        avg_hold_duration_mins: open.avg_hold_minutes,
        best_performer: Some(PositionPerformer {
            symbol: open.best.symbol.to_owned(),
            pnl_percent: open.best.pnl_percent,
        }),
        worst_performer: Some(PositionPerformer {
            symbol: open.worst.symbol.to_owned(),
            pnl_percent: open.worst.pnl_percent,
        }),
        dca_count: 1,
    };

    let system = SystemMetrics {
        uptime_seconds: DEMO_UPTIME_SECS,
        uptime_formatted: DEMO_UPTIME_STR.to_owned(),
        memory_mb: DEMO_MEMORY_MB,
        memory_percent: 2.4,
        cpu_percent: DEMO_CPU_PERCENT,
        rpc_calls_per_min: 847.3,
        rpc_success_rate: 99.7,
        websocket_connected: true,
        services_healthy: 12,
        services_total: 12,
    };

    let tokens = TokenStatistics {
        total_in_database: 12847,
        with_prices: 8923,
        passed_filters: 347,
        rejected_filters: 8576,
        blacklisted: DEMO_BLACKLISTED,
        with_ohlcv: 2847,
        found_today: 234,
        found_this_week: 1523,
        found_this_month: 4892,
        found_all_time: 12847,
    };

    HomeDashboardResponse {
        trader,
        wallet,
        positions,
        system,
        tokens,
        trader_status: TraderStatusInfo { running: true },
        timestamp: now.to_rfc3339(),
    }
}

/// Generate demo dashboard overview response.
pub fn get_demo_dashboard_overview() -> DashboardOverview {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let trades = aggregates::closed_trades(now);
    let realized = aggregates::period_over(trades.iter());

    let wallet = WalletInfo {
        sol_balance: DEMO_SOL_BALANCE,
        sol_balance_lamports: DEMO_SOL_LAMPORTS,
        total_tokens_count: open.count,
        last_updated: Some(now.to_rfc3339()),
    };

    let open_position_details: Vec<OpenPositionDetail> = DEMO_OPEN_TOKENS
        .iter()
        .map(
            |(symbol, _name, mint, _logo, entry, current, _size, hold_min)| OpenPositionDetail {
                mint: mint.to_string(),
                symbol: symbol.to_string(),
                entry_price: *entry,
                current_price: Some(*current),
                pnl_percent: Some((current - entry) / entry * 100.0),
                hold_duration_minutes: *hold_min,
            },
        )
        .collect();

    let positions = PositionsSummary {
        total_positions: (open.count + trades.len()) as i64,
        open_positions: open.count as i64,
        closed_positions: trades.len() as i64,
        total_invested_sol: open.invested_sol,
        total_pnl: realized.net_pnl_sol + open.unrealized_pnl_sol,
        win_rate: realized.win_rate,
        open_position_details,
    };

    let system = SystemInfo {
        all_services_ready: true,
        services: ServiceStatus {
            tokens_system: true,
            positions_system: true,
            pool_service: true,
            transactions_system: true,
        },
        uptime_seconds: DEMO_UPTIME_SECS,
        uptime_formatted: DEMO_UPTIME_STR.to_owned(),
        memory_mb: DEMO_MEMORY_MB,
        cpu_percent: DEMO_CPU_PERCENT,
        active_threads: 24,
    };

    let rpc = RpcInfo {
        total_calls: 847_234,
        calls_per_second: 4.7,
        uptime_seconds: DEMO_UPTIME_SECS,
    };

    let mut by_reason = HashMap::new();
    by_reason.insert("Manual".to_owned(), 47);
    by_reason.insert("MintAuthority".to_owned(), 523);
    by_reason.insert("FreezeAuthority".to_owned(), 412);
    by_reason.insert("NonAuthority::RugPull".to_owned(), 271);

    let blacklist = BlacklistInfo {
        total_blacklisted: DEMO_BLACKLISTED,
        by_reason,
    };

    let monitoring = MonitoringInfo {
        tokens_tracked: DEMO_TOKENS_TRACKED,
        entry_check_interval_secs: 10,
        position_monitor_interval_secs: 5,
    };

    DashboardOverview {
        wallet,
        positions,
        system,
        rpc,
        blacklist,
        monitoring,
        timestamp: now.to_rfc3339(),
    }
}

/// Generate the demo portfolio calendar for a given month. Real closed trades are
/// placed on their actual exit days; the remaining days get a gentle deterministic
/// baseline so the month looks lived-in (as a real active month would).
pub fn get_demo_portfolio_calendar(
    year: i32,
    month: u32,
    days_in_month: u32,
    first_weekday: u32,
) -> PortfolioCalendarResponse {
    let now = Utc::now();
    let trades = aggregates::closed_trades(now);

    // Bucket real closed trades onto their exit date (YYYY-MM-DD).
    let mut real: HashMap<String, (f64, f64, f64, i64, i64)> = HashMap::new(); // net, profit, loss, trades, wins
    for t in &trades {
        let key = t.exit_time.format("%Y-%m-%d").to_string();
        let e = real.entry(key).or_insert((0.0, 0.0, 0.0, 0, 0));
        e.0 += t.pnl_sol;
        if t.pnl_sol >= 0.0 {
            e.1 += t.pnl_sol;
            e.4 += 1;
        } else {
            e.2 += -t.pnl_sol;
        }
        e.3 += 1;
    }

    let today = now.date_naive();
    let mut days = Vec::with_capacity(days_in_month as usize);
    let mut month_net_pnl_sol = 0.0f64;
    let mut month_trades = 0i64;
    let mut balance = DEMO_START_BALANCE;

    for day in 1..=days_in_month {
        let date = format!("{year:04}-{month:02}-{day:02}");
        let is_future = NaiveDate::from_ymd_opt(year, month, day).is_some_and(|d| d > today);

        let (net, profit, loss, trades_n, wins) = if let Some(r) = real.get(&date) {
            *r
        } else if is_future {
            (0.0, 0.0, 0.0, 0, 0)
        } else {
            // Deterministic per-day baseline: mostly-green active trading.
            let swing = (day as f64 * 1.37).sin() * 0.18 + 0.04;
            let n = (2 + (day % 4)) as i64;
            let w = if swing >= 0.0 { (n * 2 + 2) / 3 } else { n / 3 };
            (swing, swing.max(0.0), (-swing).max(0.0), n, w.min(n))
        };

        balance += net;
        month_net_pnl_sol += net;
        month_trades += trades_n;

        days.push(CalendarDay {
            day,
            date,
            net_pnl_sol: net,
            profit_sol: profit,
            loss_sol: loss,
            trades: trades_n,
            wins,
            portfolio_value_sol: if is_future { None } else { Some(balance) },
            has_data: trades_n > 0 || net != 0.0,
        });
    }

    PortfolioCalendarResponse {
        year,
        month,
        first_weekday,
        days_in_month,
        days,
        month_net_pnl_sol,
        month_trades,
    }
}
