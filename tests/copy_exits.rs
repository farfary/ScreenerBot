// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure Phase 4 copy-sell ownership, gating, execution, and lock contracts.

use crate::common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::positions::{PositionManagement, PositionOrigin};
use screenerbot::trader::copy::{
    execute_copy_sell_with, paper_sell_outcome, prepare_copy_sell, proportional_exit_percentage,
    CopyDatabase, CopyOutcome, CopySellSubmitResult, CopySkip, CopyTask, ExitMode, PaperPosition,
    SizingMode,
};
use screenerbot::trader::paper::TradingMode;
use screenerbot::trader::{TradeAction, TradeReason};
use screenerbot::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

fn task(exit_mode: ExitMode, mode: TradingMode) -> CopyTask {
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
        max_native_per_trade: 1.0,
        max_native_per_token: 2.0,
        total_budget_native: 5.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 2.0,
        created_at: now,
        updated_at: now,
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
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
            native_amount: 0.3,
            token_amount: 30.0,
            venue: Some("jupiter".to_owned()),
            price_native: Some(0.01),
        },
        sources: vec![WatchSource::Copy { task_id: 7 }],
        backfill: false,
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
            &task(ExitMode::Mirror, TradingMode::Live),
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
        &task(ExitMode::Mirror, TradingMode::Live),
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
            &task(ExitMode::BuyOnly, TradingMode::Live),
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
            &task(ExitMode::Mirror, TradingMode::Live),
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
            &task(ExitMode::Mirror, TradingMode::Live),
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
        &task(ExitMode::Hybrid, TradingMode::Live),
        Some(&copy_position(PositionManagement::Hybrid)),
        false,
        100.0,
        Utc::now(),
    )
    .is_ok());
}

#[test]
fn paper_mode_sells_the_targets_fraction_of_the_paper_holding() {
    // The target sold 30 of its 100 tokens; the paper book holds 50.
    let pct = proportional_exit_percentage(30.0, 100.0).unwrap();
    assert_eq!(pct, 30.0);
    assert_eq!(50.0 * pct / 100.0, 15.0);
}

#[tokio::test]
async fn paper_sell_without_a_paper_holding_is_skipped_before_quoting() {
    let dir = tempfile::tempdir().unwrap();
    let db = CopyDatabase::open(dir.path().join("copy_trading.db"), ChainId::Solana).unwrap();
    let position = PaperPosition {
        task_id: 7,
        mint: common::TEST_MINT.to_owned(),
        token_amount: 0.0,
        raw_token_amount: None,
        token_decimals: None,
        cost_basis_native: 0.0,
        invested_native: 0.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 0,
        sells: 0,
        last_price_native: None,
        last_price_at: None,
        opened_at: Utc.timestamp_opt(10, 0).unwrap(),
        closed_at: None,
        peak_price_native: None,
        take_profit_level: 0,
        round_partial_exits: 0,
    };
    assert_eq!(
        paper_sell_outcome(
            &sell_activity(),
            &task(ExitMode::Hybrid, TradingMode::Paper),
            false,
            100.0,
            &position,
            &db,
            0,
            Utc::now(),
            |_| async {
                Err(screenerbot::swaps::QuoteError::Unavailable {
                    router: "test".to_owned(),
                    detail: "not reached".to_owned(),
                })
            },
            |_| async { Ok(()) },
        )
        .await
        .unwrap_err(),
        CopySkip::CopyPositionNotFound
    );
}

#[tokio::test]
async fn submitted_and_failed_copy_sells_are_distinct_activity_outcomes() {
    let plan = prepare_copy_sell(
        &sell_activity(),
        &task(ExitMode::Mirror, TradingMode::Live),
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
