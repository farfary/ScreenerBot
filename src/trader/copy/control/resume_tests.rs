// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The bulk resume and the single-task update it runs through, over a temporary copy
//! store with injected watch and position synchronization.

use std::collections::HashSet;
use std::future::{ready, Ready};
use std::sync::Mutex;

use super::super::merge_task_patch;
use super::super::tasks::apply_task_update;
use super::super::tests::{budget_pause, global_policy, stored};
use super::{CopyResumeKept, CopyResumeKeptReason, CopyResumeOutcome};
use crate::trader::copy::{CopyDatabase, CopyPauseReason, CopyTask, CopyTaskInput, ExitMode};
use crate::trader::paper::{TradingMode, LIVE_ARM_CONFIRMATION};
use crate::trader::{Error, Result};

/// A copy store holding one task per entry, in order: enabled when the entry has no
/// pause reason, paused for it otherwise, and armed for live when its mode is live.
/// Each task follows its own wallet.
async fn store_of(
    entries: &[(TradingMode, Option<CopyPauseReason>)],
) -> (tempfile::TempDir, CopyDatabase, Vec<i64>) {
    crate::config::utils::install_default_config();
    let directory = tempfile::tempdir().unwrap();
    let database = CopyDatabase::open(
        directory.path().join("copy_trading.db"),
        crate::chains::ChainId::Solana,
    )
    .unwrap();
    let mut ids = Vec::new();
    for (index, (mode, pause)) in entries.iter().enumerate() {
        let task = database
            .insert_task(CopyTask {
                target_address: format!("target-{index}"),
                ..stored()
            })
            .await
            .unwrap();
        if *mode == TradingMode::Live {
            database
                .set_task_mode(
                    task.id,
                    TradingMode::Live,
                    Some(LIVE_ARM_CONFIRMATION.to_owned()),
                )
                .await
                .unwrap();
        }
        if let Some(reason) = pause {
            assert!(database.pause_task(task.id, reason.clone()).await.unwrap());
        }
        ids.push(task.id);
    }
    (directory, database, ids)
}

/// Runs a bulk resume over `database` with a watch that refuses to attach for the
/// task `failing`, and position ownership that always follows.
async fn resume_all(
    database: &CopyDatabase,
    include_live: bool,
    failing: Option<i64>,
) -> CopyResumeOutcome {
    super::resume_paused_tasks_with(
        database,
        include_live,
        global_policy(),
        &move |task: CopyTask| {
            ready(if Some(task.id) == failing {
                Err(Error::CopyWatchRejected {
                    detail: "the watch refused the wallet".to_owned(),
                })
            } else {
                Ok(())
            })
        },
        &|_: i64, _: ExitMode| ready(Ok(())),
    )
    .await
    .unwrap()
}

async fn stored_task(database: &CopyDatabase, id: i64) -> CopyTask {
    database.get_task(id).await.unwrap().unwrap()
}

fn kept(task_id: i64, reason: CopyResumeKeptReason) -> CopyResumeKept {
    CopyResumeKept { task_id, reason }
}

#[tokio::test]
async fn a_user_paused_paper_task_resumes() {
    let (_directory, database, ids) =
        store_of(&[(TradingMode::Paper, Some(CopyPauseReason::User))]).await;
    let outcome = resume_all(&database, false, None).await;
    assert_eq!(outcome.resumed, ids);
    assert!(outcome.kept_paused.is_empty());
    let task = stored_task(&database, ids[0]).await;
    assert!(task.enabled);
    assert_eq!((task.pause_reason, task.paused_at), (None, None));
}

/// A watch-recovery pause is never resumed in bulk and keeps its reason; the reply
/// still names it.
#[tokio::test]
async fn a_watch_recovery_pause_stays_paused() {
    let (_directory, database, ids) = store_of(&[
        (TradingMode::Paper, Some(budget_pause())),
        (TradingMode::Paper, Some(CopyPauseReason::HeliusUnavailable)),
        (
            TradingMode::Paper,
            Some(CopyPauseReason::WatchProcessingFailed),
        ),
    ])
    .await;
    let outcome = resume_all(&database, true, None).await;
    assert!(outcome.resumed.is_empty());
    assert_eq!(
        outcome.kept_paused,
        ids.iter()
            .map(|id| kept(*id, CopyResumeKeptReason::NeedsWatchRecovery))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        stored_task(&database, ids[0]).await.pause_reason,
        Some(budget_pause())
    );
    let reply = serde_json::to_value(&outcome).unwrap();
    assert_eq!(
        reply["kept_paused"][0]["reason"]["kind"],
        "needs_watch_recovery"
    );
}

