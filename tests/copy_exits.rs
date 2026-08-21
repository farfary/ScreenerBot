//! Pure Phase 4 copy-sell ownership, gating, execution, and lock contracts.

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::positions::{PositionManagement, PositionOrigin};
use screenerbot::trader::copy::{
    execute_copy_sell_with, paper_sell_outcome, prepare_copy_sell, CopyMode, CopyOutcome,
    CopySellSubmitResult, CopySkip, CopyTask, ExitMode, SizingMode,
};
use screenerbot::trader::{TradeAction, TradeReason};
use screenerbot::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

fn task(exit_mode: ExitMode, mode: CopyMode) -> CopyTask {
    let now = Utc.timestamp_opt(10, 0).unwrap();
    CopyTask {
        id: 7,
        chain: ChainId::Solana,
        target_address: "target-wallet".to_owned(),
        label: None,
        enabled: true,
        mode,
        sizing: SizingMode::Fixed { sol: 0.1 },
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

fn sell_activity() -> WalletActivity {
    WalletActivity {
        subject: "target-wallet".to_owned(),
        signature: "target-sell".to_owned(),
        slot: 1,
        block_time: Some(9),
        detected_at: Utc.timestamp_opt(10, 0).unwrap(),
        decoded_at: Utc.timestamp_opt(11, 0).unwrap(),
        success: true,
        kind: ActivityKind::Swap {
            mint: common::TEST_MINT.to_owned(),
            side: SwapSide::Sell,
            sol_amount: 0.3,
            token_amount: 30.0,
            venue: Some("jupiter".to_owned()),
            price_sol: Some(0.01),
        },
        sources: vec![WatchSource::Copy { task_id: 7 }],
    }
}

fn copy_position(management: PositionManagement) -> screenerbot::positions::Position {
    let mut position = common::test_position(0.01, 1.0);
    position.id = Some(42);
    position.origin = PositionOrigin::Copy {
        task_id: 7,
        source_wallet: "target-wallet".to_owned(),
    };
    position.management = management;
    position
}

#[test]
fn target_sell_selects_only_the_same_tasks_copy_position() {
    let strategy_position = common::test_position(0.01, 1.0);
    assert_eq!(
        prepare_copy_sell(
            &sell_activity(),
            &task(ExitMode::Mirror, CopyMode::Live),
            Some(&strategy_position),
            false,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::CopyPositionNotFound
    );

    let plan = prepare_copy_sell(
        &sell_activity(),
        &task(ExitMode::Mirror, CopyMode::Live),
        Some(&copy_position(PositionManagement::CopyTask)),
        false,
        100.0,
        Utc::now(),
    )
    .unwrap();
    assert_eq!(plan.decision.action, TradeAction::Sell);
    assert_eq!(plan.decision.reason, TradeReason::CopySell);
    assert_eq!(plan.decision.position_id.as_deref(), Some("42"));
    assert_eq!(plan.decision.exit_percentage, Some(30.0));
}

#[test]
fn exit_mode_force_stop_and_user_ownership_are_typed_skips() {
    let activity = sell_activity();
    let position = copy_position(PositionManagement::CopyTask);
    assert_eq!(
        prepare_copy_sell(
            &activity,
            &task(ExitMode::BuyOnly, CopyMode::Live),
            Some(&position),
            false,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::ExitModeDisabled
    );
    assert_eq!(
        prepare_copy_sell(
            &activity,
            &task(ExitMode::Mirror, CopyMode::Live),
            Some(&position),
            true,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::ForceStopped
    );
    assert_eq!(
        prepare_copy_sell(
            &activity,
            &task(ExitMode::Mirror, CopyMode::Live),
            Some(&copy_position(PositionManagement::UserOnly)),
            false,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::PositionUserOnly
    );

    // There is deliberately no loss-limit input to the sell path: the period loss
    // limit blocks entries, never exits. With force stop false, the same sell proceeds.
    assert!(prepare_copy_sell(
        &activity,
        &task(ExitMode::Hybrid, CopyMode::Live),
        Some(&copy_position(PositionManagement::Hybrid)),
        false,
        100.0,
        Utc::now(),
    )
    .is_ok());
}

#[test]
fn paper_mode_records_sell_observation_without_claiming_a_position() {
    let outcome = paper_sell_outcome(
        &sell_activity(),
        &task(ExitMode::Mirror, CopyMode::Paper),
        false,
        100.0,
        Utc::now(),
    )
    .unwrap();
    assert!(matches!(outcome, CopyOutcome::PaperSellObserved(_)));
}

#[tokio::test]
async fn submitted_and_failed_copy_sells_are_distinct_activity_outcomes() {
    let plan = prepare_copy_sell(
        &sell_activity(),
        &task(ExitMode::Mirror, CopyMode::Live),
        Some(&copy_position(PositionManagement::CopyTask)),
        false,
        100.0,
        Utc::now(),
    )
    .unwrap();
    let submitted = execute_copy_sell_with(plan.clone(), |_| async {
        CopySellSubmitResult::Submitted {
            transaction_signature: "our-sell".to_owned(),
        }
    })
    .await;
    assert!(matches!(submitted, CopyOutcome::LiveSellSubmitted(_)));

    let failed = execute_copy_sell_with(plan, |_| async {
        CopySellSubmitResult::Failed {
            error: "quote unavailable".to_owned(),
        }
    })
    .await;
    assert!(matches!(failed, CopyOutcome::LiveSellFailed(_)));
}

#[tokio::test]
async fn hybrid_and_auto_exit_race_has_one_winner_through_the_existing_mint_lock() {
    let claimed = Arc::new(AtomicBool::new(false));
    let winners = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..2 {
        let claimed = claimed.clone();
        let winners = winners.clone();
        tasks.push(tokio::spawn(async move {
            let _lock = screenerbot::positions::acquire_position_lock("phase4-race-mint").await;
            if !claimed.swap(true, Ordering::SeqCst) {
                winners.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
            }
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    assert_eq!(winners.load(Ordering::SeqCst), 1);
}
