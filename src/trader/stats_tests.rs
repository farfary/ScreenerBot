// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The stats aggregation of one book: its figures come only from the rounds it is
//! given, an empty window derives nothing, a round a paper run reset closed is not a
//! trade, the whole history starts at the first close and turns weekly past the daily
//! span, and a cached realized part lives only as long as its fingerprint.

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

/// A round of `book` closed at `exit`, an hour after it opened.
fn closed_at(book: Book, exit: DateTime<Utc>, pnl: f64) -> Position {
    let mut position = Position::opening(Opening {
        mint: format!("Mint{}", exit.timestamp()),
        symbol: format!("T{}", exit.timestamp()),
        name: "Token".to_owned(),
        entry_price: 0.001,
        entry_time: exit - Duration::hours(1),
        size_native: 0.1,
        entry_signature: None,
        origin: PositionOrigin::Auto { strategy_id: None },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book,
    });
    position.total_size_native = 0.1;
    position.exit_time = Some(exit);
    position.pnl = Some(pnl);
    position.pnl_percent = Some(pnl / 0.1 * 100.0);
    position.closed_reason = Some("TakeProfit".to_owned());
    position
}

/// A fixed instant, so day and week boundaries do not depend on when the test runs.
fn fixed_now() -> DateTime<Utc> {
    "2026-10-28T12:00:00+00:00".parse().unwrap()
}

/// The stats of `window` over `closed` at `now`, with no open exposure.
fn stats_of(window: StatsWindow, closed: Vec<Position>, now: DateTime<Utc>) -> TraderStats {
    RealizedStats::of(window, closed, now).with_exposure(&[], 5)
}

#[test]
fn an_empty_day_window_derives_nothing_but_keeps_its_days() {
    let stats = stats_of(StatsWindow::Days(7), Vec::new(), Utc::now());
    let realized = &stats.realized;
    assert_eq!(realized.period_days, Some(7));
    assert_eq!(realized.total_trades, 0);
    assert_eq!(stats.open_positions_count, 0);
    assert_eq!(realized.win_rate_pct, None);
    assert_eq!(realized.best_trade_pct, None);
    assert_eq!(realized.profit_factor, None);
    assert_eq!(realized.daily_pnl.len(), 8);
    assert!(realized
        .daily_pnl
        .iter()
        .all(|point| point.bucket == PnlBucket::Day && point.trades == 0));
}

#[test]
fn the_whole_history_of_an_empty_book_has_no_window_and_no_points() {
    let stats = stats_of(StatsWindow::All, Vec::new(), fixed_now());
    let realized = &stats.realized;
    assert_eq!(realized.period_days, None);
    assert!(realized.daily_pnl.is_empty());
    assert_eq!(realized.total_trades, 0);
    assert_eq!(realized.win_rate_pct, None);
    assert!(realized.exit_breakdown.is_empty());
}

#[test]
fn the_whole_history_starts_at_the_first_closed_round() {
    let now = fixed_now();
    let first = now - Duration::days(10);
    let closed = vec![
        closed_at(Book::Live, now - Duration::days(2), 0.02),
        closed_at(Book::Live, first, -0.01),
    ];
    let stats = stats_of(StatsWindow::All, closed, now);
    let realized = &stats.realized;
    assert_eq!(realized.period_days, None);
    assert_eq!(realized.total_trades, 2);
    let dates: Vec<&str> = realized.daily_pnl.iter().map(|p| p.date.as_str()).collect();
    assert_eq!(dates.first().copied(), Some("2026-10-18"));
    assert_eq!(dates.last().copied(), Some("2026-10-28"));
    assert_eq!(dates.len(), 11);
    assert!(realized
        .daily_pnl
        .iter()
        .all(|point| point.bucket == PnlBucket::Day));
    assert_eq!(realized.daily_pnl[0].trades, 1);
    assert!((realized.daily_pnl[0].net_pnl_native + 0.01).abs() < 1e-12);
}

#[test]
fn a_history_of_ninety_days_stays_daily() {
    let now = fixed_now();
    let closed = vec![closed_at(Book::Live, now - Duration::days(89), 0.01)];
    let realized = RealizedStats::of(StatsWindow::All, closed, now);
    assert_eq!(realized.daily_pnl.len(), 90);
    assert!(realized
        .daily_pnl
        .iter()
        .all(|point| point.bucket == PnlBucket::Day));
}

