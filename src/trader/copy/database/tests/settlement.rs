// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Final transaction-time task and budget checks for newly settled paper buys.

use super::*;
use crate::trader::copy::CopySkip;

async fn counts(db: &CopyDatabase, task_id: i64) -> (i64, i64, i64) {
    let connection = db.connection().unwrap();
    let decisions = connection
        .query_row(
            "SELECT COUNT(*) FROM copy_decisions WHERE task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .unwrap();
    let activity = connection
        .query_row(
            "SELECT COUNT(*) FROM copy_activity WHERE task_id=?1",
            [task_id],
            |row| row.get(0),
        )
        .unwrap();
    let paper_spend = connection
        .query_row(
            "SELECT COUNT(*) FROM copy_spend WHERE task_id=?1 AND mode='paper'",
            [task_id],
            |row| row.get(0),
        )
        .unwrap();
    (decisions, activity, paper_spend)
}

async fn assert_rejected_without_writes(
    db: &CopyDatabase,
    task: &CopyTask,
    outcome: CopyOutcome,
    expected: CopySkip,
) {
    let before_counts = counts(db, task.id).await;
    let before_activity = db.list_task_activity(task.id, 100).await.unwrap();
    let before_spend = db
        .spend_state(task.id, TradingMode::Paper, "book-mint")
        .await
        .unwrap();
    let before_positions = db.paper_positions(task.id).await.unwrap();
    assert!(matches!(
        db.record_outcome(outcome).await,
        Err(crate::trader::Error::CopyTaskRejected { reason }) if reason == expected
    ));
    assert_eq!(counts(db, task.id).await, before_counts);
    assert_eq!(
        db.list_task_activity(task.id, 100).await.unwrap(),
        before_activity
    );
    assert_eq!(
        db.spend_state(task.id, TradingMode::Paper, "book-mint")
            .await
            .unwrap(),
        before_spend
    );
    assert_eq!(db.paper_positions(task.id).await.unwrap(), before_positions);
}

#[tokio::test]
async fn task_edits_and_cumulative_caps_are_rechecked_inside_buy_settlement() {
    for (edit, expected, seed) in [
        ("disable", CopySkip::TaskDisabled, false),
        ("live", CopySkip::TaskDisabled, false),
        ("total", CopySkip::BudgetExhausted, true),
        ("token", CopySkip::TokenCapReached, true),
        ("trade", CopySkip::PaperExecutionUnavailable, false),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let db = CopyDatabase::open(
            dir.path().join("copy_trading.db"),
            crate::chains::active_chain(),
        )
        .unwrap();
        let task = db.insert_task(task()).await.unwrap();
        if seed {
            db.record_outcome(paper_buy(&task, "prior-paper-fill", 10.0, 0.100005))
                .await
                .unwrap();
            let spent = db
                .spend_state(task.id, TradingMode::Paper, "book-mint")
                .await
                .unwrap();
            assert!((spent.total_spent_native - 0.100005).abs() < 1e-12);
        }
        let outcome = paper_buy(&task, "edited-before-settlement", 10.0, 0.100005);
        match edit {
            "disable" => {
                let mut changed = db.get_task(task.id).await.unwrap().unwrap();
                changed.enabled = false;
                db.update_task(changed).await.unwrap();
            }
            "live" => {
                db.set_task_mode(
                    task.id,
                    TradingMode::Live,
                    Some(crate::trader::paper::LIVE_ARM_CONFIRMATION.to_owned()),
                )
                .await
                .unwrap();
            }
            "total" => {
                let mut changed = db.get_task(task.id).await.unwrap().unwrap();
                changed.total_budget_native = 0.15;
                db.update_task(changed).await.unwrap();
            }
            "token" => {
                let mut changed = db.get_task(task.id).await.unwrap().unwrap();
                changed.max_native_per_token = 0.15;
                db.update_task(changed).await.unwrap();
            }
            "trade" => {
                let mut changed = db.get_task(task.id).await.unwrap().unwrap();
                changed.max_native_per_trade = 0.05;
                db.update_task(changed).await.unwrap();
            }
            _ => unreachable!(),
        }
        assert_rejected_without_writes(&db, &task, outcome, expected).await;
    }
}

#[tokio::test]
async fn invalid_quote_input_provenance_rolls_back_the_provisional_decision() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let task = db.insert_task(task()).await.unwrap();
    let mut outcome = paper_buy(&task, "mismatched-native-input", 10.0, 0.100005);
    let CopyOutcome::PaperFilled(decision) = &mut outcome else {
        unreachable!()
    };
    decision.fill.quote.as_mut().unwrap().input_amount =
        Some(crate::chains::RawAmount::from(1_u64));
    let before = counts(&db, task.id).await;
    assert!(matches!(
        db.record_outcome(outcome).await,
        Err(crate::trader::Error::CopyValidation { .. })
    ));
    assert_eq!(counts(&db, task.id).await, before);
    assert!(db.paper_positions(task.id).await.unwrap().is_empty());
}

#[tokio::test]
async fn duplicate_buy_replay_stays_idempotent_after_the_task_is_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(
        dir.path().join("copy_trading.db"),
        crate::chains::active_chain(),
    )
    .unwrap();
    let task = db.insert_task(task()).await.unwrap();
    let outcome = paper_buy(&task, "replay-after-edit", 10.0, 0.100005);
    assert!(db.record_outcome(outcome.clone()).await.unwrap());
    let mut changed = db.get_task(task.id).await.unwrap().unwrap();
    changed.enabled = false;
    db.update_task(changed).await.unwrap();
    assert!(!db.record_outcome(outcome).await.unwrap());
    assert_eq!(counts(&db, task.id).await, (1, 1, 1));
    let spend = db
        .spend_state(task.id, TradingMode::Paper, "book-mint")
        .await
        .unwrap();
    assert!((spend.total_spent_native - 0.100005).abs() < 1e-12);
    assert_eq!(spend.token_buy_count, 1);
    assert_eq!(db.paper_positions(task.id).await.unwrap().len(), 1);
}
