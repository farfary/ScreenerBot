// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task patch merging, the per-task book stats, the pause recovery and the removal of a
//! task.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use chrono::Utc;

use super::tasks::{attach_while_stored, delete_task_with};
use super::{book_stats, followed_wallets, merge_task_patch, CopyTradingStatus, TaskRounds};
use crate::positions::{Book, Position};
use crate::trader::copy::paper_execution::PaperExecutionHandle;
use crate::trader::copy::pause::every_pause_reason;
use crate::trader::copy::round_fixtures::{closed, marked, open_round};
use crate::trader::copy::{
    CopyActivityRow, CopyDatabase, CopyOutcome, CopyPauseReason, CopyRecovery, CopySkip, CopyTask,
    ExitMode, SizingMode,
};
use crate::trader::paper::TradingMode;
use crate::trader::types::TradeReason;

pub(super) fn stored() -> CopyTask {
    CopyTask {
        id: 3,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Kept".to_owned()),
        enabled: true,
        mode: TradingMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::Mirror,
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
        paper_started_at: None,
    }
}

#[test]
fn a_partial_patch_changes_only_the_fields_it_names() {
    let merged = merge_task_patch(&stored(), serde_json::json!({ "enabled": false })).unwrap();
    assert!(!merged.enabled);
    assert_eq!(merged.label.as_deref(), Some("Kept"));
    assert_eq!(merged.exit_mode, ExitMode::Mirror);
    assert_eq!(merged.total_budget_native, 5.0);
}

#[test]
fn the_filter_override_is_patchable_and_inherits_when_null() {
    let merged = merge_task_patch(
        &stored(),
        serde_json::json!({ "require_filter_pass": false }),
    )
    .unwrap();
    assert_eq!(merged.require_filter_pass, Some(false));
    let merged = merge_task_patch(
        &stored(),
        serde_json::json!({ "require_filter_pass": null }),
    )
    .unwrap();
    assert_eq!(merged.require_filter_pass, None);
}

/// A task stored with a trailing distance of 12% and no activation of its own: over
/// the default global activation (10%) the merged trail could never sell at a profit,
/// as happens to a stored task when the global activation is lowered under it.
fn stored_with_stale_trail() -> CopyTask {
    let mut task = stored();
    task.exit_policy_overrides.trailing.distance_pct = Some(12.0);
    task
}

pub(super) fn global_policy() -> crate::trader::policy::ExitPolicy {
    crate::trader::policy::ExitPolicy::from_trader_config(&crate::config::TraderConfig::default())
}

#[test]
fn a_task_whose_stored_overrides_no_longer_fit_still_pauses_resumes_and_takes_edits() {
    let original = stored_with_stale_trail();
    assert!(original
        .exit_policy_overrides
        .validate_over(global_policy())
        .is_err());
    for patch in [
        serde_json::json!({ "enabled": false }),
        serde_json::json!({ "enabled": true }),
        serde_json::json!({ "label": "Renamed" }),
    ] {
        let task = super::tasks::patched_task(&original, patch.clone(), global_policy())
            .unwrap_or_else(|error| panic!("{patch} was refused: {error}"));
        assert_eq!(task.exit_policy_overrides, original.exit_policy_overrides);
    }
}

#[test]
fn a_patch_that_changes_the_overrides_is_checked_on_the_merged_policy() {
    let original = stored_with_stale_trail();
    let still_wide = super::tasks::patched_task(
        &original,
        serde_json::json!({
            "enabled": false,
            "exit_policy_overrides": { "trailing": { "distance_pct": 15.0 } }
        }),
        global_policy(),
    );
    assert!(matches!(
        still_wide,
        Err(crate::trader::Error::CopyTaskRejected {
            reason: crate::trader::copy::CopySkip::InvalidExitPolicy
        })
    ));
    let fixed = super::tasks::patched_task(
        &original,
        serde_json::json!({
            "exit_policy_overrides": { "trailing": { "activation_pct": 30.0, "distance_pct": 12.0 } }
        }),
        global_policy(),
    )
    .expect("a distance below the task's own activation");
    assert_eq!(
        fixed.exit_policy_overrides.trailing.activation_pct,
        Some(30.0)
    );
}

