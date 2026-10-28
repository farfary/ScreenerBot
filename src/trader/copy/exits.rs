// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task-scoped copy-sell planning and execution through the shared exit pipeline.

use std::future::Future;

use chrono::{DateTime, Utc};

use crate::positions::{Book, Position, PositionManagement, PositionOrigin};
use crate::trader::paper::{PaperExecution, PaperLegFill, PaperRefusal, PaperSellFill};
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeResult};
use crate::trader::TradeReason;
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::entries::{leg_key, quote_context};
use super::types::{CopyOutcome, CopySellDecision, CopySkip, CopyTask, CopyTelemetry, ExitMode};

#[derive(Debug, Clone)]
pub struct PreparedCopySell {
    pub task: CopyTask,
    pub target_signature: String,
    pub target_token_amount: f64,
    pub target_native_amount: f64,
    pub decision: TradeDecision,
    pub telemetry: CopyTelemetry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopySellSubmitResult {
    Submitted {
        transaction_signature: String,
    },
    /// The paper executor booked the sale on the task's paper round.
    PaperFilled {
        fill: PaperSellFill,
    },
    /// The paper executor booked nothing.
    PaperRefused {
        refusal: PaperRefusal,
    },
    Failed {
        error: String,
    },
}

impl CopySellSubmitResult {
    pub fn from_trade_result(result: crate::trader::Result<TradeResult>) -> Self {
        if let Ok(TradeResult {
            paper: Some(execution),
            ..
        }) = &result
        {
            return match execution {
                PaperExecution::Filled(evidence) => match &evidence.fill {
                    PaperLegFill::Sell(fill) => Self::PaperFilled { fill: fill.clone() },
                    PaperLegFill::Buy(_) => Self::Failed {
                        error: "a copy sell booked a paper buy".to_owned(),
                    },
                },
                PaperExecution::Refused(refusal) => Self::PaperRefused { refusal: *refusal },
            };
        }
        match result {
            Ok(result) if result.success => match result.tx_signature {
                Some(transaction_signature) => Self::Submitted {
                    transaction_signature,
                },
                None => Self::Failed {
                    error: "successful copy sell returned no transaction signature".to_owned(),
                },
            },
            Ok(result) => Self::Failed {
                error: result
                    .error
                    .unwrap_or_else(|| "copy sell submission failed".to_owned()),
            },
            Err(error) => Self::Failed {
                error: error.to_string(),
            },
        }
    }
}

pub fn prepare_copy_sell(
    activity: &WalletActivity,
    task: &CopyTask,
    position: Option<&Position>,
    force_stopped: bool,
    target_holding_before_sell: f64,
    decided_at: DateTime<Utc>,
) -> Result<PreparedCopySell, CopySkip> {
    if task.exit_mode == ExitMode::BuyOnly {
        return Err(CopySkip::ExitModeDisabled);
    }
    let (mint, target_token_amount, target_native_amount, target_price_native) =
        sell_activity(activity)?;
    if force_stopped {
        return Err(CopySkip::ForceStopped);
    }
    let position = position
        .filter(|position| owns_copy_position(position, task))
        .ok_or(CopySkip::CopyPositionNotFound)?;
    if position.book != task.mode.book() {
        return Err(CopySkip::ModeTransitionRequired);
    }
    if position.management == PositionManagement::UserOnly {
        return Err(CopySkip::PositionUserOnly);
    }
    if !position.management.allows_copy_sell() {
        return Err(CopySkip::PositionManagementMismatch);
    }

    let telemetry = telemetry(activity, target_price_native, decided_at);
    let exit_percentage =
        proportional_exit_percentage(target_token_amount, target_holding_before_sell);
    Ok(PreparedCopySell {
        task: task.clone(),
        target_signature: activity.signature.clone(),
        target_token_amount,
        target_native_amount,
        decision: TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: mint.to_owned(),
            action: TradeAction::Sell,
            reason: TradeReason::CopySell,
            strategy_id: None,
            book: position.book,
            timestamp: decided_at,
            priority: TradePriority::High,
            price_native: target_price_native,
            size_native: None,
            exit_percentage,
            slippage_pct: Some(task.slippage_pct),
            take_profit_level: None,
            paper_leg_key: (position.book == Book::Paper)
                .then(|| leg_key(task.id, &activity.signature)),
        },
        telemetry,
    })
}