/// A live task stays paused unless live tasks are included, and resumes when they are.
#[tokio::test]
async fn a_live_task_resumes_only_when_live_tasks_are_included() {
    let (_directory, database, ids) = store_of(&[
        (TradingMode::Live, Some(CopyPauseReason::User)),
        (TradingMode::Paper, Some(CopyPauseReason::User)),
    ])
    .await;
    let outcome = resume_all(&database, false, None).await;
    assert_eq!(outcome.resumed, vec![ids[1]]);
    assert_eq!(
        outcome.kept_paused,
        vec![kept(ids[0], CopyResumeKeptReason::LiveNotIncluded)]
    );
    let live = stored_task(&database, ids[0]).await;
    assert!(!live.enabled);
    assert_eq!(live.pause_reason, Some(CopyPauseReason::User));

    let outcome = resume_all(&database, true, None).await;
    assert_eq!(outcome.resumed, vec![ids[0]]);
    assert!(outcome.kept_paused.is_empty());
    let live = stored_task(&database, ids[0]).await;
    assert!(live.enabled);
    assert_eq!(live.mode, TradingMode::Live);
}

/// Resumes stop at the active-task limit; every task past it is kept paused with the
/// limit, in ascending id.
#[tokio::test]
async fn the_active_task_limit_keeps_the_rest_paused() {
    let (_directory, database, ids) = store_of(&[
        (TradingMode::Paper, None),
        (TradingMode::Paper, Some(CopyPauseReason::User)),
        (
            TradingMode::Paper,
            Some(CopyPauseReason::LatencyKillSwitch {
                average_ms: 4200,
                threshold_ms: 3000,
            }),
        ),
        (TradingMode::Paper, Some(CopyPauseReason::WatchDetached)),
    ])
    .await;
    let maximum = crate::config::with_config(|config| config.copy_trading.max_active_tasks);
    crate::config::update_config_section(|config| config.copy_trading.max_active_tasks = 2, false)
        .unwrap();
    let outcome = resume_all(&database, false, None).await;
    crate::config::update_config_section(
        |config| config.copy_trading.max_active_tasks = maximum,
        false,
    )
    .unwrap();
    assert_eq!(outcome.resumed, vec![ids[1]]);
    assert_eq!(
        outcome.kept_paused,
        vec![
            kept(ids[2], CopyResumeKeptReason::TaskLimit { maximum: 2 }),
            kept(ids[3], CopyResumeKeptReason::TaskLimit { maximum: 2 }),
        ]
    );
    for id in &ids[2..] {
        assert!(!stored_task(&database, *id).await.enabled);
    }
}

/// A task whose watch refuses is rolled back to its pause; the tasks resumed before and
/// after it stay resumed.
#[tokio::test]
async fn one_failing_task_does_not_undo_the_others() {
    let (_directory, database, ids) = store_of(&[
        (TradingMode::Paper, Some(CopyPauseReason::User)),
        (TradingMode::Paper, Some(CopyPauseReason::StorageRestored)),
        (TradingMode::Paper, Some(CopyPauseReason::User)),
    ])
    .await;
    let outcome = resume_all(&database, false, Some(ids[1])).await;
    assert_eq!(outcome.resumed, vec![ids[0], ids[2]]);
    assert_eq!(
        outcome.kept_paused,
        vec![kept(
            ids[1],
            CopyResumeKeptReason::Failed {
                message: crate::i18n::UiText::new(crate::i18n::ids::ERRORS_COPY_WATCH_REJECTED),
            }
        )]
    );
    assert!(stored_task(&database, ids[0]).await.enabled);
    assert!(stored_task(&database, ids[2]).await.enabled);
    let failed = stored_task(&database, ids[1]).await;
    assert!(!failed.enabled);
    assert_eq!(failed.pause_reason, Some(CopyPauseReason::StorageRestored));
}

/// The reply accounts for every task paused when the resume started, and names no
/// task that was enabled.
#[tokio::test]
async fn the_reply_accounts_for_every_paused_task() {
    let (_directory, database, ids) = store_of(&[
        (TradingMode::Paper, None),
        (TradingMode::Live, Some(CopyPauseReason::User)),
        (TradingMode::Paper, Some(budget_pause())),
        (TradingMode::Paper, Some(CopyPauseReason::WatchDetached)),
        (TradingMode::Paper, Some(CopyPauseReason::User)),
    ])
    .await;
    let outcome = resume_all(&database, false, Some(ids[4])).await;
    let mut named = outcome
        .resumed
        .iter()
        .copied()
        .chain(outcome.kept_paused.iter().map(|kept| kept.task_id))
        .collect::<Vec<_>>();
    named.sort_unstable();
    assert_eq!(named, ids[1..].to_vec());
}