#[test]
fn a_task_whose_overrides_no_longer_fit_is_flagged_with_the_rule_they_break() {
    let issue = super::books::exit_policy_issue(&stored_with_stale_trail(), global_policy())
        .expect("a 12% distance over the 10% global activation is flagged");
    assert_eq!(issue.id, "copy-exit-issue-warning");
    let crate::i18n::UiArg::Nested(reason) = &issue.args["reason"] else {
        panic!("the reason is the broken rule's own text");
    };
    assert_eq!(
        reason.id,
        "copy-exit-issue-trailing-distance-above-activation"
    );
    let en: crate::i18n::LanguageIdentifier = "en".parse().unwrap();
    assert!(!issue.render_plain(&en).is_empty());

    assert_eq!(
        super::books::exit_policy_issue(&stored(), global_policy()),
        None
    );
}

fn paper_row(id: i64, mint: &str, at: chrono::DateTime<Utc>, sell: Option<f64>) -> CopyActivityRow {
    use crate::trader::copy::{CopyOutcome, CopySellDecision, CopyTelemetry, PaperDecision};
    use crate::trader::paper::{PaperFill, PaperQuoteMetadata, PaperSellFill};
    let quote = |output_amount, input_amount| PaperQuoteMetadata {
        router_id: "fixture".to_owned(),
        initial_at: at,
        landing_at: at,
        requested_delay_ms: 1000,
        queue_duration_ms: 0,
        minimum_output_amount: output_amount,
        final_output_amount: output_amount,
        input_amount: Some(input_amount),
        token_decimals: Some(6),
        epoch: 0,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    };
    let telemetry = CopyTelemetry {
        target_block_time: Some(at.timestamp() - 1),
        detected_at: at,
        decoded_at: at,
        decided_at: at,
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: None,
        fill_price_sol: None,
        backfill: false,
    };
    let (kind, outcome) = match sell {
        None => (
            "paper_filled",
            CopyOutcome::PaperFilled(PaperDecision {
                task_id: 3,
                target_address: "target".to_owned(),
                signature: format!("buy-{id}"),
                mint: mint.to_owned(),
                target_size_sol: 1.0,
                target_token_amount: 100.0,
                sized_sol: 1.0,
                fill: PaperFill {
                    input_sol: 1.0,
                    market_price_sol: 0.01,
                    priced_from_pool: false,
                    fill_price_sol: 0.01,
                    token_amount: 100.0,
                    referral_fee_sol: 0.0,
                    network_fee_sol: 0.0,
                    priority_fee_sol: 0.0,
                    total_cost_sol: 1.0,
                    model: crate::trader::paper::PaperModel::Quote,
                    quote: Some(quote(
                        crate::chains::RawAmount::from(100_000_000_u64),
                        crate::chains::RawAmount::from(1_000_000_000_u64),
                    )),
                },
                telemetry,
            }),
        ),
        Some(proceeds) => (
            "paper_sell_observed",
            CopyOutcome::PaperSellObserved(CopySellDecision {
                task_id: 3,
                target_address: "target".to_owned(),
                target_signature: format!("sell-{id}"),
                mint: mint.to_owned(),
                target_token_amount: 100.0,
                target_sol_amount: proceeds,
                exit_percentage: None,
                transaction_signature: None,
                error: None,
                telemetry,
                paper_fill: Some(PaperSellFill {
                    token_amount: 100.0,
                    market_price_sol: proceeds / 100.0,
                    priced_from_pool: false,
                    fill_price_sol: proceeds / 100.0,
                    gross_sol: proceeds,
                    referral_fee_sol: 0.0,
                    network_fee_sol: 0.0,
                    priority_fee_sol: 0.0,
                    net_proceeds_sol: proceeds,
                    model: crate::trader::paper::PaperModel::Quote,
                    quote: Some(quote(
                        crate::chains::RawAmount::from((proceeds * 1_000_000_000.0) as u64),
                        crate::chains::RawAmount::from(100_000_000_u64),
                    )),
                }),
                exit_rule: None,
                take_profit_level: None,
                paper_context: None,
            }),
        ),
    };
    CopyActivityRow {
        id,
        task_id: 3,
        kind: kind.to_owned(),
        outcome,
        created_at: at,
        earlier_run: false,
    }
}

