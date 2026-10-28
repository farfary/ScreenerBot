// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy-trading insight aggregation: rounds of the task's book, range filtering, slippage
//! sampling, and latency histogram bucket edges.

use chrono::{Duration, Utc};

use super::*;
use crate::positions::Book;
use crate::trader::copy::round_fixtures::{closed, open_round};
use crate::trader::copy::types::{CopySellDecision, CopyTelemetry, PaperDecision};
use crate::trader::paper::{PaperFill, PaperQuoteMetadata, PaperSellFill};
use crate::trader::TradeReason;

fn quote_provenance(at: DateTime<Utc>, output: u64, input: u64) -> PaperQuoteMetadata {
    PaperQuoteMetadata {
        router_id: "test-router".to_owned(),
        initial_at: at,
        landing_at: at,
        requested_delay_ms: 1000,
        queue_duration_ms: 0,
        minimum_output_amount: crate::chains::RawAmount::from(1_u64),
        final_output_amount: crate::chains::RawAmount::from(output),
        input_amount: Some(crate::chains::RawAmount::from(input)),
        token_decimals: Some(6),
        epoch: 0,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    }
}

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
        earlier_run: false,
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
                priced_from_pool: false,
                fill_price_sol: cost / tokens * 1.01,
                token_amount: tokens,
                referral_fee_sol: 0.0,
                network_fee_sol: 0.0,
                priority_fee_sol: 0.0,
                total_cost_sol: cost,
                model: crate::trader::paper::PaperModel::Quote,
                quote: Some(quote_provenance(
                    at,
                    (tokens * 1_000_000.0).round() as u64,
                    1,
                )),
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
        earlier_run: false,
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
                priced_from_pool: false,
                fill_price_sol: proceeds / tokens,
                gross_sol: proceeds,
                referral_fee_sol: 0.0,
                network_fee_sol: 0.0,
                priority_fee_sol: 0.0,
                net_proceeds_sol: proceeds,
                model: crate::trader::paper::PaperModel::Quote,
                quote: Some(quote_provenance(
                    at,
                    (proceeds * 1_000_000_000.0).round() as u64,
                    (tokens * 1_000_000.0).round() as u64,
                )),
            }),
            exit_rule: rule,
            take_profit_level: None,
            paper_context: None,
        }),
    }
}

#[test]
fn paper_rounds_are_the_task_rounds_of_the_paper_book_with_their_close_reasons() {
    let t0 = Utc::now() - Duration::hours(3);
    let positions = vec![
        closed(
            open_round(1, Book::Paper, "a", 0.01, 1.0, 180),
            0.7,
            TradeReason::StopLoss,
            t0 + Duration::minutes(30),
        ),
        closed(
            open_round(1, Book::Paper, "b", 0.01, 1.0, 120),
            -0.6,
            TradeReason::ManualExit,
            t0 + Duration::hours(2),
        ),
        open_round(1, Book::Paper, "c", 0.01, 1.0, 60),
        // Another task's round and this task's live round are not this book's.
        closed(
            open_round(2, Book::Paper, "d", 0.01, 1.0, 100),
            5.0,
            TradeReason::TakeProfit,
            t0 + Duration::hours(1),
        ),
        closed(
            open_round(1, Book::Live, "e", 0.01, 1.0, 100),
            5.0,
            TradeReason::TakeProfit,
            t0 + Duration::hours(1),
        ),
    ];
    let activity = vec![
        buy(1, "a", 100.0, 1.0, t0),
        sell(2, "a", 50.0, 0.8, None, t0 + Duration::minutes(10)),
        buy(3, "b", 10.0, 1.0, t0 + Duration::hours(1)),
    ];
    let insights = build_insights(
        1,
        CopyBook::Paper,
        &activity,
        &positions,
        InsightRange::default(),
        None,
    );
    assert_eq!(insights.rounds, 2, "the open round in c is not closed");
    assert_eq!((insights.wins, insights.losses), (1, 1));
    assert!((insights.realized_pnl_native - 0.1).abs() < 1e-9);
    assert_eq!(insights.recent_rounds[0].exit, "ManualExit");
    assert_eq!(insights.recent_rounds[0].book, CopyBook::Paper);
    let stop = insights
        .exit_breakdown
        .iter()
        .find(|bucket| bucket.exit == "StopLoss")
        .unwrap();
    assert_eq!(stop.legs, 1);
    assert!((stop.pnl_native - 0.7).abs() < 1e-9);
    assert_eq!(insights.pnl_curve.len(), 2);
    assert!((insights.pnl_curve[1].cumulative_pnl_native - 0.1).abs() < 1e-9);
    assert_eq!(insights.decisions.fills, 2);
    assert_eq!(insights.decisions.exits, 1);
    assert_eq!(
        closed_rounds(1, CopyBook::Paper, &positions)
            .iter()
            .map(|round| round.mint.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn a_range_keeps_rounds_that_closed_inside_it() {
    let t0 = Utc::now() - Duration::days(2);
    let positions = vec![
        closed(
            open_round(1, Book::Paper, "a", 0.01, 1.0, 3_000),
            1.0,
            TradeReason::CopySell,
            t0 + Duration::hours(1),
        ),
        closed(
            open_round(1, Book::Paper, "b", 0.01, 1.0, 1_500),
            -0.5,
            TradeReason::CopySell,
            t0 + Duration::days(1) + Duration::hours(1),
        ),
    ];
    let activity = vec![
        buy(1, "a", 10.0, 1.0, t0),
        buy(3, "b", 10.0, 1.0, t0 + Duration::days(1)),
    ];
    let range = InsightRange {
        from: Some(t0 + Duration::hours(12)),
        to: None,
    };
    let insights = build_insights(1, CopyBook::Paper, &activity, &positions, range, None);
    assert_eq!(insights.rounds, 1);
    assert!((insights.realized_pnl_native + 0.5).abs() < 1e-9);
    assert_eq!(insights.decisions.fills, 1);
}

#[test]
fn quote_slippage_samples_do_not_require_a_pool_price_source() {
    let t0 = Utc::now();
    let mut observed = buy(1, "a", 100.0, 1.0, t0);
    if let CopyOutcome::PaperFilled(decision) = &mut observed.outcome {
        decision.fill.priced_from_pool = false;
    }
    let activity = vec![observed, buy(2, "b", 100.0, 1.0, t0)];
    let insights = build_insights(
        1,
        CopyBook::Paper,
        &activity,
        &[],
        InsightRange::default(),
        None,
    );
    assert_eq!(insights.slippage.samples, 2);
}

#[test]
fn arrival_buckets_are_upper_exclusive() {
    let buckets = latency_histogram(&[100, 500, 999, 9_000], None);
    let counts = buckets
        .iter()
        .map(|bucket| bucket.count)
        .collect::<Vec<_>>();
    assert_eq!(counts, [1, 2, 0, 0, 0, 1]);
}

#[test]
fn the_arrival_limit_is_always_a_bucket_edge() {
    let buckets = latency_histogram(&[9_000, 11_000], Some(10_000));
    let edges = buckets
        .iter()
        .map(|bucket| bucket.upper_ms)
        .collect::<Vec<_>>();
    assert_eq!(
        edges,
        [
            Some(500),
            Some(1_000),
            Some(2_000),
            Some(4_000),
            Some(8_000),
            Some(10_000),
            None
        ]
    );
    assert_eq!((buckets[5].count, buckets[6].count), (1, 1));
    assert_eq!(latency_histogram(&[], Some(4_000)).len(), 6);
}
