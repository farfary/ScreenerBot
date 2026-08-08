//! Live-copy planning and orchestration with injectable admission/submission seams.

use std::future::Future;

use chrono::{DateTime, Utc};

use crate::positions::{PositionManagement, PositionOrigin};
use crate::trader::admission::EntryBlock;
use crate::trader::entry::EntryContext;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason, TradeResult};
use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::risk::precheck;
use super::sizing::size_for;
use super::types::{
    CopyMode, CopyOutcome, CopySkip, CopyTask, CopyTelemetry, ExitMode, LiveDecision,
    PipelinePolicy, RiskContext, SpendState,
};

#[derive(Debug, Clone)]
pub struct PreparedLiveEntry {
    pub task: CopyTask,
    pub target_signature: String,
    pub target_size_sol: f64,
    pub decision: TradeDecision,
    pub context: EntryContext,
    pub telemetry: CopyTelemetry,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiveSubmitResult {
    Confirmed {
        transaction_signature: String,
        fill_price_sol: Option<f64>,
    },
    Submitted {
        transaction_signature: String,
        fill_price_sol: Option<f64>,
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
                    fill_price_sol: result.executed_price_sol,
                }
            } else {
                Self::Confirmed {
                    transaction_signature,
                    fill_price_sol: result.executed_price_sol,
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

/// Keep already-open positions aligned when a task changes between BuyOnly,
/// Mirror, and Hybrid. Provenance is the selector; no same-mint foreign position
/// is ever modified.
pub async fn sync_open_position_management(task_id: i64, mode: ExitMode) -> Result<(), String> {
    let management = management_for_exit_mode(mode);
    let positions = crate::positions::get_open_positions().await;
    for position in positions {
        if !matches!(
            position.origin,
            PositionOrigin::Copy {
                task_id: origin_task_id,
                ..
            } if origin_task_id == task_id
        ) {
            continue;
        }
        let position_id = position
            .id
            .ok_or_else(|| format!("Open copy position {} has no database id", position.mint))?;
        crate::positions::set_position_management_db(position_id, management).await?;
        if !crate::positions::set_position_management_in_memory(position_id, management).await {
            return Err(format!(
                "Copy position {position_id} disappeared while updating management"
            ));
        }
    }
    Ok(())
}

pub fn prepare_live_entry(
    activity: &WalletActivity,
    task: &CopyTask,
    spend: SpendState,
    risk: RiskContext,
    policy: PipelinePolicy,
    decided_at: DateTime<Utc>,
) -> Result<PreparedLiveEntry, CopySkip> {
    if task.mode != CopyMode::Live {
        return Err(CopySkip::ModeTransitionRequired);
    }
    let ActivityKind::Swap {
        mint,
        side: SwapSide::Buy,
        sol_amount: target_size_sol,
        price_sol: target_price_sol,
        ..
    } = &activity.kind
    else {
        return Err(CopySkip::NotBuySwap);
    };
    precheck(task, *target_size_sol, spend, risk, policy)?;
    let sized_sol = size_for(task, *target_size_sol, spend, policy.engine_trade_size_sol)?;
    Ok(PreparedLiveEntry {
        task: task.clone(),
        target_signature: activity.signature.clone(),
        target_size_sol: *target_size_sol,
        decision: TradeDecision {
            position_id: None,
            mint: mint.clone(),
            action: TradeAction::Buy,
            reason: TradeReason::CopyBuy,
            strategy_id: None,
            timestamp: decided_at,
            priority: TradePriority::High,
            price_sol: *target_price_sol,
            size_sol: Some(sized_sol),
            exit_percentage: None,
            slippage_pct: Some(task.slippage_pct),
        },
        context: EntryContext {
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
            target_price_sol: *target_price_sol,
            fill_price_sol: None,
        },
    })
}

pub async fn execute_live_with<A, AFut, S, SFut>(
    plan: PreparedLiveEntry,
    admission: A,
    submit: S,
) -> CopyOutcome
where
    A: FnOnce(String) -> AFut,
    AFut: Future<Output = Result<(), EntryBlock>>,
    S: FnOnce(TradeDecision, EntryContext) -> SFut,
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
            fill_price_sol,
        } => {
            decision.telemetry.submitted_at = Some(submit_started_at);
            decision.transaction_signature = Some(transaction_signature);
            decision.telemetry.confirmed_at = Some(Utc::now());
            decision.telemetry.fill_price_sol = fill_price_sol;
            CopyOutcome::LiveConfirmed(decision)
        }
        LiveSubmitResult::Submitted {
            transaction_signature,
            fill_price_sol,
        } => {
            decision.telemetry.submitted_at = Some(submit_started_at);
            decision.transaction_signature = Some(transaction_signature);
            decision.telemetry.fill_price_sol = fill_price_sol;
            CopyOutcome::LiveSubmitted(decision)
        }
        LiveSubmitResult::Failed { error } => {
            decision.error = Some(error);
            CopyOutcome::LiveFailed(decision)
        }
    }
}

fn live_decision(plan: &PreparedLiveEntry) -> LiveDecision {
    LiveDecision {
        task_id: plan.task.id,
        target_address: plan.task.target_address.clone(),
        target_signature: plan.target_signature.clone(),
        mint: plan.decision.mint.clone(),
        target_size_sol: plan.target_size_sol,
        sized_sol: plan.decision.size_sol.unwrap_or_default(),
        transaction_signature: None,
        error: None,
        telemetry: plan.telemetry.clone(),
    }
}

fn skipped_from_plan(plan: PreparedLiveEntry, reason: CopySkip) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id: plan.task.id,
        signature: plan.target_signature,
        mint: Some(plan.decision.mint),
        reason,
        decided_at: plan.telemetry.decided_at,
        telemetry: Some(plan.telemetry),
    }
}
