//! Offline-capable paper pipeline from observed activity to typed outcomes.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::matcher::matching_tasks;
use super::paper::{simulate_fill, PaperCosts};
use super::risk::precheck;
use super::sizing::size_for;
use super::types::{
    CopyOutcome, CopySkip, CopyTask, CopyTelemetry, PaperDecision, PipelinePolicy, RiskContext,
    SpendState,
};

pub fn run_paper_pipeline(
    activity: &WalletActivity,
    tasks: &[CopyTask],
    spend_by_task: &HashMap<i64, SpendState>,
    risk_by_task: &HashMap<i64, RiskContext>,
    policy: PipelinePolicy,
    market_price_sol: f64,
    costs: PaperCosts,
    decided_at: DateTime<Utc>,
) -> Vec<CopyOutcome> {
    let matches = matching_tasks(activity, tasks);
    matches
        .into_iter()
        .map(|task| {
            let ActivityKind::Swap {
                mint,
                side: SwapSide::Buy,
                sol_amount: target_size_sol,
                price_sol: target_price_sol,
                ..
            } = &activity.kind
            else {
                return skipped(task, activity, None, CopySkip::NotBuySwap, decided_at);
            };

            let spend = spend_by_task.get(&task.id).copied().unwrap_or_default();
            let context = risk_by_task.get(&task.id).copied().unwrap_or_default();
            if let Err(reason) = precheck(task, *target_size_sol, spend, context, policy) {
                return skipped(task, activity, Some(mint.clone()), reason, decided_at);
            }
            let sized_sol =
                match size_for(task, *target_size_sol, spend, policy.engine_trade_size_sol) {
                    Ok(size) => size,
                    Err(reason) => {
                        return skipped(task, activity, Some(mint.clone()), reason, decided_at)
                    }
                };
            let fill = match simulate_fill(sized_sol, market_price_sol, task.slippage_pct, costs) {
                Ok(fill) => fill,
                Err(reason) => {
                    return skipped(task, activity, Some(mint.clone()), reason, decided_at)
                }
            };

            CopyOutcome::PaperFilled(PaperDecision {
                task_id: task.id,
                target_address: task.target_address.clone(),
                signature: activity.signature.clone(),
                mint: mint.clone(),
                target_size_sol: *target_size_sol,
                sized_sol,
                fill: fill.clone(),
                telemetry: CopyTelemetry {
                    target_block_time: activity.block_time,
                    detected_at: activity.detected_at,
                    decoded_at: activity.decoded_at,
                    decided_at,
                    submitted_at: None,
                    confirmed_at: Some(decided_at),
                    target_price_sol: *target_price_sol,
                    fill_price_sol: Some(fill.fill_price_sol),
                },
            })
        })
        .collect()
}

fn skipped(
    task: &CopyTask,
    activity: &WalletActivity,
    mint: Option<String>,
    reason: CopySkip,
    decided_at: DateTime<Utc>,
) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id: task.id,
        signature: activity.signature.clone(),
        mint,
        reason,
        decided_at,
    }
}
