//! Portfolio calendar route — per-day realized P&L and end-of-day portfolio value.

use axum::{extract::Query, response::Json};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use crate::positions;
use crate::webserver::demo;

use super::types::*;

/// Query parameters for the portfolio calendar (both optional; default = current UTC month).
#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

/// GET /api/dashboard/portfolio-calendar?year=&month=
/// Returns a month grid of realized daily P&L and end-of-day portfolio value.
pub async fn get_portfolio_calendar(
    Query(params): Query<CalendarQuery>,
) -> Json<PortfolioCalendarResponse> {
    let now = Utc::now();
    let year = params.year.unwrap_or_else(|| now.year());
    let month = params
        .month
        .filter(|m| (1..=12).contains(m))
        .unwrap_or_else(|| now.month());

    let days_in_month = days_in_month(year, month);
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| now.date_naive());
    // Sunday=0 .. Saturday=6
    let first_weekday = first_day.weekday().num_days_from_sunday();

    if demo::is_demo_mode() {
        return Json(demo_calendar(year, month, days_in_month, first_weekday));
    }

    // Month bounds [start, end) in UTC.
    let start = Utc.from_utc_datetime(&first_day.and_hms_opt(0, 0, 0).unwrap_or_default());
    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap_or(first_day);
    let end = Utc.from_utc_datetime(&next_month_first.and_hms_opt(0, 0, 0).unwrap());

    let (stats_result, balances_result) = tokio::join!(
        positions::get_daily_trading_stats(start, end),
        crate::wallet::get_daily_end_balances(start, end),
    );

    // Index the results by YYYY-MM-DD for fast per-day lookup.
    let stats_by_day: HashMap<String, positions::DailyTradingStats> = stats_result
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.date.clone(), s))
        .collect();
    let balances_by_day: HashMap<String, f64> =
        balances_result.unwrap_or_default().into_iter().collect();

    let mut days = Vec::with_capacity(days_in_month as usize);
    let mut month_net_pnl_sol = 0.0f64;
    let mut month_trades = 0i64;

    for day in 1..=days_in_month {
        let date = format!("{year:04}-{month:02}-{day:02}");
        let stats = stats_by_day.get(&date);
        let net_pnl_sol = stats.map(|s| s.net_pnl_sol).unwrap_or(0.0);
        let profit_sol = stats.map(|s| s.profit_sol).unwrap_or(0.0);
        let loss_sol = stats.map(|s| s.loss_sol).unwrap_or(0.0);
        let trades = stats.map(|s| s.trades).unwrap_or(0);
        let wins = stats.map(|s| s.wins).unwrap_or(0);
        let portfolio_value_sol = balances_by_day.get(&date).copied();

        month_net_pnl_sol += net_pnl_sol;
        month_trades += trades;

        days.push(CalendarDay {
            day,
            date,
            net_pnl_sol,
            profit_sol,
            loss_sol,
            trades,
            wins,
            portfolio_value_sol,
            has_data: trades > 0 || net_pnl_sol != 0.0,
        });
    }

    Json(PortfolioCalendarResponse {
        year,
        month,
        first_weekday,
        days_in_month,
        days,
        month_net_pnl_sol,
        month_trades,
    })
}

/// Number of days in a given month/year.
fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let this = NaiveDate::from_ymd_opt(year, month, 1);
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1);
    match (this, next) {
        (Some(a), Some(b)) => (b - a).num_days() as u32,
        _ => 30,
    }
}

/// Synthetic calendar for demo mode (never used in normal runs).
fn demo_calendar(
    year: i32,
    month: u32,
    days_in_month: u32,
    first_weekday: u32,
) -> PortfolioCalendarResponse {
    let mut days = Vec::with_capacity(days_in_month as usize);
    let mut month_net_pnl_sol = 0.0f64;
    let mut month_trades = 0i64;
    let mut balance = 10.0f64;

    for day in 1..=days_in_month {
        // Deterministic pseudo-random swing per day.
        let swing = ((day as f64 * 7.13).sin()) * 0.5;
        let trades = ((day % 5) as i64).max(0);
        balance += swing;
        month_net_pnl_sol += swing;
        month_trades += trades;
        let wins = if swing >= 0.0 { trades } else { trades / 2 };
        days.push(CalendarDay {
            day,
            date: format!("{year:04}-{month:02}-{day:02}"),
            net_pnl_sol: swing,
            profit_sol: swing.max(0.0),
            loss_sol: (-swing).max(0.0),
            trades,
            wins,
            portfolio_value_sol: Some(balance),
            has_data: trades > 0,
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
