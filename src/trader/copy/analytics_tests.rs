// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! A task's stats from the rounds it opened in the book of its mode.

use chrono::Utc;

use super::*;
use crate::trader::copy::round_fixtures::{closed, marked, open_round};

#[test]
fn paper_stats_count_the_task_rounds_their_marks_and_how_they_closed() {
    let now = Utc::now();
    let rounds = [
        marked(
            open_round(1, Book::Paper, "priced", 0.01, 0.5, 30),
            Some(0.012),
        ),
        marked(open_round(1, Book::Paper, "unpriced", 0.01, 0.4, 20), None),
        closed(
            open_round(1, Book::Paper, "ruled", 0.01, 0.3, 90),
            0.1,
            TradeReason::TakeProfit,
            now,
        ),
        closed(
            open_round(1, Book::Paper, "manual", 0.01, 0.2, 80),
            -0.05,
            TradeReason::ManualExit,
            now,
        ),
        closed(
            open_round(1, Book::Paper, "mirrored", 0.01, 0.2, 70),
            0.02,
            TradeReason::CopySell,
            now,
        ),
        // Another task's round, and a live round of this task, are not this book.
        marked(
            open_round(2, Book::Paper, "other-task", 0.01, 0.5, 10),
            Some(0.02),
        ),
        marked(open_round(1, Book::Live, "live", 0.01, 0.5, 10), Some(0.02)),
    ];
    let stats = build_task_stats(1, CopyBook::Paper, &[], &rounds);
    assert_eq!(stats.book, CopyBook::Paper);
    assert_eq!((stats.open_positions, stats.closed_positions), (2, 3));
    assert_eq!(stats.unpriced_positions, 1);
    assert!((stats.unrealized_pnl_native - 0.1).abs() < 1e-9);
    assert!((stats.realized_pnl_native - 0.07).abs() < 1e-9);
    assert_eq!((stats.policy_exits, stats.manual_closes), (1, 1));
}

#[test]
fn live_stats_count_only_verified_exits_as_closed() {
    let now = Utc::now();
    let mut unverified = closed(
        open_round(1, Book::Live, "pending", 0.01, 0.5, 30),
        0.1,
        TradeReason::StopLoss,
        now,
    );
    unverified.transaction_exit_verified = false;
    let rounds = [
        unverified,
        closed(
            open_round(1, Book::Live, "verified", 0.01, 0.5, 40),
            -0.1,
            TradeReason::StopLoss,
            now,
        ),
        closed(
            open_round(1, Book::Paper, "paper", 0.01, 0.5, 40),
            0.3,
            TradeReason::StopLoss,
            now,
        ),
    ];
    let stats = build_task_stats(1, CopyBook::Live, &[], &rounds);
    assert_eq!((stats.open_positions, stats.closed_positions), (1, 1));
    assert_eq!(stats.policy_exits, 1);
    assert_eq!(stats.unpriced_positions, 0);
    assert!((stats.realized_pnl_native + 0.1).abs() < 1e-9);
}

#[test]
fn a_close_reason_keeps_its_kind_through_its_decorations() {
    assert_eq!(round_close(Some("StopLoss")), RoundClose::Policy);
    assert_eq!(
        round_close(Some("TrailingStop_pending_verification")),
        RoundClose::Policy
    );
    assert_eq!(round_close(Some("ManualExit")), RoundClose::Manual);
    assert_eq!(round_close(Some("ForceSell")), RoundClose::Manual);
    assert_eq!(round_close(Some("CopySell")), RoundClose::Other);
    assert_eq!(round_close(Some("PaperReset")), RoundClose::Other);
    assert_eq!(round_close(None), RoundClose::Other);
}
