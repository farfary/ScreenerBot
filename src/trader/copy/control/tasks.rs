// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task lifecycle: create, update, delete and the guarded paper/live transition,
//! with the active-task limit and the watch source kept in step with each task.

use chrono::Utc;

use super::super::{
    confirm_mode_transition, sync_open_position_management, CopyDatabase, CopyMode,
    CopyPauseReason, CopyTask, CopyTaskInput, ExitMode,
};
use super::live_block_reason;
use super::open_database;
use crate::positions::PositionOrigin;
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
/// in either book: its paper ledger, or a real position it opened while live.
pub(in crate::trader::copy) async fn holds_open_copies(
    db: &CopyDatabase,
    task: &CopyTask,
    mint: Option<&str>,
) -> Result<bool> {
    let wanted = |held: &str| mint.is_none_or(|mint| mint == held);
    if db
        .paper_positions(task.id)
        .await?
        .iter()
        .any(|position| position.is_open() && wanted(&position.mint))
    {
        return Ok(true);
    }
    Ok(crate::positions::get_open_positions()
        .await
        .iter()
        .any(|position| {
            wanted(&position.mint)
                && matches!(position.origin, PositionOrigin::Copy { task_id, .. } if task_id == task.id)
        }))
}

/// A task watches its wallet while it copies, and while paused for as long as the
/// wallet's sells still close something it holds: pausing stops new copies, never
/// the exits of what is already held.
pub(in crate::trader::copy) async fn wants_watch(
    db: &CopyDatabase,
    task: &CopyTask,
) -> Result<bool> {
    Ok(task.enabled
        || (task.exit_mode != ExitMode::BuyOnly && holds_open_copies(db, task, None).await?))
}

/// Attach or release the task's watch source so it matches `wants_watch`. Both
/// directions are idempotent.
pub(in crate::trader::copy) async fn sync_task_watch(
    db: &CopyDatabase,
    task: &CopyTask,
) -> Result<()> {
    if wants_watch(db, task).await? {
        watch::add_copy_source(task.id, &task.target_address, task.label.as_deref())
            .await
            .map(|_| ())
            .map_err(|error| Error::CopyWatchRejected {
                detail: error.to_string(),
            })
    } else {
        watch::remove_copy_source(task.id, &task.target_address)
            .await
            .map_err(|error| Error::CopyReconciliation {
                detail: format!("failed to detach the copy target: {error}"),
            })
    }
}

/// Create a task. New tasks always start in paper mode; arming live is a
/// separate confirmed transition (`set_task_mode`).
pub async fn create_task(input: CopyTaskInput) -> Result<CopyTask> {
    let task = input
        .into_task(crate::chains::active_chain(), Utc::now())
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
    let mut task = merge_task_patch(&original, patch)?
        .into_task_for_update(crate::chains::active_chain(), Utc::now(), original.mode)
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
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
    if let Err(error) = sync_task_watch(&db, &updated).await {
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

/// Delete a task and detach its watch source. Its decisions, spend and paper book
/// go with it (foreign-key cascade). Refused while it could still move money: an
/// enabled live task must be paused first, and a task whose live positions are
/// still open keeps them until they are closed.
pub async fn delete_task(id: i64) -> Result<()> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.enabled && task.mode == CopyMode::Live {
        return Err(Error::CopyTaskLive { task_id: id });
    }
    let open_positions = crate::positions::get_open_positions()
        .await
        .into_iter()
        .filter(|position| {
            matches!(position.origin, PositionOrigin::Copy { task_id, .. } if task_id == id)
        })
        .count();
    if open_positions > 0 {
        return Err(Error::CopyTaskOwnsPositions {
            task_id: id,
            open_positions,
        });
    }
    watch::remove_copy_source(id, &task.target_address)
        .await
        .map_err(|e| Error::CopyReconciliation {
            detail: format!("failed to detach the copy target: {e}"),
        })?;
    if db.delete_task(id).await? {
        Ok(())
    } else {
        Err(Error::CopyTaskNotFound { task_id: id })
    }
}

/// The guarded paper/live transition. Arming live requires the exact
/// `LIVE_ARM_CONFIRMATION` phrase and a runtime that could execute live copies
/// (`live_block_reason`); returning to paper is always allowed.
pub async fn set_task_mode(
    id: i64,
    mode: CopyMode,
    confirmation: Option<String>,
) -> Result<CopyTask> {
    let db = open_database().await?;
    let current = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if mode == CopyMode::Live && current.mode != CopyMode::Live {
        if let Some(reason) = live_block_reason() {
            return Err(Error::CopyLiveUnavailable { reason });
        }
    }
    let mode = confirm_mode_transition(current.mode, mode, confirmation.as_deref())
        .map_err(|reason| Error::CopyTaskRejected { reason })?;
    db.set_task_mode(id, mode, confirmation).await
}
