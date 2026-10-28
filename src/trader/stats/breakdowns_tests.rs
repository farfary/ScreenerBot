// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The breakdowns of the counted rounds: exit groups merge every spelling of one reason
//! and keep a stable order, sources keep a fixed order, the return buckets cover every
//! round once, a flat round ends a streak, and the recent list is the newest counted
//! rounds.

use chrono::{DateTime, Duration, Utc};

use super::*;
use crate::positions::{Book, Opening, PositionManagement, PositionOrigin};
use crate::trader::stats::{RealizedStats, StatsWindow};

fn fixed_now() -> DateTime<Utc> {
    "2026-10-28T12:00:00+00:00".parse().unwrap()
}

/// A counted round closed `exit_hours` before [`fixed_now`], an hour after it opened, with
/// a return of `pnl_percent` on a 0.1 SOL size.
fn closed(exit_hours: i64, pnl_percent: f64, reason: Option<&str>) -> Position {
    let exit = fixed_now() - Duration::hours(exit_hours);
    let mut position = Position::opening(Opening {
        mint: format!("Mint{exit_hours}"),
        symbol: format!("T{exit_hours}"),
        name: format!("Token {exit_hours}"),
        entry_price: 0.001,
        entry_time: exit - Duration::hours(1),
        size_native: 0.1,
        entry_signature: None,
        origin: PositionOrigin::Auto { strategy_id: None },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book: Book::Live,
    });
    position.id = Some(exit_hours);
    position.total_size_native = 0.1;
    position.exit_time = Some(exit);
    position.pnl = Some(pnl_percent / 100.0 * 0.1);
    position.pnl_percent = Some(pnl_percent);
    position.closed_reason = reason.map(str::to_owned);
    position
}

fn refs(rounds: &[Position]) -> Vec<&Position> {
    rounds.iter().collect()
}

fn group<'a>(exits: &'a [ExitBreakdown], key: &str) -> &'a ExitBreakdown {
    exits
        .iter()
        .find(|exit| exit.exit_type == key)
        .unwrap_or_else(|| panic!("no {key} group in {exits:?}"))
}

#[test]
fn exit_groups_merge_a_pending_suffix_and_every_force_close() {
    let rounds = [
        closed(1, 10.0, Some("TakeProfit")),
        closed(2, 12.0, Some("TakeProfit_pending_verification")),
        closed(
            3,
            -5.0,
            Some("force_closed: Ghost position, no tokens in the wallet"),
        ),
        closed(4, -8.0, Some("force_closed_ghost")),
    ];
    let exits = exit_breakdown(&refs(&rounds));
    assert_eq!(exits.len(), 2, "{exits:?}");
    let take_profit = group(&exits, "TakeProfit");
    assert_eq!(take_profit.count, 2);
    assert_eq!(take_profit.label.id, "trade-reason-take-profit");
    let force_closed = group(&exits, "force_closed");
    assert_eq!(force_closed.count, 2);
    assert_eq!(force_closed.label.id, "trade-reason-force-closed-any");
}

#[test]
fn exit_groups_name_a_missing_reason_not_recorded() {
    let rounds = [closed(1, 3.0, None), closed(2, -3.0, Some(""))];
    let exits = exit_breakdown(&refs(&rounds));
    assert_eq!(exits.len(), 1, "{exits:?}");
    let unknown = group(&exits, crate::trader::UNKNOWN_REASON_KEY);
    assert_eq!(unknown.count, 2);
    assert_eq!(unknown.label.id, "trade-reason-unknown");
    assert_eq!(unknown.label.render_source_plain(), "Not recorded");
}

