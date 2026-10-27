// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Deterministic tests for the service-owned paper dispatcher.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn database() -> (tempfile::TempDir, CopyDatabase) {
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    (directory, database)
}

fn paper_task() -> super::super::CopyTask {
    let now = chrono::Utc::now();
    super::super::CopyTask {
        id: 0,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: crate::trader::paper::TradingMode::Paper,
        sizing: super::super::SizingMode::Fixed { sol: 0.1 },
        exit_mode: super::super::ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.2,
        max_native_per_token: 1.0,
        total_budget_native: 1.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: now,
        updated_at: now,
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
    }
}

fn outcome(task_id: i64, signature: &str) -> CopyOutcome {
    skipped_paper_intent(
        task_id,
        signature.to_owned(),
        "mint",
        super::super::CopySkip::PaperExecutionUnavailable,
        0,
    )
}

fn runner(
    settlement: Settlement,
) -> (
    PaperExecutionHandle,
    watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    ensure_config();
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = PaperExecutionHandle::new(sender);
    let (stop, stopped) = watch::channel(false);
    let join = tokio::spawn(run_dispatcher(
        handle.clone(),
        receiver,
        stopped,
        settlement,
    ));
    (handle, stop, join)
}

fn ensure_config() {
    crate::config::utils::install_default_config();
}

fn immediate_settlement(counter: Arc<AtomicUsize>) -> Settlement {
    Arc::new(move |_database, _outcome| {
        let counter = counter.clone();
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        })
    })
}

fn set_capacity(capacity: usize) -> usize {
    crate::config::utils::install_default_config();
    let config = crate::config::utils::CONFIG.get().unwrap();
    let mut config = config.write().unwrap();
    let previous = config.copy_trading.paper_queue_capacity;
    config.copy_trading.paper_queue_capacity = capacity;
    previous
}

#[tokio::test]
async fn dispatcher_cancels_a_job_when_stop_is_already_set_at_launch() {
    let (_directory, database) = database();
    ensure_config();
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = PaperExecutionHandle::new(sender);
    let (stop, stopped) = watch::channel(true);
    let ran = Arc::new(AtomicUsize::new(0));
    let ran_job = ran.clone();
    let completion = handle
        .enqueue_manual(11, "stopped".into(), "mint".into(), database, async move {
            ran_job.fetch_add(1, Ordering::SeqCst);
            outcome(11, "stopped")
        })
        .unwrap();
    let join = tokio::spawn(run_dispatcher(
        handle.clone(),
        receiver,
        stopped,
        immediate_settlement(Arc::new(AtomicUsize::new(0))),
    ));
    assert!(completion.await.unwrap().is_err());
    assert_eq!(ran.load(Ordering::SeqCst), 0);
    let _ = stop.send(true);
    join.await.unwrap();
}

