// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The bulk resume: every paused task a single resume would restart, each through the
//! single-task update.

use std::future::Future;

use serde::Serialize;

use super::super::{sync_open_position_management, CopyDatabase, CopyRecovery, CopyTask, ExitMode};
use super::open_database;
use super::tasks::{apply_task_update, sync_task_watch};
use crate::i18n::UiText;
use crate::trader::paper::TradingMode;
use crate::trader::policy::ExitPolicy;
use crate::trader::{Error, Result};

/// Why a bulk resume left a paused task paused.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopyResumeKeptReason {
    /// The task comes back through a repair of its wallet watch, never a resume.
    NeedsWatchRecovery,
    /// A live task, and the resume did not include live tasks.
    LiveNotIncluded,
    /// The active-task limit was reached; paper and live tasks both count.
    TaskLimit { maximum: usize },
    /// The resume failed. The task is rolled back to its pause unless the rollback failed
    /// too; the message is then the request-failed text.
    Failed { message: UiText },
}

/// A paused task a bulk resume left paused, with the reason.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CopyResumeKept {
    pub task_id: i64,
    pub reason: CopyResumeKeptReason,
}

/// What a bulk resume did with each task that was paused when it started: resumed, or
/// kept paused with the reason. Both lists are in ascending task id.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CopyResumeOutcome {
    pub resumed: Vec<i64>,
    pub kept_paused: Vec<CopyResumeKept>,
}

/// Resume every paused task a single resume would restart (see
/// [`CopyTask::recovery`]), each through the same update a single resume runs, so the
/// pause is cleared, the active slot is checked and the watch is synchronized with
/// rollback. A task whose recovery is a watch repair stays paused, and a live task
/// resumes only with `include_live`. Each task stands alone: one that fails is rolled
/// back and the tasks resumed before it stay resumed.
pub async fn resume_paused_tasks(include_live: bool) -> Result<CopyResumeOutcome> {
    resume_paused_tasks_with(
        &open_database().await?,
        include_live,
        ExitPolicy::from_config(),
        &|task: CopyTask| async move { sync_task_watch(&task).await },
        &sync_open_position_management,
    )
    .await
}

pub(super) async fn resume_paused_tasks_with<W, WFut, P, PFut>(
    db: &CopyDatabase,
    include_live: bool,
    exit_base: ExitPolicy,
    sync_watch: &W,
    sync_positions: &P,
) -> Result<CopyResumeOutcome>
where
    W: Fn(CopyTask) -> WFut,
    WFut: Future<Output = Result<()>>,
    P: Fn(i64, ExitMode) -> PFut,
    PFut: Future<Output = Result<()>>,
{
    let mut paused = db
        .list_tasks()
        .await?
        .into_iter()
        .filter(|task| !task.enabled)
        .map(|task| task.id)
        .collect::<Vec<_>>();
    paused.sort_unstable();
    let mut outcome = CopyResumeOutcome::default();
    for task_id in paused {
        let resumed = resume_paused_task(
            db,
            task_id,
            include_live,
            exit_base.clone(),
            sync_watch,
            sync_positions,
        )
        .await;
        match resumed {
            Ok(()) => outcome.resumed.push(task_id),
            Err(reason) => outcome.kept_paused.push(CopyResumeKept { task_id, reason }),
        }
    }
    crate::logger::info(
        crate::logger::LogTag::Trader,
        &format!(
            "Copy bulk resume (live included: {include_live}): resumed {:?}, kept paused {:?}",
            outcome.resumed,
            outcome
                .kept_paused
                .iter()
                .map(|kept| (kept.task_id, &kept.reason))
                .collect::<Vec<_>>()
        ),
    );
    Ok(outcome)
}

/// Resume one task of a bulk resume, judged on its row as stored now: a task enabled
/// in the meantime counts as resumed, and the row checked is the row the update
/// applies to.
async fn resume_paused_task<W, WFut, P, PFut>(
    db: &CopyDatabase,
    task_id: i64,
    include_live: bool,
    exit_base: ExitPolicy,
    sync_watch: &W,
    sync_positions: &P,
) -> std::result::Result<(), CopyResumeKeptReason>
where
    W: Fn(CopyTask) -> WFut,
    WFut: Future<Output = Result<()>>,
    P: Fn(i64, ExitMode) -> PFut,
    PFut: Future<Output = Result<()>>,
{
    let task = match db.get_task(task_id).await {
        Ok(Some(task)) => task,
        Ok(None) => return Err(resume_failed(task_id, &Error::CopyTaskNotFound { task_id })),
        Err(error) => return Err(resume_failed(task_id, &error)),
    };
    match task.recovery() {
        None => return Ok(()),
        Some(CopyRecovery::Resume) => {}
        Some(CopyRecovery::RetryWatch | CopyRecovery::RestoreWatch) => {
            return Err(CopyResumeKeptReason::NeedsWatchRecovery)
        }
    }
    if task.mode == TradingMode::Live && !include_live {
        return Err(CopyResumeKeptReason::LiveNotIncluded);
    }
    match apply_task_update(
        db,
        task,
        serde_json::json!({ "enabled": true }),
        exit_base,
        sync_watch,
        sync_positions,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(Error::CopyTaskLimit { maximum }) => Err(CopyResumeKeptReason::TaskLimit { maximum }),
        Err(error) => Err(resume_failed(task_id, &error)),
    }
}

fn resume_failed(task_id: i64, error: &Error) -> CopyResumeKeptReason {
    crate::logger::warning(
        crate::logger::LogTag::Trader,
        &format!("Copy task {task_id} stays paused: its resume failed: {error}"),
    );
    CopyResumeKeptReason::Failed {
        message: error.copy_ui_text(),
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod tests;
