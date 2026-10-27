// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the strategies overview composition: the evaluation order it ranks, the status
//! each strategy reads under every run state, the trader-level notice and the figures it
//! takes from each book.

use chrono::Utc;

use super::*;
use crate::positions::{Opening, PositionManagement, PositionOrigin};
use crate::strategies::types::{Condition, RuleTree};

fn strategy(id: &str, strategy_type: StrategyType, priority: i32, enabled: bool) -> Strategy {
    Strategy {
        id: id.to_owned(),
        name: id.to_owned(),
        description: None,
        strategy_type,
        enabled,
        priority,
        timeframe: "5m".to_owned(),
        rules: RuleTree::leaf(Condition {
            condition_type: "PriceChangePercent".to_owned(),
            parameters: std::collections::HashMap::new(),
        }),
        parameters: std::collections::HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: TradingMode::Paper,
    }
}

fn position(book: Book, entry: Option<&str>, unrealized: Option<f64>) -> Position {
    let mut position = Position::opening(Opening {
        mint: format!("Mint{}", entry.unwrap_or("none")),
        symbol: "T".to_owned(),
        name: "Token".to_owned(),
        entry_price: 0.001,
        entry_time: Utc::now(),
        size_native: 0.1,
        entry_signature: None,
        origin: PositionOrigin::Auto {
            strategy_id: entry.map(str::to_owned),
        },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book,
    });
    position.unrealized_pnl = unrealized;
    position
}

const RUNNING: Monitors = Monitors {
    entry: true,
    exit: true,
};

#[test]
fn the_order_ranks_enabled_entries_in_the_order_the_store_lists_them() {
    let strategies = [
        strategy("first", StrategyType::Entry, 1, true),
        strategy("off", StrategyType::Entry, 5, false),
        strategy("exit", StrategyType::Exit, 5, true),
        strategy("b", StrategyType::Entry, 10, true),
        strategy("a", StrategyType::Entry, 10, true),
    ];
    let order = entry_order(&strategies);
    assert_eq!(order.get("first"), Some(&1));
    assert_eq!(
        order.get("b"),
        Some(&2),
        "the listed order is kept, never sorted again"
    );
    assert_eq!(order.get("a"), Some(&3));
    assert_eq!(
        order.get("off"),
        None,
        "a strategy that is off is not evaluated"
    );
    assert_eq!(
        order.get("exit"),
        None,
        "exit strategies take no entry order"
    );
}

/// Each book runs its own pass, so the rank restarts at 1 within each mode while keeping
/// the listed order inside it.
#[test]
fn the_order_restarts_at_one_within_each_mode() {
    let live = |id: &str, priority: i32, enabled: bool| Strategy {
        mode: TradingMode::Live,
        ..strategy(id, StrategyType::Entry, priority, enabled)
    };
    let strategies = [
        strategy("paper-1", StrategyType::Entry, 1, true),
        live("live-1", 2, true),
        live("live-off", 3, false),
        live("live-2", 4, true),
        strategy("paper-2", StrategyType::Entry, 5, true),
    ];
    let order = entry_order(&strategies);
    assert_eq!(order.get("paper-1"), Some(&1));
    assert_eq!(
        order.get("live-1"),
        Some(&1),
        "a paper rank pushed a live one"
    );
    assert_eq!(order.get("live-2"), Some(&2));
    assert_eq!(
        order.get("paper-2"),
        Some(&2),
        "a live rank pushed a paper one"
    );
    assert_eq!(order.get("live-off"), None);
}

#[test]
fn explore_reads_setup_required_whatever_the_strategy() {
    for enabled in [true, false] {
        let entry = strategy("e", StrategyType::Entry, 1, enabled);
        assert_eq!(
            status_of(&entry, TraderRunState::Explore, RUNNING, None),
            StrategyRunStatus::SetupRequired
        );
    }
    assert_eq!(
        notice_of(TraderRunState::Explore),
        Some(RunNotice::SetupRequired)
    );
}

