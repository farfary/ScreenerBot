// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task patch merging and the per-task book stats.

use chrono::Utc;

use super::{book_stats, merge_task_patch};
use crate::trader::copy::{CopyActivityRow, CopyMode, CopyOutcome, CopyTask, ExitMode, SizingMode};

fn stored() -> CopyTask {
    CopyTask {
        id: 3,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Kept".to_owned()),
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::Mirror,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.2,
        max_native_per_token: 1.0,
        total_budget_native: 5.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
    }
}

#[test]
fn a_partial_patch_changes_only_the_fields_it_names() {
    let merged = merge_task_patch(&stored(), serde_json::json!({ "enabled": false })).unwrap();
    assert!(!merged.enabled);
    assert_eq!(merged.label.as_deref(), Some("Kept"));
    assert_eq!(merged.exit_mode, ExitMode::Mirror);
    assert_eq!(merged.total_budget_native, 5.0);
}

#[test]
fn the_filter_override_is_patchable_and_inherits_when_null() {
    let merged = merge_task_patch(
        &stored(),
        serde_json::json!({ "require_filter_pass": false }),
    )
    .unwrap();
    assert_eq!(merged.require_filter_pass, Some(false));
    let merged = merge_task_patch(
        &stored(),
        serde_json::json!({ "require_filter_pass": null }),
    )
    .unwrap();
    assert_eq!(merged.require_filter_pass, None);
}

fn paper_row(id: i64, mint: &str, at: chrono::DateTime<Utc>, sell: Option<f64>) -> CopyActivityRow {
    use crate::trader::copy::{
        CopyOutcome, CopySellDecision, CopyTelemetry, PaperDecision, PaperFill, PaperQuoteMetadata,
        PaperSellFill,
    };
    let quote = |output_amount, input_amount| PaperQuoteMetadata {
        router_id: "fixture".to_owned(),
        initial_at: at,
        landing_at: at,
        requested_delay_ms: 1000,
        queue_duration_ms: 0,
        minimum_output_amount: output_amount,
        final_output_amount: output_amount,
        input_amount: Some(input_amount),
        token_decimals: Some(6),
        epoch: 0,
        network_fee_estimated: true,
        platform_fee_included_in_output: true,
    };
    let telemetry = CopyTelemetry {
        target_block_time: Some(at.timestamp() - 1),
        detected_at: at,
        decoded_at: at,
        decided_at: at,
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: None,
        fill_price_sol: None,
        backfill: false,
    };
    let (kind, outcome) = match sell {
        None => (
            "paper_filled",
            CopyOutcome::PaperFilled(PaperDecision {
                task_id: 3,
                target_address: "target".to_owned(),
                signature: format!("buy-{id}"),
                mint: mint.to_owned(),
                target_size_sol: 1.0,
                target_token_amount: 100.0,
                sized_sol: 1.0,
                fill: PaperFill {
                    input_sol: 1.0,
                    market_price_sol: 0.01,
                    priced_from_pool: false,
                    fill_price_sol: 0.01,
                    token_amount: 100.0,
                    referral_fee_sol: 0.0,
                    network_fee_sol: 0.0,
                    priority_fee_sol: 0.0,
                    total_cost_sol: 1.0,
                    model: crate::trader::copy::PaperModel::Quote,
                    quote: Some(quote(
                        crate::chains::RawAmount::from(100_000_000_u64),
                        crate::chains::RawAmount::from(1_000_000_000_u64),
                    )),
                },
                telemetry,
            }),
        ),
        Some(proceeds) => (
            "paper_sell_observed",
            CopyOutcome::PaperSellObserved(CopySellDecision {
                task_id: 3,
                target_address: "target".to_owned(),
                target_signature: format!("sell-{id}"),
                mint: mint.to_owned(),
                target_token_amount: 100.0,
                target_sol_amount: proceeds,
                exit_percentage: None,
                transaction_signature: None,
                error: None,
                telemetry,
                paper_fill: Some(PaperSellFill {
                    token_amount: 100.0,
                    market_price_sol: proceeds / 100.0,
                    priced_from_pool: false,
                    fill_price_sol: proceeds / 100.0,
                    gross_sol: proceeds,
                    referral_fee_sol: 0.0,
                    network_fee_sol: 0.0,
                    priority_fee_sol: 0.0,
                    net_proceeds_sol: proceeds,
                    model: crate::trader::copy::PaperModel::Quote,
                    quote: Some(quote(
                        crate::chains::RawAmount::from((proceeds * 1_000_000_000.0) as u64),
                        crate::chains::RawAmount::from(100_000_000_u64),
                    )),
                }),
                exit_rule: None,
                paper_context: None,
            }),
        ),
    };
    CopyActivityRow {
        id,
        task_id: 3,
        kind: kind.to_owned(),
        outcome,
        created_at: at,
    }
}

#[test]
fn book_stats_count_wins_and_losses_from_the_closed_rounds() {
    let now = Utc::now();
    let activity = [
        paper_row(1, "won", now, None),
        paper_row(2, "won", now, Some(1.4)),
        paper_row(3, "lost", now, None),
        paper_row(4, "lost", now, Some(0.7)),
        paper_row(5, "open", now, None),
    ];
    let (stats, rounds) = book_stats(&stored(), &activity, &[], &[], |_| None);
    assert_eq!(rounds.len(), 2);
    assert_eq!((stats.wins, stats.losses), (1, 1));
    assert_eq!(stats.filled_buys, 3);
    assert_eq!(stats.target_sells, 2);
}

#[test]
fn legacy_paper_rows_do_not_enter_current_task_totals() {
    let now = Utc::now();
    let mut buy = paper_row(1, "legacy", now, None);
    if let CopyOutcome::PaperFilled(decision) = &mut buy.outcome {
        decision.fill.model = super::super::PaperModel::Legacy;
        decision.fill.quote = None;
    }
    let mut sell = paper_row(2, "legacy", now, Some(2.0));
    if let CopyOutcome::PaperSellObserved(decision) = &mut sell.outcome {
        let fill = decision.paper_fill.as_mut().unwrap();
        fill.model = super::super::PaperModel::Legacy;
        fill.quote = None;
    }
    let activity = [buy, sell];

    let (stats, rounds) = book_stats(&stored(), &activity, &[], &[], |_| None);
    assert!(rounds.is_empty());
    assert_eq!(stats.decisions, 0);
    assert_eq!(stats.filled_buys, 0);
    assert_eq!(stats.target_sells, 0);
}

#[test]
fn a_patch_with_an_unknown_or_mistyped_field_is_refused() {
    assert!(merge_task_patch(&stored(), serde_json::json!({ "enabld": false })).is_err());
    assert!(merge_task_patch(&stored(), serde_json::json!({ "enabled": "no" })).is_err());
    assert!(merge_task_patch(&stored(), serde_json::json!([1])).is_err());
}
