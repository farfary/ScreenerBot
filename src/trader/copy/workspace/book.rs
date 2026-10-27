// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper maintenance behind the workspace: clone a task, start its paper run over,
//! and close one paper round by hand.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::future::Future;

use super::super::control::{self, open_database};
use super::super::entries::quote_context;
use super::super::exits::{owns_copy_position, sell_decision};
use super::super::paper_execution::{
    self, queue_refusal, skipped_paper_intent, EnqueueError, PaperExecutionHandle,
};
use super::super::{
    notify, paper_book, CopyDatabase, CopyOutcome, CopySellSubmitResult, CopySkip, CopyTask,
    CopyTaskInput, CopyTelemetry, PaperExitRule,
};
use crate::positions::{books, Book, Position};
use crate::trader::paper::{PaperSellFill, TradingMode};
use crate::trader::{Error, Result};

#[cfg(test)]
#[path = "book_tests.rs"]
mod tests;

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
    input.mode = TradingMode::Paper;
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
    /// Open paper rounds of the task closed at their last price by the reset.
    pub closed_rounds: usize,
}

/// Starts a paper task's run over: its open paper rounds close at their last price
/// and its figures count only what happens from now on. Nothing is deleted; the closed
/// rounds stay in the paper history and earlier decisions stay in Activity. Runs on the
/// task's paper queue so no copy of the task interleaves with it. Refused for a live
/// task, whose history is the record of real trades.
pub async fn reset_paper_book(id: i64) -> Result<ResetResult> {
    let db = open_database().await?;
    reset_paper_book_with(
        db,
        paper_execution::active(),
        id,
        |task| async move {
            Ok(paper_book::task_run(&task)
                .await?
                .open_rounds()
                .cloned()
                .collect())
        },
        crate::trader::paper::settle_rounds,
    )
    .await
}

pub(super) async fn reset_paper_book_with<R, RFut, S, SFut>(
    db: CopyDatabase,
    runtime: Option<PaperExecutionHandle>,
    id: i64,
    open_rounds: R,
    settle: S,
) -> Result<ResetResult>
where
    R: FnOnce(CopyTask) -> RFut + Send + 'static,
    RFut: Future<Output = Result<Vec<Position>>> + Send + 'static,
    S: FnOnce(Vec<Position>) -> SFut + Send + 'static,
    SFut: Future<Output = Result<usize>> + Send + 'static,
{
    paper_task(
        &db,
        id,
        "return the task to paper before resetting its paper run",
    )
    .await?;
    let job_db = db.clone();
    let job = async move {
        let task = paper_task(&job_db, id, "the task left paper before its run was reset").await?;
        let closed_rounds = settle(open_rounds(task).await?).await?;
        job_db.start_paper_run(id, Utc::now()).await?;
        Ok(closed_rounds)
    };
    // Without a running queue no paper job of the task can run, so the reset runs here.
    let closed_rounds = match runtime {
        None => job.await?,
        Some(runtime) => runtime
            .enqueue_step(id, paper_execution::reset_key(), db, job)
            .map_err(|error| Error::CopyTaskRejected {
                reason: queue_refusal(error),
            })?
            .await
            .map_err(|_| Error::CopyTaskRejected {
                reason: CopySkip::PaperExecutionUnavailable,
            })??,
    };
    Ok(ResetResult {
        task_id: id,
        closed_rounds,
    })
}

/// The task `id` while it runs in paper; `detail` says why another mode is refused.
async fn paper_task(db: &CopyDatabase, id: i64, detail: &str) -> Result<CopyTask> {
    let task = db
        .get_task(id)
        .await?
        .ok_or(Error::CopyTaskNotFound { task_id: id })?;
    if task.mode != TradingMode::Paper {
        return Err(Error::CopyValidation {
            detail: detail.to_owned(),
        });
    }
    Ok(task)
}

#[derive(Debug, Serialize)]
pub struct ClosedHolding {
    pub task_id: i64,
    pub mint: String,
    pub fill_price_native: f64,
}