#[tokio::test]
async fn stop_wins_when_preparation_and_stop_are_ready_together() {
    let (_directory, database) = database();
    let settlements = Arc::new(AtomicUsize::new(0));
    let (handle, stop, join) = runner(immediate_settlement(settlements.clone()));
    let (preparing, preparing_rx) = oneshot::channel();
    let (finish, finish_rx) = oneshot::channel();
    let completion = handle
        .enqueue_manual(
            12,
            "stop-race".into(),
            "mint".into(),
            database,
            async move {
                let _ = preparing.send(());
                let _ = finish_rx.await;
                outcome(12, "stop-race")
            },
        )
        .unwrap();
    preparing_rx.await.unwrap();
    let _ = finish.send(());
    let _ = stop.send(true);
    assert!(completion.await.unwrap().is_err());
    join.await.unwrap();
    assert_eq!(settlements.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn failed_preparation_releases_its_key_and_advances_the_task_queue() {
    let (_directory, database) = database();
    let settlements = Arc::new(AtomicUsize::new(0));
    let (handle, stop, join) = runner(immediate_settlement(settlements.clone()));
    let (next_started, next_started_rx) = oneshot::channel();
    let (readmitted_started, readmitted_started_rx) = oneshot::channel();
    let next_handle = handle.clone();
    let next_database = database.clone();
    handle
        .enqueue(
            13,
            "failed-preparation".into(),
            database.clone(),
            None,
            async {
                panic!("paper preparation failed");
                #[allow(unreachable_code)]
                outcome(13, "never-settled")
            },
        )
        .unwrap();
    handle
        .enqueue(
            13,
            "following-job".into(),
            database.clone(),
            None,
            async move {
                let readmitted_started = Arc::new(Mutex::new(Some(readmitted_started)));
                let notify = readmitted_started.clone();
                let admitted = next_handle
                    .enqueue(
                        13,
                        "failed-preparation".into(),
                        next_database,
                        None,
                        async move {
                            if let Ok(mut notify) = notify.lock() {
                                if let Some(notify) = notify.take() {
                                    let _ = notify.send(());
                                }
                            }
                            outcome(13, "re-admitted")
                        },
                    )
                    .is_ok();
                let _ = next_started.send(admitted);
                outcome(13, "following-job")
            },
        )
        .unwrap();
    assert!(next_started_rx.await.unwrap());
    readmitted_started_rx.await.unwrap();
    let _ = stop.send(true);
    join.await.unwrap();
    assert_eq!(settlements.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn dispatcher_serializes_each_task_and_runs_other_tasks_independently() {
    let (_directory, database) = database();
    let settlements = Arc::new(AtomicUsize::new(0));
    let (handle, stop, join) = runner(immediate_settlement(settlements.clone()));
    let (first_started, first_rx) = oneshot::channel();
    let (release_first, release_rx) = oneshot::channel();
    let (second_started, mut second_rx) = oneshot::channel();
    let (other_started, other_rx) = oneshot::channel();

    handle
        .enqueue(1, "first".into(), database.clone(), None, async move {
            let _ = first_started.send(());
            let _ = release_rx.await;
            outcome(1, "first")
        })
        .unwrap();
    first_rx.await.unwrap();
    handle
        .enqueue(1, "second".into(), database.clone(), None, async move {
            let _ = second_started.send(());
            outcome(1, "second")
        })
        .unwrap();
    handle
        .enqueue(2, "other".into(), database.clone(), None, async move {
            let _ = other_started.send(());
            outcome(2, "other")
        })
        .unwrap();
    other_rx.await.unwrap();
    assert!(second_rx.try_recv().is_err());
    let _ = release_first.send(());
    // The first result settles before the next task-1 job starts.
    second_rx.await.unwrap();
    let _ = stop.send(true);
    join.await.unwrap();
    assert_eq!(settlements.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn duplicate_intents_are_rejected_before_queue_capacity_and_manual_state_tracks_lifecycle() {
    let (_directory, database) = database();
    let settlements = Arc::new(AtomicUsize::new(0));
    let (handle, stop, join) = runner(immediate_settlement(settlements.clone()));
    let (started, started_rx) = oneshot::channel();
    let (release, release_rx) = oneshot::channel();
    let receiver = handle
        .enqueue_manual(
            9,
            "manual:mint:round:0".into(),
            "mint".into(),
            database.clone(),
            async move {
                let _ = started.send(());
                let _ = release_rx.await;
                outcome(9, "manual-close")
            },
        )
        .unwrap();
    assert!(handle.manual_close_pending(9, "mint", 0));
    assert!(!handle.manual_close_pending(9, "mint", 1));
    assert_eq!(
        handle
            .enqueue_manual(
                9,
                "manual:mint:round:0".into(),
                "mint".into(),
                database.clone(),
                async { outcome(9, "duplicate") }
            )
            .unwrap_err(),
        EnqueueError::Duplicate
    );
    started_rx.await.unwrap();
    assert!(handle.manual_close_pending(9, "mint", 0));
    assert!(!handle.manual_close_pending(9, "mint", 1));
    let _ = release.send(());
    let (settled, inserted) = receiver.await.unwrap().unwrap();
    assert!(inserted);
    assert!(matches!(settled, CopyOutcome::Skipped { .. }));
    let _ = stop.send(true);
    join.await.unwrap();
    assert!(!handle.manual_close_pending(9, "mint", 0));
    assert_eq!(settlements.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn shutdown_cancels_quote_stage_releases_manual_pending_and_joins_settlement() {
    let (_directory, database) = database();
    let settlements = Arc::new(AtomicUsize::new(0));
    let (handle, stop, join) = runner(immediate_settlement(settlements.clone()));
    let (started, started_rx) = oneshot::channel();
    let (_release, release_rx) = oneshot::channel::<()>();
    let (completion, completion_rx) = oneshot::channel();
    handle
        .enqueue_inner(
            3,
            "manual:mint:round:0".into(),
            database,
            Some("mint".into()),
            Box::pin(async move {
                let _ = started.send(());
                let _ = release_rx.await;
                outcome(3, "cancelled")
            }),
            Some(completion),
        )
        .unwrap();
    started_rx.await.unwrap();
    assert!(handle.manual_close_pending(3, "mint", 0));
    let _ = stop.send(true);
    join.await.unwrap();
    assert!(!handle.manual_close_pending(3, "mint", 0));
    assert_eq!(settlements.load(Ordering::SeqCst), 0);
    assert!(completion_rx.await.unwrap().is_err());
}

#[tokio::test]
async fn capacity_changes_apply_to_new_admission_and_keep_accepted_fifo_jobs() {
    let (_directory, database) = database();
    let previous_capacity = set_capacity(2);
    let settlements = Arc::new(AtomicUsize::new(0));
    let (handle, stop, join) = runner(immediate_settlement(settlements));
    let (first_started, first_rx) = oneshot::channel();
    let (release_first, release_rx) = oneshot::channel();
    handle
        .enqueue(4, "active".into(), database.clone(), None, async move {
            let _ = first_started.send(());
            let _ = release_rx.await;
            outcome(4, "active")
        })
        .unwrap();
    first_rx.await.unwrap();
    let (order_tx, mut order_rx) = mpsc::unbounded_channel();
    for key in ["queued-a", "queued-b"] {
        let order = order_tx.clone();
        handle
            .enqueue(4, key.into(), database.clone(), None, async move {
                let _ = order.send(key);
                outcome(4, key)
            })
            .unwrap();
    }
    set_capacity(1);
    assert_eq!(
        handle
            .enqueue(4, "shrink-rejected".into(), database.clone(), None, async {
                outcome(4, "shrink")
            })
            .unwrap_err(),
        EnqueueError::Full
    );
    set_capacity(3);
    let order = order_tx.clone();
    handle
        .enqueue(4, "queued-c".into(), database.clone(), None, async move {
            let _ = order.send("queued-c");
            outcome(4, "queued-c")
        })
        .unwrap();
    let _ = release_first.send(());
    assert_eq!(order_rx.recv().await.as_deref(), Some("queued-a"));
    assert_eq!(order_rx.recv().await.as_deref(), Some("queued-b"));
    assert_eq!(order_rx.recv().await.as_deref(), Some("queued-c"));
    let _ = stop.send(true);
    join.await.unwrap();
    set_capacity(previous_capacity);
}

#[tokio::test]
async fn shutdown_waits_for_a_settlement_that_has_started() {
    let (_directory, database) = database();
    let (settlement_started, settlement_started_rx) = oneshot::channel();
    let (finish_settlement, finish_settlement_rx) = oneshot::channel();
    let started = Arc::new(Mutex::new(Some(settlement_started)));
    let finish = Arc::new(Mutex::new(Some(finish_settlement_rx)));
    let settlement: Settlement = Arc::new(move |_database, _outcome| {
        let started = started.clone();
        let finish = finish.clone();
        Box::pin(async move {
            if let Ok(mut started) = started.lock() {
                if let Some(started) = started.take() {
                    let _ = started.send(());
                }
            }
            let receiver = finish.lock().ok().and_then(|mut finish| finish.take());
            let Some(receiver) = receiver else {
                return Err(crate::trader::Error::CopyTaskRejected {
                    reason: super::super::CopySkip::PaperExecutionUnavailable,
                });
            };
            receiver
                .await
                .map_err(|_| crate::trader::Error::CopyTaskRejected {
                    reason: super::super::CopySkip::PaperExecutionUnavailable,
                })?;
            Ok(true)
        })
    });
    let (handle, stop, join) = runner(settlement);
    let (completion, completion_rx) = oneshot::channel();
    handle
        .enqueue_inner(
            5,
            "settlement".into(),
            database,
            None,
            Box::pin(async { outcome(5, "settlement") }),
            Some(completion),
        )
        .unwrap();
    settlement_started_rx.await.unwrap();
    let _ = stop.send(true);
    assert!(!join.is_finished());
    let _ = finish_settlement.send(());
    join.await.unwrap();
    assert!(completion_rx.await.unwrap().unwrap().1);
}

#[tokio::test]
async fn dispatcher_uses_the_production_record_owner_and_reports_inserted_result() {
    let (_directory, database) = database();
    let task = database.insert_task(paper_task()).await.unwrap();
    let settlement: Settlement = Arc::new(|database, outcome| {
        Box::pin(async move { super::super::notify::record(&database, outcome).await })
    });
    let (handle, stop, join) = runner(settlement);
    let receiver = handle
        .enqueue_manual(
            task.id,
            "manual:mint:round:0".into(),
            "mint".into(),
            database.clone(),
            async move { outcome(task.id, "dispatcher-recorded") },
        )
        .unwrap();
    let (_, inserted) = receiver.await.unwrap().unwrap();
    assert!(inserted);
    assert_eq!(
        database
            .list_task_activity(task.id, 10)
            .await
            .unwrap()
            .len(),
        1
    );
    let _ = stop.send(true);
    join.await.unwrap();
}

#[tokio::test]
async fn install_publishes_the_scheduler_before_service_work_can_be_accepted() {
    let (handle, stop, join) = install();
    assert!(active().is_some());
    assert!(!handle.manual_close_pending(99, "mint", 0));
    let _ = stop.send(true);
    join.await.unwrap();
    assert!(active().is_none());
}

#[test]
fn newly_recorded_paper_guard_skips_keep_quote_context() {
    let now = chrono::Utc::now();
    let activity = crate::wallets::watch::WalletActivity {
        subject: "target".to_owned(),
        signature: "guarded-signature".to_owned(),
        slot: 1,
        block_time: Some(now.timestamp()),
        detected_at: now,
        decoded_at: now,
        success: true,
        kind: crate::wallets::watch::ActivityKind::Other,
        sources: Vec::new(),
        backfill: true,
    };
    let outcome = skipped_paper(
        7,
        &activity,
        "mint",
        super::super::CopySkip::PaperExecutionUnavailable,
        4,
    );
    assert!(matches!(
        outcome,
        CopyOutcome::Skipped {
            paper: Some(super::super::types::PaperContext {
                model: crate::trader::paper::PaperModel::Quote,
                epoch: 4
            }),
            ..
        }
    ));
}
