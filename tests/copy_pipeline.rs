// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Offline observed-activity to paper-book contract: the copy buy is prechecked, sized
//! and planned as a shared trade decision, and its paper run admits, dispatches and
//! records exactly what the shared paper executor booked or refused.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::positions::{Book, PositionOrigin};
use screenerbot::trader::admission::EntryBlock;
use screenerbot::trader::copy::{
    execute_paper_with, leg_key, prepare_entry, CopyOutcome, CopySkip, CopyTask, ExitMode,
    PipelinePolicy, PreparedEntry, RiskContext, SizingMode, SpendState,
};
use screenerbot::trader::paper::{
    PaperExecution, PaperFill, PaperLegEvidence, PaperLegFill, PaperModel, PaperRefusal,
    TradingMode,
};
use screenerbot::trader::{Error, TradeAction, TradeDecision, TradeReason, TradeResult, TradeStep};
use screenerbot::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

fn task(mode: TradingMode) -> CopyTask {
    let created = Utc.timestamp_opt(10, 0).unwrap();
    CopyTask {
        id: 7,
        chain: ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Paper target".to_owned()),
        enabled: true,
        mode,
        sizing: SizingMode::RatioOfTarget { pct: 50.0 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 1.0,
        max_native_per_token: 2.0,
        total_budget_native: 5.0,
        min_target_trade_native: Some(0.1),
        max_target_trade_native: Some(10.0),
        buy_once_per_token: true,
        slippage_pct: 1.0,
        created_at: created,
        updated_at: created,
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
        paper_started_at: None,
    }
}

fn activity() -> WalletActivity {
    WalletActivity {
        subject: "target".to_owned(),
        signature: "signature".to_owned(),
        slot: 1,
        block_time: Some(9),
        detected_at: Utc.timestamp_opt(10, 0).unwrap(),
        decoded_at: Utc.timestamp_opt(11, 0).unwrap(),
        success: true,
        kind: ActivityKind::Swap {
            mint: "mint".to_owned(),
            side: SwapSide::Buy,
            native_amount: 0.4,
            token_amount: 100.0,
            venue: Some("jupiter".to_owned()),
            price_native: Some(0.004),
        },
        sources: vec![WatchSource::Copy { task_id: 7 }],
        backfill: false,
    }
}

fn plan(mode: TradingMode) -> PreparedEntry {
    prepare_entry(
        &activity(),
        &task(mode),
        SpendState::default(),
        RiskContext {
            filter_passed: true,
            ..RiskContext::default()
        },
        PipelinePolicy {
            require_filter_pass: true,
            engine_trade_size_native: 1.0,
        },
        Utc.timestamp_opt(12, 0).unwrap(),
    )
    .expect("entry plan")
}

fn buy_fill() -> PaperFill {
    PaperFill {
        input_sol: 0.2,
        market_price_sol: 0.004,
        priced_from_pool: false,
        fill_price_sol: 0.0041,
        token_amount: 48.0,
        referral_fee_sol: 0.0,
        network_fee_sol: 0.000005,
        priority_fee_sol: 0.0,
        total_cost_sol: 0.200005,
        model: PaperModel::Quote,
        quote: None,
    }
}

fn booked(decision: TradeDecision) -> screenerbot::trader::Result<TradeResult> {
    Ok(TradeResult::paper(
        decision,
        PaperLegEvidence {
            leg_ref: "leg".to_owned(),
            position_id: 41,
            fill: PaperLegFill::Buy(buy_fill()),
            replayed: false,
        },
    ))
}

fn refused(
    decision: TradeDecision,
    refusal: PaperRefusal,
) -> screenerbot::trader::Result<TradeResult> {
    let mut result =
        TradeResult::failure_at(decision, TradeStep::Validation, "refused".to_owned(), 0);
    result.paper = Some(PaperExecution::Refused(refusal));
    Ok(result)
}

#[test]
fn observed_buy_is_prechecked_and_sized_into_a_paper_decision_with_its_leg_key() {
    let plan = plan(TradingMode::Paper);
    assert_eq!(plan.task.id, 7);
    assert_eq!(plan.target_signature, "signature");
    assert_eq!(plan.target_size_native, 0.4);
    assert_eq!(plan.target_token_amount, 100.0);
    assert_eq!(plan.decision.mint, "mint");
    assert_eq!(plan.decision.action, TradeAction::Buy);
    assert_eq!(plan.decision.reason, TradeReason::CopyBuy);
    assert_eq!(plan.decision.book, Book::Paper);
    assert_eq!(plan.decision.size_native, Some(0.2));
    assert_eq!(
        plan.decision.paper_leg_key.as_deref(),
        Some(leg_key(7, "signature").as_str())
    );
    assert_eq!(
        plan.context.origin,
        PositionOrigin::Copy {
            task_id: 7,
            source_wallet: "target".to_owned()
        }
    );
}

#[test]
fn a_skipped_precheck_plans_nothing() {
    let result = prepare_entry(
        &activity(),
        &task(TradingMode::Paper),
        SpendState::default(),
        RiskContext::default(),
        PipelinePolicy {
            require_filter_pass: true,
            engine_trade_size_native: 1.0,
        },
        Utc.timestamp_opt(12, 0).unwrap(),
    );
    assert!(
        matches!(result, Err(CopySkip::FilterRequired)),
        "a buy that fails the filter gate is not planned"
    );
}

#[tokio::test]
async fn a_paper_fill_is_recorded_from_the_executor_evidence() {
    let dispatched = Arc::new(AtomicUsize::new(0));
    let counted = dispatched.clone();
    let outcome = execute_paper_with(
        plan(TradingMode::Paper),
        |_| async { Ok(()) },
        move |decision, context| {
            counted.fetch_add(1, Ordering::SeqCst);
            assert_eq!(decision.book, Book::Paper);
            assert!(matches!(
                context.origin,
                PositionOrigin::Copy { task_id: 7, .. }
            ));
            async move { booked(decision) }
        },
    )
    .await;
    assert_eq!(dispatched.load(Ordering::SeqCst), 1);
    let CopyOutcome::PaperFilled(decision) = outcome else {
        panic!("expected a paper fill, got {outcome:?}")
    };
    assert_eq!(decision.fill, buy_fill());
    assert_eq!(decision.sized_sol, 0.2, "the spend is what the fill drew");
    assert_eq!(decision.telemetry.fill_price_sol, Some(0.0041));
}

#[tokio::test]
async fn an_admission_block_never_dispatches_and_is_recorded_typed() {
    let dispatched = Arc::new(AtomicUsize::new(0));
    let counted = dispatched.clone();
    let outcome = execute_paper_with(
        plan(TradingMode::Paper),
        |_| async { Err(EntryBlock::PaperBalanceExhausted) },
        move |decision, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            async move { booked(decision) }
        },
    )
    .await;
    assert_eq!(dispatched.load(Ordering::SeqCst), 0);
    assert!(matches!(
        outcome,
        CopyOutcome::Skipped {
            reason: CopySkip::EntryBlocked {
                block: EntryBlock::PaperBalanceExhausted
            },
            paper: Some(_),
            ..
        }
    ));
}

