// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task lifecycle: create, update, delete and the guarded paper/live transition,
//! with the active-task limit and the watch source kept in step with each task.

use std::future::Future;

use chrono::Utc;

use super::super::paper_execution::{self, queue_refusal, PaperExecutionHandle};
use super::super::{
    confirm_mode_transition, sync_open_position_management, CopyDatabase, CopyPauseReason,
    CopySkip, CopyTask, CopyTaskInput, ExitMode,
};
use super::live_block_reason;
use super::open_database;
use crate::positions::{books, Book, Position, PositionOrigin};
use crate::trader::paper::TradingMode;
use crate::trader::policy::ExitPolicy;
use crate::trader::{Error, Result};
use crate::wallets::watch;

pub async fn list_tasks() -> Result<Vec<CopyTask>> {
    open_database().await?.list_tasks().await
}

pub async fn get_task(id: i64) -> Result<CopyTask> {
    open_database()
        .await?
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })
}

async fn ensure_active_slot(db: &CopyDatabase) -> Result<()> {
    let active = db
        .list_tasks()
        .await?
        .into_iter()
        .filter(|task| task.enabled)
        .count();
    let maximum = crate::config::with_config(|config| config.copy_trading.max_active_tasks);
    if active >= maximum {
        return Err(Error::CopyTaskLimit { maximum });
    }
    Ok(())
}

/// Whether the task still holds an open copy of `mint` (of any token when `None`)
/// in either book: a paper round or a real position it opened.
pub(in crate::trader::copy) async fn holds_open_copies(
    task: &CopyTask,
    mint: Option<&str>,
) -> bool {
    let owned = |position: &Position| {
        mint.is_none_or(|mint| mint == position.mint) && owned_by(position, task.id)
    };
    books::open_positions(Book::Paper).await.iter().any(owned)
        || books::open_positions(Book::Live).await.iter().any(owned)
}

/// A task watches its wallet while it copies, and while paused for as long as the
/// wallet's sells still close something it holds: pausing stops new copies, never
/// the exits of what is already held.
pub(in crate::trader::copy) async fn wants_watch(task: &CopyTask) -> bool {
    task.enabled || (task.exit_mode != ExitMode::BuyOnly && holds_open_copies(task, None).await)
}

/// Attach or release the task's watch source so it matches `wants_watch`. Both
/// directions are idempotent, and an attach never outlives the task row.
pub(in crate::trader::copy) async fn sync_task_watch(task: &CopyTask) -> Result<()> {
    if wants_watch(task).await {
        attach_while_stored(
            &open_database().await?,
            task.clone(),
            attach_watch,
            detach_watch,
        )
        .await
    } else {
        detach_watch(task.clone()).await
    }
}

/// Attach the task's watch source, then release it again when the task row is gone by
/// the time the attach finished. A delete detaches once more after removing the row, so
/// whichever of the two runs last leaves no source of a deleted task attached.
pub(super) async fn attach_while_stored<A, AFut, D, DFut>(
    db: &CopyDatabase,
    task: CopyTask,
    attach: A,
    detach: D,
) -> Result<()>
where
    A: FnOnce(CopyTask) -> AFut,
    AFut: Future<Output = Result<()>>,
    D: FnOnce(CopyTask) -> DFut,
    DFut: Future<Output = Result<()>>,
{
    attach(task.clone()).await?;
    if db.get_task(task.id).await?.is_some() {
        return Ok(());
    }
    detach(task).await
}

async fn attach_watch(task: CopyTask) -> Result<()> {
    watch::add_copy_source(task.id, &task.target_address, task.label.as_deref())
        .await
        .map(|_| ())
        .map_err(|error| Error::CopyWatchRejected {
            detail: error.to_string(),
        })
}

async fn detach_watch(task: CopyTask) -> Result<()> {
    watch::remove_copy_source(task.id, &task.target_address)
        .await
        .map_err(|error| Error::CopyReconciliation {
            detail: format!("failed to detach the copy target: {error}"),
        })
}

/// Create a task. New tasks always start in paper mode; arming live is a
/// separate confirmed transition (`set_task_mode`).
pub async fn create_task(input: CopyTaskInput) -> Result<CopyTask> {
    let task = input
        .into_task(
            crate::chains::active_chain(),
            Utc::now(),
            ExitPolicy::from_config(),
        )
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    crate::chains::adapter()
        .validate_address(&task.target_address)
        .map_err(|e| Error::CopyWatchRejected {
            detail: format!("invalid target wallet: {e}"),
        })?;
    let db = open_database().await?;
    if task.enabled {
        ensure_active_slot(&db).await?;
    }
    let inserted = db.insert_task(task).await?;
    if inserted.enabled {
        if let Err(error) = watch::add_copy_source(
            inserted.id,
            &inserted.target_address,
            inserted.label.as_deref(),
        )
        .await
        {
            let _ = db.delete_task(inserted.id).await;
            return Err(Error::CopyWatchRejected {
                detail: error.to_string(),
            });
        }
    }
    Ok(inserted)
}

