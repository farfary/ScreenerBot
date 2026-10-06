// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Offline-capable paper pipeline from observed activity to typed outcomes.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::matcher::matching_tasks;
use super::paper::{simulate_fill, PaperCosts, PaperMarket};
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
    market: PaperMarket,
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
                native_amount: target_size_native,
                price_native: target_price_native,
                ..
            } = &activity.kind
            else {
                return skipped(task, activity, None, CopySkip::NotBuySwap, decided_at);
            };

            let spend = spend_by_task.get(&task.id).copied().unwrap_or_default();
            let context = risk_by_task.get(&task.id).copied().unwrap_or_default();
            if let Err(reason) = precheck(task, *target_size_native, spend, context, policy) {
                return skipped(task, activity, Some(mint.clone()), reason, decided_at);
            }
            let sized_native = match size_for(
                task,
                *target_size_native,
                spend,
                policy.engine_trade_size_native,
            ) {
                Ok(size) => size,
                Err(reason) => {
                    return skipped(task, activity, Some(mint.clone()), reason, decided_at)
                }
            };
            let fill = match simulate_fill(sized_native, market, task.slippage_pct, costs) {
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
                target_size_sol: *target_size_native,
                target_token_amount: match &activity.kind {
                    ActivityKind::Swap { token_amount, .. } => *token_amount,
                    _ => 0.0,
                },
                sized_sol: sized_native,
                fill: fill.clone(),
                telemetry: CopyTelemetry {
                    target_block_time: activity.block_time,
                    detected_at: activity.detected_at,
                    decoded_at: activity.decoded_at,
                    decided_at,
                    submitted_at: None,
                    confirmed_at: Some(decided_at),
                    target_price_sol: *target_price_native,
                    fill_price_sol: Some(fill.fill_price_sol),
                    backfill: activity.backfill,
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
        telemetry: Some(CopyTelemetry {
            target_block_time: activity.block_time,
            detected_at: activity.detected_at,
            decoded_at: activity.decoded_at,
            decided_at,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: None,
            backfill: activity.backfill,
        }),
    }
}
