//! Pure copy sizing and risk-cap contracts.

use chrono::Utc;
use screenerbot::trader::copy::{
    precheck, size_for, CopyMode, CopySkip, CopyTask, CopyTaskInput, ExitMode, PipelinePolicy,
    RiskContext, SizingMode, SpendState,
};
use screenerbot::trader::{MAX_MANUAL_SLIPPAGE_PCT, MAX_TRADE_SIZE_MULTIPLIER};

fn task(sizing: SizingMode) -> CopyTask {
    CopyTask {
        id: 1,
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: CopyMode::Paper,
        sizing,
        exit_mode: ExitMode::BuyOnly,
        max_sol_per_trade: 0.5,
        max_sol_per_token: 1.0,
        total_budget_sol: 2.0,
        min_target_trade_sol: None,
        max_target_trade_sol: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn fixed_and_ratio_modes_are_clamped_by_every_hard_cap() {
    let fixed = task(SizingMode::Fixed { sol: 5.0 });
    assert_eq!(size_for(&fixed, 10.0, SpendState::default(), 1.0), Ok(0.5));

    let ratio = task(SizingMode::RatioOfTarget { pct: 25.0 });
    assert_eq!(size_for(&ratio, 1.0, SpendState::default(), 1.0), Ok(0.25));

    let spend = SpendState {
        total_spent_sol: 1.9,
        token_spent_sol: 0.95,
        token_buy_count: 1,
    };
    let remaining = size_for(&fixed, 10.0, spend, 1.0).unwrap();
    assert!((remaining - 0.05).abs() < 1e-12);

    let mut engine_capped = fixed.clone();
    engine_capped.max_sol_per_trade = 1_000.0;
    engine_capped.max_sol_per_token = 1_000.0;
    engine_capped.total_budget_sol = 1_000.0;
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
                total_spent_sol: 2.0,
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
                token_spent_sol: 1.0,
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
    configured.min_target_trade_sol = Some(0.2);
    configured.max_target_trade_sol = Some(2.0);
    let policy = PipelinePolicy {
        require_filter_pass: false,
        engine_trade_size_sol: 1.0,
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
    live.mode = CopyMode::Live;
    assert_eq!(
        precheck(
            &live,
            1.0,
            SpendState::default(),
            RiskContext::default(),
            PipelinePolicy {
                require_filter_pass: false,
                engine_trade_size_sol: 1.0
            }
        ),
        Ok(())
    );
}

#[test]
fn task_input_rejects_invalid_mode_sizing_ranges_and_slippage() {
    let input = |sizing| CopyTaskInput {
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: CopyMode::Paper,
        sizing,
        exit_mode: ExitMode::BuyOnly,
        max_sol_per_trade: 0.2,
        max_sol_per_token: 1.0,
        total_budget_sol: 2.0,
        min_target_trade_sol: None,
        max_target_trade_sol: None,
        buy_once_per_token: true,
        slippage_pct: 1.0,
    };

    assert_eq!(
        input(SizingMode::Fixed { sol: f64::NAN }).into_task(Utc::now()),
        Err(CopySkip::InvalidSizing)
    );
    assert_eq!(
        input(SizingMode::RatioOfTarget { pct: 0.0 }).into_task(Utc::now()),
        Err(CopySkip::InvalidSizing)
    );
    let mut invalid_range = input(SizingMode::Fixed { sol: 0.1 });
    invalid_range.min_target_trade_sol = Some(-1.0);
    assert_eq!(
        invalid_range.into_task(Utc::now()),
        Err(CopySkip::InvalidSizing)
    );
    let mut live = input(SizingMode::Fixed { sol: 0.1 });
    live.mode = CopyMode::Live;
    assert_eq!(
        live.into_task(Utc::now()),
        Err(CopySkip::ModeTransitionRequired)
    );
}
