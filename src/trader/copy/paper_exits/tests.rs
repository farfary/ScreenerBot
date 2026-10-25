// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Paper exit sizing, policy, and idempotency tests.

use super::*;

use chrono::Duration;

use crate::trader::evaluators::dca::DcaConfigSnapshot;
use crate::trader::policy::{RoiPolicy, StopLossPolicy, TimePolicy, TrailingPolicy};

fn task() -> CopyTask {
    CopyTask {
        id: 1,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: CopyMode::Paper,
        sizing: super::super::types::SizingMode::Fixed { sol: 0.1 },
        exit_mode: super::super::types::ExitMode::Hybrid,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 1.0,
        max_native_per_token: 2.0,
        total_budget_native: 5.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
    }
}

#[test]
fn policy_and_mirror_close_planning_keep_exact_raw_inputs() {
    let task = task();
    let full = paper_sell_request(
        &task,
        "mint",
        RawAmount::from(15_u64),
        None,
        "wallet".to_owned(),
    )
    .unwrap();
    assert_eq!(full.input_amount, RawAmount::from(15_u64));

    let partial = paper_sell_request(
        &task,
        "mint",
        RawAmount::from(17_u64),
        Some(50.0),
        "wallet".to_owned(),
    )
    .unwrap();
    assert_eq!(partial.input_amount, RawAmount::from(8_u64));
    assert_eq!(
        RawAmount::from(17_u64).checked_sub(partial.input_amount),
        Some(RawAmount::from(9_u64))
    );
    assert!(matches!(
        paper_sell_request(
            &task,
            "mint",
            RawAmount::from(15_u64),
            Some(1e-9),
            "wallet".to_owned()
        ),
        Err(CopySkip::InvalidSizing)
    ));
}

#[test]
fn partial_sell_sizing_preserves_large_and_full_width_raw_balances() {
    let above_f64_integer = RawAmount::new((1_u128 << 53) + 17);
    let selected = raw_sell_amount(above_f64_integer, Some(50.0)).unwrap();
    assert_eq!(selected, RawAmount::new(((1_u128 << 53) + 17) / 2));
    assert_eq!(
        raw_sell_amount(RawAmount::MAX, Some(50.0)).unwrap(),
        RawAmount::new(u128::MAX / 2)
    );
    assert_eq!(
        raw_sell_amount(RawAmount::MAX, None).unwrap(),
        RawAmount::MAX
    );
}

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

#[test]
fn policy_decision_signatures_distinguish_subsecond_rounds_and_epochs() {
    let opened_at = Utc::now();
    let base = PaperPosition {
        task_id: 1,
        mint: "mint".to_owned(),
        token_amount: 1.0,
        raw_token_amount: Some(RawAmount::from(1_u64)),
        token_decimals: Some(0),
        cost_basis_native: 1.0,
        invested_native: 1.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 1,
        sells: 0,
        last_price_native: Some(1.0),
        last_price_at: Some(opened_at),
        opened_at,
        closed_at: None,
        peak_price_native: Some(1.0),
    };
    let mut next_round = base.clone();
    next_round.opened_at = opened_at + chrono::Duration::nanoseconds(1);
    assert_ne!(
        paper_exit_signature(&base, 3),
        paper_exit_signature(&next_round, 3)
    );
    assert_ne!(
        paper_exit_signature(&base, 3),
        paper_exit_signature(&base, 4)
    );
}

/// 100 tokens for 1 SOL: entry 0.01 SOL per token.
fn holding(sells: u64, age: Duration) -> PaperPosition {
    let now = Utc::now();
    PaperPosition {
        task_id: 1,
        mint: "mint".to_owned(),
        token_amount: 100.0,
        raw_token_amount: Some(RawAmount::from(100_u64)),
        token_decimals: Some(0),
        cost_basis_native: 1.0,
        invested_native: 1.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 1,
        sells,
        last_price_native: None,
        last_price_at: None,
        opened_at: now - age,
        closed_at: None,
        peak_price_native: Some(0.01),
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
    assert_ne!(
        paper_exit_signature(&holding(0, Duration::seconds(60)), 0),
        paper_exit_signature(&holding(1, Duration::seconds(60)), 0)
    );
}
