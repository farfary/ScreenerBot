// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper-book maintenance behind the workspace: clone a task, reset its paper
//! book, and close one paper holding by hand.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::future::Future;

use super::super::control::{self, open_database};
use super::super::{notify, CopyMode, CopyTask, CopyTaskInput, PaperPosition};
use crate::trader::{Error, Result};

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloneRequest {
    /// Another wallet to copy with the same rules; the same wallet when absent.
    #[serde(default)]
    pub target_address: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    /// Clones start paused unless asked otherwise.
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// A new paper task with this task's rules, through the normal create path.
pub async fn clone_task(id: i64, request: CloneRequest) -> Result<CopyTask> {
    let original = control::get_task(id).await?;
    let mut input = CopyTaskInput::from(&original);
    input.mode = CopyMode::Paper;
    if let Some(address) = request
        .target_address
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty())
    {
        input.target_address = address;
    }
    input.label = request
        .label
        .map(|label| label.trim().to_owned())
        .filter(|label| !label.is_empty())
        .or_else(|| Some(format!("{} (copy)", notify::task_name(&original))));
    input.enabled = request.enabled.unwrap_or(false);
    control::create_task(input).await
}

#[derive(Debug, Serialize)]
pub struct ResetResult {
    pub task_id: i64,
    pub removed_decisions: usize,
}

/// Start a paper task's evaluation over. Refused for a live task, whose history
/// is the record of real trades.
pub async fn reset_paper_book(id: i64) -> Result<ResetResult> {
    let db = open_database().await?;
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.mode != CopyMode::Paper {
        return Err(Error::CopyValidation {
            detail: "return the task to paper before resetting its paper book".to_owned(),
        });
    }
    let removed_decisions = db.reset_paper_book(id).await?;
    Ok(ResetResult {
        task_id: id,
        removed_decisions,
    })
}

#[derive(Debug, Serialize)]
pub struct ClosedHolding {
    pub task_id: i64,
    pub mint: String,
    pub fill_price_native: f64,
}

/// Sell one paper holding round through the same serialized quote worker as
/// other exits. `expected_opened_at` is the exact opening instant of the round
/// the caller confirmed; a different or newer round is refused unchanged.
pub async fn close_paper_holding(
    id: i64,
    mint: &str,
    expected_opened_at: chrono::DateTime<Utc>,
) -> Result<ClosedHolding> {
    let database = open_database().await?;
    close_paper_holding_with(
        database,
        id,
        mint,
        expected_opened_at,
        |database, task, position, epoch, queued_at| async move {
            super::super::paper_exits::paper_exit_outcome(
                &database,
                &task,
                &position,
                super::super::PaperExitRule::Manual,
                None,
                epoch,
                queued_at,
                crate::swaps::try_get_best_quote,
                |delay| async move {
                    tokio::time::sleep(delay).await;
                    if crate::global::is_force_stopped() {
                        return Err(super::super::CopySkip::ForceStopped);
                    }
                    Ok(())
                },
            )
            .await
        },
    )
    .await
}