#[test]
fn exit_groups_order_by_count_then_key() {
    let rounds = [
        closed(1, 3.0, Some("StopLoss")),
        closed(2, 3.0, Some("TakeProfit")),
        closed(3, 3.0, Some("ManualExit")),
        closed(4, 3.0, Some("TakeProfit")),
        closed(5, 3.0, Some("Blacklisted")),
    ];
    // Every input order gives the same output order.
    for reversed in [false, true] {
        let mut ordered = refs(&rounds);
        if reversed {
            ordered.reverse();
        }
        let keys: Vec<String> = exit_breakdown(&ordered)
            .into_iter()
            .map(|exit| exit.exit_type)
            .collect();
        assert_eq!(
            keys,
            ["TakeProfit", "Blacklisted", "ManualExit", "StopLoss"],
            "reversed: {reversed}"
        );
    }
}

#[test]
fn exit_group_win_rate_counts_only_winners() {
    let rounds = [
        closed(1, 20.0, Some("TakeProfit")),
        closed(2, 0.0, Some("TakeProfit")),
        closed(3, -10.0, Some("TakeProfit")),
        closed(4, 10.0, Some("TakeProfit")),
    ];
    let exits = exit_breakdown(&refs(&rounds));
    let take_profit = group(&exits, "TakeProfit");
    assert_eq!(take_profit.count, 4);
    assert_eq!(take_profit.winners, 2);
    assert_eq!(take_profit.win_rate_pct, Some(50.0));
    assert!((take_profit.avg_profit_pct.unwrap() - 5.0).abs() < 1e-12);
    assert!((take_profit.net_pnl_native.unwrap() - 0.02).abs() < 1e-12);
}

/// A counted round whose P&L is unknown: no return and no SOL result.
fn unknown_result(exit_hours: i64, reason: Option<&str>) -> Position {
    let mut position = closed(exit_hours, 0.0, reason);
    position.pnl = None;
    position.pnl_percent = None;
    position
}

#[test]
fn a_group_of_unknown_results_reports_no_figures() {
    let rounds = [unknown_result(1, None), unknown_result(2, None)];
    let exits = exit_breakdown(&refs(&rounds));
    let not_recorded = &exits[0];
    assert_eq!(not_recorded.count, 2);
    assert_eq!(not_recorded.winners, 0);
    assert_eq!(not_recorded.win_rate_pct, None);
    assert_eq!(not_recorded.avg_profit_pct, None);
    assert_eq!(not_recorded.net_pnl_native, None);

    let sources = source_breakdown(&refs(&rounds));
    assert_eq!(sources[0].count, 2);
    assert_eq!(sources[0].win_rate_pct, None);
    assert_eq!(sources[0].avg_profit_pct, None);
    assert_eq!(sources[0].net_pnl_native, None);
}

#[test]
fn an_unknown_result_is_left_out_of_rates_buckets_and_sums() {
    let rounds = [
        closed(1, 20.0, Some("TakeProfit")),
        closed(2, -10.0, Some("TakeProfit")),
        unknown_result(3, Some("TakeProfit")),
    ];
    let exits = exit_breakdown(&refs(&rounds));
    let take_profit = group(&exits, "TakeProfit");
    assert_eq!(take_profit.count, 3);
    assert_eq!(take_profit.win_rate_pct, Some(50.0));
    assert!((take_profit.avg_profit_pct.unwrap() - 5.0).abs() < 1e-12);
    assert!((take_profit.net_pnl_native.unwrap() - 0.01).abs() < 1e-12);

    let buckets = outcome_distribution(&refs(&rounds));
    assert_eq!(buckets.iter().map(|bucket| bucket.count).sum::<usize>(), 2);
    let zero_bucket = buckets
        .iter()
        .find(|bucket| bucket.lower_pct == Some(0.0))
        .expect("a bucket starts at 0%");
    assert_eq!(zero_bucket.count, 0);

    let realized = RealizedStats::of(StatsWindow::All, rounds.to_vec(), fixed_now());
    assert_eq!(realized.total_trades, 3);
    assert_eq!(realized.winners, 1);
    assert_eq!(realized.losers, 1);
    assert_eq!(realized.win_rate_pct, Some(50.0));
}