#[test]
fn book_stats_count_wins_and_losses_from_the_closed_rounds() {
    let now = Utc::now();
    let activity = [
        paper_row(1, "won", now, None),
        paper_row(2, "won", now, Some(1.4)),
        paper_row(3, "lost", now, None),
        paper_row(4, "lost", now, Some(0.7)),
        paper_row(5, "open", now, None),
    ];
    let rounds = TaskRounds {
        positions: vec![
            closed(
                open_round(3, Book::Paper, "won", 0.01, 1.0, 30),
                0.4,
                TradeReason::CopySell,
                now,
            ),
            closed(
                open_round(3, Book::Paper, "lost", 0.01, 1.0, 20),
                -0.3,
                TradeReason::StopLoss,
                now,
            ),
            open_round(3, Book::Paper, "open", 0.01, 1.0, 10),
        ],
        run_start: Some(now - chrono::Duration::hours(1)),
    };
    let (stats, closed_rounds) = book_stats(&stored(), &activity, &rounds);
    assert_eq!(closed_rounds.len(), 2);
    assert_eq!((stats.wins, stats.losses), (1, 1));
    assert_eq!(stats.filled_buys, 3);
    assert_eq!(stats.target_sells, 2);
    assert_eq!((stats.open_positions, stats.policy_exits), (1, 1));
}

#[test]
fn decisions_before_the_current_paper_run_do_not_count() {
    let now = Utc::now();
    let start = now - chrono::Duration::minutes(5);
    let activity = [
        paper_row(1, "earlier", start - chrono::Duration::minutes(1), None),
        paper_row(2, "current", now, None),
    ];
    let rounds = TaskRounds {
        positions: Vec::new(),
        run_start: Some(start),
    };
    let (stats, _) = book_stats(&stored(), &activity, &rounds);
    assert_eq!(stats.filled_buys, 1);
    assert_eq!(stats.decisions, 1);
}

#[test]
fn legacy_paper_rows_do_not_enter_current_task_totals() {
    let now = Utc::now();
    let mut buy = paper_row(1, "legacy", now, None);
    if let CopyOutcome::PaperFilled(decision) = &mut buy.outcome {
        decision.fill.model = crate::trader::paper::PaperModel::Legacy;
        decision.fill.quote = None;
    }
    let mut sell = paper_row(2, "legacy", now, Some(2.0));
    if let CopyOutcome::PaperSellObserved(decision) = &mut sell.outcome {
        let fill = decision.paper_fill.as_mut().unwrap();
        fill.model = crate::trader::paper::PaperModel::Legacy;
        fill.quote = None;
    }
    let activity = [buy, sell];

    let (stats, rounds) = book_stats(&stored(), &activity, &TaskRounds::default());
    assert!(rounds.is_empty());
    assert_eq!(stats.decisions, 0);
    assert_eq!(stats.filled_buys, 0);
    assert_eq!(stats.target_sells, 0);
}

#[test]
fn a_patch_with_an_unknown_or_mistyped_field_is_refused() {
    assert!(merge_task_patch(&stored(), serde_json::json!({ "enabld": false })).is_err());
    assert!(merge_task_patch(&stored(), serde_json::json!({ "enabled": "no" })).is_err());
    assert!(merge_task_patch(&stored(), serde_json::json!([1])).is_err());
}

fn copy_status(
    enabled: bool,
    blocked_reason: Option<&'static str>,
    active_tasks: usize,
) -> CopyTradingStatus {
    CopyTradingStatus {
        enabled,
        live_available: true,
        blocked_reason,
        default_mode: "paper".to_owned(),
        default_slippage_pct: 1.0,
        force_stop_blocks: true,
        total_tasks: active_tasks + 1,
        active_tasks,
        paper_tasks: active_tasks,
        live_tasks: 0,
        resumable_tasks: 0,
        resumable_live_tasks: 0,
    }
}

/// Copying needs the system on, no force stop and an enabled task; a loss-limit block
/// still copies exits.
#[test]
fn copying_needs_the_system_on_an_enabled_task_and_no_force_stop() {
    assert!(copy_status(true, None, 1).is_copying());
    assert!(copy_status(true, Some("loss_limit"), 1).is_copying());
    assert!(!copy_status(false, None, 1).is_copying());
    assert!(!copy_status(true, Some("force_stop"), 1).is_copying());
    assert!(!copy_status(true, None, 0).is_copying());
}

