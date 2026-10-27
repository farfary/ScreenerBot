// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-buy planning for both books and the paper entry run through the shared paper
//! executor, with injectable admission and dispatch seams.

use std::future::Future;

use chrono::{DateTime, Utc};

use crate::positions::{Book, PositionOrigin};
use crate::trader::admission::EntryBlock;
use crate::trader::executors::ExecutionContext;
use crate::trader::paper::{PaperExecution, PaperLegFill, PaperModel};
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::live::management_for_exit_mode;
use super::risk::precheck;
use super::sizing::size_for;
use super::types::{
    CopyOutcome, CopySkip, CopyTask, CopyTelemetry, PaperContext, PaperDecision, PipelinePolicy,
    RiskContext, SpendState,
};

/// A copy buy planned for the book of its task: the shared trade decision, the
/// provenance its round is opened with, and what the target wallet did.
#[derive(Debug, Clone)]
pub struct PreparedEntry {
    pub task: CopyTask,
    pub target_signature: String,
    pub target_size_native: f64,
    pub target_token_amount: f64,
    pub decision: TradeDecision,
    pub context: ExecutionContext,
    pub telemetry: CopyTelemetry,
}

/// The key a paper leg of copy task `task_id` mirroring the target transaction
/// `signature` is booked under, so the trade books once however often it is replayed.
pub fn leg_key(task_id: i64, signature: &str) -> String {
    format!("copy-{task_id}-{signature}")
}

/// Plans the copy of a target buy for `task`: the risk precheck, the size, and the
/// decision on the task's book. A paper decision carries the leg key of the target
/// transaction; a live decision carries none.
pub fn prepare_entry(
    activity: &WalletActivity,
    task: &CopyTask,
    spend: SpendState,
    risk: RiskContext,
    policy: PipelinePolicy,
    decided_at: DateTime<Utc>,
) -> Result<PreparedEntry, CopySkip> {
    let ActivityKind::Swap {
        mint,
        side: SwapSide::Buy,
        native_amount: target_size_native,
        token_amount: target_token_amount,
        price_native: target_price_native,
        ..
    } = &activity.kind
    else {
        return Err(CopySkip::NotBuySwap);
    };
    precheck(task, *target_size_native, spend, risk, policy)?;
    let sized_native = size_for(
        task,
        *target_size_native,
        spend,
        policy.engine_trade_size_native,
    )?;
    let book = task.mode.book();
    Ok(PreparedEntry {
        task: task.clone(),
        target_signature: activity.signature.clone(),
        target_size_native: *target_size_native,
        target_token_amount: *target_token_amount,
        decision: TradeDecision {
            position_id: None,
            mint: mint.clone(),
            action: TradeAction::Buy,
            reason: TradeReason::CopyBuy,
            strategy_id: None,
            book,
            timestamp: decided_at,
            priority: TradePriority::High,
            price_native: *target_price_native,
            size_native: Some(sized_native),
            exit_percentage: None,
            slippage_pct: Some(task.slippage_pct),
            take_profit_level: None,
            paper_leg_key: (book == Book::Paper).then(|| leg_key(task.id, &activity.signature)),
        },
        context: ExecutionContext {
            origin: PositionOrigin::Copy {
                task_id: task.id,
                source_wallet: task.target_address.clone(),
            },
            management: management_for_exit_mode(task.exit_mode),
        },
        telemetry: CopyTelemetry {
            target_block_time: activity.block_time,
            detected_at: activity.detected_at,
            decoded_at: activity.decoded_at,
            decided_at,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: *target_price_native,
            fill_price_sol: None,
            backfill: activity.backfill,
        },
    })
}

/// Runs a planned paper copy buy: the paper admission of the decision, then the
/// dispatch to the shared paper executor, whose typed evidence becomes the copy
/// outcome. Nothing here reserves an entry or reads live capacity.
pub async fn execute_paper_with<A, AFut, D, DFut>(
    plan: PreparedEntry,
    admission: A,
    dispatch: D,
) -> CopyOutcome
where
    A: FnOnce(TradeDecision) -> AFut,
    AFut: Future<Output = Result<(), EntryBlock>>,
    D: FnOnce(TradeDecision, ExecutionContext) -> DFut,
    DFut: Future<Output = crate::trader::Result<TradeResult>>,
{
    if plan.decision.book != Book::Paper {
        return paper_skip(plan, CopySkip::ModeTransitionRequired);
    }
    if let Err(block) = admission(plan.decision.clone()).await {
        return paper_skip(plan, CopySkip::EntryBlocked { block });
    }
    let execution = match dispatch(plan.decision.clone(), plan.context.clone()).await {
        Ok(result) => result.paper,
        Err(error) => {
            crate::logger::warning(
                crate::logger::LogTag::Trader,
                &format!(
                    "Paper copy buy of {} for task {} did not run: {error}",
                    plan.decision.mint, plan.task.id
                ),
            );
            None
        }
    };
    match execution {
        Some(PaperExecution::Filled(evidence)) => match evidence.fill {
            PaperLegFill::Buy(fill) => {
                let mut telemetry = plan.telemetry;
                telemetry.fill_price_sol = Some(fill.fill_price_sol);
                CopyOutcome::PaperFilled(PaperDecision {
                    task_id: plan.task.id,
                    target_address: plan.task.target_address,
                    signature: plan.target_signature,
                    mint: plan.decision.mint,
                    target_size_sol: plan.target_size_native,
                    target_token_amount: plan.target_token_amount,
                    sized_sol: fill.input_sol,
                    fill,
                    telemetry,
                })
            }
            PaperLegFill::Sell(_) => paper_skip(plan, CopySkip::PaperExecutionUnavailable),
        },
        Some(PaperExecution::Refused(refusal)) => paper_skip(plan, CopySkip::from(refusal)),
        None => paper_skip(plan, CopySkip::PaperExecutionUnavailable),
    }
}

/// The paper decision context every paper copy decision records.
pub(super) fn quote_context() -> PaperContext {
    PaperContext {
        model: PaperModel::Quote,
    }
}

fn paper_skip(plan: PreparedEntry, reason: CopySkip) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id: plan.task.id,
        signature: plan.target_signature,
        mint: Some(plan.decision.mint),
        reason,
        decided_at: plan.telemetry.decided_at,
        telemetry: Some(plan.telemetry),
        paper: Some(quote_context()),
    }
}
