//! The task's own exit policy applied to its paper book. A live copy position in
//! `buy_only`/`hybrid` mode is managed by the exit monitor (stop loss, trailing
//! stop, ROI, time override); paper holdings get the same rules here, evaluated by
//! the evaluators' pure cores, so paper results predict what live would have done.
//! `mirror` tasks hand exits to the target's sells alone, exactly as live does.

use chrono::{DateTime, Utc};

use crate::logger::{self, LogTag};
use crate::positions::PositionManagement;
use crate::trader::evaluators::{exit_roi, exit_stop_loss, exit_time, exit_trailing};
use crate::trader::policy::ExitPolicy;

use super::{
    management_for_exit_mode, simulate_sell, CopyDatabase, CopyMode, CopyOutcome, CopySellDecision,
    CopySkip, CopyTask, CopyTelemetry, PaperCosts, PaperExitRule, PaperPosition,
};

/// Which rule, if any, closes this holding at `mark_price_sol`, checked in the
/// live monitor's order. The inner percentage is a partial exit; `None` sells all.
/// `Err` carries the detail of an impossible policy configuration.
pub fn evaluate_paper_exit(
    position: &PaperPosition,
    mark_price_sol: f64,
    peak_price_sol: f64,
    policy: &ExitPolicy,
    now: DateTime<Utc>,
) -> Result<Option<(PaperExitRule, Option<f64>)>, String> {
    if !position.is_open() || !mark_price_sol.is_finite() || mark_price_sol <= 0.0 {
        return Ok(None);
    }
    let entry_price = position.cost_basis_sol / position.token_amount;
    let held_seconds = (now - position.opened_at).num_seconds();
    if let Some(exit_percentage) = exit_stop_loss::stop_loss_exit(
        entry_price,
        mark_price_sol,
        held_seconds,
        position.sells > 0,
        &policy.stop_loss,
    ) {
        return Ok(Some((PaperExitRule::StopLoss, exit_percentage)));
    }
    if exit_trailing::trailing_stop_triggered(
        entry_price,
        peak_price_sol,
        mark_price_sol,
        &policy.trailing,
    )? {
        return Ok(Some((PaperExitRule::TrailingStop, None)));
    }
    if exit_roi::roi_target_reached(entry_price, mark_price_sol, &policy.roi) {
        return Ok(Some((PaperExitRule::TakeProfit, None)));
    }
    if exit_time::time_override_triggered(
        entry_price,
        mark_price_sol,
        held_seconds as f64,
        &policy.time,
    )? {
        return Ok(Some((PaperExitRule::TimeOverride, None)));
    }
    Ok(None)
}

/// The simulated sell a triggered rule books. Its signature is unique per exit
/// leg of a round (`sells` grows with every booked sell), so a sweep that repeats
/// before the ledger moves is absorbed by the decision table's uniqueness.
pub fn paper_exit_outcome(
    task: &CopyTask,
    position: &PaperPosition,
    rule: PaperExitRule,
    exit_percentage: Option<f64>,
    mark_price_sol: f64,
    costs: PaperCosts,
    now: DateTime<Utc>,
) -> Result<CopyOutcome, CopySkip> {
    let sell_amount = match exit_percentage {
        Some(pct) if pct < 100.0 => position.token_amount * pct / 100.0,
        _ => position.token_amount,
    };
    let fill = simulate_sell(sell_amount, mark_price_sol, task.slippage_pct, costs)?;
    Ok(CopyOutcome::PaperSellObserved(CopySellDecision {
        task_id: task.id,
        target_address: task.target_address.clone(),
        target_signature: format!(
            "paper-exit:{}:{}:{}",
            position.mint,
            position.opened_at.timestamp(),
            position.sells
        ),
        mint: position.mint.clone(),
        target_token_amount: 0.0,
        target_sol_amount: 0.0,
        exit_percentage,
        transaction_signature: None,
        error: None,
        telemetry: CopyTelemetry {
            target_block_time: None,
            detected_at: now,
            decoded_at: now,
            decided_at: now,
            submitted_at: None,
            confirmed_at: None,
            target_price_sol: None,
            fill_price_sol: Some(fill.fill_price_sol),
            backfill: false,
        },
        paper_fill: Some(fill),
        exit_rule: Some(rule),
    }))
}