pub(super) async fn close_paper_holding_with<F, Fut>(
    db: super::super::CopyDatabase,
    id: i64,
    mint: &str,
    expected_opened_at: chrono::DateTime<Utc>,
    execute_exit: F,
) -> Result<ClosedHolding>
where
    F: FnOnce(
            super::super::CopyDatabase,
            CopyTask,
            PaperPosition,
            u64,
            chrono::DateTime<Utc>,
        ) -> Fut
        + Send
        + 'static,
    Fut: Future<Output = std::result::Result<super::super::CopyOutcome, super::super::CopySkip>>
        + Send
        + 'static,
{
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.mode != CopyMode::Paper {
        return Err(Error::CopyValidation {
            detail: "return the task to paper before closing a paper holding".to_owned(),
        });
    }
    let round_key = db
        .paper_position(id, mint)
        .await?
        .filter(PaperPosition::is_open)
        .ok_or(Error::CopyHoldingNotFound {
            task_id: id,
            mint: mint.to_owned(),
        })?;
    if round_key.opened_at != expected_opened_at {
        return Err(Error::CopyTaskRejected {
            reason: super::super::CopySkip::PaperExecutionUnavailable,
        });
    }
    let epoch = db.paper_epoch(id).await?;
    let Some(runtime) = super::super::paper_execution::active() else {
        let reason = super::super::CopySkip::PaperExecutionUnavailable;
        notify::record(
            &db,
            super::super::paper_execution::skipped_paper_intent(
                id,
                format!(
                    "manual-close-unavailable:{mint}:{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                mint,
                reason.clone(),
                epoch,
            ),
        )
        .await?;
        return Err(Error::CopyTaskRejected { reason });
    };
    let db_job = db.clone();
    let mint_job = mint.to_owned();
    let queued_at = Utc::now();
    let key = format!(
        "manual:{mint}:{}:{}",
        round_key
            .opened_at
            .timestamp_nanos_opt()
            .unwrap_or_default(),
        epoch
    );
    let fallback_mint = mint.to_owned();
    let receiver = match runtime.enqueue_manual(id, key, mint.to_owned(), db.clone(), async move {
        let result: std::result::Result<super::super::CopyOutcome, super::super::CopySkip> =
            async {
                if crate::global::is_force_stopped() {
                    return Err(super::super::CopySkip::ForceStopped);
                }
                let current_task = db_job
                    .get_task(id)
                    .await
                    .map_err(|_| super::super::CopySkip::PaperExecutionUnavailable)?
                    .filter(|task| task.mode == CopyMode::Paper)
                    .ok_or(super::super::CopySkip::TaskDisabled)?;
                let position = db_job
                    .paper_position(id, &mint_job)
                    .await
                    .map_err(|_| super::super::CopySkip::PaperExecutionUnavailable)?
                    .filter(PaperPosition::is_open)
                    .ok_or(super::super::CopySkip::CopyPositionNotFound)?;
                if position.opened_at != round_key.opened_at {
                    return Err(super::super::CopySkip::PaperExecutionUnavailable);
                }
                if !matches!(db_job.paper_epoch(id).await, Ok(current) if current == epoch) {
                    return Err(super::super::CopySkip::PaperExecutionUnavailable);
                }
                let execution_position = position.clone();
                let outcome = execute_exit(
                    db_job.clone(),
                    current_task.clone(),
                    execution_position.clone(),
                    epoch,
                    queued_at,
                )
                .await?;
                if crate::global::is_force_stopped()
                    || !matches!(db_job.paper_epoch(id).await, Ok(current) if current == epoch)
                {
                    return Err(super::super::CopySkip::PaperExecutionUnavailable);
                }
                let current_task_after = db_job
                    .get_task(id)
                    .await
                    .map_err(|_| super::super::CopySkip::PaperExecutionUnavailable)?
                    .filter(|task| {
                        task.mode == CopyMode::Paper
                            && task.slippage_pct == current_task.slippage_pct
                    })
                    .ok_or(super::super::CopySkip::TaskDisabled)?;
                let current_position_after = db_job
                    .paper_position(id, &mint_job)
                    .await
                    .map_err(|_| super::super::CopySkip::PaperExecutionUnavailable)?
                    .filter(PaperPosition::is_open)
                    .filter(|current| {
                        current.opened_at == execution_position.opened_at
                            && current.sells == execution_position.sells
                    })
                    .ok_or(super::super::CopySkip::PaperExecutionUnavailable)?;
                if current_position_after
                    .raw_token_amount
                    .is_none_or(|current| {
                        execution_position
                            .raw_token_amount
                            .is_none_or(|expected| current < expected)
                    })
                    || current_task_after.mode != CopyMode::Paper
                {
                    return Err(super::super::CopySkip::PaperExecutionUnavailable);
                }
                Ok(outcome)
            }
            .await;
        result.unwrap_or_else(|reason| {
            super::super::paper_execution::skipped_paper_intent(
                id,
                format!(
                    "manual-close-skip:{fallback_mint}:{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                &fallback_mint,
                reason,
                epoch,
            )
        })
    }) {
        Ok(receiver) => receiver,
        Err(super::super::paper_execution::EnqueueError::Duplicate) => {
            return Err(Error::CopyTaskRejected {
                reason: super::super::CopySkip::PaperExecutionUnavailable,
            });
        }
        Err(error) => {
            let reason = match error {
                super::super::paper_execution::EnqueueError::Full => {
                    super::super::CopySkip::PaperQueueFull
                }
                super::super::paper_execution::EnqueueError::Unavailable => {
                    super::super::CopySkip::PaperExecutionUnavailable
                }
                super::super::paper_execution::EnqueueError::Duplicate => unreachable!(),
            };
            let skipped = super::super::paper_execution::skipped_paper_intent(
                id,
                format!(
                    "manual-close-queue:{}:{}",
                    mint,
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                mint,
                reason.clone(),
                epoch,
            );
            notify::record(&db, skipped).await?;
            return Err(Error::CopyTaskRejected { reason });
        }
    };
    match receiver.await.map_err(|_| Error::CopyTaskRejected {
        reason: super::super::CopySkip::PaperExecutionUnavailable,
    })? {
        Err(error) => Err(error),
        Ok((outcome, inserted)) => {
            if !inserted {
                return Err(Error::CopyTaskRejected {
                    reason: super::super::CopySkip::PaperExecutionUnavailable,
                });
            }
            match outcome {
                super::super::CopyOutcome::PaperSellObserved(decision) => Ok(ClosedHolding {
                    task_id: id,
                    mint: mint.to_owned(),
                    fill_price_native: decision.telemetry.fill_price_sol.ok_or(
                        Error::CopyTaskRejected {
                            reason: super::super::CopySkip::PaperQuoteUnfillable,
                        },
                    )?,
                }),
                super::super::CopyOutcome::Skipped { reason, .. } => {
                    Err(Error::CopyTaskRejected { reason })
                }
                _ => Err(Error::CopyTaskRejected {
                    reason: super::super::CopySkip::PaperExecutionUnavailable,
                }),
            }
        }
    }
}
