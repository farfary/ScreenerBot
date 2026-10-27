// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Shared exit decision rules applied to paper holdings using pool marks. Live
//! `buy_only`/`hybrid` positions are evaluated by the exit monitor; `mirror` mode
//! exits only when target sells are observed.

use std::future::Future;
use std::sync::{LazyLock, RwLock};

use chrono::{DateTime, Utc};

use crate::chains::RawAmount;
use crate::logger::{self, LogTag};
use crate::positions::PositionManagement;
use crate::swaps::{QuoteRequest, SwapMode};
use crate::trader::evaluators::exit_rules::{self, ExitInputs, ExitRule};
use crate::trader::policy::ExitPolicy;

use super::{
    management_for_exit_mode, CopyDatabase, CopyMode, CopyOutcome, CopySellDecision, CopySkip,
    CopyTask, CopyTelemetry, PaperExitRule, PaperPosition,
};

const EXIT_PERCENT_SCALE: u128 = 10_u128.pow(9);
const EXIT_PERCENT_DENOMINATOR: u128 = 100 * EXIT_PERCENT_SCALE;

/// Select the exact raw token input shared by policy, mirror, and manual paper exits.
pub(super) fn raw_sell_amount(
    held: RawAmount,
    exit_percentage: Option<f64>,
) -> Result<RawAmount, CopySkip> {
    if held == RawAmount::ZERO {
        return Err(CopySkip::CopyPositionNotFound);
    }
    let Some(percentage) = exit_percentage else {
        return Ok(held);
    };
    if !percentage.is_finite() {
        return Err(CopySkip::InvalidSizing);
    }
    if percentage >= 100.0 {
        return Ok(held);
    }
    if percentage <= 0.0 {
        return Err(CopySkip::InvalidSizing);
    }
    let parts = RawAmount::from_integral_f64((percentage * EXIT_PERCENT_SCALE as f64).floor())
        .filter(|parts| parts.raw() > 0 && parts.raw() < EXIT_PERCENT_DENOMINATOR)
        .ok_or(CopySkip::InvalidSizing)?;
    let amount = held
        .checked_mul_div(parts, RawAmount::new(EXIT_PERCENT_DENOMINATOR))
        .filter(|amount| *amount > RawAmount::ZERO && *amount <= held)
        .ok_or(CopySkip::InvalidSizing)?;
    Ok(amount)
}

/// Build the exact request consumed by both policy and mirrored paper close paths.
pub(super) fn paper_sell_request(
    task: &CopyTask,
    mint: &str,
    held: RawAmount,
    exit_percentage: Option<f64>,
    wallet_address: String,
) -> Result<QuoteRequest, CopySkip> {
    let adapter = crate::chains::adapter_for(task.chain);
    Ok(QuoteRequest {
        chain: task.chain,
        input_mint: mint.to_owned(),
        output_mint: adapter.native_asset_address().to_owned(),
        input_amount: raw_sell_amount(held, exit_percentage)?,
        wallet_address,
        slippage_pct: task.slippage_pct,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    })
}

/// A sale of a paper holding: the rule it is booked under, the share of the holding it
/// sells (`None` sells all), and the take-profit step it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaperExit {
    pub rule: PaperExitRule,
    pub exit_percentage: Option<f64>,
    pub take_profit_level: Option<u32>,
}

/// Which rule, if any, closes this holding at `mark_price_native`, checked in the
/// order the live monitor shares (`exit_rules`), from the round's own partial sales
/// and take-profit steps. A rule whose configuration is impossible is logged against
/// `task_id` and does not fire.
pub fn evaluate_paper_exit(
    task_id: i64,
    position: &PaperPosition,
    mark_price_native: f64,
    peak_price_native: f64,
    policy: &ExitPolicy,
    now: DateTime<Utc>,
) -> Option<PaperExit> {
    if !position.is_open() || !mark_price_native.is_finite() || mark_price_native <= 0.0 {
        return None;
    }
    let inputs = ExitInputs {
        average_entry_price: position.cost_basis_native / position.token_amount,
        current_price: mark_price_native,
        basis_peak_price: peak_price_native,
        held_seconds: (now - position.opened_at).num_seconds(),
        partial_exits_taken: position.round_partial_exits > 0,
        take_profit_levels_taken: position.take_profit_level,
    };
    let decision = exit_rules::decide(&inputs, policy);
    for (rule, e) in &decision.rule_errors {
        logger::warning(
            LogTag::Trader,
            &format!(
                "Copy task {task_id} paper {rule:?} rule skipped for {}: {e}",
                position.mint
            ),
        );
    }
    decision.outcome.map(|outcome| PaperExit {
        rule: paper_exit_rule(outcome.rule),
        exit_percentage: outcome.exit_percentage,
        take_profit_level: match outcome.rule {
            ExitRule::TakeProfit { level } => Some(level),
            ExitRule::StopLoss
            | ExitRule::Trailing
            | ExitRule::BreakEven
            | ExitRule::TimeLoss
            | ExitRule::TimeStagnation => None,
        },
    })
}