/// A task armed for live between the list and its own read is judged on the row as
/// stored at that read: without live tasks included it stays paused, live, with its
/// pause reason.
#[tokio::test]
async fn a_task_armed_live_during_the_run_is_not_resumed_without_live_tasks() {
    let (_directory, database, ids) = store_of(&[
        (TradingMode::Paper, Some(CopyPauseReason::User)),
        (TradingMode::Paper, Some(CopyPauseReason::User)),
    ])
    .await;
    let (first, second) = (ids[0], ids[1]);
    let arming = database.clone();
    let sync_watch = move |task: CopyTask| {
        let database = arming.clone();
        async move {
            if task.id == first {
                database
                    .set_task_mode(
                        second,
                        TradingMode::Live,
                        Some(LIVE_ARM_CONFIRMATION.to_owned()),
                    )
                    .await?;
            }
            Ok::<(), Error>(())
        }
    };
    let outcome = super::resume_paused_tasks_with(
        &database,
        false,
        global_policy(),
        &sync_watch,
        &|_: i64, _: ExitMode| ready(Ok(())),
    )
    .await
    .unwrap();
    assert_eq!(outcome.resumed, vec![first]);
    assert_eq!(
        outcome.kept_paused,
        vec![kept(second, CopyResumeKeptReason::LiveNotIncluded)]
    );
    let armed = stored_task(&database, second).await;
    assert!(!armed.enabled);
    assert_eq!(armed.mode, TradingMode::Live);
    assert_eq!(armed.pause_reason, Some(CopyPauseReason::User));
}

/// Position ownership that always follows the task.
fn positions_follow(_: i64, _: ExitMode) -> Ready<Result<()>> {
    ready(Ok(()))
}

/// The PATCH update of stored task `id` over a watch that always attaches and the
/// position synchronization `sync_positions`.
async fn patch_task<P, PFut>(
    database: &CopyDatabase,
    id: i64,
    patch: serde_json::Value,
    sync_positions: &P,
) -> Result<CopyTask>
where
    P: Fn(i64, ExitMode) -> PFut,
    PFut: std::future::Future<Output = Result<()>>,
{
    apply_task_update(
        database,
        stored_task(database, id).await,
        patch,
        global_policy(),
        &|_: CopyTask| ready(Ok(())),
        sync_positions,
    )
    .await
}

/// Every editable field a PATCH names is stored as the merged patch states it and no
/// other field moves. The patches cover every field of the task input except the
/// target wallet, the mode and `enabled`, which have their own rules.
#[tokio::test]
async fn a_patch_stores_each_field_it_names_and_nothing_else() {
    let (_directory, database, ids) = store_of(&[(TradingMode::Paper, None)]).await;
    let patches = [
        serde_json::json!({ "label": "Renamed" }),
        serde_json::json!({ "label": null }),
        serde_json::json!({ "sizing": { "kind": "ratio_of_target", "pct": 25.0 } }),
        serde_json::json!({ "exit_mode": "hybrid" }),
        serde_json::json!({
            "exit_policy_overrides": { "trailing": { "activation_pct": 30.0, "distance_pct": 12.0 } }
        }),
        serde_json::json!({ "max_native_per_trade": 0.15 }),
        serde_json::json!({ "max_native_per_token": 0.8 }),
        serde_json::json!({ "total_budget_native": 4.0 }),
        serde_json::json!({ "min_target_trade_native": 0.05 }),
        serde_json::json!({ "max_target_trade_native": 2.0 }),
        serde_json::json!({ "buy_once_per_token": true }),
        serde_json::json!({ "slippage_pct": 2.5 }),
        serde_json::json!({ "require_filter_pass": false }),
        serde_json::json!({ "copy_stable_quoted": true }),
    ];
    let patched = patches
        .iter()
        .flat_map(|patch| patch.as_object().unwrap().keys().cloned())
        .collect::<HashSet<_>>();
    let editable = serde_json::to_value(CopyTaskInput::from(&stored()))
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| !matches!(key.as_str(), "target_address" | "mode" | "enabled"))
        .cloned()
        .collect::<HashSet<_>>();
    assert_eq!(patched, editable);

    for patch in patches {
        let original = stored_task(&database, ids[0]).await;
        let expected = merge_task_patch(&original, patch.clone()).unwrap();
        patch_task(&database, ids[0], patch.clone(), &positions_follow)
            .await
            .unwrap_or_else(|error| panic!("{patch} was refused: {error}"));
        let written = stored_task(&database, ids[0]).await;
        assert_eq!(CopyTaskInput::from(&written), expected, "{patch}");
        assert_eq!(written.created_at, original.created_at, "{patch}");
        assert_eq!((written.pause_reason, written.paused_at), (None, None));
    }
}

