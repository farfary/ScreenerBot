// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper maintenance of a copy task over a real copy store and scripted paper book
//! reads: the run reset and the manual close of one round.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use super::*;
use crate::positions::PositionOrigin;
use crate::trader::copy::round_fixtures::open_round;
use crate::trader::copy::workspace::tests::task;
use crate::trader::paper::{PaperModel, PaperRefusal};

const MINT: &str = "manual-close-mint";

async fn store(mode: TradingMode) -> (tempfile::TempDir, CopyDatabase, CopyTask) {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let task = database
        .insert_task(CopyTask {
            mode,
            ..task(Some("Paper maintenance"))
        })
        .await
        .unwrap();
    (directory, database, task)
}

/// An open paper round of `task` from its own target wallet.
fn round_of(task: &CopyTask, mint: &str, minutes_ago: i64) -> Position {
    let mut position = open_round(task.id, Book::Paper, mint, 0.01, 0.5, minutes_ago);
    position.origin = PositionOrigin::Copy {
        task_id: task.id,
        source_wallet: task.target_address.clone(),
    };
    position.current_price = Some(0.012);
    position
}

fn sell_fill() -> PaperSellFill {
    PaperSellFill {
        token_amount: 50.0,
        market_price_sol: 0.012,
        priced_from_pool: false,
        fill_price_sol: 0.0118,
        gross_sol: 0.59,
        referral_fee_sol: 0.0,
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
        net_proceeds_sol: 0.589995,
        model: PaperModel::Quote,
        quote: None,
    }
}

/// A scripted read of the paper book's open round of a mint.
fn reads(round: Option<Position>) -> impl Fn(String) -> std::future::Ready<Option<Position>> {
    move |_| std::future::ready(round.clone())
}

#[tokio::test]
async fn a_reset_closes_the_task_rounds_it_is_given_and_starts_a_new_run_on_the_queue() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let rounds = vec![round_of(&task, "one", 10), round_of(&task, "two", 20)];
    let settled = Arc::new(Mutex::new(Vec::new()));
    let settled_in_job = settled.clone();
    let before = Utc::now();
    let reset = reset_paper_book_with(
        database.clone(),
        Some(handle),
        task.id,
        move |_| std::future::ready(Ok(rounds)),
        move |rounds: Vec<Position>| {
            let count = rounds.len();
            settled_in_job
                .lock()
                .unwrap()
                .extend(rounds.into_iter().filter_map(|round| round.id));
            std::future::ready(Ok(count))
        },
    )
    .await
    .unwrap();
    assert_eq!((reset.task_id, reset.closed_rounds), (task.id, 2));
    assert_eq!(
        *settled.lock().unwrap(),
        vec![task.id * 1_000 + 10, task.id * 1_000 + 20]
    );
    let started = database
        .get_task(task.id)
        .await
        .unwrap()
        .unwrap()
        .paper_started_at
        .expect("the reset starts a new run");
    assert!(started >= before);
    assert!(
        database
            .list_task_activity(task.id, 10)
            .await
            .unwrap()
            .is_empty(),
        "a reset records no decision and deletes none"
    );
    let _ = stop.send(true);
    dispatcher.await.unwrap();
}

#[tokio::test]
async fn a_reset_without_a_running_queue_runs_in_place() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let reset = reset_paper_book_with(
        database.clone(),
        None,
        task.id,
        |_| std::future::ready(Ok(Vec::new())),
        |rounds: Vec<Position>| std::future::ready(Ok(rounds.len())),
    )
    .await
    .unwrap();
    assert_eq!(reset.closed_rounds, 0);
    assert!(database
        .get_task(task.id)
        .await
        .unwrap()
        .unwrap()
        .paper_started_at
        .is_some());
}

#[tokio::test]
async fn a_live_task_is_never_reset() {
    let (_directory, database, task) = store(TradingMode::Live).await;
    let settled = Arc::new(Mutex::new(false));
    let settled_in_job = settled.clone();
    let result = reset_paper_book_with(
        database.clone(),
        None,
        task.id,
        |_| std::future::ready(Ok(Vec::new())),
        move |_: Vec<Position>| {
            *settled_in_job.lock().unwrap() = true;
            std::future::ready(Ok(0))
        },
    )
    .await;
    assert!(matches!(result, Err(Error::CopyValidation { .. })));
    assert!(!*settled.lock().unwrap());
    assert_eq!(
        database
            .get_task(task.id)
            .await
            .unwrap()
            .unwrap()
            .paper_started_at,
        None
    );
}

async fn close_with(
    database: &CopyDatabase,
    task: &CopyTask,
    opened_at: DateTime<Utc>,
    first: Option<Position>,
    queued: Option<Position>,
    result: CopySellSubmitResult,
) -> (Result<ClosedHolding>, usize) {
    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let calls = Arc::new(Mutex::new(0));
    let calls_in_job = calls.clone();
    let reads_seen = Arc::new(Mutex::new(0));
    let read = move |_: String| {
        let mut seen = reads_seen.lock().unwrap();
        *seen += 1;
        std::future::ready(if *seen == 1 {
            first.clone()
        } else {
            queued.clone()
        })
    };
    let closed = close_paper_holding_with(
        database.clone(),
        Some(handle),
        task.id,
        MINT,
        opened_at,
        read,
        move |_| {
            *calls_in_job.lock().unwrap() += 1;
            std::future::ready(result)
        },
    )
    .await;
    let _ = stop.send(true);
    dispatcher.await.unwrap();
    let calls = *calls.lock().unwrap();
    (closed, calls)
}

