// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The boundary of a paper task's current run, the spend its rounds make, and the
//! flag on Activity rows decided before that run.

use chrono::{Duration, TimeZone};

use super::*;
use crate::positions::Book;
use crate::trader::copy::round_fixtures::open_round;
use crate::trader::copy::types::{CopySkip, ExitMode, SizingMode};

fn task(id: i64, mode: TradingMode, paper_started_at: Option<DateTime<Utc>>) -> CopyTask {
    CopyTask {
        id,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.2,
        max_native_per_token: 1.0,
        total_budget_native: 5.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
        paper_started_at,
    }
}

fn epoch(started_at: DateTime<Utc>) -> PaperEpoch {
    PaperEpoch {
        epoch: 3,
        starting_balance_native: 10.0,
        started_at,
    }
}

fn skipped(task_id: i64, decided_at: DateTime<Utc>) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id,
        signature: format!("signature-{}", decided_at.timestamp()),
        mint: Some("mint".to_owned()),
        reason: CopySkip::NotBuySwap,
        decided_at,
        telemetry: None,
        paper: None,
    }
}

fn row(id: i64, task_id: i64, decided_at: DateTime<Utc>) -> CopyActivityRow {
    CopyActivityRow {
        id,
        task_id,
        kind: "skipped".to_owned(),
        outcome: skipped(task_id, decided_at),
        created_at: decided_at,
        earlier_run: false,
    }
}

#[test]
fn the_current_run_starts_at_the_later_of_the_task_start_and_the_epoch_start() {
    let task_start = Utc.with_ymd_and_hms(2026, 3, 1, 0, 0, 0).unwrap();
    let epoch_start = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let configured = task(1, TradingMode::Paper, Some(task_start));
    assert_eq!(
        effective_start(&configured, Some(&epoch(epoch_start))),
        epoch_start
    );
    assert_eq!(
        effective_start(&configured, Some(&epoch(task_start - Duration::days(1)))),
        task_start
    );
    assert_eq!(effective_start(&configured, None), task_start);
    assert_eq!(
        effective_start(&task(1, TradingMode::Paper, None), None),
        DateTime::<Utc>::MIN_UTC,
        "without either start every round belongs to the run"
    );
}

#[test]
fn paper_spend_counts_the_copy_buy_of_each_round_and_never_its_additions() {
    let mut added = open_round(1, Book::Paper, "mint", 0.01, 0.5, 30);
    added.total_size_native = 0.8;
    let rounds = [
        added,
        open_round(1, Book::Paper, "mint", 0.01, 0.2, 20),
        open_round(1, Book::Paper, "other", 0.01, 0.3, 10),
    ];
    let spend = paper_spend(&rounds, "mint");
    assert!((spend.total_spent_native - 1.0).abs() < 1e-9);
    assert!((spend.token_spent_native - 0.7).abs() < 1e-9);
    assert_eq!(spend.token_buy_count, 2);
    assert_eq!(paper_spend(&[], "mint"), SpendState::default());
}

#[test]
fn a_decision_at_the_start_belongs_to_the_current_run() {
    let start = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    assert!(in_current_run(&skipped(1, start), start));
    assert!(in_current_run(
        &skipped(1, start + Duration::seconds(1)),
        start
    ));
    assert!(!in_current_run(
        &skipped(1, start - Duration::milliseconds(1)),
        start
    ));
}

#[test]
fn only_rows_of_a_paper_task_decided_before_its_run_are_flagged() {
    let start = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let before = start - Duration::hours(1);
    let after = start + Duration::hours(1);
    let tasks = [
        task(1, TradingMode::Paper, Some(start)),
        task(2, TradingMode::Live, Some(start)),
        task(3, TradingMode::Paper, None),
    ];
    let mut rows = vec![
        row(1, 1, before),
        row(2, 1, after),
        row(3, 2, before),
        row(4, 3, before),
        row(5, 99, before),
    ];
    rows[1].earlier_run = true;
    flag_earlier_runs(&mut rows, &tasks, None);
    let flagged = rows
        .iter()
        .map(|row| (row.id, row.earlier_run))
        .collect::<Vec<_>>();
    assert_eq!(
        flagged,
        [(1, true), (2, false), (3, false), (4, false), (5, false)],
        "a live task, a paper task without a start and an unknown task flag nothing"
    );
}
