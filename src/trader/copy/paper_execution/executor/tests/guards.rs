// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task, mode, global, force-stop, slippage, and epoch guards across quote boundaries.

use super::support::{execute, raw_sol, setup, task, FakeIo, Mutation, TaskEdit, MINT};
use crate::trader::copy::{CopyOutcome, CopySkip, SpendState};
use crate::trader::paper::TradingMode;

async fn assert_rejected_without_spend(
    mutation: Mutation,
    expected: CopySkip,
    expected_quotes: usize,
    reset_epoch: bool,
) {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1)).at_wait(mutation);
    let outcome = execute(&database, configured.clone(), "guarded-entry", epoch, &io).await;
    assert_eq!(io.quote_calls(), expected_quotes);
    assert_eq!(super::support::skip(&outcome), &expected);
    assert_eq!(
        super::support::record(&database, outcome).await,
        !reset_epoch,
        "stale paper-epoch outcomes are ignored by the real settlement writer"
    );
    assert_eq!(
        database
            .spend_state(configured.id, TradingMode::Paper, MINT)
            .await
            .unwrap(),
        SpendState::default()
    );
}

#[tokio::test]
async fn reset_disable_mode_global_disable_and_force_stop_block_between_quotes() {
    assert_rejected_without_spend(
        Mutation::Reset,
        CopySkip::PaperExecutionUnavailable,
        1,
        true,
    )
    .await;
    assert_rejected_without_spend(
        Mutation::Task(TaskEdit::Disable),
        CopySkip::TaskDisabled,
        1,
        false,
    )
    .await;
    assert_rejected_without_spend(Mutation::ModeLive, CopySkip::TaskDisabled, 1, false).await;
    assert_rejected_without_spend(
        Mutation::GlobalDisabled,
        CopySkip::PaperExecutionUnavailable,
        1,
        false,
    )
    .await;
    assert_rejected_without_spend(Mutation::ForceStop, CopySkip::ForceStopped, 1, false).await;
}

#[tokio::test]
async fn force_stop_before_execution_prevents_any_quote_request() {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io =
        FakeIo::new(database.clone(), configured.id, raw_sol(0.1)).force_stopped_before_start();
    let outcome = execute(&database, configured.clone(), "already-stopped", epoch, &io).await;
    assert_eq!(io.quote_calls(), 0);
    assert_eq!(super::support::skip(&outcome), &CopySkip::ForceStopped);
    assert!(super::support::record(&database, outcome).await);
    assert_eq!(
        database
            .spend_state(configured.id, TradingMode::Paper, MINT)
            .await
            .unwrap(),
        SpendState::default()
    );
}

#[tokio::test]
async fn lowered_total_token_and_trade_caps_reject_the_landed_quote_by_current_cap() {
    for (edit, expected) in [
        (TaskEdit::TotalBudget(0.05), CopySkip::BudgetExhausted),
        (TaskEdit::TokenCap(0.100002), CopySkip::TokenCapReached),
        (
            TaskEdit::TradeCap(0.05),
            CopySkip::PaperExecutionUnavailable,
        ),
    ] {
        let (_dir, database, configured, epoch) = setup(task()).await;
        let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1))
            .at_wait(Mutation::Task(edit));
        let outcome = execute(&database, configured.clone(), "lowered-cap", epoch, &io).await;
        assert_eq!(io.quote_calls(), 2);
        assert_eq!(super::support::skip(&outcome), &expected);
        assert!(super::support::record(&database, outcome).await);
        assert_eq!(
            database
                .spend_state(configured.id, TradingMode::Paper, MINT)
                .await
                .unwrap(),
            SpendState::default()
        );
    }
}

#[tokio::test]
async fn reset_after_landing_quote_is_rejected_by_executor_and_ledger_epoch_gate() {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io =
        FakeIo::new(database.clone(), configured.id, raw_sol(0.1)).after_landing(Mutation::Reset);
    let outcome = execute(
        &database,
        configured.clone(),
        "reset-after-quote",
        epoch,
        &io,
    )
    .await;
    assert_eq!(io.quote_calls(), 2);
    assert_eq!(
        super::support::skip(&outcome),
        &CopySkip::PaperExecutionUnavailable
    );
    assert!(!super::support::record(&database, outcome).await);
    assert_eq!(
        database
            .spend_state(configured.id, TradingMode::Paper, MINT)
            .await
            .unwrap(),
        SpendState::default()
    );
}

#[tokio::test]
async fn reset_after_executor_result_before_ledger_settlement_drops_the_stale_fill() {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1));
    let outcome = execute(
        &database,
        configured.clone(),
        "reset-before-settlement",
        epoch,
        &io,
    )
    .await;
    assert!(matches!(outcome, CopyOutcome::PaperFilled(_)));
    assert_eq!(io.quote_calls(), 2);

    database.reset_paper_book(configured.id).await.unwrap();
    assert!(!super::support::record(&database, outcome).await);
    assert_eq!(
        database
            .spend_state(configured.id, TradingMode::Paper, MINT)
            .await
            .unwrap(),
        SpendState::default()
    );
}

#[tokio::test]
async fn slippage_edit_invalidates_quote_without_misreporting_the_old_limit() {
    let (_dir, database, configured, epoch) = setup(task()).await;
    let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1))
        .at_wait(Mutation::Task(TaskEdit::Slippage(2.0)));
    let outcome = execute(&database, configured.clone(), "slippage-edit", epoch, &io).await;
    assert_eq!(io.quote_calls(), 1);
    assert_eq!(
        super::support::skip(&outcome),
        &CopySkip::PaperExecutionUnavailable
    );
    assert!(super::support::record(&database, outcome).await);

    let (_dir, database, configured, epoch) = setup(task()).await;
    let io = FakeIo::new(database.clone(), configured.id, raw_sol(0.1))
        .after_landing(Mutation::Task(TaskEdit::Slippage(2.0)));
    let outcome = execute(
        &database,
        configured.clone(),
        "late-slippage-edit",
        epoch,
        &io,
    )
    .await;
    assert_eq!(io.quote_calls(), 2);
    assert_eq!(
        super::support::skip(&outcome),
        &CopySkip::PaperExecutionUnavailable
    );
    assert!(super::support::record(&database, outcome).await);
}
