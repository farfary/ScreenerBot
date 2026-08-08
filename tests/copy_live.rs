//! Offline contracts for arming and orchestrating live copy entries.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use screenerbot::positions::{PositionManagement, PositionOrigin};
use screenerbot::trader::admission::EntryBlock;
use screenerbot::trader::copy::{
    confirm_mode_transition, execute_live_with, management_for_exit_mode, prepare_live_entry,
    CopyMode, CopyOutcome, CopySkip, CopyTask, CopyTaskInput, ExitMode, LiveSubmitResult,
    PipelinePolicy, RiskContext, SizingMode, SpendState, LIVE_ARM_CONFIRMATION,
};
use screenerbot::trader::TradeResult;
use screenerbot::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

fn task(exit_mode: ExitMode) -> CopyTask {
    let now = Utc.timestamp_opt(10, 0).unwrap();
    CopyTask {
        id: 7,
        target_address: "target-wallet".to_owned(),
        label: None,
        enabled: true,
        mode: CopyMode::Live,
        sizing: SizingMode::RatioOfTarget { pct: 50.0 },
        exit_mode,
        exit_policy_overrides: Default::default(),
        max_sol_per_trade: 1.0,
        max_sol_per_token: 2.0,
        total_budget_sol: 5.0,
        min_target_trade_sol: None,
        max_target_trade_sol: None,
        buy_once_per_token: false,
        slippage_pct: 2.0,
        created_at: now,
        updated_at: now,
    }
}

fn input(mode: CopyMode) -> CopyTaskInput {
    let task = task(ExitMode::BuyOnly);
    CopyTaskInput {
        target_address: task.target_address,
        label: task.label,
        enabled: task.enabled,
        mode,
        sizing: task.sizing,
        exit_mode: task.exit_mode,
        exit_policy_overrides: task.exit_policy_overrides,
        max_sol_per_trade: task.max_sol_per_trade,
        max_sol_per_token: task.max_sol_per_token,
        total_budget_sol: task.total_budget_sol,
        min_target_trade_sol: task.min_target_trade_sol,
        max_target_trade_sol: task.max_target_trade_sol,
        buy_once_per_token: task.buy_once_per_token,
        slippage_pct: task.slippage_pct,
    }
}

fn activity() -> WalletActivity {
    WalletActivity {
        subject: "target-wallet".to_owned(),
        signature: "target-signature".to_owned(),
        slot: 1,
        block_time: Some(9),
        detected_at: Utc.timestamp_opt(10, 0).unwrap(),
        decoded_at: Utc.timestamp_opt(11, 0).unwrap(),
        success: true,
        kind: ActivityKind::Swap {
            mint: "mint".to_owned(),
            side: SwapSide::Buy,
            sol_amount: 0.4,
            token_amount: 100.0,
            venue: Some("jupiter".to_owned()),
            price_sol: Some(0.004),
        },
        sources: vec![WatchSource::Copy { task_id: 7 }],
    }
}

fn plan(exit_mode: ExitMode) -> screenerbot::trader::copy::PreparedLiveEntry {
    prepare_live_entry(
        &activity(),
        &task(exit_mode),
        SpendState::default(),
        RiskContext {
            filter_passed: true,
            ..RiskContext::default()
        },
        PipelinePolicy {
            require_filter_pass: true,
            engine_trade_size_sol: 1.0,
        },
        Utc.timestamp_opt(12, 0).unwrap(),
    )
    .unwrap()
}

#[test]
fn live_arming_requires_the_dedicated_confirmation_and_crud_cannot_bypass_it() {
    assert_eq!(
        confirm_mode_transition(CopyMode::Paper, CopyMode::Live, None),
        Err(CopySkip::LiveConfirmationRequired)
    );
    assert_eq!(
        confirm_mode_transition(CopyMode::Paper, CopyMode::Live, Some(LIVE_ARM_CONFIRMATION)),
        Ok(CopyMode::Live)
    );
    assert_eq!(
        input(CopyMode::Live).into_task(Utc::now()),
        Err(CopySkip::ModeTransitionRequired)
    );
    assert_eq!(
        input(CopyMode::Live).into_task_for_update(Utc::now(), CopyMode::Paper),
        Err(CopySkip::ModeTransitionRequired)
    );
    assert!(input(CopyMode::Live)
        .into_task_for_update(Utc::now(), CopyMode::Live)
        .is_ok());
}