/// The persisted paper rule a fired built-in rule is booked under.
fn paper_exit_rule(rule: ExitRule) -> PaperExitRule {
    match rule {
        ExitRule::StopLoss => PaperExitRule::StopLoss,
        ExitRule::Trailing => PaperExitRule::TrailingStop,
        ExitRule::BreakEven => PaperExitRule::BreakEven,
        ExitRule::TakeProfit { .. } => PaperExitRule::TakeProfit,
        ExitRule::TimeLoss => PaperExitRule::TimeOverride,
        ExitRule::TimeStagnation => PaperExitRule::TimeStagnation,
    }
}

/// The simulated sell `exit` books. Its signature is unique per exit leg of a round
/// (`sells` grows with every booked sell), so a sweep that repeats before the ledger
/// moves is absorbed by the decision table's uniqueness.
pub async fn paper_exit_outcome<Q, QFut, W, WFut>(
    database: &CopyDatabase,
    task: &CopyTask,
    position: &PaperPosition,
    exit: PaperExit,
    epoch: u64,
    now: DateTime<Utc>,
    quote: Q,
    wait: W,
) -> Result<CopyOutcome, CopySkip>
where
    Q: FnMut(QuoteRequest) -> QFut,
    QFut: Future<Output = Result<crate::swaps::Quote, crate::swaps::QuoteError>>,
    W: FnOnce(std::time::Duration) -> WFut,
    WFut: Future<Output = Result<(), CopySkip>>,
{
    let held_raw = position
        .raw_token_amount
        .filter(|amount| *amount > RawAmount::ZERO)
        .ok_or(CopySkip::CopyPositionNotFound)?;
    let stored_decimals = position
        .token_decimals
        .ok_or(CopySkip::PaperQuoteUnfillable)?;
    let decimals = stored_decimals;
    let wallet_address = crate::wallets::get_main_address()
        .await
        .map_err(|_| CopySkip::PaperQuoteUnfillable)?;
    let request = paper_sell_request(
        task,
        &position.mint,
        held_raw,
        exit.exit_percentage,
        wallet_address,
    )?;
    let raw = request.input_amount;
    let delay_ms = crate::config::with_config(|config| config.copy_trading.paper_landing_delay_ms);
    let fill = super::paper::simulate_sell(
        request,
        decimals,
        delay_ms,
        now,
        epoch,
        quote,
        |delay| async move {
            wait(delay).await?;
            if crate::global::is_force_stopped() {
                return Err(CopySkip::ForceStopped);
            }
            match database.get_task(task.id).await {
                Ok(Some(current))
                    if current.mode == CopyMode::Paper
                        && current.slippage_pct == task.slippage_pct => {}
                _ => return Err(CopySkip::TaskDisabled),
            }
            let current = database
                .paper_position(task.id, &position.mint)
                .await
                .map_err(|_| CopySkip::PaperExecutionUnavailable)?;
            if current
                .filter(PaperPosition::is_open)
                .is_none_or(|current| {
                    current.opened_at != position.opened_at
                        || current.sells != position.sells
                        || current.raw_token_amount.is_none_or(|held| held < raw)
                })
            {
                return Err(CopySkip::PaperExecutionUnavailable);
            }
            if database
                .paper_epoch(task.id)
                .await
                .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                != epoch
            {
                return Err(CopySkip::PaperExecutionUnavailable);
            }
            Ok(())
        },
    )
    .await?;
    Ok(CopyOutcome::PaperSellObserved(CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: paper_exit_signature(position, epoch),
        mint: position.mint.clone(),
        target_token_amount: 0.0,
        target_sol_amount: 0.0,
        exit_percentage: exit.exit_percentage,
        transaction_signature: None,
        error: None,
        telemetry: CopyTelemetry {
            target_block_time: None,
            detected_at: now,
            decoded_at: now,
            decided_at: Utc::now(),
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: Some(fill.fill_price_sol),
            backfill: false,
        },
        paper_fill: Some(fill),
        exit_rule: Some(exit.rule),
        take_profit_level: exit.take_profit_level,
        paper_context: Some(super::types::PaperContext {
            model: super::types::PaperModel::Quote,
            epoch,
        }),
    }))
}