#[test]
fn a_stopped_trader_stops_every_enabled_strategy_with_one_notice() {
    let entry = strategy("e", StrategyType::Entry, 1, true);
    let exit = strategy("x", StrategyType::Exit, 1, true);
    let off = strategy("o", StrategyType::Entry, 1, false);
    for (run_state, notice) in [
        (TraderRunState::Stopped, RunNotice::TraderOff),
        (TraderRunState::ForceStopped, RunNotice::ForceStopped),
        (TraderRunState::Waiting, RunNotice::Waiting),
    ] {
        assert_eq!(notice_of(run_state), Some(notice));
        for enabled in [&entry, &exit] {
            assert_eq!(
                status_of(enabled, run_state, RUNNING, None),
                StrategyRunStatus::NotRunning,
                "{run_state:?}"
            );
        }
        assert_eq!(
            status_of(&off, run_state, RUNNING, None),
            StrategyRunStatus::Off
        );
    }
    assert_eq!(notice_of(TraderRunState::Running), None);
    assert_eq!(
        notice_of(TraderRunState::EntryPaused),
        None,
        "a live entry pause does not stop paper entries or any exit"
    );
}

#[test]
fn a_running_trader_reads_each_entry_block_and_monitor_switch() {
    let entry = strategy("e", StrategyType::Entry, 1, true);
    let exit = strategy("x", StrategyType::Exit, 1, true);
    for (block, status) in [
        (None, StrategyRunStatus::Running),
        (Some(EntryBlock::LossLimit), StrategyRunStatus::LivePaused),
        (
            Some(EntryBlock::PositionLimit),
            StrategyRunStatus::SlotsFull,
        ),
        (
            Some(EntryBlock::PaperBalanceExhausted),
            StrategyRunStatus::PaperCashShort,
        ),
        (
            Some(EntryBlock::CheckFailed("store".to_owned())),
            StrategyRunStatus::NotRunning,
        ),
    ] {
        assert_eq!(
            status_of(&entry, TraderRunState::Running, RUNNING, block.as_ref()),
            status,
            "{block:?}"
        );
    }
    let entry_off = Monitors {
        entry: false,
        exit: true,
    };
    assert_eq!(
        status_of(&entry, TraderRunState::Running, entry_off, None),
        StrategyRunStatus::EntryMonitorOff
    );
    assert_eq!(
        status_of(&exit, TraderRunState::Running, entry_off, None),
        StrategyRunStatus::Running
    );
    let exit_off = Monitors {
        entry: true,
        exit: false,
    };
    assert_eq!(
        status_of(&exit, TraderRunState::EntryPaused, exit_off, None),
        StrategyRunStatus::ExitMonitorOff
    );
}

#[test]
fn counts_tell_enabled_from_total_per_kind() {
    let strategies = [
        strategy("a", StrategyType::Entry, 1, true),
        strategy("b", StrategyType::Entry, 2, false),
        strategy("x", StrategyType::Exit, 1, false),
    ];
    let counts = counts_of(strategies.iter());
    assert_eq!(
        counts.entry,
        KindCount {
            enabled: 1,
            total: 2
        }
    );
    assert_eq!(
        counts.exit,
        KindCount {
            enabled: 0,
            total: 1
        }
    );
}

#[test]
fn an_entry_reads_its_book_and_counts_the_other_books_open_positions_apart() {
    crate::config::utils::install_default_config();
    let mut live_entry = strategy("live-entry", StrategyType::Entry, 1, true);
    live_entry.mode = TradingMode::Live;
    let mut closed = position(Book::Live, Some("live-entry"), None);
    closed.exit_time = Some(Utc::now());
    closed.pnl = Some(0.02);
    closed.pnl_percent = Some(20.0);
    closed.transaction_exit_verified = true;
    let books = BookPositions {
        open_live: vec![
            position(Book::Live, Some("live-entry"), Some(0.01)),
            position(Book::Live, Some("live-entry"), None),
            position(Book::Live, Some("another"), Some(0.5)),
        ],
        open_paper: vec![position(Book::Paper, Some("live-entry"), Some(0.3))],
        live_rounds: HashMap::from([("live-entry".to_owned(), vec![closed])]),
    };
    let figures = books.figures_of(&live_entry, Some(&PaperStrategyEvidence::default()));
    assert_eq!(figures.open, Some(2));
    assert_eq!(figures.open_other_book, Some(1));
    assert_eq!(figures.unpriced, Some(1));
    assert_eq!(
        figures.unrealized_native,
        Some(0.01),
        "the other book's P&L is never summed in"
    );
    let rounds = figures.rounds.expect("live rounds");
    assert_eq!(rounds.closed, 1);
    assert_eq!(rounds.win_rate_pct, Some(100.0));
    assert!(
        figures.readiness.is_none(),
        "a live strategy has no readiness"
    );
    assert!(figures.watching.is_none());

    let paper_entry = strategy("paper-entry", StrategyType::Entry, 1, true);
    let figures = books.figures_of(&paper_entry, Some(&PaperStrategyEvidence::default()));
    assert_eq!(figures.open, Some(0));
    assert_eq!(figures.unrealized_native, None);
    assert_eq!(figures.rounds.map(|rounds| rounds.closed), Some(0));
    assert_eq!(figures.rounds.and_then(|rounds| rounds.win_rate_pct), None);
    assert!(figures.readiness.is_some());
}

