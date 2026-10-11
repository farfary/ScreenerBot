// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Portfolio calendar route — per-day realized P&L and end-of-day portfolio value.

use axum::{extract::Query, response::Json};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use std::collections::HashMap;

use crate::positions;
use crate::webserver::promo;

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

    if promo::are_promo_fixtures_enabled() {
        return Json(promo::get_promo_portfolio_calendar(
            year,
            month,
            days_in_month,
            first_weekday,
        ));
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

    for day in 1..=days_in_month {
        let date = format!("{year:04}-{month:02}-{day:02}");
        let stats = stats_by_day.get(&date);
        let net_pnl_native = stats.map(|s| s.net_pnl_native).unwrap_or(0.0);
        let profit_native = stats.map(|s| s.profit_native).unwrap_or(0.0);
        let loss_native = stats.map(|s| s.loss_native).unwrap_or(0.0);
        let trades = stats.map(|s| s.trades).unwrap_or(0);
        let wins = stats.map(|s| s.wins).unwrap_or(0);
        let portfolio_value_native = balances_by_day.get(&date).copied();

        days.push(CalendarDay {
            day,
            date,
            net_pnl_native,
            profit_native,
            loss_native,
            trades,
            wins,
            portfolio_value_native,
            has_data: trades > 0 || net_pnl_native != 0.0,
        });
    }

    Json(PortfolioCalendarResponse::from_days(
        year,
        month,
        first_weekday,
        days,
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn day(day: u32, net_pnl_native: f64, trades: i64) -> CalendarDay {
        CalendarDay {
            day,
            date: format!("2026-10-{day:02}"),
            net_pnl_native,
            profit_native: net_pnl_native.max(0.0),
            loss_native: (-net_pnl_native).max(0.0),
            trades,
            wins: i64::from(net_pnl_native > 0.0),
            portfolio_value_native: None,
            has_data: trades > 0 || net_pnl_native != 0.0,
        }
    }

    #[test]
    fn month_summary_is_summed_from_its_days() {
        let calendar = PortfolioCalendarResponse::from_days(
            2026,
            10,
            4,
            vec![
                day(1, 0.4, 2),
                day(2, -0.1, 1),
                day(3, 0.0, 0),
                day(4, 1.2, 3),
                day(5, -0.3, 2),
            ],
        );
        assert_eq!(calendar.days_in_month, 5);
        assert!((calendar.month_net_pnl_native - 1.2).abs() < 1e-12);
        assert_eq!(calendar.month_trades, 8);
        assert_eq!(calendar.month_win_days, 2);
        assert_eq!(calendar.month_loss_days, 2);
        let best = calendar.best_day.expect("a profitable day");
        assert_eq!(best.date, "2026-10-04");
        assert!((best.net_pnl_native - 1.2).abs() < 1e-12);
    }

    #[test]
    fn a_month_without_profit_has_no_best_day() {
        let calendar = PortfolioCalendarResponse::from_days(
            2026,
            10,
            4,
            vec![day(1, -0.2, 1), day(2, 0.0, 0)],
        );
        assert!(calendar.best_day.is_none());
        assert_eq!((calendar.month_win_days, calendar.month_loss_days), (0, 1));
    }
}
