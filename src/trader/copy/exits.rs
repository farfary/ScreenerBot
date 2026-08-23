//! Task-scoped copy-sell planning and execution through the shared exit pipeline.

use std::future::Future;

use chrono::{DateTime, Utc};

use crate::positions::{Position, PositionManagement, PositionOrigin};
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::types::{
    CopyMode, CopyOutcome, CopySellDecision, CopySkip, CopyTask, CopyTelemetry, ExitMode,
};

#[derive(Debug, Clone)]
pub struct PreparedCopySell {
    pub task: CopyTask,
    pub target_signature: String,
    pub target_token_amount: f64,
    pub target_sol_amount: f64,
    pub decision: TradeDecision,
    pub telemetry: CopyTelemetry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopySellSubmitResult {
    Submitted { transaction_signature: String },
    Failed { error: String },
}

impl CopySellSubmitResult {
    pub fn from_trade_result(result: crate::trader::Result<TradeResult>) -> Self {
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

pub fn paper_sell_outcome(
    activity: &WalletActivity,
    task: &CopyTask,
    force_stopped: bool,
    target_holding_before_sell: f64,
    decided_at: DateTime<Utc>,
) -> Result<CopyOutcome, CopySkip> {
    if task.exit_mode == ExitMode::BuyOnly {
        return Err(CopySkip::ExitModeDisabled);
    }
    if force_stopped {
        return Err(CopySkip::ForceStopped);
    }
    let (mint, target_token_amount, target_sol_amount, target_price_sol) = sell_activity(activity)?;
    Ok(CopyOutcome::PaperSellObserved(sell_decision(
        task,
        &activity.signature,
        mint,
        target_token_amount,
        target_sol_amount,
        proportional_exit_percentage(target_token_amount, target_holding_before_sell),
        telemetry(activity, target_price_sol, decided_at),
    )))
}

pub fn prepare_copy_sell(
    activity: &WalletActivity,
    task: &CopyTask,
    position: Option<&Position>,
    force_stopped: bool,
    target_holding_before_sell: f64,
    decided_at: DateTime<Utc>,
) -> Result<PreparedCopySell, CopySkip> {
    if task.mode != CopyMode::Live {
        return Err(CopySkip::ModeTransitionRequired);
    }
    if task.exit_mode == ExitMode::BuyOnly {
        return Err(CopySkip::ExitModeDisabled);
    }
    let (mint, target_token_amount, target_sol_amount, target_price_sol) = sell_activity(activity)?;
    if force_stopped {
        return Err(CopySkip::ForceStopped);
    }
    let position = position.ok_or(CopySkip::CopyPositionNotFound)?;
    let owns_position = matches!(
        &position.origin,
        PositionOrigin::Copy {
            task_id,
            source_wallet,
        } if *task_id == task.id && source_wallet == &task.target_address
    );
    if !owns_position {
        return Err(CopySkip::CopyPositionNotFound);
    }
    if position.management == PositionManagement::UserOnly {
        return Err(CopySkip::PositionUserOnly);
    }
    if !position.management.allows_copy_sell() {
        return Err(CopySkip::PositionManagementMismatch);
    }

    let telemetry = telemetry(activity, target_price_sol, decided_at);
    let exit_percentage =
        proportional_exit_percentage(target_token_amount, target_holding_before_sell);
    Ok(PreparedCopySell {
        task: task.clone(),
        target_signature: activity.signature.clone(),
        target_token_amount,
        target_sol_amount,
        decision: TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: mint.to_owned(),
            action: TradeAction::Sell,
            reason: TradeReason::CopySell,
            strategy_id: None,
            timestamp: decided_at,
            priority: TradePriority::High,
            price_sol: target_price_sol,
            size_sol: None,
            exit_percentage,
            slippage_pct: Some(task.slippage_pct),
        },
        telemetry,
    })
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
        plan.target_sol_amount,
        plan.decision.exit_percentage,
        plan.telemetry.clone(),
    );
    match submit(plan.decision).await {
        CopySellSubmitResult::Submitted {
            transaction_signature,
        } => {
            decision.transaction_signature = Some(transaction_signature);
            decision.telemetry.submitted_at = Some(Utc::now());
            CopyOutcome::LiveSellSubmitted(decision)
        }
        CopySellSubmitResult::Failed { error } => {
            decision.error = Some(error);
            CopyOutcome::LiveSellFailed(decision)
        }
    }
}

fn sell_activity(activity: &WalletActivity) -> Result<(&str, f64, f64, Option<f64>), CopySkip> {
    match &activity.kind {
        ActivityKind::Swap {
            mint,
            side: SwapSide::Sell,
            sol_amount,
            token_amount,
            price_sol,
            ..
        } => Ok((mint, *token_amount, *sol_amount, *price_sol)),
        _ => Err(CopySkip::NotSellSwap),
    }
}

fn sell_decision(
    task: &CopyTask,
    target_signature: &str,
    mint: &str,
    target_token_amount: f64,
    target_sol_amount: f64,
    exit_percentage: Option<f64>,
    telemetry: CopyTelemetry,
) -> CopySellDecision {
    CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: target_signature.to_owned(),
        mint: mint.to_owned(),
        target_token_amount,
        target_sol_amount,
        exit_percentage,
        transaction_signature: None,
        error: None,
        telemetry,
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
    target_price_sol: Option<f64>,
    decided_at: DateTime<Utc>,
) -> CopyTelemetry {
    CopyTelemetry {
        target_block_time: activity.block_time,
        detected_at: activity.detected_at,
        decoded_at: activity.decoded_at,
        decided_at,
        submitted_at: None,
        confirmed_at: None,
        target_price_sol,
        fill_price_sol: None,
    }
}
