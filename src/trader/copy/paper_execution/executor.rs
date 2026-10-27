// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper fill planning, quote settlement, and recorded skip construction.

use super::super::{
    CopyDatabase, CopyOutcome, CopySkip, CopyTelemetry, PaperContext, PaperDecision, PaperEntryPlan,
};
use crate::trader::copy::{risk, sizing, types};
use crate::trader::paper::TradingMode;
use crate::trader::paper::{fill, PaperModel};
use crate::wallets::watch::{ActivityKind, WalletActivity};
use chrono::Utc;

mod io;
use io::{current_policy, PaperEntryIo, ProductionPaperEntryIo};

#[cfg(test)]
mod tests;

pub(in crate::trader::copy) async fn execute_paper_entry(
    database: &CopyDatabase,
    activity: &WalletActivity,
    plan: PaperEntryPlan,
    epoch: u64,
    queued_at: chrono::DateTime<Utc>,
) -> CopyOutcome {
    execute_paper_entry_with(
        database,
        activity,
        plan,
        epoch,
        queued_at,
        &ProductionPaperEntryIo,
    )
    .await
}

async fn execute_paper_entry_with(
    database: &CopyDatabase,
    activity: &WalletActivity,
    plan: PaperEntryPlan,
    epoch: u64,
    queued_at: chrono::DateTime<Utc>,
    io: &impl PaperEntryIo,
) -> CopyOutcome {
    use crate::chains::RawAmount;
    use crate::swaps::{QuoteRequest, SwapMode};
    let task_id = plan.task.id;
    let reject = |reason| skipped_paper(task_id, activity, &plan.mint, reason, epoch);
    if io.force_stopped() {
        return reject(CopySkip::ForceStopped);
    }
    let settings = io.settings();
    if !settings.enabled {
        return reject(CopySkip::TaskDisabled);
    };
    let task = match database.get_task(task_id).await {
        Ok(Some(task)) if task.enabled && task.mode == TradingMode::Paper => task,
        _ => return reject(CopySkip::TaskDisabled),
    };
    if !matches!(database.paper_epoch(task_id).await, Ok(current) if current == epoch) {
        return reject(CopySkip::PaperExecutionUnavailable);
    }
    if let Err(block) = io.check_entry_admission(&plan.mint, &["rpc"]).await {
        return reject(CopySkip::EntryBlocked { block });
    }
    let policy = current_policy(&task, settings);
    let risk = match io
        .risk_context(&task, &plan.mint, policy.require_filter_pass)
        .await
    {
        Ok(risk) => risk,
        Err(reason) => return reject(reason),
    };
    let spend = match database
        .spend_state(task_id, TradingMode::Paper, &plan.mint)
        .await
    {
        Ok(spend) => spend,
        Err(_) => return reject(CopySkip::PaperExecutionUnavailable),
    };
    let target_size = plan.target_size_sol;
    if let Err(reason) = risk::precheck(&task, target_size, spend, risk, policy) {
        return reject(reason);
    }
    let sized_sol =
        match sizing::size_for(&task, target_size, spend, policy.engine_trade_size_native) {
            Ok(size) => size,
            Err(reason) => return reject(reason),
        };
    let Some(decimals) = io.token_decimals(task.chain, &plan.mint).await else {
        return reject(CopySkip::PaperQuoteUnfillable);
    };
    let adapter = crate::chains::adapter_for(task.chain);
    let Some(input_amount) = RawAmount::from_integral_f64(
        (sized_sol * 10_f64.powi(i32::from(adapter.native_asset_decimals()))).floor(),
    )
    .filter(|amount| *amount > RawAmount::ZERO) else {
        return reject(CopySkip::InvalidSizing);
    };
    let actual_input_sol = input_amount.to_whole_units(adapter.native_asset_decimals());
    let wallet_address = match io.wallet_address().await {
        Ok(address) => address,
        Err(_) => return reject(CopySkip::PaperQuoteUnfillable),
    };
    let request = QuoteRequest {
        chain: task.chain,
        input_mint: adapter.native_asset_address().to_owned(),
        output_mint: plan.mint.clone(),
        input_amount,
        wallet_address,
        slippage_pct: task.slippage_pct,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    };
    let delay_ms = settings.paper_landing_delay_ms;
    let late_guard_mint = plan.mint.clone();
    let late_guard_task = task.clone();
    let fill = match crate::global::paper_scope(fill::simulate_buy(
        request,
        decimals,
        delay_ms,
        queued_at,
        epoch,
        |request| io.quote(request),
        |delay| async move {
            io.wait(delay).await?;
            late_wait_guard(
                database,
                io,
                task_id,
                &late_guard_mint,
                &late_guard_task,
                epoch,
            )
            .await
        },
    ))
    .await
    {
        Ok(fill) => fill,
        Err(reason) => return reject(reason),
    };
    if io.force_stopped() {
        return reject(CopySkip::ForceStopped);
    }
    let current_settings = io.settings();
    if !current_settings.enabled {
        return reject(CopySkip::TaskDisabled);
    }
    if !matches!(database.paper_epoch(task_id).await, Ok(current) if current == epoch) {
        return reject(CopySkip::PaperExecutionUnavailable);
    }
    let current = match database.get_task(task_id).await {
        Ok(Some(current)) if current.enabled && current.mode == TradingMode::Paper => current,
        _ => return reject(CopySkip::TaskDisabled),
    };
    if current.slippage_pct != task.slippage_pct {
        return reject(CopySkip::PaperExecutionUnavailable);
    }
    if let Err(block) = io.check_entry_admission(&plan.mint, &["rpc"]).await {
        return reject(CopySkip::EntryBlocked { block });
    }
    let current_policy = current_policy(&current, current_settings);
    let current_risk = match io
        .risk_context(&current, &plan.mint, current_policy.require_filter_pass)
        .await
    {
        Ok(risk) => risk,
        Err(reason) => return reject(reason),
    };
    let current_spend = match database
        .spend_state(task_id, TradingMode::Paper, &plan.mint)
        .await
    {
        Ok(spend) => spend,
        Err(_) => return reject(CopySkip::PaperExecutionUnavailable),
    };
    if let Err(reason) = risk::precheck(
        &current,
        plan.target_size_sol,
        current_spend,
        current_risk,
        current_policy,
    ) {
        return reject(reason);
    }
    if current_spend.total_spent_native + fill.total_cost_sol > current.total_budget_native {
        return reject(CopySkip::BudgetExhausted);
    }
    if current_spend.token_spent_native + fill.total_cost_sol > current.max_native_per_token {
        return reject(CopySkip::TokenCapReached);
    }
    let latest_size = match sizing::size_for(
        &current,
        plan.target_size_sol,
        current_spend,
        current_policy.engine_trade_size_native,
    ) {
        Ok(size) => size,
        Err(reason) => return reject(reason),
    };
    if fill.input_sol > latest_size + f64::EPSILON {
        return reject(CopySkip::PaperExecutionUnavailable);
    }
    if (fill.input_sol - actual_input_sol).abs() > f64::EPSILON {
        return reject(CopySkip::PaperQuoteUnfillable);
    }
    let telemetry = CopyTelemetry {
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
        fill_price_sol: Some(fill.fill_price_sol),
        backfill: activity.backfill,
    };
    CopyOutcome::PaperFilled(PaperDecision {
        task_id,
        target_address: task.target_address,
        signature: plan.signature,
        mint: plan.mint,
        target_size_sol: plan.target_size_sol,
        target_token_amount: plan.target_token_amount,
        sized_sol: fill.input_sol,
        fill,
        telemetry,
    })
}