#[tokio::test]
async fn a_manual_close_sells_the_confirmed_round_and_records_it_once() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let round = round_of(&task, MINT, 30);
    let (closed, calls) = close_with(
        &database,
        &task,
        round.entry_time,
        Some(round.clone()),
        Some(round.clone()),
        CopySellSubmitResult::PaperFilled { fill: sell_fill() },
    )
    .await;
    let closed = closed.unwrap();
    assert_eq!(calls, 1);
    assert_eq!((closed.task_id, closed.mint.as_str()), (task.id, MINT));
    assert_eq!(closed.fill_price_native, 0.0118);
    let activity = database.list_task_activity(task.id, 10).await.unwrap();
    assert_eq!(activity.len(), 1);
    let CopyOutcome::PaperSellObserved(decision) = &activity[0].outcome else {
        panic!("expected a paper sell, got {:?}", activity[0].outcome);
    };
    assert_eq!(decision.exit_rule, Some(PaperExitRule::Manual));
    assert_eq!(
        decision.target_signature,
        format!("manual-close:{}", round.id.unwrap())
    );
    assert_eq!(decision.paper_fill, Some(sell_fill()));
    assert_eq!(decision.paper_context, Some(quote_context()));
}

#[tokio::test]
async fn a_manual_close_of_a_round_other_than_the_confirmed_one_never_sells() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let round = round_of(&task, MINT, 30);
    let (stale, calls) = close_with(
        &database,
        &task,
        round.entry_time - chrono::Duration::seconds(1),
        Some(round.clone()),
        Some(round.clone()),
        CopySellSubmitResult::PaperFilled { fill: sell_fill() },
    )
    .await;
    assert!(matches!(
        stale,
        Err(Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable
        })
    ));
    assert_eq!(calls, 0);
    assert!(database
        .list_task_activity(task.id, 10)
        .await
        .unwrap()
        .is_empty());

    let mut strategy_round = round.clone();
    strategy_round.origin = PositionOrigin::Auto {
        strategy_id: Some("a-strategy".to_owned()),
    };
    let (foreign, calls) = close_with(
        &database,
        &task,
        round.entry_time,
        Some(strategy_round),
        None,
        CopySellSubmitResult::PaperFilled { fill: sell_fill() },
    )
    .await;
    assert!(matches!(foreign, Err(Error::CopyHoldingNotFound { .. })));
    assert_eq!(calls, 0);
}

#[tokio::test]
async fn a_round_closed_while_its_manual_close_waited_is_recorded_as_not_found() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let round = round_of(&task, MINT, 30);
    let mut reopened = round_of(&task, MINT, 5);
    reopened.entry_time = Utc::now();
    let (closed, calls) = close_with(
        &database,
        &task,
        round.entry_time,
        Some(round),
        Some(reopened),
        CopySellSubmitResult::PaperFilled { fill: sell_fill() },
    )
    .await;
    assert!(matches!(
        closed,
        Err(Error::CopyTaskRejected {
            reason: CopySkip::CopyPositionNotFound
        })
    ));
    assert_eq!(calls, 0);
    let activity = database.list_task_activity(task.id, 10).await.unwrap();
    assert!(matches!(
        &activity[0].outcome,
        CopyOutcome::Skipped { signature, reason: CopySkip::CopyPositionNotFound, .. }
            if signature.starts_with("manual-close-skip:")
    ));
}

#[tokio::test]
async fn a_refused_manual_close_is_recorded_with_its_typed_reason() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let round = round_of(&task, MINT, 30);
    let (closed, calls) = close_with(
        &database,
        &task,
        round.entry_time,
        Some(round.clone()),
        Some(round),
        CopySellSubmitResult::PaperRefused {
            refusal: PaperRefusal::DecimalsUnknown,
        },
    )
    .await;
    assert_eq!(calls, 1);
    assert!(matches!(
        closed,
        Err(Error::CopyTaskRejected {
            reason: CopySkip::PaperQuoteUnfillable
        })
    ));
    let activity = database.list_task_activity(task.id, 10).await.unwrap();
    assert!(matches!(
        &activity[0].outcome,
        CopyOutcome::Skipped {
            reason: CopySkip::PaperQuoteUnfillable,
            paper: Some(_),
            ..
        }
    ));
}

#[tokio::test]
async fn a_manual_close_without_a_running_queue_is_recorded_and_refused() {
    let (_directory, database, task) = store(TradingMode::Paper).await;
    let round = round_of(&task, MINT, 30);
    let result = close_paper_holding_with(
        database.clone(),
        None,
        task.id,
        MINT,
        round.entry_time,
        reads(Some(round)),
        |_| std::future::ready(CopySellSubmitResult::PaperFilled { fill: sell_fill() }),
    )
    .await;
    assert!(matches!(
        result,
        Err(Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable
        })
    ));
    let activity = database.list_task_activity(task.id, 10).await.unwrap();
    assert!(matches!(
        &activity[0].outcome,
        CopyOutcome::Skipped { signature, .. } if signature.starts_with("manual-close-unavailable:")
    ));
}
