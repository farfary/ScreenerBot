// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests the copy workspace: task comparison rows, default settings, and legacy paper summaries. Manual paper closes are in `manual_close`.

use super::*;
use crate::trader::copy::{ExitMode, SizingMode};

#[path = "workspace_tests/manual_close.rs"]
mod manual_close;

const ADDRESS: &str = "CopyTarget1111111111111111111111111111111111";

fn task(label: Option<&str>) -> CopyTask {
    CopyTask {
        id: 7,
        chain: crate::chains::ChainId::Solana,
        target_address: ADDRESS.to_owned(),
        label: label.map(str::to_owned),
        enabled: true,
        mode: TradingMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.2,
        max_native_per_token: 1.0,
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
fn an_unnamed_task_compares_by_its_full_address() {
    let row = comparison(&task(None), CopyInsights::default());
    assert_eq!(row.label, None);
    assert_eq!(row.target_address, ADDRESS);

    let json = serde_json::to_value(&row).expect("comparison serializes");
    assert!(json.get("name").is_none(), "no pre-shortened display name");
    assert_eq!(json["target_address"], ADDRESS);
}

#[test]
fn a_named_task_compares_by_its_label() {
    let row = comparison(&task(Some("Momentum desk")), CopyInsights::default());
    assert_eq!(row.label.as_deref(), Some("Momentum desk"));
    assert_eq!(row.target_address, ADDRESS);
}

#[test]
fn legacy_workspace_values_use_only_saved_paper_estimates() {
    let now = Utc::now();
    let position = PaperPosition {
        task_id: 7,
        mint: "legacy-mint".to_owned(),
        token_amount: 10.0,
        raw_token_amount: None,
        token_decimals: None,
        cost_basis_native: 2.0,
        invested_native: 2.5,
        realized_proceeds_native: 1.0,
        realized_cost_native: 0.5,
        buys: 2,
        sells: 1,
        last_price_native: Some(0.3),
        last_price_at: Some(now),
        opened_at: now,
        closed_at: None,
        peak_price_native: None,
        take_profit_level: 0,
        round_partial_exits: 0,
    };

    let summary = legacy_paper_summary(&[position], 2.5);
    assert_eq!(summary.spent_native, 2.5);
    assert_eq!(summary.realized_pnl_native, 0.5);
    assert_eq!(summary.holdings[0].last_recorded_price_native, Some(0.3));
    assert_eq!(
        summary.holdings[0].remaining_pnl_at_last_recorded_price_native,
        Some(1.0)
    );
}

#[test]
fn copy_defaults_fee_rate_uses_the_chain_adapter_policy() {
    crate::config::utils::install_default_config();
    let defaults = super::defaults();
    assert_eq!(
        defaults.swap_fee_pct,
        f64::from(crate::chains::platform_swap_fee_bps()) / 100.0
    );
}

/// Every exit rule a copy task can run shows on its holdings: with only that rule on, the
/// holding's exit watch names at least one price or moment. A new `ExitRule` variant fails
/// to compile here until it says what its watch shows.
#[test]
fn every_exit_rule_alone_shows_on_the_holding_watch() {
    use crate::trader::evaluators::exit_rules::ExitRule;

    crate::config::utils::install_default_config();
    let now = Utc::now();
    let position = PaperPosition {
        task_id: 7,
        mint: "watch-mint".to_owned(),
        token_amount: 10.0,
        raw_token_amount: None,
        token_decimals: None,
        cost_basis_native: 1.0,
        invested_native: 1.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 1,
        sells: 0,
        last_price_native: Some(0.1),
        last_price_at: Some(now),
        opened_at: now,
        closed_at: None,
        peak_price_native: Some(0.1),
        take_profit_level: 0,
        round_partial_exits: 0,
    };
    let mut none = ExitPolicy::from_config();
    none.stop_loss.enabled = false;
    none.trailing.enabled = false;
    none.break_even.enabled = false;
    none.take_profit.enabled = false;
    none.time.enabled = false;
    none.time_stagnation.enabled = false;
    let shows_something = |policy: &ExitPolicy| {
        let watch = serde_json::to_value(exit_watch(&position, 0.1, policy)).expect("serializes");
        watch
            .as_object()
            .expect("an object")
            .values()
            .any(|value| !value.is_null() && *value != serde_json::Value::Bool(false))
    };
    assert!(!shows_something(&none), "an all-off policy shows nothing");

    for rule in [
        ExitRule::StopLoss,
        ExitRule::Trailing,
        ExitRule::BreakEven,
        ExitRule::TakeProfit { level: 1 },
        ExitRule::TimeLoss,
        ExitRule::TimeStagnation,
    ] {
        let mut policy = none.clone();
        match rule {
            ExitRule::StopLoss => {
                policy.stop_loss.enabled = true;
                policy.stop_loss.threshold_pct = 20.0;
            }
            ExitRule::Trailing => {
                policy.trailing.enabled = true;
                policy.trailing.activation_pct = 15.0;
                policy.trailing.distance_pct = 5.0;
            }
            ExitRule::BreakEven => {
                policy.break_even.enabled = true;
                policy.break_even.trigger_pct = 15.0;
                policy.break_even.offset_pct = 1.0;
            }
            ExitRule::TakeProfit { .. } => {
                policy.take_profit.enabled = true;
                policy.take_profit.levels = vec![TakeProfitLevel::sell_all_at(30.0)];
            }
            ExitRule::TimeLoss => {
                policy.time.enabled = true;
                policy.time.duration_seconds = 3600.0;
                policy.time.loss_threshold_pct = -10.0;
            }
            ExitRule::TimeStagnation => {
                policy.time_stagnation.enabled = true;
                policy.time_stagnation.duration_seconds = 3600.0;
                policy.time_stagnation.band_pct = 5.0;
            }
        }
        assert!(shows_something(&policy), "{rule:?} alone shows nothing");
    }
}

#[test]
fn the_holding_watch_places_the_break_even_floor_and_the_flat_band() {
    crate::config::utils::install_default_config();
    let now = Utc::now();
    let position = PaperPosition {
        task_id: 7,
        mint: "watch-mint".to_owned(),
        token_amount: 10.0,
        raw_token_amount: None,
        token_decimals: None,
        cost_basis_native: 1.0,
        invested_native: 1.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 1,
        sells: 0,
        last_price_native: Some(1.0),
        last_price_at: Some(now),
        opened_at: now,
        closed_at: None,
        peak_price_native: Some(1.2),
        take_profit_level: 0,
        round_partial_exits: 0,
    };
    let mut policy = ExitPolicy::from_config();
    policy.break_even.enabled = true;
    policy.break_even.trigger_pct = 15.0;
    policy.break_even.offset_pct = 1.0;
    policy.time_stagnation.enabled = true;
    policy.time_stagnation.duration_seconds = 4.0 * 3600.0;
    policy.time_stagnation.band_pct = 5.0;

    let watch = exit_watch(&position, 1.0, &policy);
    let close = |value: Option<f64>, expected: f64| {
        value.is_some_and(|value| (value - expected).abs() < 1e-9)
    };
    assert!(close(watch.break_even_activation_price_native, 1.15));
    assert!(watch.break_even_armed, "a peak of +20% arms a +15% trigger");
    assert!(close(watch.break_even_floor_price_native, 1.01));
    assert_eq!(watch.stagnation_from, Some(now + Duration::hours(4)));
    assert!(close(watch.stagnation_band_low_price_native, 0.95));
    assert!(close(watch.stagnation_band_high_price_native, 1.05));

    let unarmed = PaperPosition {
        peak_price_native: Some(1.1),
        ..position
    };
    assert!(!exit_watch(&unarmed, 1.0, &policy).break_even_armed);
}