/// PATCH semantics: fields present in the patch replace the stored ones, absent
/// fields keep their value, and an unknown field is refused rather than silently
/// dropped. The merged input then runs the full task validation.
pub fn merge_task_patch(original: &CopyTask, patch: serde_json::Value) -> Result<CopyTaskInput> {
    let invalid = |detail: String| Error::CopyValidation { detail };
    let serde_json::Value::Object(fields) = patch else {
        return Err(invalid("the patch must be a JSON object".to_owned()));
    };
    let mut merged =
        serde_json::to_value(CopyTaskInput::from(original)).map_err(|e| invalid(e.to_string()))?;
    let target = merged
        .as_object_mut()
        .ok_or_else(|| invalid("stored task did not serialize to an object".to_owned()))?;
    for (key, value) in fields {
        if !target.contains_key(&key) {
            return Err(invalid(format!("unknown field `{key}`")));
        }
        target.insert(key, value);
    }
    serde_json::from_value(merged).map_err(|e| invalid(e.to_string()))
}

/// The task a PATCH makes of `original`, validated as an update: exit overrides the
/// patch changes are checked over `exit_base`, unchanged stored ones are not, so a task
/// whose overrides a global change invalidated still pauses, resumes and takes edits.
pub(super) fn patched_task(
    original: &CopyTask,
    patch: serde_json::Value,
    exit_base: ExitPolicy,
) -> Result<CopyTask> {
    merge_task_patch(original, patch)?
        .into_task_for_update(
            crate::chains::active_chain(),
            Utc::now(),
            original.mode,
            &original.exit_policy_overrides,
            exit_base,
        )
        .map_err(|reason| Error::CopyTaskRejected { reason })
}

/// Apply a partial update, keeping the watch source and the ownership of the
/// task's open positions consistent with the stored task. The task is written
/// first and every watch or position change after it rolls the task back on
/// failure, so a failed write never leaves the watch source carrying values the
/// task does not. The target wallet is the task's identity: a different wallet
/// is a new task (clone this one).
pub async fn update_task(id: i64, patch: serde_json::Value) -> Result<CopyTask> {
    let db = open_database().await?;
    let original = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    let mut task = patched_task(&original, patch, ExitPolicy::from_config())?;
    if task.target_address != original.target_address {
        return Err(Error::CopyValidation {
            detail:
                "the target wallet of a task cannot change; clone the task to copy another wallet"
                    .to_owned(),
        });
    }
    task.id = id;
    task.created_at = original.created_at;
    (task.pause_reason, task.paused_at) = if task.enabled {
        (None, None)
    } else if original.enabled {
        (Some(CopyPauseReason::User), Some(Utc::now()))
    } else {
        (original.pause_reason.clone(), original.paused_at)
    };
    if task.enabled && !original.enabled {
        ensure_active_slot(&db).await?;
    }
    let updated = db.update_task(task).await?;
    if let Err(error) = sync_task_watch(&updated).await {
        return Err(match db.update_task(original.clone()).await {
            Ok(_) => error,
            Err(rollback) => Error::CopyReconciliation {
                detail: format!("{error}; rolling the task update back failed too: {rollback}"),
            },
        });
    }
    if let Err(error) = sync_open_position_management(id, updated.exit_mode).await {
        let rollback = db.update_task(original.clone()).await;
        let position_rollback = sync_open_position_management(id, original.exit_mode).await;
        return Err(Error::CopyReconciliation {
            detail: match (rollback, position_rollback) {
                (Ok(_), Ok(())) => format!(
                    "failed to update copy position ownership; task update was rolled back: {error}"
                ),
                (task_result, position_result) => format!(
                    "failed to update copy position ownership ({error}); rollback results: task={task_result:?}, positions={position_result:?}"
                ),
            },
        });
    }
    Ok(updated)
}

