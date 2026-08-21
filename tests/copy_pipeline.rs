//! Offline observed-activity to paper-fill pipeline contract.

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use screenerbot::chains::ChainId;
use screenerbot::trader::copy::{
    run_paper_pipeline, CopyMode, CopyOutcome, CopyTask, ExitMode, PaperCosts, PipelinePolicy,
    RiskContext, SizingMode, SpendState,
};
use screenerbot::wallets::watch::{ActivityKind, SwapSide, WalletActivity, WatchSource};

#[test]
fn observed_buy_matches_sizes_and_produces_costed_paper_fill_with_telemetry() {
    let detected = Utc.timestamp_opt(10, 0).unwrap();
    let decoded = Utc.timestamp_opt(11, 0).unwrap();
    let decided = Utc.timestamp_opt(12, 0).unwrap();
    let task = CopyTask {
        id: 7,
        chain: ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Paper target".to_owned()),
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::RatioOfTarget { pct: 50.0 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_sol_per_trade: 1.0,
        max_sol_per_token: 2.0,
        total_budget_sol: 5.0,
        min_target_trade_sol: Some(0.1),
        max_target_trade_sol: Some(10.0),
        buy_once_per_token: true,
        slippage_pct: 1.0,
        created_at: detected,
        updated_at: detected,
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
            sol_amount: 0.4,
            token_amount: 100.0,
            venue: Some("jupiter".to_owned()),
            price_sol: Some(0.004),
        },
        sources: vec![WatchSource::Copy { task_id: 7 }],
    };
    let outcomes = run_paper_pipeline(
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
            engine_trade_size_sol: 1.0,
        },
        0.005,
        PaperCosts {
            network_fee_sol: 0.000005,
            priority_fee_sol: 0.00001,
        },
        decided,
    );

    let CopyOutcome::PaperFilled(decision) = &outcomes[0] else {
        panic!("expected paper fill");
    };
    assert_eq!(decision.sized_sol, 0.2);
    assert_eq!(decision.fill.fill_price_sol, 0.00505);
    assert_eq!(decision.fill.referral_fee_sol, 0.001);
    assert!((decision.fill.total_cost_sol - 0.200015).abs() < 1e-12);
    assert_eq!(decision.telemetry.target_block_time, Some(9));
    assert_eq!(decision.telemetry.detected_at, detected);
    assert_eq!(decision.telemetry.decoded_at, decoded);
    assert_eq!(decision.telemetry.decided_at, decided);
    assert_eq!(decision.telemetry.confirmed_at, Some(decided));
}
