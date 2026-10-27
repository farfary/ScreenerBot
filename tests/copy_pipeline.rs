// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Offline observed-activity to paper-fill pipeline contract.

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::trader::copy::{
    plan_paper_entries, CopyTask, ExitMode, PipelinePolicy, RiskContext, SizingMode, SpendState,
};
use screenerbot::trader::paper::TradingMode;
use screenerbot::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

#[test]
fn observed_buy_is_prechecked_and_sized_before_quote_execution() {
    let detected = Utc.timestamp_opt(10, 0).unwrap();
    let decoded = Utc.timestamp_opt(11, 0).unwrap();
    let task = CopyTask {
        id: 7,
        chain: ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Paper target".to_owned()),
        enabled: true,
        mode: TradingMode::Paper,
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
        created_at: detected,
        updated_at: detected,
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
    };
    let activity = WalletActivity {
        subject: "target".to_owned(),
        signature: "signature".to_owned(),
        slot: 1,
        block_time: Some(9),
        detected_at: detected,
        decoded_at: decoded,
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
    };
    let plans = plan_paper_entries(
        &activity,
        &[task],
        &HashMap::<i64, SpendState>::new(),
        &HashMap::from([(
            7,
            RiskContext {
                filter_passed: true,
                ..RiskContext::default()
            },
        )]),
        PipelinePolicy {
            require_filter_pass: true,
            engine_trade_size_native: 1.0,
        },
    );
    let plan = plans[0].as_ref().expect("entry plan");
    assert_eq!(plan.task.id, 7);
    assert_eq!(plan.signature, "signature");
    assert_eq!(plan.mint, "mint");
    assert_eq!(plan.target_size_sol, 0.4);
    assert_eq!(plan.target_token_amount, 100.0);
    assert_eq!(plan.sized_sol, 0.2);
}