/// Delete a task. Its watch source is detached first, so no new copy of the task is
/// queued; then, as one step on the task's paper queue, every paper job queued before it
/// runs out, the task's open paper rounds close at their last price and the task row is
/// removed. A paper job queued behind the step finds no task and opens no round. The
/// closed rounds stay in the paper history; the task's decisions, spend and
/// archived copy paper book go with it (foreign-key cascade). Refused while it could
/// still move money: an enabled live task must be paused first, and a task whose live
/// positions are still open keeps them until they are closed. When any step after the
/// detach fails, the task row is still there and its watch source is attached again as
/// the task needs it, so the delete can be retried; rounds already closed stay closed.
/// Once the row is gone the watch source is detached again: an update or reconcile that
/// attached it while the step waited in the queue does not outlive the task.
pub async fn delete_task(id: i64) -> Result<()> {
    let db = open_database().await?;
    delete_task_with(
        db,
        paper_execution::active(),
        id,
        |id| async move { live_copies(id).await },
        |id| async move { paper_rounds(id).await },
        crate::trader::paper::settle_rounds,
        detach_watch,
        |task| async move { sync_task_watch(&task).await },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn delete_task_with<L, LFut, R, RFut, S, SFut, D, DFut, A, AFut>(
    db: CopyDatabase,
    runtime: Option<PaperExecutionHandle>,
    id: i64,
    live_copies: L,
    paper_rounds: R,
    settle: S,
    detach: D,
    attach: A,
) -> Result<()>
where
    L: Fn(i64) -> LFut + Send + Sync + 'static,
    LFut: Future<Output = usize> + Send + 'static,
    R: FnOnce(i64) -> RFut + Send + 'static,
    RFut: Future<Output = Vec<Position>> + Send + 'static,
    S: FnOnce(Vec<Position>) -> SFut + Send + 'static,
    SFut: Future<Output = Result<usize>> + Send + 'static,
    D: Fn(CopyTask) -> DFut,
    DFut: Future<Output = Result<()>>,
    A: FnOnce(CopyTask) -> AFut,
    AFut: Future<Output = Result<()>>,
{
    let task = deletable_task(&db, id, &live_copies).await?;
    detach(task.clone()).await?;
    let job_db = db.clone();
    let job = async move {
        deletable_task(&job_db, id, &live_copies).await?;
        settle(paper_rounds(id).await).await?;
        if job_db.delete_task(id).await? {
            Ok(())
        } else {
            Err(Error::CopyTaskNotFound { task_id: id })
        }
    };
    // Without a running queue no paper job of the task can run, so the removal runs here.
    let removed = match runtime {
        None => job.await,
        Some(runtime) => {
            match runtime.enqueue_step(id, paper_execution::delete_key(), db.clone(), job) {
                Ok(receiver) => receiver.await.unwrap_or(Err(Error::CopyTaskRejected {
                    reason: CopySkip::PaperExecutionUnavailable,
                })),
                Err(error) => Err(Error::CopyTaskRejected {
                    reason: queue_refusal(error),
                }),
            }
        }
    };
    let error = match removed {
        Ok(()) => return detach(task).await,
        Err(error) => error,
    };
    let restored = match db.get_task(id).await {
        Ok(Some(task)) => attach(task).await,
        Ok(None) => Ok(()),
        Err(lookup) => Err(lookup),
    };
    Err(match restored {
        Ok(()) => error,
        Err(restore) => Error::CopyReconciliation {
            detail: format!("{error}; attaching the copy target again failed too: {restore}"),
        },
    })
}

/// The task `id` while deleting it moves no money: not enabled in live, and holding no
/// open live position.
async fn deletable_task<L, LFut>(db: &CopyDatabase, id: i64, live_copies: &L) -> Result<CopyTask>
where
    L: Fn(i64) -> LFut,
    LFut: Future<Output = usize>,
{
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.enabled && task.mode == TradingMode::Live {
        return Err(Error::CopyTaskLive { task_id: id });
    }
    let open_positions = live_copies(id).await;
    if open_positions > 0 {
        return Err(Error::CopyTaskOwnsPositions {
            task_id: id,
            open_positions,
        });
    }
    Ok(task)
}

/// The open live positions task `id` opened.
async fn live_copies(id: i64) -> usize {
    crate::positions::get_open_positions()
        .await
        .into_iter()
        .filter(|position| owned_by(position, id))
        .count()
}

/// The open paper rounds task `id` opened.
async fn paper_rounds(id: i64) -> Vec<Position> {
    books::open_positions(Book::Paper)
        .await
        .into_iter()
        .filter(|position| owned_by(position, id))
        .collect()
}

fn owned_by(position: &Position, id: i64) -> bool {
    matches!(position.origin, PositionOrigin::Copy { task_id, .. } if task_id == id)
}

/// The guarded paper/live transition. Arming live requires the exact
/// `LIVE_ARM_CONFIRMATION` phrase and a runtime that could execute live copies
/// (`live_block_reason`); returning to paper is always allowed.
pub async fn set_task_mode(
    id: i64,
    mode: TradingMode,
    confirmation: Option<String>,
) -> Result<CopyTask> {
    let db = open_database().await?;
    let current = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if mode == TradingMode::Live && current.mode != TradingMode::Live {
        if let Some(reason) = live_block_reason() {
            return Err(Error::CopyLiveUnavailable { reason });
        }
    }
    let mode = confirm_mode_transition(current.mode, mode, confirmation.as_deref())
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    db.set_task_mode(id, mode, confirmation).await
}
