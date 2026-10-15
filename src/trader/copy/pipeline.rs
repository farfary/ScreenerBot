// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Offline-capable paper pipeline from observed activity to typed outcomes.

use std::collections::HashMap;

use crate::wallets::watch::{ActivityKind, SwapSide, WalletActivity};

use super::matcher::matching_tasks;
use super::risk::precheck;
use super::sizing::size_for;
use super::types::{CopySkip, CopyTask, PipelinePolicy, RiskContext, SpendState};

#[derive(Debug, Clone)]
pub struct PaperEntryPlan {
    pub task: CopyTask,
    pub signature: String,
    pub mint: String,
    pub target_size_sol: f64,
    pub target_token_amount: f64,
    pub sized_sol: f64,
}

pub fn plan_paper_entries(
    activity: &WalletActivity,
    tasks: &[CopyTask],
    spend_by_task: &HashMap<i64, SpendState>,
    risk_by_task: &HashMap<i64, RiskContext>,
    policy: PipelinePolicy,
) -> Vec<Result<PaperEntryPlan, (CopyTask, CopySkip)>> {
    let matches = matching_tasks(activity, tasks);
    matches
        .into_iter()
        .map(|task| {
            let ActivityKind::Swap {
                mint,
                side: SwapSide::Buy,
                native_amount: target_size_native,
                ..
            } = &activity.kind
            else {
                return Err((task.clone(), CopySkip::NotBuySwap));
            };

            let spend = spend_by_task.get(&task.id).copied().unwrap_or_default();
            let context = risk_by_task.get(&task.id).copied().unwrap_or_default();
            if let Err(reason) = precheck(task, *target_size_native, spend, context, policy) {
                return Err((task.clone(), reason));
            }
            let sized_native = match size_for(
                task,
                *target_size_native,
                spend,
                policy.engine_trade_size_native,
            ) {
                Ok(size) => size,
                Err(reason) => return Err((task.clone(), reason)),
            };
            let target_token_amount = match &activity.kind {
                ActivityKind::Swap { token_amount, .. } => *token_amount,
                _ => 0.0,
            };
            Ok(PaperEntryPlan {
                task: task.clone(),
                signature: activity.signature.clone(),
                mint: mint.clone(),
                target_size_sol: *target_size_native,
                target_token_amount,
                sized_sol: sized_native,
            })
        })
        .collect()
}