#[test]
fn a_history_beyond_ninety_days_is_one_point_per_iso_week() {
    let now = fixed_now();
    let first_exit = now - Duration::days(100);
    let closed = vec![
        closed_at(Book::Live, now - Duration::hours(1), 0.03),
        closed_at(Book::Live, now - Duration::days(40), -0.02),
        closed_at(Book::Live, first_exit, 0.01),
    ];
    let realized = RealizedStats::of(StatsWindow::All, closed, now);
    let starts: Vec<NaiveDate> = realized
        .daily_pnl
        .iter()
        .map(|point| NaiveDate::parse_from_str(&point.date, "%Y-%m-%d").unwrap())
        .collect();

    assert!(realized
        .daily_pnl
        .iter()
        .all(|point| point.bucket == PnlBucket::Week));
    assert!(starts
        .iter()
        .all(|start| start.weekday() == chrono::Weekday::Mon));
    assert!(starts
        .windows(2)
        .all(|pair| pair[1] - pair[0] == Duration::weeks(1)));
    let first_day = first_exit.date_naive();
    assert!(starts[0] <= first_day && first_day - starts[0] < Duration::weeks(1));
    let today = now.date_naive();
    let last = *starts.last().unwrap();
    assert!(last <= today && today - last < Duration::weeks(1));
    assert_eq!(
        realized.daily_pnl.iter().map(|p| p.trades).sum::<usize>(),
        3
    );
    let net: f64 = realized.daily_pnl.iter().map(|p| p.net_pnl_native).sum();
    assert!((net - realized.total_pnl_native).abs() < 1e-12);
}