/// Closes one paper round of a task in full by hand, on the task's paper queue, through
/// the shared paper executor. `expected_opened_at` is the exact opening instant of the
/// round the caller confirmed; a different or newer round of the mint is refused
/// unchanged.
pub async fn close_paper_holding(
    id: i64,
    mint: &str,
    expected_opened_at: DateTime<Utc>,
) -> Result<ClosedHolding> {
    let database = open_database().await?;
    close_paper_holding_with(
        database,
        paper_execution::active(),
        id,
        mint,
        expected_opened_at,
        |mint| async move { books::open_round(Book::Paper, &mint).await },
        |position| async move {
            CopySellSubmitResult::from_trade_result(
                crate::trader::manual::manual_paper_close(position).await,
            )
        },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn close_paper_holding_with<R, RFut, C, CFut>(
    db: CopyDatabase,
    runtime: Option<PaperExecutionHandle>,
    id: i64,
    mint: &str,
    expected_opened_at: DateTime<Utc>,
    open_round: R,
    close: C,
) -> Result<ClosedHolding>
where
    R: Fn(String) -> RFut + Send + Sync + 'static,
    RFut: Future<Output = Option<Position>> + Send + 'static,
    C: FnOnce(Position) -> CFut + Send + 'static,
    CFut: Future<Output = CopySellSubmitResult> + Send + 'static,
{
    let task = paper_task(
        &db,
        id,
        "return the task to paper before closing a paper holding",
    )
    .await?;
    let round = open_round(mint.to_owned())
        .await
        .filter(|position| owns_copy_position(position, &task))
        .ok_or(Error::CopyHoldingNotFound {
            task_id: id,
            mint: mint.to_owned(),
        })?;
    let position_id = match round.id {
        Some(position_id) if round.entry_time == expected_opened_at => position_id,
        _ => {
            return Err(Error::CopyTaskRejected {
                reason: CopySkip::PaperExecutionUnavailable,
            })
        }
    };
    let Some(runtime) = runtime else {
        let reason = CopySkip::PaperExecutionUnavailable;
        record_close_skip(&db, id, "manual-close-unavailable", mint, reason.clone()).await?;
        return Err(Error::CopyTaskRejected { reason });
    };
    let job_db = db.clone();
    let job_mint = mint.to_owned();
    let job = async move {
        let outcome = async {
            if crate::global::is_force_stopped() {
                return Err(CopySkip::ForceStopped);
            }
            let task = job_db
                .get_task(id)
                .await
                .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                .filter(|task| task.mode == TradingMode::Paper)
                .ok_or(CopySkip::TaskDisabled)?;
            let position = open_round(job_mint.clone())
                .await
                .filter(|position| {
                    position.id == Some(position_id) && owns_copy_position(position, &task)
                })
                .ok_or(CopySkip::CopyPositionNotFound)?;
            match close(position).await {
                CopySellSubmitResult::PaperFilled { fill } => {
                    Ok(manual_close_outcome(&task, &job_mint, position_id, fill))
                }
                CopySellSubmitResult::PaperRefused { refusal } => Err(CopySkip::from(refusal)),
                _ => Err(CopySkip::PaperExecutionUnavailable),
            }
        }
        .await;
        outcome.unwrap_or_else(|reason| {
            skipped_paper_intent(
                id,
                close_signature("manual-close-skip", &job_mint),
                &job_mint,
                reason,
            )
        })
    };
    let receiver = match runtime.enqueue_manual(
        id,
        paper_execution::manual_close_key(position_id),
        mint.to_owned(),
        db.clone(),
        job,
    ) {
        Ok(receiver) => receiver,
        Err(EnqueueError::Duplicate) => {
            return Err(Error::CopyTaskRejected {
                reason: CopySkip::PaperExecutionUnavailable,
            });
        }
        Err(error) => {
            let reason = queue_refusal(error);
            record_close_skip(&db, id, "manual-close-queue", mint, reason.clone()).await?;
            return Err(Error::CopyTaskRejected { reason });
        }
    };
    let (outcome, inserted) = receiver.await.map_err(|_| Error::CopyTaskRejected {
        reason: CopySkip::PaperExecutionUnavailable,
    })??;
    if !inserted {
        return Err(Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable,
        });
    }
    match outcome {
        CopyOutcome::PaperSellObserved(decision) => Ok(ClosedHolding {
            task_id: id,
            mint: mint.to_owned(),
            fill_price_native: decision.telemetry.fill_price_sol.ok_or(
                Error::CopyTaskRejected {
                    reason: CopySkip::PaperQuoteUnfillable,
                },
            )?,
        }),
        CopyOutcome::Skipped { reason, .. } => Err(Error::CopyTaskRejected { reason }),
        _ => Err(Error::CopyTaskRejected {
            reason: CopySkip::PaperExecutionUnavailable,
        }),
    }
}

/// The signature of a recorded manual-close skip: `prefix`, the mint and the moment, so
/// every refused attempt is its own Activity row.
fn close_signature(prefix: &str, mint: &str) -> String {
    format!(
        "{prefix}:{mint}:{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

async fn record_close_skip(
    db: &CopyDatabase,
    id: i64,
    prefix: &str,
    mint: &str,
    reason: CopySkip,
) -> Result<()> {
    notify::record(
        db,
        skipped_paper_intent(id, close_signature(prefix, mint), mint, reason),
    )
    .await?;
    Ok(())
}

/// The Activity record of a manual close of paper round `position_id` that filled.
/// The round's id names it, so a round's close is recorded once.
fn manual_close_outcome(
    task: &CopyTask,
    mint: &str,
    position_id: i64,
    fill: PaperSellFill,
) -> CopyOutcome {
    let now = Utc::now();
    let mut decision = sell_decision(
        task,
        &format!("manual-close:{position_id}"),
        mint,
        0.0,
        0.0,
        None,
        CopyTelemetry {
            target_block_time: None,
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: Some(fill.fill_price_sol),
            backfill: false,
        },
    );
    decision.paper_fill = Some(fill);
    decision.exit_rule = Some(PaperExitRule::Manual);
    decision.paper_context = Some(quote_context());
    CopyOutcome::PaperSellObserved(decision)
}
