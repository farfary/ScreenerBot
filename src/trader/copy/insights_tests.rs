use chrono::{Duration, Utc};

use super::*;
use crate::trader::copy::types::{
    CopySellDecision, CopyTelemetry, PaperDecision, PaperFill, PaperSellFill,
};

fn telemetry(at: DateTime<Utc>, target_price: Option<f64>) -> CopyTelemetry {
    CopyTelemetry {
        target_block_time: Some(at.timestamp() - 1),
        detected_at: at,
        decoded_at: at,
        decided_at: at,
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: target_price,
        fill_price_sol: None,
        backfill: false,
    }
}

fn buy(id: i64, mint: &str, tokens: f64, cost: f64, at: DateTime<Utc>) -> CopyActivityRow {
    CopyActivityRow {
        id,
        task_id: 1,
        kind: "paper_filled".to_owned(),
        created_at: at,
        outcome: CopyOutcome::PaperFilled(PaperDecision {
            task_id: 1,
            target_address: "target".to_owned(),
            signature: format!("buy-{id}"),
            mint: mint.to_owned(),
            target_size_sol: cost,
            target_token_amount: tokens,
            sized_sol: cost,
            fill: PaperFill {
                input_sol: cost,
                market_price_sol: cost / tokens,
                fill_price_sol: cost / tokens * 1.01,
                token_amount: tokens,
                referral_fee_sol: 0.0,
                network_fee_sol: 0.0,
                priority_fee_sol: 0.0,
                total_cost_sol: cost,
            },
            telemetry: telemetry(at, Some(cost / tokens)),
        }),
    }
}

fn sell(
    id: i64,
    mint: &str,
    tokens: f64,
    proceeds: f64,
    rule: Option<PaperExitRule>,
    at: DateTime<Utc>,
) -> CopyActivityRow {
    CopyActivityRow {
        id,
        task_id: 1,
        kind: "paper_sell_observed".to_owned(),
        created_at: at,
        outcome: CopyOutcome::PaperSellObserved(CopySellDecision {
            task_id: 1,
            target_address: "target".to_owned(),
            target_signature: format!("sell-{id}"),
            mint: mint.to_owned(),
            target_token_amount: tokens,
            target_sol_amount: proceeds,
            exit_percentage: None,
            transaction_signature: None,
            error: None,
            telemetry: telemetry(at, None),
            paper_fill: Some(PaperSellFill {
                token_amount: tokens,
                market_price_sol: proceeds / tokens,
                fill_price_sol: proceeds / tokens,
                gross_sol: proceeds,
                referral_fee_sol: 0.0,
                network_fee_sol: 0.0,
                priority_fee_sol: 0.0,
                net_proceeds_sol: proceeds,
            }),
            exit_rule: rule,
        }),
    }
}

#[test]
fn paper_rounds_replay_partial_exits_into_one_closed_round() {
    let t0 = Utc::now() - Duration::hours(3);
    let activity = vec![
        buy(1, "a", 100.0, 1.0, t0),
        sell(
            2,
            "a",
            50.0,
            0.8,
            Some(PaperExitRule::StopLoss),
            t0 + Duration::minutes(10),
        ),
        sell(3, "a", 50.0, 0.9, None, t0 + Duration::minutes(30)),
        buy(4, "b", 10.0, 1.0, t0 + Duration::hours(1)),
        sell(
            5,
            "b",
            10.0,
            0.4,
            Some(PaperExitRule::Manual),
            t0 + Duration::hours(2),
        ),
        buy(6, "c", 10.0, 1.0, t0 + Duration::hours(2)),
    ];
    let insights = build_insights(1, CopyBook::Paper, &activity, &[], InsightRange::default());
    assert_eq!(insights.rounds, 2, "the open round in c is not closed");
    assert_eq!((insights.wins, insights.losses), (1, 1));
    assert!((insights.realized_pnl_sol - 0.1).abs() < 1e-9);
    assert_eq!(insights.recent_rounds[0].exit, "manual");
    assert_eq!(insights.recent_rounds[1].hold_seconds, 30 * 60);
    let stop = insights
        .exit_breakdown
        .iter()
        .find(|bucket| bucket.exit == "stop_loss")
        .unwrap();
    assert!((stop.pnl_sol - 0.3).abs() < 1e-9);
    assert_eq!(insights.pnl_curve.len(), 2);
    assert!((insights.pnl_curve[1].cumulative_pnl_sol - 0.1).abs() < 1e-9);
    assert_eq!(insights.decisions.fills, 3);
    assert_eq!(insights.slippage.samples, 3);
    assert!((insights.slippage.median_pct.unwrap() - 1.0).abs() < 1e-6);
}

#[test]
fn a_range_keeps_rounds_that_closed_inside_it() {
    let t0 = Utc::now() - Duration::days(2);
    let activity = vec![
        buy(1, "a", 10.0, 1.0, t0),
        sell(2, "a", 10.0, 2.0, None, t0 + Duration::hours(1)),
        buy(3, "b", 10.0, 1.0, t0 + Duration::days(1)),
        sell(
            4,
            "b",
            10.0,
            0.5,
            None,
            t0 + Duration::days(1) + Duration::hours(1),
        ),
    ];
    let range = InsightRange {
        from: Some(t0 + Duration::hours(12)),
        to: None,
    };
    let insights = build_insights(1, CopyBook::Paper, &activity, &[], range);
    assert_eq!(insights.rounds, 1);
    assert!((insights.realized_pnl_sol + 0.5).abs() < 1e-9);
    assert_eq!(insights.decisions.fills, 1);
}

#[test]
fn arrival_buckets_are_upper_exclusive() {
    let buckets = latency_histogram(&[100, 500, 999, 9_000]);
    let counts = buckets
        .iter()
        .map(|bucket| bucket.count)
        .collect::<Vec<_>>();
    assert_eq!(counts, [1, 2, 0, 0, 0, 1]);
}