#[test]
fn paper_stats_come_from_the_paper_rounds_alone() {
    let open = [round(Book::Paper, 2, None, "")];
    let closed = vec![
        round(Book::Paper, 3, Some(0.02), "TakeProfit"),
        round(Book::Paper, 4, Some(-0.01), "StopLoss"),
    ];
    let stats = RealizedStats::of(StatsWindow::Days(7), closed, Utc::now()).with_exposure(&open, 5);
    assert_eq!(stats.open_positions_count, 1);
    assert!((stats.locked_native - 0.1).abs() < 1e-12);
    let stats = stats.realized;
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
    let stats = RealizedStats::of(StatsWindow::Days(7), closed, Utc::now());
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

/// A recorded set holding every case the counting rule decides: winners, a loser, a
/// break-even round, a round with an incomplete basis and a round a paper run reset
/// closed.
fn recorded_rounds() -> Vec<Position> {
    let reset = format!("{:?}", crate::trader::types::TradeReason::PaperReset);
    let mut untrusted = round(Book::Live, 6, Some(0.5), "TakeProfit");
    untrusted.basis_complete = false;
    vec![
        round(Book::Live, 2, Some(0.03), "TakeProfit"),
        round(Book::Live, 3, Some(-0.01), "StopLoss"),
        round(Book::Live, 4, Some(0.0), "TimeExit"),
        round(Book::Live, 5, Some(0.02), "StrategyExit"),
        untrusted,
        round(Book::Live, 7, Some(-0.04), &reset),
    ]
}

#[test]
fn round_totals_count_closed_rounds_as_the_stats_tab_counts_them() {
    let rounds = recorded_rounds();
    let totals = round_totals(&rounds);
    // The figures the Stats aggregation produced for this set before the counting rule
    // was shared.
    assert_eq!(totals.closed, 4);
    assert_eq!(totals.winners, 2);
    assert_eq!(totals.losers, 1);
    assert_eq!(totals.excluded_untrusted, 1);
    assert!((totals.realized_native - 0.04).abs() < 1e-12);
    assert_eq!(totals.win_rate_pct, Some(50.0));

    let stats = RealizedStats::of(StatsWindow::Days(7), rounds, Utc::now());
    assert_eq!(stats.total_trades, totals.closed);
    assert_eq!(stats.winners, totals.winners);
    assert_eq!(stats.losers, totals.losers);
    assert_eq!(stats.excluded_untrusted, totals.excluded_untrusted);
    assert_eq!(stats.total_pnl_native, totals.realized_native);
    assert_eq!(stats.win_rate_pct, totals.win_rate_pct);
}

#[test]
fn round_totals_of_no_counted_round_have_no_win_rate() {
    let reset = format!("{:?}", crate::trader::types::TradeReason::PaperReset);
    let totals = round_totals(&[round(Book::Paper, 2, Some(0.01), &reset)]);
    assert_eq!(totals.closed, 0);
    assert_eq!(totals.excluded_untrusted, 0);
    assert_eq!(totals.win_rate_pct, None);
    assert_eq!(round_totals(&[]).win_rate_pct, None);
}

fn key(book: Book, day: NaiveDate) -> StatsKey {
    StatsKey {
        book,
        wallet: "wallet-a".to_owned(),
        window: StatsWindow::Days(30),
        day,
    }
}

fn fingerprint(count: i64, total_pnl: f64) -> ClosedFingerprint {
    ClosedFingerprint {
        count,
        first_exit: Some("2026-10-20T10:00:00+00:00".to_owned()),
        last_exit: Some("2026-10-27T10:00:00+00:00".to_owned()),
        last_update: Some("2026-10-27 10:00:01".to_owned()),
        total_pnl,
    }
}

#[test]
fn round_totals_and_the_cached_realized_part_agree_on_one_recorded_set() {
    let rounds = recorded_rounds();
    let totals = round_totals(&rounds);
    let cache = RealizedCache::default();
    let day = Utc::now().date_naive();
    let realized = Arc::new(RealizedStats::of(StatsWindow::Days(30), rounds, Utc::now()));
    cache.insert(key(Book::Live, day), fingerprint(6, 0.0), realized);

    let cached = cache
        .get(&key(Book::Live, day), &fingerprint(6, 0.0))
        .expect("an unchanged fingerprint is served from the cache");
    assert_eq!(cached.total_trades, totals.closed);
    assert_eq!(cached.winners, totals.winners);
    assert_eq!(cached.losers, totals.losers);
    assert_eq!(cached.excluded_untrusted, totals.excluded_untrusted);
    assert_eq!(cached.total_pnl_native, totals.realized_native);
    assert_eq!(cached.win_rate_pct, totals.win_rate_pct);
}

#[test]
fn the_realized_part_is_reused_until_its_fingerprint_moves() {
    let cache = RealizedCache::default();
    let day = fixed_now().date_naive();
    let base = fingerprint(2, 0.01);
    let realized = Arc::new(RealizedStats::of(
        StatsWindow::Days(30),
        vec![closed_at(Book::Live, fixed_now() - Duration::days(1), 0.01)],
        fixed_now(),
    ));
    cache.insert(key(Book::Live, day), base.clone(), Arc::clone(&realized));

    let hit = cache.get(&key(Book::Live, day), &base).unwrap();
    assert!(
        Arc::ptr_eq(&hit, &realized),
        "an unchanged book reuses the same part"
    );

    let after_close = fingerprint(3, 0.01);
    assert!(cache.get(&key(Book::Live, day), &after_close).is_none());
    let after_rewrite = fingerprint(2, 0.04);
    assert!(cache.get(&key(Book::Live, day), &after_rewrite).is_none());
    let after_touch = ClosedFingerprint {
        last_update: Some("2026-10-28 09:00:00".to_owned()),
        ..base.clone()
    };
    assert!(cache.get(&key(Book::Live, day), &after_touch).is_none());

    let recomputed = Arc::new(RealizedStats::of(
        StatsWindow::Days(30),
        Vec::new(),
        fixed_now(),
    ));
    cache.insert(
        key(Book::Live, day),
        after_close.clone(),
        Arc::clone(&recomputed),
    );
    assert!(cache.get(&key(Book::Live, day), &base).is_none());
    assert!(Arc::ptr_eq(
        &cache.get(&key(Book::Live, day), &after_close).unwrap(),
        &recomputed
    ));
}

#[test]
fn live_and_paper_never_share_a_cached_realized_part() {
    let cache = RealizedCache::default();
    let day = fixed_now().date_naive();
    let same = fingerprint(1, 0.02);
    cache.insert(
        key(Book::Live, day),
        same.clone(),
        Arc::new(RealizedStats::of(
            StatsWindow::Days(30),
            Vec::new(),
            fixed_now(),
        )),
    );
    assert!(
        cache.get(&key(Book::Paper, day), &same).is_none(),
        "a paper poll never reads the live entry, even with an equal fingerprint"
    );
    let other_window = StatsKey {
        window: StatsWindow::All,
        ..key(Book::Live, day)
    };
    assert!(cache.get(&other_window, &same).is_none());
    let other_wallet = StatsKey {
        wallet: "wallet-b".to_owned(),
        ..key(Book::Live, day)
    };
    assert!(cache.get(&other_wallet, &same).is_none());
}

#[test]
fn a_new_day_drops_the_parts_of_earlier_days() {
    let cache = RealizedCache::default();
    let today = fixed_now().date_naive();
    let yesterday = today.pred_opt().unwrap();
    let same = fingerprint(1, 0.02);
    let part = || {
        Arc::new(RealizedStats::of(
            StatsWindow::Days(30),
            Vec::new(),
            fixed_now(),
        ))
    };
    cache.insert(key(Book::Live, yesterday), same.clone(), part());
    assert!(cache.get(&key(Book::Live, today), &same).is_none());
    cache.insert(key(Book::Paper, today), same.clone(), part());
    assert!(cache.get(&key(Book::Live, yesterday), &same).is_none());
    assert!(cache.get(&key(Book::Paper, today), &same).is_some());
}