#[tokio::test]
async fn every_executor_refusal_class_is_recorded_as_its_typed_skip() {
    for refusal in [
        PaperRefusal::BalanceExhausted,
        PaperRefusal::PositionLimit,
        PaperRefusal::AlreadyOpen,
        PaperRefusal::OpenCooldown { wait_secs: 3 },
        PaperRefusal::DecimalsUnknown,
        PaperRefusal::InvalidSize,
    ] {
        let outcome = execute_paper_with(
            plan(TradingMode::Paper),
            |_| async { Ok(()) },
            move |decision, _| async move { refused(decision, refusal) },
        )
        .await;
        let CopyOutcome::Skipped { reason, .. } = outcome else {
            panic!("expected a skip for {refusal:?}, got {outcome:?}")
        };
        assert_eq!(reason, CopySkip::from(refusal));
    }
}

#[tokio::test]
async fn a_dispatch_that_never_ran_or_a_live_plan_books_nothing() {
    let unavailable = execute_paper_with(
        plan(TradingMode::Paper),
        |_| async { Ok(()) },
        |_, _| async { Err(Error::PaperBookUnavailable) },
    )
    .await;
    assert!(matches!(
        unavailable,
        CopyOutcome::Skipped {
            reason: CopySkip::PaperExecutionUnavailable,
            ..
        }
    ));

    let dispatched = Arc::new(AtomicUsize::new(0));
    let counted = dispatched.clone();
    let live = execute_paper_with(
        plan(TradingMode::Live),
        move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
            async { Ok(()) }
        },
        |decision, _| async move { booked(decision) },
    )
    .await;
    assert_eq!(
        dispatched.load(Ordering::SeqCst),
        0,
        "a live plan is never admitted on the paper book"
    );
    assert!(matches!(
        live,
        CopyOutcome::Skipped {
            reason: CopySkip::ModeTransitionRequired,
            ..
        }
    ));
}
