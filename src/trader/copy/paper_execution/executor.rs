// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper-entry job a copy task's FIFO runs, and the recorded skips of copy decisions.

use chrono::Utc;

use super::super::entries::{execute_paper_with, prepare_entry, quote_context};
use super::super::{CopyDatabase, CopyOutcome, CopySkip, CopyTelemetry};
use super::PaperDispatch;
use crate::trader::paper::TradingMode;
use crate::wallets::watch::{ActivityKind, WalletActivity};

mod io;
use io::{current_policy, PaperEntryIo, ProductionPaperEntryIo};

#[cfg(test)]
mod tests;

/// Copies the target buy `activity` into the paper book for task `task_id`: the task as
/// it is now, its risk context and its spend in the current paper run, then the shared
/// paper executor through `dispatch`. Runs on the task's FIFO, so a task's paper
/// fills never interleave.
pub(in crate::trader::copy) async fn execute_paper_entry(
    database: &CopyDatabase,
    activity: &WalletActivity,
    task_id: i64,
    mint: &str,
    dispatch: PaperDispatch,
) -> CopyOutcome {
    execute_paper_entry_with(
        database,
        activity,
        task_id,
        mint,
        &ProductionPaperEntryIo { dispatch },
    )
    .await
}

async fn execute_paper_entry_with(
    database: &CopyDatabase,
    activity: &WalletActivity,
    task_id: i64,
    mint: &str,
    io: &impl PaperEntryIo,
) -> CopyOutcome {
    let reject = |reason| skipped_paper(task_id, activity, mint, reason);
    if io.force_stopped() {
        return reject(CopySkip::ForceStopped);
    }
    let settings = io.settings();
    if !settings.enabled {
        return reject(CopySkip::TaskDisabled);
    }
    let task = match database.get_task(task_id).await {
        Ok(Some(task)) if task.enabled && task.mode == TradingMode::Paper => task,
        Ok(_) => return reject(CopySkip::TaskDisabled),
        Err(_) => return reject(CopySkip::PaperExecutionUnavailable),
    };
    let policy = current_policy(&task, settings);
    let risk = match io
        .risk_context(&task, mint, policy.require_filter_pass)
        .await
    {
        Ok(risk) => risk,
        Err(reason) => return reject(reason),
    };
    let spend = match io.spend(&task, mint).await {
        Ok(spend) => spend,
        Err(reason) => return reject(reason),
    };
    let plan = match prepare_entry(activity, &task, spend, risk, policy, Utc::now()) {
        Ok(plan) => plan,
        Err(reason) => return reject(reason),
    };
    execute_paper_with(
        plan,
        |decision| async move { io.admission(&decision).await },
        |decision, context| io.dispatch(decision, context),
    )
    .await
}

pub(in crate::trader::copy) fn observation_telemetry(activity: &WalletActivity) -> CopyTelemetry {
    CopyTelemetry {
        target_block_time: activity.block_time,
        detected_at: activity.detected_at,
        decoded_at: activity.decoded_at,
        decided_at: Utc::now(),
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: match &activity.kind {
            ActivityKind::Swap { price_native, .. } => *price_native,
            _ => None,
        },
        fill_price_sol: None,
        backfill: activity.backfill,
    }
}

pub(in crate::trader::copy) fn skipped(
    task_id: i64,
    activity: &WalletActivity,
    mint: &str,
    reason: CopySkip,
) -> CopyOutcome {
    skipped_activity(task_id, activity, Some(mint), reason)
}

/// A skip of an observed activity whose traded token may be unknown.
pub(in crate::trader::copy) fn skipped_activity(
    task_id: i64,
    activity: &WalletActivity,
    mint: Option<&str>,
    reason: CopySkip,
) -> CopyOutcome {
    let telemetry = observation_telemetry(activity);
    CopyOutcome::Skipped {
        task_id,
        signature: activity.signature.clone(),
        mint: mint.map(str::to_owned),
        reason,
        decided_at: telemetry.decided_at,
        telemetry: Some(telemetry),
        paper: None,
    }
}

/// A paper task's skip of an observed activity.
pub(in crate::trader::copy) fn skipped_paper(
    task_id: i64,
    activity: &WalletActivity,
    mint: &str,
    reason: CopySkip,
) -> CopyOutcome {
    as_paper(skipped(task_id, activity, mint, reason))
}

/// Marks a skip as a paper task's decision.
pub(in crate::trader::copy) fn as_paper(mut outcome: CopyOutcome) -> CopyOutcome {
    if let CopyOutcome::Skipped { paper, .. } = &mut outcome {
        *paper = Some(quote_context());
    }
    outcome
}

/// A paper task's skip of an action it took itself, such as a close from the Copy page.
pub(in crate::trader::copy) fn skipped_paper_intent(
    task_id: i64,
    signature: String,
    mint: &str,
    reason: CopySkip,
) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id,
        signature,
        mint: Some(mint.to_owned()),
        reason,
        decided_at: Utc::now(),
        telemetry: None,
        paper: Some(quote_context()),
    }
}
