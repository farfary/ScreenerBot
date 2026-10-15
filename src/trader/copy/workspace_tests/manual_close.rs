// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Manual paper closes: the FIFO queue position, the confirmed holding round and
//! every race between a round reopening and its quote.

use super::task;
use crate::trader::copy::paper_execution::PaperExecutionHandle;
use crate::trader::copy::{
    CopyDatabase, CopyMode, CopyOutcome, CopySellDecision, CopySkip, CopyTask, CopyTelemetry,
    PaperDecision, PaperExitRule, PaperFill, PaperModel, PaperPosition, PaperQuoteMetadata,
    PaperSellFill,
};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

fn paper_quote_metadata(epoch: u64, amount: crate::chains::RawAmount) -> PaperQuoteMetadata {
    let now = Utc::now();
    PaperQuoteMetadata {
        router_id: "test-router".to_owned(),
        initial_at: now,
        landing_at: now,
        requested_delay_ms: 1000,
        queue_duration_ms: 0,
        minimum_output_amount: amount,
        final_output_amount: amount,
        input_amount: Some(crate::chains::RawAmount::from(1_u64)),
        token_decimals: Some(6),
        epoch,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    }
}

fn paper_buy_outcome(
    task: &CopyTask,
    mint: &str,
    signature: &str,
    tokens: f64,
    decided_at: chrono::DateTime<Utc>,
) -> CopyOutcome {
    let now = decided_at;
    let spent_native = 0.1;
    let fill_price = spent_native / tokens;
    let mut quote = paper_quote_metadata(
        0,
        crate::chains::RawAmount::from_integral_f64(tokens * 1_000_000.0).unwrap(),
    );
    quote.initial_at = now - chrono::Duration::milliseconds(1000);
    quote.landing_at = now;
    quote.input_amount = Some(crate::chains::RawAmount::from(100_000_000_u64));
    CopyOutcome::PaperFilled(PaperDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        signature: signature.to_owned(),
        mint: mint.to_owned(),
        target_size_sol: 0.1,
        target_token_amount: tokens,
        sized_sol: spent_native,
        fill: PaperFill {
            input_sol: spent_native,
            market_price_sol: fill_price,
            priced_from_pool: false,
            fill_price_sol: fill_price,
            token_amount: tokens,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            total_cost_sol: spent_native,
            model: PaperModel::Quote,
            quote: Some(quote),
        },
        telemetry: CopyTelemetry {
            target_block_time: Some(now.timestamp()),
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: Some(0.01),
            fill_price_sol: Some(fill_price),
            backfill: false,
        },
    })
}

fn paper_sell_outcome(
    task: &CopyTask,
    position: &PaperPosition,
    signature: &str,
    tokens: f64,
    epoch: u64,
    decided_at: chrono::DateTime<Utc>,
    manual: bool,
) -> CopyOutcome {
    let proceeds = tokens * 0.01;
    let proceeds_raw =
        crate::chains::RawAmount::from_integral_f64(proceeds * 1_000_000_000.0).unwrap();
    let raw_tokens = crate::chains::RawAmount::from_integral_f64(tokens * 1_000_000.0).unwrap();
    let mut quote = paper_quote_metadata(epoch, proceeds_raw);
    quote.input_amount = Some(raw_tokens);
    quote.token_decimals = Some(6);
    CopyOutcome::PaperSellObserved(CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: signature.to_owned(),
        mint: position.mint.clone(),
        target_token_amount: 0.0,
        target_sol_amount: 0.0,
        exit_percentage: None,
        transaction_signature: None,
        error: None,
        exit_rule: manual.then_some(PaperExitRule::Manual),
        telemetry: CopyTelemetry {
            target_block_time: None,
            detected_at: decided_at,
            decoded_at: decided_at,
            decided_at,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: Some(0.01),
            backfill: false,
        },
        paper_fill: Some(PaperSellFill {
            token_amount: tokens,
            market_price_sol: 0.01,
            priced_from_pool: false,
            fill_price_sol: 0.01,
            gross_sol: proceeds,
            referral_fee_sol: 0.0,
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
            net_proceeds_sol: proceeds,
            model: PaperModel::Quote,
            quote: Some(quote),
        }),
        paper_context: None,
    })
}