#[test]
fn exit_mode_maps_to_typed_position_ownership_and_copy_origin() {
    for (exit_mode, expected) in [
        (ExitMode::BuyOnly, PositionManagement::AutoTrader),
        (ExitMode::Mirror, PositionManagement::CopyTask),
        (ExitMode::Hybrid, PositionManagement::Hybrid),
    ] {
        assert_eq!(management_for_exit_mode(exit_mode), expected);
        let plan = plan(exit_mode);
        assert_eq!(plan.context.management, expected);
        assert_eq!(
            plan.context.origin,
            PositionOrigin::Copy {
                task_id: 7,
                source_wallet: "target-wallet".to_owned()
            }
        );
        assert_eq!(plan.decision.size_sol, Some(0.2));
    }
}

#[test]
fn confirmation_pending_trade_result_is_never_reported_as_confirmed() {
    let prepared = plan(ExitMode::Mirror);
    let mut result = TradeResult::success(
        prepared.decision,
        "ours-ambiguous".to_owned(),
        0.005,
        0.2,
        None,
    );
    result.confirmation_pending = true;
    assert!(matches!(
        LiveSubmitResult::from_trade_result(Some(result)),
        LiveSubmitResult::Submitted {
            transaction_signature,
            fill_price_sol: Some(0.005)
        } if transaction_signature == "ours-ambiguous"
    ));
}

#[tokio::test]
async fn admission_skip_is_typed_persistable_and_never_calls_submission() {
    let submissions = Arc::new(AtomicUsize::new(0));
    let called = submissions.clone();
    let outcome = execute_live_with(
        plan(ExitMode::Mirror),
        |_| async { Err(EntryBlock::OpenCooldown { wait_secs: 4 }) },
        move |_, _| {
            called.fetch_add(1, Ordering::SeqCst);
            async {
                LiveSubmitResult::Failed {
                    error: "must not run".to_owned(),
                }
            }
        },
    )
    .await;
    assert_eq!(submissions.load(Ordering::SeqCst), 0);
    let CopyOutcome::Skipped {
        reason: CopySkip::EntryBlocked { block },
        telemetry: Some(telemetry),
        ..
    } = outcome
    else {
        panic!("expected typed admission skip")
    };
    assert_eq!(block, EntryBlock::OpenCooldown { wait_secs: 4 });
    assert_eq!(telemetry.submitted_at, None);
}

#[tokio::test]
async fn injected_submission_distinguishes_confirmed_ambiguous_and_definite_failure() {
    let confirmed = execute_live_with(
        plan(ExitMode::Mirror),
        |_| async { Ok(()) },
        |_, _| async {
            LiveSubmitResult::Confirmed {
                transaction_signature: "ours-confirmed".to_owned(),
                fill_price_sol: Some(0.005),
            }
        },
    )
    .await;
    let CopyOutcome::LiveConfirmed(confirmed) = confirmed else {
        panic!("expected confirmed")
    };
    assert!(confirmed.telemetry.submitted_at.is_some());
    assert!(confirmed.telemetry.confirmed_at.is_some());
    assert_eq!(confirmed.telemetry.fill_price_sol, Some(0.005));

    let submitted = execute_live_with(
        plan(ExitMode::Mirror),
        |_| async { Ok(()) },
        |_, _| async {
            LiveSubmitResult::Submitted {
                transaction_signature: "ours-ambiguous".to_owned(),
                fill_price_sol: Some(0.005),
            }
        },
    )
    .await;
    let CopyOutcome::LiveSubmitted(submitted) = submitted else {
        panic!("expected submitted")
    };
    assert_eq!(
        submitted.transaction_signature.as_deref(),
        Some("ours-ambiguous")
    );
    assert!(submitted.telemetry.submitted_at.is_some());
    assert_eq!(submitted.telemetry.confirmed_at, None);

    let failed = execute_live_with(
        plan(ExitMode::Mirror),
        |_| async { Ok(()) },
        |_, _| async {
            LiveSubmitResult::Failed {
                error: "quote rejected before send".to_owned(),
            }
        },
    )
    .await;
    let CopyOutcome::LiveFailed(failed) = failed else {
        panic!("expected failure")
    };
    assert_eq!(failed.transaction_signature, None);
    assert_eq!(failed.telemetry.submitted_at, None);
    assert_eq!(failed.error.as_deref(), Some("quote rejected before send"));
}
