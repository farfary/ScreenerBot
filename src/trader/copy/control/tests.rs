// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Task patch merging and the per-task book stats.

use chrono::Utc;

use super::{book_stats, followed_wallets, merge_task_patch, CopyTradingStatus, TaskRounds};
use crate::positions::Book;
use crate::trader::copy::round_fixtures::{closed, open_round};
use crate::trader::copy::{CopyActivityRow, CopyOutcome, CopyTask, ExitMode, SizingMode};
use crate::trader::paper::TradingMode;
use crate::trader::types::TradeReason;

fn stored() -> CopyTask {
    CopyTask {
        id: 3,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: Some("Kept".to_owned()),
        enabled: true,
        mode: TradingMode::Paper,
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
        paper_started_at: None,
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

/// A task stored with a trailing distance of 12% and no activation of its own: over
/// the default global activation (10%) the merged trail could never sell at a profit,
/// as happens to a stored task when the global activation is lowered under it.
fn stored_with_stale_trail() -> CopyTask {
    let mut task = stored();
    task.exit_policy_overrides.trailing.distance_pct = Some(12.0);
    task
}

fn global_policy() -> crate::trader::policy::ExitPolicy {
    crate::trader::policy::ExitPolicy::from_trader_config(&crate::config::TraderConfig::default())
}

#[test]
fn a_task_whose_stored_overrides_no_longer_fit_still_pauses_resumes_and_takes_edits() {
    let original = stored_with_stale_trail();
    assert!(original
        .exit_policy_overrides
        .validate_over(global_policy())
        .is_err());
    for patch in [
        serde_json::json!({ "enabled": false }),
        serde_json::json!({ "enabled": true }),
        serde_json::json!({ "label": "Renamed" }),
    ] {
        let task = super::tasks::patched_task(&original, patch.clone(), global_policy())
            .unwrap_or_else(|error| panic!("{patch} was refused: {error}"));
        assert_eq!(task.exit_policy_overrides, original.exit_policy_overrides);
    }
}

#[test]
fn a_patch_that_changes_the_overrides_is_checked_on_the_merged_policy() {
    let original = stored_with_stale_trail();
    let still_wide = super::tasks::patched_task(
        &original,
        serde_json::json!({
            "enabled": false,
            "exit_policy_overrides": { "trailing": { "distance_pct": 15.0 } }
        }),
        global_policy(),
    );
    assert!(matches!(
        still_wide,
        Err(crate::trader::Error::CopyTaskRejected {
            reason: crate::trader::copy::CopySkip::InvalidExitPolicy
        })
    ));
    let fixed = super::tasks::patched_task(
        &original,
        serde_json::json!({
            "exit_policy_overrides": { "trailing": { "activation_pct": 30.0, "distance_pct": 12.0 } }
        }),
        global_policy(),
    )
    .expect("a distance below the task's own activation");
    assert_eq!(
        fixed.exit_policy_overrides.trailing.activation_pct,
        Some(30.0)
    );
}

#[test]
fn a_task_whose_overrides_no_longer_fit_is_flagged_with_the_rule_they_break() {
    let issue = super::books::exit_policy_issue(&stored_with_stale_trail(), global_policy())
        .expect("a 12% distance over the 10% global activation is flagged");
    assert_eq!(issue.id, "copy-exit-issue-warning");
    let crate::i18n::UiArg::Nested(reason) = &issue.args["reason"] else {
        panic!("the reason is the broken rule's own text");
    };
    assert_eq!(
        reason.id,
        "copy-exit-issue-trailing-distance-above-activation"
    );
    let en: crate::i18n::LanguageIdentifier = "en".parse().unwrap();
    assert!(!issue.render_plain(&en).is_empty());

    assert_eq!(
        super::books::exit_policy_issue(&stored(), global_policy()),
        None
    );
}

fn paper_row(id: i64, mint: &str, at: chrono::DateTime<Utc>, sell: Option<f64>) -> CopyActivityRow {
    use crate::trader::copy::{CopyOutcome, CopySellDecision, CopyTelemetry, PaperDecision};
    use crate::trader::paper::{PaperFill, PaperQuoteMetadata, PaperSellFill};
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
                    model: crate::trader::paper::PaperModel::Quote,
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
                    model: crate::trader::paper::PaperModel::Quote,
                    quote: Some(quote(
                        crate::chains::RawAmount::from((proceeds * 1_000_000_000.0) as u64),
                        crate::chains::RawAmount::from(100_000_000_u64),
                    )),
                }),
                exit_rule: None,
                take_profit_level: None,
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
        earlier_run: false,
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
    let rounds = TaskRounds {
        positions: vec![
            closed(
                open_round(3, Book::Paper, "won", 0.01, 1.0, 30),
                0.4,
                TradeReason::CopySell,
                now,
            ),
            closed(
                open_round(3, Book::Paper, "lost", 0.01, 1.0, 20),
                -0.3,
                TradeReason::StopLoss,
                now,
            ),
            open_round(3, Book::Paper, "open", 0.01, 1.0, 10),
        ],
        run_start: Some(now - chrono::Duration::hours(1)),
    };
    let (stats, closed_rounds) = book_stats(&stored(), &activity, &rounds);
    assert_eq!(closed_rounds.len(), 2);
    assert_eq!((stats.wins, stats.losses), (1, 1));
    assert_eq!(stats.filled_buys, 3);
    assert_eq!(stats.target_sells, 2);
    assert_eq!((stats.open_positions, stats.policy_exits), (1, 1));
}