#[test]
fn sources_follow_the_fixed_order_and_omit_empty_kinds() {
    let with_origin = |exit_hours: i64, pnl_percent: f64, origin: PositionOrigin| {
        let mut position = closed(exit_hours, pnl_percent, Some("ManualExit"));
        position.origin = origin;
        position
    };
    let rounds = [
        with_origin(1, 10.0, PositionOrigin::External),
        with_origin(2, -10.0, PositionOrigin::Manual),
        with_origin(3, 30.0, PositionOrigin::Auto { strategy_id: None }),
        with_origin(4, 10.0, PositionOrigin::Manual),
    ];
    let sources = source_breakdown(&refs(&rounds));
    let kinds: Vec<&str> = sources
        .iter()
        .map(|source| source.source.as_str())
        .collect();
    assert_eq!(kinds, ["auto", "manual", "external"]);
    let manual = &sources[1];
    assert_eq!(manual.count, 2);
    assert_eq!(manual.winners, 1);
    assert_eq!(manual.win_rate_pct, Some(50.0));
    assert!(manual.avg_profit_pct.unwrap().abs() < 1e-12);
    assert_eq!(
        sources.iter().map(|source| source.count).sum::<usize>(),
        rounds.len()
    );
}

#[test]
fn outcome_buckets_take_the_lower_edge_and_leave_the_upper() {
    let bucket_of = |pnl_percent: f64| {
        let rounds = [closed(1, pnl_percent, Some("ManualExit"))];
        let buckets = outcome_distribution(&refs(&rounds));
        let index = buckets
            .iter()
            .position(|bucket| bucket.count == 1)
            .expect("the round is in one bucket");
        (buckets[index].lower_pct, buckets[index].upper_pct)
    };
    assert_eq!(bucket_of(-50.000_1), (None, Some(-50.0)));
    assert_eq!(bucket_of(-50.0), (Some(-50.0), Some(-20.0)));
    assert_eq!(bucket_of(-0.000_1), (Some(-5.0), Some(0.0)));
    assert_eq!(bucket_of(0.0), (Some(0.0), Some(5.0)));
    assert_eq!(bucket_of(99.9), (Some(50.0), Some(100.0)));
    assert_eq!(bucket_of(100.0), (Some(100.0), None));
    assert_eq!(bucket_of(450.0), (Some(100.0), None));
}

#[test]
fn outcome_buckets_cover_every_counted_round_once() {
    let rounds: Vec<Position> = [-80.0, -50.0, -12.0, -1.0, 0.0, 4.0, 19.0, 60.0, 140.0, 7.5]
        .into_iter()
        .enumerate()
        .map(|(index, pnl_percent)| closed(index as i64 + 1, pnl_percent, Some("ManualExit")))
        .collect();
    let buckets = outcome_distribution(&refs(&rounds));
    assert_eq!(buckets.len(), OUTCOME_EDGES_PCT.len() + 1);
    assert_eq!(
        buckets.iter().map(|bucket| bucket.count).sum::<usize>(),
        rounds.len()
    );
    let net: f64 = buckets.iter().map(|bucket| bucket.net_pnl_native).sum();
    let total: f64 = rounds.iter().filter_map(|round| round.pnl).sum();
    assert!((net - total).abs() < 1e-12, "{net} vs {total}");
    assert_eq!(buckets.first().and_then(|bucket| bucket.lower_pct), None);
    assert_eq!(buckets.last().and_then(|bucket| bucket.upper_pct), None);

    let realized = RealizedStats::of(StatsWindow::All, rounds, fixed_now());
    let counted: usize = realized
        .outcome_distribution
        .iter()
        .map(|bucket| bucket.count)
        .sum();
    assert_eq!(counted, realized.total_trades);
    assert_eq!(
        realized
            .exit_breakdown
            .iter()
            .map(|exit| exit.count)
            .sum::<usize>(),
        realized.total_trades
    );
    assert_eq!(
        realized
            .source_breakdown
            .iter()
            .map(|source| source.count)
            .sum::<usize>(),
        realized.total_trades
    );
}