/// Two tasks on one wallet follow one wallet.
#[test]
fn followed_wallets_counts_distinct_targets() {
    let mut second = stored();
    second.id = 4;
    second.enabled = false;
    let mut other = stored();
    other.id = 5;
    other.target_address = "other".to_owned();
    assert_eq!(followed_wallets(&[]), 0);
    assert_eq!(followed_wallets(&[stored(), second, other]), 2);
}

/// What a scripted removal did: the paper rounds it settled and how often it detached
/// and attached the watch source.
#[derive(Default)]
struct Removal {
    settled: Vec<i64>,
    detached: usize,
    attached: usize,
}

/// A copy store holding `task`, as stored with the id the store gives it. A task starts
/// in paper mode; a live one is armed through the confirmed transition, as the user arms
/// it.
async fn store_with(task: CopyTask) -> (tempfile::TempDir, CopyDatabase, CopyTask) {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let mode = task.mode;
    let mut task = database
        .insert_task(CopyTask {
            mode: TradingMode::Paper,
            ..task
        })
        .await
        .unwrap();
    if mode == TradingMode::Live {
        task = database
            .set_task_mode(
                task.id,
                TradingMode::Live,
                Some(crate::trader::paper::LIVE_ARM_CONFIRMATION.to_owned()),
            )
            .await
            .unwrap();
    }
    (directory, database, task)
}

/// Deletes task `id` over scripted reads: `live` open live positions, the paper rounds
/// in `book` at the moment of the settle, and a settle that fails when `settle_fails`.
async fn remove(
    database: &CopyDatabase,
    runtime: Option<PaperExecutionHandle>,
    id: i64,
    live: usize,
    book: Arc<Mutex<Vec<Position>>>,
    settle_fails: bool,
) -> (crate::trader::Result<()>, Removal) {
    let removal = Arc::new(Mutex::new(Removal::default()));
    let (settled, detached, attached) = (removal.clone(), removal.clone(), removal.clone());
    let result = delete_task_with(
        database.clone(),
        runtime,
        id,
        move |_| std::future::ready(live),
        move |_| std::future::ready(book.lock().unwrap().clone()),
        move |rounds: Vec<Position>| {
            let count = rounds.len();
            if !settle_fails {
                settled
                    .lock()
                    .unwrap()
                    .settled
                    .extend(rounds.into_iter().filter_map(|round| round.id));
            }
            std::future::ready(if settle_fails {
                Err(crate::trader::Error::PaperBookUnavailable)
            } else {
                Ok(count)
            })
        },
        move |_| {
            detached.lock().unwrap().detached += 1;
            std::future::ready(Ok(()))
        },
        move |_| {
            attached.lock().unwrap().attached += 1;
            std::future::ready(Ok(()))
        },
    )
    .await;
    let removal = std::mem::take(&mut *removal.lock().unwrap());
    (result, removal)
}

