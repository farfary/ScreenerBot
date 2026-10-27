// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure copy sizing and risk-cap contracts.

use chrono::Utc;
use screenerbot::chains::ChainId;
use screenerbot::config::TraderConfig;
use screenerbot::trader::copy::{
    precheck, size_for, CopySkip, CopyTask, CopyTaskInput, ExitMode, PipelinePolicy, RiskContext,
    SizingMode, SpendState,
};
use screenerbot::trader::paper::TradingMode;
use screenerbot::trader::policy::ExitPolicy;
use screenerbot::trader::{MAX_MANUAL_SLIPPAGE_PCT, MAX_TRADE_SIZE_MULTIPLIER};

/// The exit policy of a default trader config, the base copy overrides are checked over.
fn default_exit_policy() -> ExitPolicy {
    ExitPolicy::from_trader_config(&TraderConfig::default())
}

fn task(sizing: SizingMode) -> CopyTask {
    CopyTask {
        id: 1,
        chain: ChainId::Solana,
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: TradingMode::Paper,
        sizing,
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.5,
        max_native_per_token: 1.0,
        total_budget_native: 2.0,
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
fn fixed_and_ratio_modes_are_clamped_by_every_hard_cap() {
    let fixed = task(SizingMode::Fixed { sol: 5.0 });
    assert_eq!(size_for(&fixed, 10.0, SpendState::default(), 1.0), Ok(0.5));

    let ratio = task(SizingMode::RatioOfTarget { pct: 25.0 });
    assert_eq!(size_for(&ratio, 1.0, SpendState::default(), 1.0), Ok(0.25));

    let spend = SpendState {
        total_spent_native: 1.9,
        token_spent_native: 0.95,
        token_buy_count: 1,
    };
    let remaining = size_for(&fixed, 10.0, spend, 1.0).unwrap();
    assert!((remaining - 0.05).abs() < 1e-12);

    let mut engine_capped = fixed.clone();
    engine_capped.max_native_per_trade = 1_000.0;
    engine_capped.max_native_per_token = 1_000.0;
    engine_capped.total_budget_native = 1_000.0;
    assert_eq!(
        size_for(&engine_capped, 10.0, SpendState::default(), 0.01),
        Ok(0.01 * MAX_TRADE_SIZE_MULTIPLIER)
    );
}

#[test]
fn exhausted_budget_token_cap_and_v2_mode_are_typed_skips() {
    let fixed = task(SizingMode::Fixed { sol: 0.1 });
    assert_eq!(
        size_for(
            &fixed,
            1.0,
            SpendState {
                total_spent_native: 2.0,
                ..SpendState::default()
            },
            1.0
        ),
        Err(CopySkip::BudgetExhausted)
    );
    assert_eq!(
        size_for(
            &fixed,
            1.0,
            SpendState {
                token_spent_native: 1.0,
                ..SpendState::default()
            },
            1.0
        ),
        Err(CopySkip::TokenCapReached)
    );
    assert_eq!(
        size_for(
            &task(SizingMode::PercentOfTargetPortfolio { pct: 1.0 }),
            1.0,
            SpendState::default(),
            1.0
        ),
        Err(CopySkip::UnsupportedSizingMode)
    );
}

#[test]
fn target_filters_buy_once_self_copy_and_slippage_are_enforced() {
    let mut configured = task(SizingMode::Fixed { sol: 0.1 });
    configured.min_target_trade_native = Some(0.2);
    configured.max_target_trade_native = Some(2.0);
    let policy = PipelinePolicy {
        require_filter_pass: false,
        engine_trade_size_native: 1.0,
    };
    assert!(matches!(
        precheck(
            &configured,
            0.1,
            SpendState::default(),
            RiskContext::default(),
            policy
        ),
        Err(CopySkip::TargetBelowMinimum { .. })
    ));
    assert!(matches!(
        precheck(
            &configured,
            3.0,
            SpendState::default(),
            RiskContext::default(),
            policy
        ),
        Err(CopySkip::TargetAboveMaximum { .. })
    ));

    configured.buy_once_per_token = true;
    assert_eq!(
        precheck(
            &configured,
            1.0,
            SpendState {
                token_buy_count: 1,
                ..SpendState::default()
            },
            RiskContext::default(),
            policy
        ),
        Err(CopySkip::AlreadyBought)
    );
    assert_eq!(
        precheck(
            &configured,
            1.0,
            SpendState::default(),
            RiskContext {
                is_self_wallet: true,
                ..RiskContext::default()
            },
            policy
        ),
        Err(CopySkip::SelfCopy)
    );

    configured.slippage_pct = MAX_MANUAL_SLIPPAGE_PCT + 0.01;
    assert!(matches!(
        precheck(
            &configured,
            1.0,
            SpendState::default(),
            RiskContext::default(),
            policy
        ),
        Err(CopySkip::InvalidSlippage { .. })
    ));
}

#[test]
fn live_mode_uses_the_same_risk_precheck_as_paper() {
    let mut live = task(SizingMode::Fixed { sol: 0.1 });
    live.mode = TradingMode::Live;
    assert_eq!(
        precheck(
            &live,
            1.0,
            SpendState::default(),
            RiskContext::default(),
            PipelinePolicy {
                require_filter_pass: false,
                engine_trade_size_native: 1.0
            }
        ),
        Ok(())
    );
}

/// A paper task input with no exit overrides.
fn task_input(sizing: SizingMode) -> CopyTaskInput {
    CopyTaskInput {
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: TradingMode::Paper,
        sizing,
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.2,
        max_native_per_token: 1.0,
        total_budget_native: 2.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: true,
        slippage_pct: 1.0,
        require_filter_pass: None,
        copy_stable_quoted: None,
    }
}

#[test]
fn task_input_rejects_invalid_mode_sizing_ranges_and_slippage() {
    assert_eq!(
        task_input(SizingMode::Fixed { sol: f64::NAN }).into_task(
            ChainId::Solana,
            Utc::now(),
            default_exit_policy()
        ),
        Err(CopySkip::InvalidSizing)
    );
    assert_eq!(
        task_input(SizingMode::RatioOfTarget { pct: 0.0 }).into_task(
            ChainId::Solana,
            Utc::now(),
            default_exit_policy()
        ),
        Err(CopySkip::InvalidSizing)
    );
    let mut invalid_range = task_input(SizingMode::Fixed { sol: 0.1 });
    invalid_range.min_target_trade_native = Some(-1.0);
    assert_eq!(
        invalid_range.into_task(ChainId::Solana, Utc::now(), default_exit_policy()),
        Err(CopySkip::InvalidSizing)
    );
    let mut live = task_input(SizingMode::Fixed { sol: 0.1 });
    live.mode = TradingMode::Live;
    assert_eq!(
        live.into_task(ChainId::Solana, Utc::now(), default_exit_policy()),
        Err(CopySkip::ModeTransitionRequired)
    );
    let mut invalid_policy = task_input(SizingMode::Fixed { sol: 0.1 });
    invalid_policy.exit_policy_overrides.trailing.distance_pct = Some(150.0);
    assert_eq!(
        invalid_policy.into_task(ChainId::Solana, Utc::now(), default_exit_policy()),
        Err(CopySkip::InvalidExitPolicy)
    );
}

/// Overrides are checked on the policy they make of the global one, with the rules the
/// trader config is checked with.
#[test]
fn task_exit_overrides_are_checked_on_the_merged_policy() {
    let with_overrides = |edit: &dyn Fn(&mut CopyTaskInput)| {
        let mut input = task_input(SizingMode::Fixed { sol: 0.1 });
        edit(&mut input);
        input.into_task(ChainId::Solana, Utc::now(), default_exit_policy())
    };

    // Global activation 10%: a task trailing 12% below the peak could never sell in profit,
    // even though the trailing stop it would switch on is inherited as off.
    assert_eq!(
        with_overrides(&|input| input.exit_policy_overrides.trailing.distance_pct = Some(12.0)),
        Err(CopySkip::InvalidExitPolicy)
    );
    // A positive loss threshold would close a winning position.
    assert_eq!(
        with_overrides(&|input| {
            input.exit_policy_overrides.time.loss_threshold_pct = Some(5.0);
        }),
        Err(CopySkip::InvalidExitPolicy)
    );
    assert_eq!(
        with_overrides(&|input| {
            input.exit_policy_overrides.stop_loss.threshold_pct = Some(0.0);
        }),
        Err(CopySkip::InvalidExitPolicy)
    );
    // Raising the activation with the distance keeps the trail in profit.
    assert!(with_overrides(&|input| {
        input.exit_policy_overrides.trailing.enabled = Some(true);
        input.exit_policy_overrides.trailing.activation_pct = Some(30.0);
        input.exit_policy_overrides.trailing.distance_pct = Some(12.0);
    })
    .is_ok());
    assert!(with_overrides(&|input| {
        input.exit_policy_overrides.time.loss_threshold_pct = Some(-20.0);
    })
    .is_ok());
}