/// Whether this task opened `position` from its own target wallet.
pub fn owns_copy_position(position: &Position, task: &CopyTask) -> bool {
    matches!(
        &position.origin,
        PositionOrigin::Copy {
            task_id,
            source_wallet,
        } if *task_id == task.id && source_wallet == &task.target_address
    )
}

pub async fn execute_copy_sell_with<S, SFut>(plan: PreparedCopySell, submit: S) -> CopyOutcome
where
    S: FnOnce(TradeDecision) -> SFut,
    SFut: Future<Output = CopySellSubmitResult>,
{
    let mut decision = sell_decision(
        &plan.task,
        &plan.target_signature,
        &plan.decision.mint,
        plan.target_token_amount,
        plan.target_native_amount,
        plan.decision.exit_percentage,
        plan.telemetry.clone(),
    );
    let paper = plan.decision.book == Book::Paper;
    let skip = |reason| CopyOutcome::Skipped {
        task_id: plan.task.id,
        signature: plan.target_signature.clone(),
        mint: Some(plan.decision.mint.clone()),
        reason,
        decided_at: plan.telemetry.decided_at,
        telemetry: Some(plan.telemetry.clone()),
        paper: Some(quote_context()),
    };
    match submit(plan.decision.clone()).await {
        CopySellSubmitResult::Submitted {
            transaction_signature,
        } if !paper => {
            decision.transaction_signature = Some(transaction_signature);
            decision.telemetry.submitted_at = Some(Utc::now());
            CopyOutcome::LiveSellSubmitted(decision)
        }
        CopySellSubmitResult::PaperFilled { fill } if paper => {
            decision.telemetry.fill_price_sol = Some(fill.fill_price_sol);
            decision.paper_fill = Some(fill);
            decision.paper_context = Some(quote_context());
            CopyOutcome::PaperSellObserved(decision)
        }
        CopySellSubmitResult::PaperRefused { refusal } if paper => skip(CopySkip::from(refusal)),
        CopySellSubmitResult::Failed { error } if !paper => {
            decision.error = Some(error);
            CopyOutcome::LiveSellFailed(decision)
        }
        _ if paper => skip(CopySkip::PaperExecutionUnavailable),
        _ => {
            decision.error = Some("a live copy sell returned a paper result".to_owned());
            CopyOutcome::LiveSellFailed(decision)
        }
    }
}

fn sell_activity(activity: &WalletActivity) -> Result<(&str, f64, f64, Option<f64>), CopySkip> {
    match &activity.kind {
        ActivityKind::Swap {
            mint,
            side: SwapSide::Sell,
            native_amount,
            token_amount,
            price_native,
            ..
        } => Ok((mint, *token_amount, *native_amount, *price_native)),
        _ => Err(CopySkip::NotSellSwap),
    }
}

/// The Activity record of a copy sell of `mint` before it is executed.
pub(super) fn sell_decision(
    task: &CopyTask,
    target_signature: &str,
    mint: &str,
    target_token_amount: f64,
    target_native_amount: f64,
    exit_percentage: Option<f64>,
    telemetry: CopyTelemetry,
) -> CopySellDecision {
    CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: target_signature.to_owned(),
        mint: mint.to_owned(),
        target_token_amount,
        target_sol_amount: target_native_amount,
        exit_percentage,
        transaction_signature: None,
        error: None,
        telemetry,
        paper_fill: None,
        exit_rule: None,
        take_profit_level: None,
        paper_context: None,
    }
}

/// Mirror the fraction of the target's observed inventory that was sold. Unknown
/// inventory fails safely to a full close; a known fraction is always explicit so
/// the global partial-exit toggle cannot widen it.
pub fn proportional_exit_percentage(
    target_token_amount: f64,
    target_holding_before_sell: f64,
) -> Option<f64> {
    if !target_token_amount.is_finite()
        || target_token_amount <= 0.0
        || !target_holding_before_sell.is_finite()
        || target_holding_before_sell <= 0.0
    {
        return None;
    }
    Some((target_token_amount / target_holding_before_sell * 100.0).clamp(0.01, 100.0))
}

fn telemetry(
    activity: &WalletActivity,
    target_price_native: Option<f64>,
    decided_at: DateTime<Utc>,
) -> CopyTelemetry {
    CopyTelemetry {
        target_block_time: activity.block_time,
        detected_at: activity.detected_at,
        decoded_at: activity.decoded_at,
        decided_at,
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: target_price_native,
        fill_price_sol: None,
        backfill: activity.backfill,
    }
}