#[test]
fn decisions_before_the_current_paper_run_do_not_count() {
    let now = Utc::now();
    let start = now - chrono::Duration::minutes(5);
    let activity = [
        paper_row(1, "earlier", start - chrono::Duration::minutes(1), None),
        paper_row(2, "current", now, None),
    ];
    let rounds = TaskRounds {
        positions: Vec::new(),
        run_start: Some(start),
    };
    let (stats, _) = book_stats(&stored(), &activity, &rounds);
    assert_eq!(stats.filled_buys, 1);
    assert_eq!(stats.decisions, 1);
}

#[test]
fn legacy_paper_rows_do_not_enter_current_task_totals() {
    let now = Utc::now();
    let mut buy = paper_row(1, "legacy", now, None);
    if let CopyOutcome::PaperFilled(decision) = &mut buy.outcome {
        decision.fill.model = crate::trader::paper::PaperModel::Legacy;
        decision.fill.quote = None;
    }
    let mut sell = paper_row(2, "legacy", now, Some(2.0));
    if let CopyOutcome::PaperSellObserved(decision) = &mut sell.outcome {
        let fill = decision.paper_fill.as_mut().unwrap();
        fill.model = crate::trader::paper::PaperModel::Legacy;
        fill.quote = None;
    }
    let activity = [buy, sell];

    let (stats, rounds) = book_stats(&stored(), &activity, &TaskRounds::default());
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

fn copy_status(
    enabled: bool,
    blocked_reason: Option<&'static str>,
    active_tasks: usize,
) -> CopyTradingStatus {
    CopyTradingStatus {
        enabled,
        live_available: true,
        blocked_reason,
        default_mode: "paper".to_owned(),
        default_slippage_pct: 1.0,
        force_stop_blocks: true,
        total_tasks: active_tasks + 1,
        active_tasks,
        paper_tasks: active_tasks,
        live_tasks: 0,
    }
}

/// Copying needs the system on, no force stop and an enabled task; a loss-limit block
/// still copies exits.
#[test]
fn copying_needs_the_system_on_an_enabled_task_and_no_force_stop() {
    assert!(copy_status(true, None, 1).is_copying());
    assert!(copy_status(true, Some("loss_limit"), 1).is_copying());
    assert!(!copy_status(false, None, 1).is_copying());
    assert!(!copy_status(true, Some("force_stop"), 1).is_copying());
    assert!(!copy_status(true, None, 0).is_copying());
}

/// Two tasks on one wallet follow one wallet.
#[test]
fn followed_wallets_counts_distinct_targets() {
    let mut second = stored();
    second.id = 4;
    second.enabled = false;
    let mut other = stored();
    other.id = 5;
    other.target_address = "other".to_owned();
    assert_eq!(followed_wallets(&[]), 0);
    assert_eq!(followed_wallets(&[stored(), second, other]), 2);
}
