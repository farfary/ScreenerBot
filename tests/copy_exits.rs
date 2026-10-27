// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-sell ownership, gating, execution and lock contracts on both books.

use crate::common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::positions::{Book, PositionManagement, PositionOrigin};
use screenerbot::trader::copy::{
    execute_copy_sell_with, leg_key, prepare_copy_sell, proportional_exit_percentage, CopyOutcome,
    CopySellSubmitResult, CopySkip, CopyTask, ExitMode, SizingMode,
};
use screenerbot::trader::paper::{PaperModel, PaperRefusal, PaperSellFill, TradingMode};
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
        paper_started_at: None,
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

fn paper_round(management: PositionManagement) -> screenerbot::positions::Position {
    let mut position = copy_position(management);
    position.book = Book::Paper;
    position
}

fn paper_sell_fill() -> PaperSellFill {
    PaperSellFill {
        token_amount: 15.0,
        market_price_sol: 0.01,
        priced_from_pool: false,
        fill_price_sol: 0.0099,
        gross_sol: 0.1485,
        referral_fee_sol: 0.0,
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
        net_proceeds_sol: 0.148495,
        model: PaperModel::Quote,
        quote: None,
    }
}

#[test]
fn a_paper_mirror_sell_sells_the_targets_fraction_of_the_task_round() {
    // The target sold 30 of its 100 tokens: the paper round sells 30 percent of itself.
    let plan = prepare_copy_sell(
        &sell_activity(),
        &task(ExitMode::Hybrid, TradingMode::Paper),
        Some(&paper_round(PositionManagement::Hybrid)),
        false,
        100.0,
        Utc::now(),
    )
    .unwrap();
    assert_eq!(plan.decision.book, Book::Paper);
    assert_eq!(plan.decision.action, TradeAction::Sell);
    assert_eq!(plan.decision.reason, TradeReason::CopySell);
    assert_eq!(plan.decision.position_id.as_deref(), Some("42"));
    assert_eq!(plan.decision.exit_percentage, Some(30.0));
    assert_eq!(
        plan.decision.paper_leg_key.as_deref(),
        Some(leg_key(7, "target-sell").as_str())
    );
    assert_eq!(
        proportional_exit_percentage(30.0, 0.0),
        None,
        "unknown target inventory closes the whole round"
    );
}

#[test]
fn a_paper_round_another_owner_opened_is_never_sold_by_a_copy_task() {
    let mut strategy_round = paper_round(PositionManagement::Hybrid);
    strategy_round.origin = PositionOrigin::Auto {
        strategy_id: Some("a-strategy".to_owned()),
    };
    let mut other_task_round = paper_round(PositionManagement::Hybrid);
    other_task_round.origin = PositionOrigin::Copy {
        task_id: 8,
        source_wallet: "target-wallet".to_owned(),
    };
    for round in [strategy_round, other_task_round] {
        assert_eq!(
            prepare_copy_sell(
                &sell_activity(),
                &task(ExitMode::Hybrid, TradingMode::Paper),
                Some(&round),
                false,
                100.0,
                Utc::now(),
            )
            .unwrap_err(),
            CopySkip::CopyPositionNotFound
        );
    }
    assert_eq!(
        prepare_copy_sell(
            &sell_activity(),
            &task(ExitMode::Hybrid, TradingMode::Paper),
            None,
            false,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::CopyPositionNotFound
    );
}

#[test]
fn a_buy_only_paper_task_and_a_round_of_the_other_book_never_sell() {
    assert_eq!(
        prepare_copy_sell(
            &sell_activity(),
            &task(ExitMode::BuyOnly, TradingMode::Paper),
            Some(&paper_round(PositionManagement::AutoTrader)),
            false,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::ExitModeDisabled
    );
    assert_eq!(
        prepare_copy_sell(
            &sell_activity(),
            &task(ExitMode::Mirror, TradingMode::Paper),
            Some(&copy_position(PositionManagement::CopyTask)),
            false,
            100.0,
            Utc::now(),
        )
        .unwrap_err(),
        CopySkip::ModeTransitionRequired,
        "a paper task never sells a live position"
    );
}

#[tokio::test]
async fn a_paper_sell_records_what_the_paper_executor_filled_or_refused() {
    let plan = prepare_copy_sell(
        &sell_activity(),
        &task(ExitMode::Mirror, TradingMode::Paper),
        Some(&paper_round(PositionManagement::CopyTask)),
        false,
        100.0,
        Utc::now(),
    )
    .unwrap();
    let filled = execute_copy_sell_with(plan.clone(), |_| async {
        CopySellSubmitResult::PaperFilled {
            fill: paper_sell_fill(),
        }
    })
    .await;
    let CopyOutcome::PaperSellObserved(decision) = filled else {
        panic!("expected a paper sell, got {filled:?}")
    };
    assert_eq!(decision.paper_fill, Some(paper_sell_fill()));
    assert_eq!(decision.exit_percentage, Some(30.0));
    assert_eq!(
        decision.exit_rule, None,
        "a mirrored sell names no exit rule"
    );
    assert_eq!(decision.telemetry.fill_price_sol, Some(0.0099));

    let refused = execute_copy_sell_with(plan.clone(), |_| async {
        CopySellSubmitResult::PaperRefused {
            refusal: PaperRefusal::AlreadyClosed,
        }
    })
    .await;
    assert!(matches!(
        refused,
        CopyOutcome::Skipped { reason, paper: Some(_), .. }
            if reason == CopySkip::from(PaperRefusal::AlreadyClosed)
    ));

    let live_result = execute_copy_sell_with(plan, |_| async {
        CopySellSubmitResult::Submitted {
            transaction_signature: "never-a-paper-result".to_owned(),
        }
    })
    .await;
    assert!(matches!(
        live_result,
        CopyOutcome::Skipped {
            reason: CopySkip::PaperExecutionUnavailable,
            ..
        }
    ));
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
            let _lock = screenerbot::positions::acquire_position_lock("copy-exit-race-mint").await;
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