#[test]
fn outcome_distribution_is_empty_without_trades() {
    assert!(outcome_distribution(&[]).is_empty());
    let realized = RealizedStats::of(StatsWindow::Days(1), Vec::new(), fixed_now());
    assert!(realized.outcome_distribution.is_empty());
    assert!(realized.source_breakdown.is_empty());
    assert!(realized.recent_rounds.is_empty());
    assert_eq!(realized.current_streak, None);
    assert_eq!(realized.longest_win_streak, 0);
    assert_eq!(realized.longest_loss_streak, 0);
}

#[test]
fn streaks_end_at_a_flat_round() {
    // Newest first; in exit order: win, win, flat, win, win, win, loss.
    let rounds = [
        closed(1, -4.0, None),
        closed(2, 3.0, None),
        closed(3, 3.0, None),
        closed(4, 3.0, None),
        closed(5, 0.0, None),
        closed(6, 3.0, None),
        closed(7, 3.0, None),
    ];
    let streaks = streaks(&refs(&rounds));
    assert_eq!(streaks.longest_win, 3);
    assert_eq!(streaks.longest_loss, 1);

    // Two wins split by a flat round never make a run of two.
    let split = [
        closed(1, 3.0, None),
        closed(2, 0.0, None),
        closed(3, 3.0, None),
    ];
    assert_eq!(streaks_of(&split).longest_win, 1);
}

fn streaks_of(rounds: &[Position]) -> Streaks {
    streaks(&refs(rounds))
}

#[test]
fn current_streak_reads_from_the_newest_close() {
    let losing = [
        closed(1, -2.0, None),
        closed(2, -1.0, None),
        closed(3, 5.0, None),
        closed(4, -1.0, None),
    ];
    assert_eq!(
        streaks_of(&losing).current,
        Some(Streak {
            outcome: Outcome::Loss,
            length: 2,
        })
    );
    let flat_last = [closed(1, 0.0, None), closed(2, 5.0, None)];
    assert_eq!(streaks_of(&flat_last).current, None);
    assert_eq!(streaks_of(&[]).current, None);
}

#[test]
fn recent_rounds_are_the_newest_ten_counted_rounds() {
    let mut rounds: Vec<Position> = (1..=12)
        .map(|hours| closed(hours, 2.0, Some("TakeProfit")))
        .collect();
    let mut untrusted = closed(0, 50.0, Some("TakeProfit"));
    untrusted.id = Some(100);
    untrusted.exit_time = Some(fixed_now() - Duration::minutes(10));
    untrusted.basis_complete = false;
    let mut reset = closed(0, -5.0, Some("PaperReset"));
    reset.id = Some(101);
    reset.exit_time = Some(fixed_now() - Duration::minutes(5));
    rounds.push(untrusted);
    rounds.push(reset);
    rounds.reverse();

    let realized = RealizedStats::of(StatsWindow::Days(7), rounds, fixed_now());
    let ids: Vec<Option<i64>> = realized
        .recent_rounds
        .iter()
        .map(|round| round.position_id)
        .collect();
    assert_eq!(ids, (1..=10).map(Some).collect::<Vec<_>>());
    let first = &realized.recent_rounds[0];
    assert_eq!(first.source, "auto");
    assert_eq!(first.hold_secs, 3600);
    assert_eq!(
        first.exit_time,
        (fixed_now() - Duration::hours(1)).timestamp()
    );
    assert_eq!(first.reason.id, "trade-reason-take-profit");
}

#[test]
fn recent_round_reason_reads_not_recorded_without_a_reason() {
    let rounds = [
        closed(1, 2.0, None),
        closed(2, 2.0, Some("")),
        closed(3, 2.0, Some("TakeProfit_pending_verification")),
    ];
    let recent = recent_rounds(&refs(&rounds));
    assert_eq!(recent[0].reason.id, "trade-reason-unknown");
    assert_eq!(recent[1].reason.id, "trade-reason-unknown");
    assert_eq!(recent[2].reason.id, "trade-reason-pending-verification");
}
