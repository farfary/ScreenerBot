// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The stats aggregation of one book: its figures come only from the rounds it is
//! given, an empty window derives nothing, and a round a paper run reset closed is not a
//! trade.

use chrono::Duration;

use super::*;
use crate::positions::{Opening, PositionManagement, PositionOrigin};

fn round(book: Book, entry_offset_hours: i64, pnl: Option<f64>, reason: &str) -> Position {
    let now = Utc::now();
    let mut position = Position::opening(Opening {
        mint: format!("Mint{entry_offset_hours}"),
        symbol: format!("T{entry_offset_hours}"),
        name: "Token".to_owned(),
        entry_price: 0.001,
        entry_time: now - Duration::hours(entry_offset_hours),
        size_native: 0.1,
        entry_signature: None,
        origin: PositionOrigin::Auto { strategy_id: None },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book,
    });
    position.total_size_native = 0.1;
    if let Some(pnl) = pnl {
        position.exit_time = Some(now - Duration::hours(entry_offset_hours - 1));
        position.pnl = Some(pnl);
        position.pnl_percent = Some(pnl / 0.1 * 100.0);
        position.closed_reason = Some(reason.to_owned());
    }
    position
}

#[test]
fn an_empty_book_derives_nothing() {
    let stats = trader_stats_from(7, &[], Vec::new(), 5, Utc::now() - Duration::days(7));
    assert_eq!(stats.total_trades, 0);
    assert_eq!(stats.open_positions_count, 0);
    assert_eq!(stats.win_rate_pct, None);
    assert_eq!(stats.best_trade_pct, None);
    assert_eq!(stats.profit_factor, None);
    assert_eq!(stats.daily_pnl.len(), 8);
}

#[test]
fn paper_stats_come_from_the_paper_rounds_alone() {
    let open = [round(Book::Paper, 2, None, "")];
    let closed = vec![
        round(Book::Paper, 3, Some(0.02), "TakeProfit"),
        round(Book::Paper, 4, Some(-0.01), "StopLoss"),
    ];
    let stats = trader_stats_from(7, &open, closed, 5, Utc::now() - Duration::days(7));
    assert_eq!(stats.open_positions_count, 1);
    assert!((stats.locked_native - 0.1).abs() < 1e-12);
    assert_eq!(stats.total_trades, 2);
    assert_eq!(stats.winners, 1);
    assert_eq!(stats.losers, 1);
    assert!((stats.total_pnl_native - 0.01).abs() < 1e-12);
    let reasons: Vec<&str> = stats
        .exit_breakdown
        .iter()
        .map(|exit| exit.exit_type.as_str())
        .collect();
    assert!(reasons.contains(&"StopLoss"), "{reasons:?}");
    assert!(reasons.contains(&"TakeProfit"), "{reasons:?}");
}

#[test]
fn a_round_closed_by_a_paper_run_reset_is_not_a_trade() {
    let reset = format!("{:?}", crate::trader::types::TradeReason::PaperReset);
    let closed = vec![
        round(Book::Paper, 3, Some(0.02), "TakeProfit"),
        round(Book::Paper, 5, Some(-0.03), &reset),
    ];
    let stats = trader_stats_from(7, &[], closed, 5, Utc::now() - Duration::days(7));
    assert_eq!(stats.total_trades, 1);
    assert_eq!(stats.winners, 1);
    assert_eq!(stats.losers, 0);
    assert_eq!(
        stats.excluded_untrusted, 0,
        "a reset close is not an untrusted trade"
    );
    assert!((stats.total_pnl_native - 0.02).abs() < 1e-12);
    assert_eq!(stats.worst_trade_pct, stats.best_trade_pct);
    assert!(stats
        .exit_breakdown
        .iter()
        .all(|exit| exit.exit_type != reset));
    assert_eq!(
        stats.daily_pnl.iter().map(|day| day.trades).sum::<usize>(),
        1
    );
}