async fn wait_for_manual_pending(handle: &PaperExecutionHandle, task_id: i64, mint: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while !handle.manual_close_pending(task_id, mint, 0) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn manual_close_uses_the_current_holding_after_an_earlier_fifo_partial_sell() {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = database
        .insert_task(task(Some("Manual close")))
        .await
        .unwrap();
    let now = Utc::now();
    database
        .record_outcome(paper_buy_outcome(
            &configured,
            "manual-mint",
            "buy",
            100.0,
            now,
        ))
        .await
        .unwrap();
    let initial = database
        .paper_position(configured.id, "manual-mint")
        .await
        .unwrap()
        .unwrap();

    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let task_id = configured.id;
    let (barrier_started, barrier_started_rx) = oneshot::channel();
    let (release_barrier, release_barrier_rx) = oneshot::channel();
    handle
        .enqueue(
            task_id,
            "manual-close-barrier".to_owned(),
            database.clone(),
            None,
            async move {
                let _ = barrier_started.send(());
                let _ = release_barrier_rx.await;
                crate::trader::copy::paper_execution::skipped_paper_intent(
                    task_id,
                    "manual-close-barrier-result".to_owned(),
                    "manual-mint",
                    CopySkip::PaperExecutionUnavailable,
                    0,
                )
            },
        )
        .unwrap();
    barrier_started_rx.await.unwrap();
    let partial = paper_sell_outcome(
        &configured,
        &initial,
        "partial-before-manual-close",
        40.0,
        0,
        now + chrono::Duration::milliseconds(1),
        false,
    );
    handle
        .enqueue(
            configured.id,
            "earlier-partial-sell".to_owned(),
            database.clone(),
            Some("manual-mint".to_owned()),
            async move { partial },
        )
        .unwrap();

    let observed = Arc::new(Mutex::new(None));
    let observed_in_job = observed.clone();
    let close_database = database.clone();
    let close = tokio::spawn(async move {
        crate::trader::copy::workspace::book::close_paper_holding_with(
            close_database,
            task_id,
            "manual-mint",
            initial.opened_at,
            move |_, current_task, position, epoch, queued_at| {
                let observed = observed_in_job.clone();
                async move {
                    *observed.lock().unwrap() = Some((position.token_amount, position.sells));
                    Ok(paper_sell_outcome(
                        &current_task,
                        &position,
                        "manual-close-after-partial",
                        position.token_amount,
                        epoch,
                        queued_at,
                        true,
                    ))
                }
            },
        )
        .await
    });
    wait_for_manual_pending(&handle, task_id, "manual-mint").await;
    let _ = release_barrier.send(());

    let closed = close.await.unwrap().unwrap();
    assert_eq!(closed.task_id, task_id);
    assert_eq!(*observed.lock().unwrap(), Some((60.0, 1)));
    let position = database
        .paper_position(task_id, "manual-mint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(position.token_amount, 0.0);
    assert_eq!(position.sells, 2);
    let _ = stop.send(true);
    dispatcher.await.unwrap();
}

#[tokio::test]
async fn manual_close_rejects_a_reopened_round_before_quoting() {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = database
        .insert_task(task(Some("Manual close")))
        .await
        .unwrap();
    let opened_at = Utc::now();
    database
        .record_outcome(paper_buy_outcome(
            &configured,
            "reopened-mint",
            "first-round-buy",
            100.0,
            opened_at,
        ))
        .await
        .unwrap();
    let opened_at_original = database
        .paper_position(configured.id, "reopened-mint")
        .await
        .unwrap()
        .unwrap()
        .opened_at;
    let path = directory.path().join("copy_trading.db");

    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let task_id = configured.id;
    let (barrier_started, barrier_started_rx) = oneshot::channel();
    let (release_barrier, release_barrier_rx) = oneshot::channel();
    handle
        .enqueue(
            task_id,
            "reopen-barrier".to_owned(),
            database.clone(),
            None,
            async move {
                let _ = barrier_started.send(());
                let _ = release_barrier_rx.await;
                crate::trader::copy::paper_execution::skipped_paper_intent(
                    task_id,
                    "reopen-barrier-result".to_owned(),
                    "reopened-mint",
                    CopySkip::PaperExecutionUnavailable,
                    0,
                )
            },
        )
        .unwrap();
    barrier_started_rx.await.unwrap();

    let quote_calls = Arc::new(Mutex::new(0));
    let quote_calls_in_job = quote_calls.clone();
    let close_database = database.clone();
    let close = tokio::spawn(async move {
        crate::trader::copy::workspace::book::close_paper_holding_with(
            close_database,
            task_id,
            "reopened-mint",
            opened_at_original,
            move |_, _, _, _, _| {
                let quote_calls = quote_calls_in_job.clone();
                async move {
                    *quote_calls.lock().unwrap() += 1;
                    Err(CopySkip::PaperExecutionUnavailable)
                }
            },
        )
        .await
    });
    wait_for_manual_pending(&handle, task_id, "reopened-mint").await;
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE copy_paper_positions SET token_amount = 75, opened_at = ?3 \
             WHERE task_id = ?1 AND mint = ?2",
            rusqlite::params![
                task_id,
                "reopened-mint",
                (opened_at + chrono::Duration::seconds(1)).to_rfc3339()
            ],
        )
        .unwrap();
    drop(connection);
    let _ = release_barrier.send(());

    assert!(close.await.unwrap().is_err());
    assert_eq!(*quote_calls.lock().unwrap(), 0);
    let position = database
        .paper_position(task_id, "reopened-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(position.is_open());
    assert_eq!(position.token_amount, 75.0);
    assert_eq!(position.opened_at, opened_at + chrono::Duration::seconds(1));
    let _ = stop.send(true);
    dispatcher.await.unwrap();
}

#[tokio::test]
async fn manual_close_rejects_a_round_reopened_while_its_quote_is_pending() {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = database
        .insert_task(task(Some("Manual close")))
        .await
        .unwrap();
    let opened_at = Utc::now();
    database
        .record_outcome(paper_buy_outcome(
            &configured,
            "pending-reopen-mint",
            "pending-first-round-buy",
            100.0,
            opened_at,
        ))
        .await
        .unwrap();
    let first_round = database
        .paper_position(configured.id, "pending-reopen-mint")
        .await
        .unwrap()
        .unwrap();

    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let (quote_started, quote_started_rx) = oneshot::channel();
    let quote_started = Arc::new(Mutex::new(Some(quote_started)));
    let (finish_quote, finish_quote_rx) = oneshot::channel();
    let finish_quote_rx = Arc::new(Mutex::new(Some(finish_quote_rx)));
    let close_database = database.clone();
    let task_id = configured.id;
    let close = tokio::spawn(async move {
        crate::trader::copy::workspace::book::close_paper_holding_with(
            close_database,
            task_id,
            "pending-reopen-mint",
            first_round.opened_at,
            move |_, current_task, position, epoch, queued_at| {
                let finish_quote_rx = finish_quote_rx.clone();
                let quote_started = quote_started.clone();
                async move {
                    if let Some(started) = quote_started.lock().unwrap().take() {
                        let _ = started.send(());
                    }
                    let receiver = { finish_quote_rx.lock().unwrap().take() };
                    if let Some(receiver) = receiver {
                        let _ = receiver.await;
                    }
                    Ok(paper_sell_outcome(
                        &current_task,
                        &position,
                        "stale-manual-close-quote",
                        position.token_amount,
                        epoch,
                        queued_at,
                        true,
                    ))
                }
            },
        )
        .await
    });
    quote_started_rx.await.unwrap();

    database
        .record_outcome(paper_sell_outcome(
            &configured,
            &first_round,
            "close-before-reopen-during-quote",
            100.0,
            0,
            opened_at + chrono::Duration::milliseconds(1),
            false,
        ))
        .await
        .unwrap();
    database
        .record_outcome(paper_buy_outcome(
            &configured,
            "pending-reopen-mint",
            "reopen-during-quote",
            75.0,
            opened_at + chrono::Duration::seconds(1),
        ))
        .await
        .unwrap();
    let _ = finish_quote.send(());

    assert!(close.await.unwrap().is_err());
    let position = database
        .paper_position(task_id, "pending-reopen-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(position.is_open());
    assert_eq!(position.token_amount, 75.0);
    assert_eq!(position.opened_at, opened_at + chrono::Duration::seconds(1));
    let _ = stop.send(true);
    dispatcher.await.unwrap();
    assert!(!handle.manual_close_pending(task_id, "pending-reopen-mint", 0));
}

#[tokio::test]
async fn manual_close_rejects_a_stale_confirmed_round_before_queueing() {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let configured = database
        .insert_task(task(Some("Manual close")))
        .await
        .unwrap();
    let task_id = configured.id;
    let opened_at = Utc::now();
    database
        .record_outcome(paper_buy_outcome(
            &configured,
            "stale-round-mint",
            "stale-first-round-buy",
            100.0,
            opened_at,
        ))
        .await
        .unwrap();
    let first_round = database
        .paper_position(task_id, "stale-round-mint")
        .await
        .unwrap()
        .unwrap();
    database
        .record_outcome(paper_sell_outcome(
            &configured,
            &first_round,
            "stale-round-close",
            100.0,
            0,
            opened_at + chrono::Duration::milliseconds(1),
            false,
        ))
        .await
        .unwrap();
    database
        .record_outcome(paper_buy_outcome(
            &configured,
            "stale-round-mint",
            "stale-round-reopen",
            75.0,
            opened_at + chrono::Duration::seconds(1),
        ))
        .await
        .unwrap();
    let reopened_before = database
        .paper_position(task_id, "stale-round-mint")
        .await
        .unwrap()
        .unwrap();
    assert!(reopened_before.is_open());
    assert_ne!(reopened_before.opened_at, first_round.opened_at);
    let spent_before = database
        .task_total_spent(task_id, CopyMode::Paper)
        .await
        .unwrap();
    let activity_before = database.list_task_activity(task_id, 100).await.unwrap();
    let ledger_before = (spent_before, activity_before.len());

    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let quote_calls = Arc::new(Mutex::new(0));
    let quote_calls_in_job = quote_calls.clone();
    let result = crate::trader::copy::workspace::book::close_paper_holding_with(
        database.clone(),
        task_id,
        "stale-round-mint",
        first_round.opened_at,
        move |_, _, _, _, _| {
            let quote_calls = quote_calls_in_job.clone();
            async move {
                *quote_calls.lock().unwrap() += 1;
                Err(CopySkip::PaperExecutionUnavailable)
            }
        },
    )
    .await;

    assert!(matches!(
        result,
        Err(crate::trader::Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable
        })
    ));
    assert_eq!(*quote_calls.lock().unwrap(), 0);
    assert!(!handle.manual_close_pending(task_id, "stale-round-mint", 0));
    let reopened_after = database
        .paper_position(task_id, "stale-round-mint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reopened_after, reopened_before);
    let spent_after = database
        .task_total_spent(task_id, CopyMode::Paper)
        .await
        .unwrap();
    let activity_after = database.list_task_activity(task_id, 100).await.unwrap();
    assert_eq!((spent_after, activity_after.len()), ledger_before);
    let _ = stop.send(true);
    dispatcher.await.unwrap();
}