#[test]
fn an_exit_watches_the_positions_it_manages_in_the_books_it_judges() {
    crate::config::utils::install_default_config();
    let books = BookPositions {
        open_live: vec![
            position(Book::Live, Some("bound"), None),
            position(Book::Live, Some("unbound"), None),
        ],
        open_paper: vec![position(Book::Paper, Some("bound"), None)],
        live_rounds: HashMap::new(),
    };
    let mut paper_exit = strategy("paper-exit", StrategyType::Exit, 1, true);
    paper_exit.entry_strategy_ids = vec!["bound".to_owned()];
    let figures = books.figures_of(&paper_exit, Some(&PaperStrategyEvidence::default()));
    assert_eq!(figures.watching, Some(1), "a paper exit judges paper only");
    assert_eq!(figures.rounds.map(|rounds| rounds.closed), Some(0));
    assert!(figures.open.is_none());

    let mut live_exit = paper_exit.clone();
    live_exit.mode = TradingMode::Live;
    let figures = books.figures_of(&live_exit, Some(&PaperStrategyEvidence::default()));
    assert_eq!(figures.watching, Some(2), "a live exit judges both books");
    assert!(
        figures.rounds.is_none(),
        "live closes are not attributed to exit strategies"
    );

    live_exit.enabled = false;
    assert!(books
        .figures_of(&live_exit, Some(&PaperStrategyEvidence::default()))
        .watching
        .is_none());
}

#[test]
fn unreadable_paper_evidence_leaves_the_live_figures_and_marks_the_paper_ones_absent() {
    crate::config::utils::install_default_config();
    assert!(available(Err(crate::Error::parse_error("paper store busy"))).is_none());
    assert!(available(Ok(PaperStrategyEvidence::default())).is_some());

    let mut live_entry = strategy("live-entry", StrategyType::Entry, 1, true);
    live_entry.mode = TradingMode::Live;
    let mut closed = position(Book::Live, Some("live-entry"), None);
    closed.exit_time = Some(Utc::now());
    closed.pnl = Some(0.02);
    closed.pnl_percent = Some(20.0);
    closed.transaction_exit_verified = true;
    let books = BookPositions {
        open_live: vec![position(Book::Live, Some("live-entry"), Some(0.01))],
        open_paper: vec![position(Book::Paper, Some("paper-entry"), Some(0.3))],
        live_rounds: HashMap::from([("live-entry".to_owned(), vec![closed])]),
    };

    let live = books.figures_of(&live_entry, None);
    assert_eq!(live.open, Some(1));
    assert_eq!(live.unrealized_native, Some(0.01));
    assert_eq!(live.rounds.map(|rounds| rounds.closed), Some(1));

    let paper_entry = strategy("paper-entry", StrategyType::Entry, 2, true);
    let paper = books.figures_of(&paper_entry, None);
    assert_eq!(paper.open, Some(1), "open positions are read from memory");
    assert_eq!(paper.unrealized_native, Some(0.3));
    assert!(
        paper.rounds.is_none(),
        "unknown paper rounds are never zero"
    );
    assert!(
        paper.readiness.is_none(),
        "unknown readiness is never not ready"
    );

    let paper_exit = strategy("paper-exit", StrategyType::Exit, 1, true);
    let paper = books.figures_of(&paper_exit, None);
    assert_eq!(paper.watching, Some(1));
    assert!(paper.rounds.is_none());
    assert!(paper.readiness.is_none());
}