/// A paper buy of the task queued before the delete books its round first, and the
/// delete settles that round too: no round of the task outlives it.
#[tokio::test]
async fn a_delete_settles_the_round_of_a_buy_queued_before_it() {
    let (_directory, database, task) = store_with(stored()).await;
    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let book = Arc::new(Mutex::new(vec![open_round(
        task.id,
        Book::Paper,
        "held",
        0.01,
        0.5,
        30,
    )]));
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let buy_book = book.clone();
    let buy_id = task.id;
    let buy = handle
        .enqueue_step(
            task.id,
            crate::trader::copy::paper_execution::activity_key("queued-buy"),
            database.clone(),
            async move {
                let _ = released.await;
                buy_book.lock().unwrap().push(open_round(
                    buy_id,
                    Book::Paper,
                    "bought",
                    0.02,
                    0.5,
                    1,
                ));
                Ok(())
            },
        )
        .unwrap();
    let delete_database = database.clone();
    let delete_handle = handle.clone();
    let delete_book = book.clone();
    let delete = tokio::spawn(async move {
        remove(
            &delete_database,
            Some(delete_handle),
            task.id,
            0,
            delete_book,
            false,
        )
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        !delete.is_finished(),
        "the delete waits for the buy queued before it"
    );
    release.send(()).unwrap();
    buy.await.unwrap().unwrap();
    let (result, removal) = delete.await.unwrap();
    result.unwrap();
    assert_eq!(
        removal.settled,
        vec![task.id * 1_000 + 30, task.id * 1_000 + 1],
        "the round the queued buy opened is settled with the others"
    );
    assert_eq!(
        (removal.detached, removal.attached),
        (2, 0),
        "detached before the step and once more after the row is gone"
    );
    assert!(database.get_task(task.id).await.unwrap().is_none());
    let _ = stop.send(true);
    dispatcher.await.unwrap();
}

#[tokio::test]
async fn a_delete_without_a_running_queue_settles_and_removes_in_place() {
    let (_directory, database, task) = store_with(stored()).await;
    let book = Arc::new(Mutex::new(vec![open_round(
        task.id,
        Book::Paper,
        "held",
        0.01,
        0.5,
        5,
    )]));
    let (result, removal) = remove(&database, None, task.id, 0, book, false).await;
    result.unwrap();
    assert_eq!(removal.settled, vec![task.id * 1_000 + 5]);
    assert_eq!(
        (removal.detached, removal.attached),
        (2, 0),
        "detached before the step and once more after the row is gone"
    );
    assert!(database.get_task(task.id).await.unwrap().is_none());
}

/// A queue that refuses the removal settles nothing, keeps the task and attaches its
/// watch source again, so the delete can be retried.
#[tokio::test]
async fn a_delete_the_queue_refuses_keeps_the_task_and_its_watch() {
    let (_directory, database, task) = store_with(stored()).await;
    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let _ = stop.send(true);
    dispatcher.await.unwrap();
    let book = Arc::new(Mutex::new(vec![open_round(
        task.id,
        Book::Paper,
        "held",
        0.01,
        0.5,
        5,
    )]));
    let (result, removal) = remove(&database, Some(handle), task.id, 0, book, false).await;
    assert!(matches!(
        result,
        Err(crate::trader::Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable
        })
    ));
    assert!(removal.settled.is_empty());
    assert_eq!((removal.detached, removal.attached), (1, 1));
    assert!(database.get_task(task.id).await.unwrap().is_some());
}

#[tokio::test]
async fn a_failed_settle_keeps_the_task_and_attaches_its_watch_again() {
    let (_directory, database, task) = store_with(stored()).await;
    let book = Arc::new(Mutex::new(vec![open_round(
        task.id,
        Book::Paper,
        "held",
        0.01,
        0.5,
        5,
    )]));
    let (result, removal) = remove(&database, None, task.id, 0, book, true).await;
    assert!(matches!(
        result,
        Err(crate::trader::Error::PaperBookUnavailable)
    ));
    assert_eq!((removal.detached, removal.attached), (1, 1));
    assert!(database.get_task(task.id).await.unwrap().is_some());
}

/// An enabled live task and a task with open live positions are refused before anything
/// is detached or settled.
#[tokio::test]
async fn a_delete_that_could_move_money_is_refused_untouched() {
    let live = CopyTask {
        mode: TradingMode::Live,
        ..stored()
    };
    let (_directory, database, task) = store_with(live).await;
    let book = Arc::new(Mutex::new(vec![open_round(
        task.id,
        Book::Paper,
        "held",
        0.01,
        0.5,
        5,
    )]));
    let (result, removal) = remove(&database, None, task.id, 0, book.clone(), false).await;
    assert!(matches!(
        result,
        Err(crate::trader::Error::CopyTaskLive { task_id }) if task_id == task.id
    ));
    assert!(removal.settled.is_empty());
    assert_eq!((removal.detached, removal.attached), (0, 0));

    let paused = CopyTask {
        enabled: false,
        ..task.clone()
    };
    let (_owned_directory, owned_database, paused) = store_with(paused).await;
    let (result, removal) = remove(&owned_database, None, paused.id, 2, book, false).await;
    assert!(matches!(
        result,
        Err(crate::trader::Error::CopyTaskOwnsPositions {
            task_id,
            open_positions: 2,
        }) if task_id == paused.id
    ));
    assert!(removal.settled.is_empty());
    assert_eq!((removal.detached, removal.attached), (0, 0));
    assert!(database.get_task(task.id).await.unwrap().is_some());
    assert!(owned_database.get_task(paused.id).await.unwrap().is_some());
}

/// The watch sources a scripted attach and detach leave attached, by task id.
type Watched = Arc<Mutex<HashSet<i64>>>;

fn attach_to(
    watched: &Watched,
) -> impl Fn(CopyTask) -> std::future::Ready<crate::trader::Result<()>> {
    let watched = watched.clone();
    move |task| {
        watched.lock().unwrap().insert(task.id);
        std::future::ready(Ok(()))
    }
}

