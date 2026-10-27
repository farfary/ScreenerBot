// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-copy orchestration with injectable admission/submission seams, and the
//! management of a task's open rounds in both books.

use std::future::Future;

use chrono::Utc;

use crate::positions::{PositionManagement, PositionOrigin};
use crate::trader::admission::EntryBlock;
use crate::trader::executors::ExecutionContext;
use crate::trader::types::{TradeDecision, TradeResult};

use super::entries::PreparedEntry;
use super::types::{CopyOutcome, CopySkip, ExitMode, LiveDecision};

#[derive(Debug, Clone, PartialEq)]
pub enum LiveSubmitResult {
    Confirmed {
        transaction_signature: String,
        fill_price_native: Option<f64>,
    },
    Submitted {
        transaction_signature: String,
        fill_price_native: Option<f64>,
    },
    Failed {
        error: String,
    },
}

impl LiveSubmitResult {
    pub fn from_trade_result(result: Option<TradeResult>) -> Self {
        let Some(result) = result else {
            return Self::Failed {
                error: "entry submission failed before returning a result".to_owned(),
            };
        };
        if result.success {
            let Some(transaction_signature) = result.tx_signature else {
                return Self::Failed {
                    error: "successful entry submission returned no transaction signature"
                        .to_owned(),
                };
            };
            if result.confirmation_pending {
                Self::Submitted {
                    transaction_signature,
                    fill_price_native: result.executed_price_native,
                }
            } else {
                Self::Confirmed {
                    transaction_signature,
                    fill_price_native: result.executed_price_native,
                }
            }
        } else {
            Self::Failed {
                error: result
                    .error
                    .unwrap_or_else(|| "entry submission failed".to_owned()),
            }
        }
    }
}

pub fn management_for_exit_mode(mode: ExitMode) -> PositionManagement {
    match mode {
        ExitMode::BuyOnly => PositionManagement::AutoTrader,
        ExitMode::Mirror => PositionManagement::CopyTask,
        ExitMode::Hybrid => PositionManagement::Hybrid,
    }
}

fn opened_by(position: &crate::positions::Position, task_id: i64) -> bool {
    matches!(
        position.origin,
        PositionOrigin::Copy {
            task_id: origin_task_id,
            ..
        } if origin_task_id == task_id
    )
}

/// Keep already-open rounds aligned when a task changes between BuyOnly, Mirror, and
/// Hybrid, in the live book and the paper book. Provenance is the selector; no
/// same-mint foreign round is ever modified.
pub async fn sync_open_position_management(
    task_id: i64,
    mode: ExitMode,
) -> crate::trader::Result<()> {
    let management = management_for_exit_mode(mode);
    if let Some(book) = crate::positions::paper::book() {
        for position in book.open_positions().await {
            if !opened_by(&position, task_id) {
                continue;
            }
            let position_id = position
                .id
                .ok_or_else(|| crate::trader::Error::CopyValidation {
                    detail: format!("open copy paper round {} has no database id", position.mint),
                })?;
            if !book.set_management(position_id, management).await? {
                return Err(crate::positions::Error::NotFoundById { position_id }.into());
            }
        }
    }
    let positions = crate::positions::get_open_positions().await;
    for position in positions {
        if !opened_by(&position, task_id) {
            continue;
        }
        let position_id = position
            .id
            .ok_or_else(|| crate::trader::Error::CopyValidation {
                detail: format!("open copy position {} has no database id", position.mint),
            })?;
        crate::positions::set_position_management_db(position_id, management).await?;
        if !crate::positions::set_position_management_in_memory(position_id, management).await {
            return Err(crate::positions::Error::NotFoundById { position_id }.into());
        }
    }
    Ok(())
}

pub async fn execute_live_with<A, AFut, S, SFut>(
    plan: PreparedEntry,
    admission: A,
    submit: S,
) -> CopyOutcome
where
    A: FnOnce(String) -> AFut,
    AFut: Future<Output = Result<(), EntryBlock>>,
    S: FnOnce(TradeDecision, ExecutionContext) -> SFut,
    SFut: Future<Output = LiveSubmitResult>,
{
    if let Err(block) = admission(plan.decision.mint.clone()).await {
        return skipped_from_plan(plan, CopySkip::EntryBlocked { block });
    }

    let submit_started_at = Utc::now();
    let result = submit(plan.decision.clone(), plan.context.clone()).await;
    let mut decision = live_decision(&plan);
    match result {
        LiveSubmitResult::Confirmed {
            transaction_signature,
            fill_price_native,
        } => {
            decision.telemetry.submitted_at = Some(submit_started_at);
            decision.transaction_signature = Some(transaction_signature);
            decision.telemetry.confirmed_at = Some(Utc::now());
            decision.telemetry.fill_price_sol = fill_price_native;
            CopyOutcome::LiveConfirmed(decision)
        }
        LiveSubmitResult::Submitted {
            transaction_signature,
            fill_price_native,
        } => {
            decision.telemetry.submitted_at = Some(submit_started_at);
            decision.transaction_signature = Some(transaction_signature);
            decision.telemetry.fill_price_sol = fill_price_native;
            CopyOutcome::LiveSubmitted(decision)
        }
        LiveSubmitResult::Failed { error } => {
            decision.error = Some(error);
            CopyOutcome::LiveFailed(decision)
        }
    }
}

fn live_decision(plan: &PreparedEntry) -> LiveDecision {
    LiveDecision {
        task_id: plan.task.id,
        target_address: plan.task.target_address.clone(),
        target_signature: plan.target_signature.clone(),
        mint: plan.decision.mint.clone(),
        target_size_sol: plan.target_size_native,
        target_token_amount: plan.target_token_amount,
        sized_sol: plan.decision.size_native.unwrap_or_default(),
        transaction_signature: None,
        error: None,
        telemetry: plan.telemetry.clone(),
    }
}

fn skipped_from_plan(plan: PreparedEntry, reason: CopySkip) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id: plan.task.id,
        signature: plan.target_signature,
        mint: Some(plan.decision.mint),
        reason,
        decided_at: plan.telemetry.decided_at,
        telemetry: Some(plan.telemetry),
        paper: None,
    }
}