fn paper_exit_signature(position: &PaperPosition, epoch: u64) -> String {
    format!(
        "paper-exit:{}:{}:{}:{}",
        position.mint,
        position.opened_at.timestamp_nanos_opt().unwrap_or_default(),
        position.sells,
        epoch,
    )
}

/// Mints held open by any paper task, refreshed every sweep. Pool marks drive
/// valuation and policy triggers; manual and mirrored sells can be quoted without
/// a pool mark.
static HELD_PAPER_MINTS: LazyLock<RwLock<Vec<String>>> = LazyLock::new(Default::default);
pub fn held_paper_mints() -> Vec<String> {
    HELD_PAPER_MINTS
        .read()
        .map(|mints| mints.clone())
        .unwrap_or_default()
}

/// One pass over every paper book with open holdings. Each holding's peak follows
/// the pool price whoever sells it; the exit rules then run on the holdings of the
/// tasks whose exits the policy manages (a `mirror` holding is sold by the
/// wallet's sells alone). Like the live exit monitor it covers paused tasks too
/// (pausing stops new copies, not the management of what is held) and stands down
/// under the emergency stop. Holdings are marked at the pool price only; a token
/// without one is left for the next pass rather than sold on a stale trade price.
pub async fn sweep(database: &CopyDatabase) -> crate::trader::Result<()> {
    let tasks = database.list_tasks().await?;
    let mut books = Vec::new();
    for task in tasks.iter().filter(|task| task.mode == CopyMode::Paper) {
        let holdings = database.paper_positions(task.id).await?;
        if holdings.iter().any(PaperPosition::is_open) {
            books.push((task, holdings));
        }
    }
    let mut held: Vec<String> = books
        .iter()
        .flat_map(|(_, holdings)| holdings.iter().filter(|position| position.is_open()))
        .map(|position| position.mint.clone())
        .collect();
    held.sort_unstable();
    held.dedup();
    if let Ok(mut mints) = HELD_PAPER_MINTS.write() {
        *mints = held;
    }
    if crate::global::is_force_stopped() {
        return Ok(());
    }
    for (task, holdings) in &books {
        let policy = (management_for_exit_mode(task.exit_mode) != PositionManagement::CopyTask)
            .then(|| {
                let mut policy = ExitPolicy::from_config();
                policy.apply_overrides(&task.exit_policy_overrides);
                policy
            });
        for position in holdings.iter().filter(|position| position.is_open()) {
            let Some(mark) = crate::pools::get_pool_price(task.chain, &position.mint)
                .map(|price| price.price_native)
                .filter(|price| price.is_finite() && *price > 0.0)
            else {
                continue;
            };
            let peak = position
                .peak_price_native
                .map_or(mark, |peak| peak.max(mark));
            database
                .raise_paper_peak(
                    task.id,
                    &position.mint,
                    position.opened_at,
                    position.buys,
                    peak,
                )
                .await?;
            let Some(policy) = &policy else {
                continue;
            };
            let now = Utc::now();
            let Some(exit) = evaluate_paper_exit(task.id, position, mark, peak, policy, now) else {
                continue;
            };
            let epoch = database.paper_epoch(task.id).await?;
            let opened_nanos = position.opened_at.timestamp_nanos_opt().unwrap_or_default();
            let pending_key = format!(
                "policy:{}:{}:{}:{}",
                position.mint, opened_nanos, position.sells, epoch
            );
            let Some(runtime) = super::paper_execution::active() else {
                super::notify::record(
                    database,
                    super::paper_execution::skipped_paper_intent(
                        task.id,
                        format!(
                            "paper-queue-unavailable:{}:{}",
                            position.mint,
                            now.timestamp_nanos_opt().unwrap_or_default()
                        ),
                        &position.mint,
                        CopySkip::PaperExecutionUnavailable,
                        epoch,
                    ),
                )
                .await?;
                continue;
            };
            let db = database.clone();
            let task = (**task).clone();
            let task_id = task.id;
            let position = position.clone();
            let key = pending_key.clone();
            let queued_at = Utc::now();
            let fallback_mint = position.mint.clone();
            let fallback_mint_job = fallback_mint.clone();
            if let Err(error) = runtime.enqueue(task_id, key, database.clone(), Some(fallback_mint.clone()), async move {
                let current = async {
                    if crate::global::is_force_stopped() {
                        return Err(CopySkip::ForceStopped);
                    }
                    let current_task = db
                        .get_task(task.id)
                        .await
                        .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                        .filter(|task| {
                            task.mode == CopyMode::Paper
                                && management_for_exit_mode(task.exit_mode)
                                    != PositionManagement::CopyTask
                        })
                        .ok_or(CopySkip::TaskDisabled)?;
                    let current_position = db
                        .paper_position(task.id, &position.mint)
                        .await
                        .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                        .filter(PaperPosition::is_open)
                        .filter(|current| {
                            current.opened_at == position.opened_at
                                && current.sells == position.sells
                        })
                        .ok_or(CopySkip::PaperExecutionUnavailable)?;
                    if !matches!(db.paper_epoch(task.id).await, Ok(current) if current == epoch) {
                        return Err(CopySkip::PaperExecutionUnavailable);
                    }
                    let outcome = paper_exit_outcome(
                        &db,
                        &current_task,
                        &current_position,
                        exit,
                        epoch,
                        queued_at,
                        crate::swaps::try_get_best_quote,
                        |delay| async move {
                            tokio::time::sleep(delay).await;
                            if crate::global::is_force_stopped() {
                                return Err(CopySkip::ForceStopped);
                            }
                            Ok(())
                        },
                    )
                    .await?;
                    let requested_raw = match &outcome {
                        CopyOutcome::PaperSellObserved(decision) => decision
                            .paper_fill
                            .as_ref()
                            .and_then(|fill| fill.quote.as_ref())
                            .and_then(|quote| quote.input_amount)
                            .ok_or(CopySkip::PaperExecutionUnavailable)?,
                        _ => return Err(CopySkip::PaperExecutionUnavailable),
                    };
                    if crate::global::is_force_stopped()
                        || !matches!(db.paper_epoch(task.id).await, Ok(current) if current == epoch)
                    {
                        return Err(CopySkip::PaperExecutionUnavailable);
                    }
                    let current_task_after = db
                        .get_task(task.id)
                        .await
                        .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                        .filter(|task| {
                            task.mode == CopyMode::Paper
                                && management_for_exit_mode(task.exit_mode)
                                    != PositionManagement::CopyTask
                                && task.slippage_pct == current_task.slippage_pct
                        })
                        .ok_or(CopySkip::TaskDisabled)?;
                    let fresh_mark = crate::pools::get_pool_price(current_task_after.chain, &position.mint)
                        .map(|price| price.price_native)
                        .filter(|price| price.is_finite() && *price > 0.0)
                        .ok_or(CopySkip::PaperExecutionUnavailable)?;
                    let fresh_peak = current_position.peak_price_native
                        .map_or(fresh_mark, |peak| peak.max(fresh_mark));
                    let mut fresh_policy = ExitPolicy::from_config();
                    fresh_policy.apply_overrides(&current_task_after.exit_policy_overrides);
                    if !matches!(
                        evaluate_paper_exit(task.id, &current_position, fresh_mark, fresh_peak, &fresh_policy, Utc::now()),
                        Some(fresh) if fresh == exit
                    ) {
                        return Err(CopySkip::PaperExecutionUnavailable);
                    }
                    let current = db
                        .paper_position(task.id, &position.mint)
                        .await
                        .map_err(|_| CopySkip::PaperExecutionUnavailable)?
                        .filter(PaperPosition::is_open)
                        .filter(|current| {
                            current.opened_at == position.opened_at
                                && current.sells == position.sells
                        })
                        .ok_or(CopySkip::PaperExecutionUnavailable)?;
                    if current
                        .raw_token_amount
                        .is_none_or(|held| held < requested_raw)
                    {
                        return Err(CopySkip::PaperExecutionUnavailable);
                    }
                    Ok(outcome)
                }.await;
                match current {
                    Ok(outcome) => outcome,
                    Err(reason) => super::paper_execution::skipped_paper_intent(
                            task.id,
                            format!(
                                "paper-exit-skip:{}:{}",
                                fallback_mint_job,
                                queued_at.timestamp_nanos_opt().unwrap_or_default()
                            ),
                            &fallback_mint_job,
                            reason,
                            epoch,
                        ),
                }
            }) {
                if error == super::paper_execution::EnqueueError::Duplicate { continue; }
                let reason = match error {
                    super::paper_execution::EnqueueError::Full => CopySkip::PaperQueueFull,
                    super::paper_execution::EnqueueError::Duplicate => unreachable!(),
                    super::paper_execution::EnqueueError::Unavailable => {
                        CopySkip::PaperExecutionUnavailable
                    }
                };
                let skip = super::paper_execution::skipped_paper_intent(
                    task_id,
                    format!(
                        "paper-queue-skip:{}:{}",
                        fallback_mint,
                        Utc::now().timestamp_nanos_opt().unwrap_or_default()
                    ),
                    &fallback_mint,
                    reason,
                    epoch,
                );
                super::notify::record(database, skip).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "paper_exits/tests.rs"]
mod tests;