/// One pass over every paper task whose exits the policy manages. Like the live
/// exit monitor it covers paused tasks too (pausing stops new copies, not the
/// management of what is held) and stands down under the emergency stop. Holdings
/// are marked at the pool price only; a token without one is left for the next
/// pass rather than sold on a stale trade price.
pub async fn sweep(database: &CopyDatabase, costs: PaperCosts) -> crate::trader::Result<()> {
    if crate::global::is_force_stopped() {
        return Ok(());
    }
    let tasks = database.list_tasks().await?;
    for task in tasks.iter().filter(|task| {
        task.mode == CopyMode::Paper
            && management_for_exit_mode(task.exit_mode) != PositionManagement::CopyTask
    }) {
        let holdings = database.paper_positions(task.id).await?;
        if !holdings.iter().any(PaperPosition::is_open) {
            continue;
        }
        let mut policy = ExitPolicy::from_config();
        policy.apply_overrides(&task.exit_policy_overrides);
        for position in holdings.iter().filter(|position| position.is_open()) {
            let Some(mark) = crate::pools::get_pool_price(&position.mint)
                .map(|price| price.price_sol)
                .filter(|price| price.is_finite() && *price > 0.0)
            else {
                continue;
            };
            let peak = position.peak_price_sol.map_or(mark, |peak| peak.max(mark));
            database
                .raise_paper_peak(task.id, &position.mint, peak)
                .await?;
            let now = Utc::now();
            let (rule, exit_percentage) =
                match evaluate_paper_exit(position, mark, peak, &policy, now) {
                    Ok(Some(triggered)) => triggered,
                    Ok(None) => continue,
                    Err(detail) => {
                        logger::warning(
                            LogTag::Trader,
                            &format!("Copy task {} paper exit skipped: {detail}", task.id),
                        );
                        continue;
                    }
                };
            match paper_exit_outcome(task, position, rule, exit_percentage, mark, costs, now) {
                Ok(outcome) => {
                    logger::info(
                        LogTag::Trader,
                        &format!(
                            "Copy task {} paper exit {rule:?} on {} at {mark:.12} SOL",
                            task.id, position.mint
                        ),
                    );
                    database.record_outcome(outcome).await?;
                }
                Err(reason) => logger::debug(
                    LogTag::Trader,
                    &format!(
                        "Copy task {} paper exit on {} not simulated: {reason:?}",
                        task.id, position.mint
                    ),
                ),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;
    use crate::trader::evaluators::dca::DcaConfigSnapshot;
    use crate::trader::policy::{RoiPolicy, StopLossPolicy, TimePolicy, TrailingPolicy};

    fn policy() -> ExitPolicy {
        ExitPolicy {
            stop_loss: StopLossPolicy {
                enabled: true,
                threshold_pct: 30.0,
                min_hold_seconds: 0,
                allow_partial: true,
                partial_exit_default_pct: 50.0,
            },
            trailing: TrailingPolicy {
                enabled: true,
                activation_pct: 20.0,
                distance_pct: 10.0,
            },
            roi: RoiPolicy {
                enabled: true,
                target_profit_pct: 100.0,
            },
            time: TimePolicy {
                enabled: true,
                loss_threshold_pct: -10.0,
                duration_seconds: 3600.0,
            },
            dca: DcaConfigSnapshot {
                enabled: false,
                max_count: 0,
                cooldown_minutes: 0,
                threshold_pct: 0.0,
                size_percentage: 0.0,
            },
        }
    }

    /// 100 tokens for 1 SOL: entry 0.01 SOL per token.
    fn holding(sells: u64, age: Duration) -> PaperPosition {
        let now = Utc::now();
        PaperPosition {
            task_id: 1,
            mint: "mint".to_owned(),
            token_amount: 100.0,
            cost_basis_sol: 1.0,
            invested_sol: 1.0,
            realized_proceeds_sol: 0.0,
            realized_cost_sol: 0.0,
            buys: 1,
            sells,
            last_price_sol: None,
            last_price_at: None,
            opened_at: now - age,
            closed_at: None,
            peak_price_sol: Some(0.01),
        }
    }

    fn evaluate(
        position: &PaperPosition,
        mark: f64,
        peak: f64,
    ) -> Option<(PaperExitRule, Option<f64>)> {
        evaluate_paper_exit(position, mark, peak, &policy(), Utc::now()).unwrap()
    }

    #[test]
    fn stop_loss_takes_one_partial_then_exits_fully() {
        let fresh = holding(0, Duration::seconds(60));
        assert_eq!(
            evaluate(&fresh, 0.0065, 0.01),
            Some((PaperExitRule::StopLoss, Some(50.0)))
        );
        let cut = holding(1, Duration::seconds(60));
        assert_eq!(
            evaluate(&cut, 0.0065, 0.01),
            Some((PaperExitRule::StopLoss, None))
        );
    }

    #[test]
    fn trailing_stop_arms_off_the_peak_and_fires_only_in_profit() {
        let position = holding(0, Duration::seconds(60));
        // Peaked at +25%, retraced to +12%: under the stop (0.0125 * 0.9 = 0.01125).
        assert_eq!(
            evaluate(&position, 0.0112, 0.0125),
            Some((PaperExitRule::TrailingStop, None))
        );
        // Same retracement without the peak ever reaching activation: nothing.
        assert_eq!(evaluate(&position, 0.0112, 0.0115), None);
        // Retraced into a loss: the trail never sells at a loss.
        assert_eq!(evaluate(&position, 0.0095, 0.0125), None);
    }

    #[test]
    fn roi_target_and_time_override_fire_in_monitor_order() {
        let young = holding(0, Duration::seconds(60));
        assert_eq!(
            evaluate(&young, 0.02, 0.02),
            Some((PaperExitRule::TakeProfit, None))
        );
        assert_eq!(evaluate(&young, 0.0085, 0.01), None);
        let old = holding(0, Duration::hours(2));
        assert_eq!(
            evaluate(&old, 0.0085, 0.01),
            Some((PaperExitRule::TimeOverride, None))
        );
    }

    #[test]
    fn an_impossible_trailing_configuration_is_reported_not_traded() {
        let mut broken = policy();
        broken.trailing.distance_pct = 25.0;
        let position = holding(0, Duration::seconds(60));
        assert!(evaluate_paper_exit(&position, 0.0105, 0.0125, &broken, Utc::now()).is_err());
    }

    #[test]
    fn exit_legs_book_distinct_idempotent_signatures() {
        let task = CopyTask {
            id: 7,
            chain: crate::chains::ChainId::Solana,
            target_address: "target".to_owned(),
            label: None,
            enabled: true,
            mode: CopyMode::Paper,
            sizing: super::super::SizingMode::Fixed { sol: 0.1 },
            exit_mode: super::super::ExitMode::Hybrid,
            exit_policy_overrides: Default::default(),
            max_sol_per_trade: 1.0,
            max_sol_per_token: 1.0,
            total_budget_sol: 1.0,
            min_target_trade_sol: None,
            max_target_trade_sol: None,
            buy_once_per_token: true,
            slippage_pct: 1.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let costs = PaperCosts {
            network_fee_sol: 0.0,
            priority_fee_sol: 0.0,
        };
        let signature = |sells| match paper_exit_outcome(
            &task,
            &holding(sells, Duration::seconds(60)),
            PaperExitRule::StopLoss,
            Some(50.0),
            0.0065,
            costs,
            Utc::now(),
        )
        .unwrap()
        {
            CopyOutcome::PaperSellObserved(decision) => {
                assert_eq!(decision.exit_rule, Some(PaperExitRule::StopLoss));
                assert!((decision.paper_fill.unwrap().token_amount - 50.0).abs() < 1e-9);
                decision.target_signature
            }
            other => panic!("unexpected outcome {other:?}"),
        };
        assert_ne!(signature(0), signature(1));
    }
}