/// Disabling a running task records the user as the pause reason, with its time.
#[tokio::test]
async fn a_patch_that_disables_a_task_records_a_user_pause() {
    let (_directory, database, ids) = store_of(&[(TradingMode::Paper, None)]).await;
    patch_task(
        &database,
        ids[0],
        serde_json::json!({ "enabled": false }),
        &positions_follow,
    )
    .await
    .unwrap();
    let paused = stored_task(&database, ids[0]).await;
    assert!(!paused.enabled);
    assert_eq!(paused.pause_reason, Some(CopyPauseReason::User));
    assert!(paused.paused_at.is_some());
}

/// A position-ownership change that fails rolls the task back to its stored exit mode
/// and hands the open positions back to that mode.
#[tokio::test]
async fn a_failed_position_handover_rolls_the_exit_mode_back() {
    let (_directory, database, ids) = store_of(&[(TradingMode::Paper, None)]).await;
    let calls = Mutex::new(Vec::new());
    let refuse_hybrid = |_: i64, mode: ExitMode| {
        calls.lock().unwrap().push(mode);
        ready(if mode == ExitMode::Hybrid {
            Err(Error::CopyReconciliation {
                detail: "the position store refused the handover".to_owned(),
            })
        } else {
            Ok(())
        })
    };
    let result = patch_task(
        &database,
        ids[0],
        serde_json::json!({ "exit_mode": "hybrid" }),
        &refuse_hybrid,
    )
    .await;
    assert!(
        matches!(result, Err(Error::CopyReconciliation { .. })),
        "{result:?}"
    );
    assert_eq!(
        stored_task(&database, ids[0]).await.exit_mode,
        ExitMode::Mirror
    );
    assert_eq!(
        *calls.lock().unwrap(),
        vec![ExitMode::Hybrid, ExitMode::Mirror]
    );
}

/// The target wallet is the task's identity: a patch that changes it is refused and
/// nothing is written.
#[tokio::test]
async fn a_patch_of_the_target_wallet_is_refused_untouched() {
    let (_directory, database, ids) = store_of(&[(TradingMode::Paper, None)]).await;
    let before = stored_task(&database, ids[0]).await;
    let result = patch_task(
        &database,
        ids[0],
        serde_json::json!({ "target_address": "another-wallet" }),
        &positions_follow,
    )
    .await;
    assert!(
        matches!(result, Err(Error::CopyValidation { .. })),
        "{result:?}"
    );
    assert_eq!(stored_task(&database, ids[0]).await, before);
}

/// Two enables racing for the last free slot leave exactly the maximum enabled: the
/// count and the write of each enable run under one lock, so the second enable counts
/// the first and is refused at the limit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_enables_at_the_last_slot_never_pass_the_limit() {
    crate::config::utils::install_default_config();
    let maximum = crate::config::with_config(|config| config.copy_trading.max_active_tasks);
    assert!(maximum >= 1);
    let mut entries = vec![(TradingMode::Paper, None); maximum - 1];
    entries.extend([
        (TradingMode::Paper, Some(CopyPauseReason::User)),
        (TradingMode::Paper, Some(CopyPauseReason::User)),
    ]);
    let (_directory, database, ids) = store_of(&entries).await;
    let enable = serde_json::json!({ "enabled": true });

    let (first, second) = tokio::join!(
        patch_task(
            &database,
            ids[maximum - 1],
            enable.clone(),
            &positions_follow
        ),
        patch_task(&database, ids[maximum], enable.clone(), &positions_follow),
    );

    let refused = [&first, &second]
        .into_iter()
        .filter(|result| matches!(result, Err(Error::CopyTaskLimit { maximum: limit }) if *limit == maximum))
        .count();
    assert_eq!(refused, 1, "{first:?} {second:?}");
    assert!(first.is_ok() || second.is_ok(), "{first:?} {second:?}");
    let enabled = database
        .list_tasks()
        .await
        .unwrap()
        .into_iter()
        .filter(|task| task.enabled)
        .count();
    assert_eq!(enabled, maximum);
}