async fn late_wait_guard(
    database: &CopyDatabase,
    io: &impl PaperEntryIo,
    task_id: i64,
    mint: &str,
    original_task: &super::super::CopyTask,
    epoch: u64,
) -> Result<(), CopySkip> {
    if io.force_stopped() {
        return Err(CopySkip::ForceStopped);
    }
    if !io.settings().enabled {
        return Err(CopySkip::PaperExecutionUnavailable);
    }
    if !matches!(database.paper_epoch(task_id).await, Ok(current) if current == epoch) {
        return Err(CopySkip::PaperExecutionUnavailable);
    }
    match database.get_task(task_id).await {
        Ok(Some(current)) if current.enabled && current.mode == TradingMode::Paper => {
            if current.slippage_pct != original_task.slippage_pct {
                return Err(CopySkip::PaperExecutionUnavailable);
            }
        }
        _ => return Err(CopySkip::TaskDisabled),
    }
    io.check_entry_admission(mint, &["rpc"])
        .await
        .map_err(|block| CopySkip::EntryBlocked { block })
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

pub(in crate::trader::copy) fn skipped_paper(
    task_id: i64,
    activity: &WalletActivity,
    mint: &str,
    reason: CopySkip,
    epoch: u64,
) -> CopyOutcome {
    in_paper_epoch(skipped(task_id, activity, mint, reason), epoch)
}

/// Binds a paper task's skip to the paper book it was decided against.
pub(in crate::trader::copy) fn in_paper_epoch(mut outcome: CopyOutcome, epoch: u64) -> CopyOutcome {
    if let CopyOutcome::Skipped { paper, .. } = &mut outcome {
        *paper = Some(PaperContext {
            model: PaperModel::Quote,
            epoch,
        });
    }
    outcome
}

pub(in crate::trader::copy) fn skipped_paper_intent(
    task_id: i64,
    signature: String,
    mint: &str,
    reason: CopySkip,
    epoch: u64,
) -> CopyOutcome {
    CopyOutcome::Skipped {
        task_id,
        signature,
        mint: Some(mint.to_owned()),
        reason,
        decided_at: Utc::now(),
        telemetry: None,
        paper: Some(types::PaperContext {
            model: PaperModel::Quote,
            epoch,
        }),
    }
}