fn detach_from(
    watched: &Watched,
) -> impl Fn(CopyTask) -> std::future::Ready<crate::trader::Result<()>> {
    let watched = watched.clone();
    move |task| {
        watched.lock().unwrap().remove(&task.id);
        std::future::ready(Ok(()))
    }
}

/// An update that lands after the delete detached the watch source, while the delete
/// waits in the task's paper queue, attaches the source again; the delete releases it
/// once the row is gone, so no source of the deleted task stays attached.
#[tokio::test]
async fn an_update_while_the_delete_waits_leaves_no_watch_source() {
    let (_directory, database, task) = store_with(stored()).await;
    let watched: Watched = Arc::new(Mutex::new(HashSet::from([task.id])));
    let (handle, stop, dispatcher) = crate::trader::copy::paper_execution::install();
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let queued = handle
        .enqueue_step(
            task.id,
            crate::trader::copy::paper_execution::activity_key("queued-buy"),
            database.clone(),
            async move {
                let _ = released.await;
                Ok(())
            },
        )
        .unwrap();
    let delete = tokio::spawn({
        let (database, handle, attach, detach) = (
            database.clone(),
            handle.clone(),
            attach_to(&watched),
            detach_from(&watched),
        );
        async move {
            delete_task_with(
                database,
                Some(handle),
                task.id,
                |_| std::future::ready(0),
                |_| std::future::ready(Vec::<Position>::new()),
                |_: Vec<Position>| std::future::ready(Ok(0)),
                detach,
                attach,
            )
            .await
        }
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        !delete.is_finished(),
        "the delete waits for the queued step"
    );
    assert!(!watched.lock().unwrap().contains(&task.id));

    let updated = database
        .update_task(CopyTask {
            label: Some("Edited".to_owned()),
            ..task.clone()
        })
        .await
        .unwrap();
    attach_while_stored(
        &database,
        updated,
        attach_to(&watched),
        detach_from(&watched),
    )
    .await
    .unwrap();
    assert!(
        watched.lock().unwrap().contains(&task.id),
        "the update attached the source again while the row still existed"
    );

    release.send(()).unwrap();
    queued.await.unwrap().unwrap();
    delete.await.unwrap().unwrap();
    assert!(database.get_task(task.id).await.unwrap().is_none());
    assert!(
        watched.lock().unwrap().is_empty(),
        "no watch source of the deleted task remains attached"
    );
    let _ = stop.send(true);
    dispatcher.await.unwrap();
}

/// An attach keeps the source while the task row exists, and releases it when the row
/// was deleted before the attach finished.
#[tokio::test]
async fn an_attach_never_outlives_the_task_row() {
    let (_directory, database, task) = store_with(stored()).await;
    let watched: Watched = Arc::new(Mutex::new(HashSet::new()));
    attach_while_stored(
        &database,
        task.clone(),
        attach_to(&watched),
        detach_from(&watched),
    )
    .await
    .unwrap();
    assert!(watched.lock().unwrap().contains(&task.id));

    assert!(database.delete_task(task.id).await.unwrap());
    attach_while_stored(
        &database,
        task.clone(),
        attach_to(&watched),
        detach_from(&watched),
    )
    .await
    .unwrap();
    assert!(watched.lock().unwrap().is_empty());
}

pub(super) fn budget_pause() -> CopyPauseReason {
    CopyPauseReason::WatchBudgetExceeded {
        page_budget: 5,
        signatures_checked: 500,
    }
}

/// Every pause reason, from the one list of them, comes back the way its kind states:
/// a resume, a watch retry or a restore of missed activity.
#[test]
fn every_pause_reason_maps_to_its_recovery() {
    let stated = [
        ("user", CopyRecovery::Resume),
        ("latency_kill_switch", CopyRecovery::Resume),
        ("watch_detached", CopyRecovery::Resume),
        ("storage_restored", CopyRecovery::Resume),
        ("helius_unavailable", CopyRecovery::RetryWatch),
        ("watch_processing_failed", CopyRecovery::RetryWatch),
        ("watch_budget_exceeded", CopyRecovery::RestoreWatch),
    ];
    for reason in every_pause_reason() {
        let kind = serde_json::to_value(&reason).unwrap()["kind"]
            .as_str()
            .unwrap()
            .to_owned();
        let (_, recovery) = stated
            .iter()
            .find(|(name, _)| *name == kind)
            .unwrap_or_else(|| panic!("no recovery stated for {kind}"));
        assert_eq!(reason.recovery(), *recovery, "{reason:?}");
        let mut task = stored();
        task.enabled = false;
        task.pause_reason = Some(reason.clone());
        assert_eq!(task.recovery(), Some(*recovery), "{reason:?}");
    }
    assert_eq!(
        serde_json::to_value(CopyRecovery::RestoreWatch).unwrap(),
        "restore_watch"
    );
}

/// An enabled task has no recovery, and a pause stored without a reason comes back
/// with a resume.
#[test]
fn only_a_paused_task_has_a_recovery() {
    assert_eq!(stored().recovery(), None);
    let mut unexplained = stored();
    unexplained.enabled = false;
    assert_eq!(unexplained.recovery(), Some(CopyRecovery::Resume));
}

/// The resumable counts take the paused tasks whose recovery is a resume, in either
/// mode, and never an enabled task or a watch-recovery pause.
#[test]
fn the_status_counts_resumable_and_resumable_live_tasks() {
    crate::config::utils::install_default_config();
    let task = |id: i64, mode: TradingMode, pause: Option<CopyPauseReason>| CopyTask {
        id,
        mode,
        enabled: pause.is_none(),
        pause_reason: pause,
        ..stored()
    };
    let tasks = [
        task(1, TradingMode::Paper, None),
        task(2, TradingMode::Live, None),
        task(3, TradingMode::Paper, Some(CopyPauseReason::User)),
        task(4, TradingMode::Live, Some(CopyPauseReason::WatchDetached)),
        task(5, TradingMode::Live, Some(CopyPauseReason::StorageRestored)),
        task(6, TradingMode::Paper, Some(budget_pause())),
        task(
            7,
            TradingMode::Live,
            Some(CopyPauseReason::HeliusUnavailable),
        ),
    ];
    let status = super::build_status(&tasks);
    assert_eq!((status.active_tasks, status.live_tasks), (2, 1));
    assert_eq!(status.resumable_tasks, 3);
    assert_eq!(status.resumable_live_tasks, 2);

    let none_paused = super::build_status(&tasks[..2]);
    assert_eq!(
        (
            none_paused.resumable_tasks,
            none_paused.resumable_live_tasks
        ),
        (0, 0)
    );
}

/// The summary's net P&L is the book's realized plus unrealized P&L, an unpriced
/// holding counting as zero, and its recovery follows the pause.
#[test]
fn the_summary_carries_net_pnl_and_the_recovery() {
    crate::config::utils::install_default_config();
    let now = Utc::now();
    let rounds = TaskRounds {
        positions: vec![
            closed(
                open_round(3, Book::Paper, "won", 0.01, 1.0, 30),
                0.4,
                TradeReason::CopySell,
                now,
            ),
            closed(
                open_round(3, Book::Paper, "lost", 0.01, 1.0, 20),
                -0.3,
                TradeReason::StopLoss,
                now,
            ),
            marked(
                open_round(3, Book::Paper, "priced", 0.01, 1.0, 10),
                Some(0.012),
            ),
            marked(open_round(3, Book::Paper, "unpriced", 0.01, 1.0, 15), None),
        ],
        run_start: Some(now - chrono::Duration::hours(1)),
    };
    let status = super::build_status(&[stored()]);
    let (summary, _) = super::summarize(&status, stored(), &[], &rounds, 0.0);
    assert_eq!(summary.stats.unpriced_positions, 1);
    assert_eq!(
        summary.net_pnl_native,
        summary.stats.realized_pnl_native + summary.stats.unrealized_pnl_native
    );
    assert!(
        (summary.net_pnl_native - 0.3).abs() < 1e-9,
        "{}",
        summary.net_pnl_native
    );
    assert_eq!(summary.recovery, None);

    let mut paused = stored();
    paused.enabled = false;
    paused.pause_reason = Some(budget_pause());
    let (summary, _) = super::summarize(&status, paused, &[], &rounds, 0.0);
    assert_eq!(summary.recovery, Some(CopyRecovery::RestoreWatch));
    assert_eq!(
        serde_json::to_value(&summary).unwrap()["recovery"],
        "restore_watch"
    );
}
