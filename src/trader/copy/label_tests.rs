//! Every id the dashboard renders through a frozen label map has catalog text
//! named after it. The maps are `STATE_LABELS`, `MODE_LABELS`, `EXIT_MODE_LABELS`
//! and `EXIT_LABELS` (pages/copy/format.js), `STATE_DETAIL_LABELS`
//! (pages/copy/workspace.js) and `OUTCOME_LABELS` (pages/copy/activity.js).

use std::collections::BTreeSet;

use chrono::Utc;
use serde::Serialize;
use serde_json::json;

use super::control::{effective_state, CopyTradingStatus};
use super::insights::exit_label;
use super::*;
use crate::i18n::format_en;

fn assert_text(key: &str) {
    assert_ne!(format_en(key, None), key, "missing catalog text {key}");
}

fn kebab(id: &str) -> String {
    id.replace('_', "-")
}

fn serialized(value: &impl Serialize, tag: &str) -> String {
    let value = serde_json::to_value(value).unwrap();
    match value {
        serde_json::Value::String(id) => id,
        other => other[tag].as_str().unwrap().to_owned(),
    }
}

#[test]
fn every_execution_mode_has_a_label() {
    let id = |mode: CopyMode| match mode {
        CopyMode::Paper => "paper",
        CopyMode::Live => "live",
    };
    for mode in [CopyMode::Paper, CopyMode::Live] {
        assert_eq!(serialized(&mode, ""), id(mode));
        assert_text(&format!("copy-mode-{}", id(mode)));
    }
}

#[test]
fn every_exit_mode_has_a_label() {
    let id = |mode: ExitMode| match mode {
        ExitMode::BuyOnly => "buy_only",
        ExitMode::Mirror => "mirror",
        ExitMode::Hybrid => "hybrid",
    };
    for mode in [ExitMode::BuyOnly, ExitMode::Mirror, ExitMode::Hybrid] {
        assert_eq!(serialized(&mode, ""), id(mode));
        assert_text(&format!("copy-exit-mode-{}", kebab(id(mode))));
    }
}

#[test]
fn every_exit_bucket_has_a_label() {
    let listed = |rule: PaperExitRule| match rule {
        PaperExitRule::StopLoss
        | PaperExitRule::TrailingStop
        | PaperExitRule::TakeProfit
        | PaperExitRule::TimeOverride
        | PaperExitRule::Manual => Some(rule),
    };
    let rules = [
        PaperExitRule::StopLoss,
        PaperExitRule::TrailingStop,
        PaperExitRule::TakeProfit,
        PaperExitRule::TimeOverride,
        PaperExitRule::Manual,
    ];
    let mut ids = vec![exit_label(None)];
    ids.extend(rules.into_iter().filter_map(listed).map(|rule| {
        let id = exit_label(Some(rule));
        assert_eq!(id, serialized(&rule, ""));
        id
    }));
    assert_eq!(ids.len(), 6);
    for id in ids {
        assert_text(&format!("copy-exit-{}", kebab(&id)));
    }
}

fn task(enabled: bool, mode: CopyMode) -> CopyTask {
    CopyTask {
        id: 1,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: None,
        enabled,
        mode,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_sol_per_trade: 0.2,
        max_sol_per_token: 1.0,
        total_budget_sol: 5.0,
        min_target_trade_sol: None,
        max_target_trade_sol: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        pause_reason: None,
        paused_at: None,
    }
}

fn status(enabled: bool, blocked_reason: Option<&'static str>) -> CopyTradingStatus {
    CopyTradingStatus {
        enabled,
        live_available: true,
        blocked_reason,
        default_mode: "paper".to_owned(),
        default_slippage_pct: 1.0,
        force_stop_blocks: true,
        total_tasks: 1,
        active_tasks: 1,
        paper_tasks: 1,
        live_tasks: 0,
    }
}

#[test]
fn every_effective_state_has_a_label_and_a_running_sentence() {
    let states: BTreeSet<&str> = [
        effective_state(&status(false, None), &task(true, CopyMode::Paper)),
        effective_state(
            &status(true, Some("force_stop")),
            &task(true, CopyMode::Paper),
        ),
        effective_state(&status(true, None), &task(false, CopyMode::Paper)),
        effective_state(
            &status(true, Some("loss_limit")),
            &task(true, CopyMode::Paper),
        ),
        effective_state(&status(true, None), &task(true, CopyMode::Live)),
        effective_state(&status(true, None), &task(true, CopyMode::Paper)),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        states,
        BTreeSet::from([
            "entries_blocked",
            "force_stopped",
            "live",
            "paper",
            "paused",
            "system_paused"
        ])
    );
    for state in states {
        match state {
            "live" | "paper" => assert_text(&format!("copy-state-running-{state}")),
            other => assert_text(&format!("copy-state-{}", kebab(other))),
        }
        if state != "paused" {
            assert_text(&format!("copy-state-detail-{}", kebab(state)));
        }
    }
}

fn telemetry() -> serde_json::Value {
    let at = Utc::now().to_rfc3339();
    json!({ "detected_at": at, "decoded_at": at, "decided_at": at })
}

fn paper_decision() -> serde_json::Value {
    json!({
        "task_id": 1, "target_address": "t", "signature": "s", "mint": "m",
        "target_size_sol": 0.1, "sized_sol": 0.05,
        "fill": {
            "input_sol": 0.05, "market_price_sol": 1.0, "fill_price_sol": 1.0,
            "token_amount": 1.0, "referral_fee_sol": 0.0, "network_fee_sol": 0.0,
            "priority_fee_sol": 0.0, "total_cost_sol": 0.05
        },
        "telemetry": telemetry(),
    })
}

fn live_decision() -> serde_json::Value {
    json!({
        "task_id": 1, "target_address": "t", "target_signature": "s", "mint": "m",
        "target_size_sol": 0.1, "sized_sol": 0.05, "telemetry": telemetry(),
    })
}

fn sell_decision() -> serde_json::Value {
    json!({
        "task_id": 1, "target_address": "t", "target_signature": "s", "mint": "m",
        "target_token_amount": 1.0, "target_sol_amount": 0.1, "telemetry": telemetry(),
    })
}

fn outcome(kind: &str, body: serde_json::Value) -> CopyOutcome {
    let mut value = body;
    value["outcome"] = json!(kind);
    serde_json::from_value(value).unwrap_or_else(|error| panic!("{kind}: {error}"))
}

#[test]
fn every_outcome_kind_has_a_label() {
    let id = |outcome: &CopyOutcome| match outcome {
        CopyOutcome::PaperFilled(_) => "paper_filled",
        CopyOutcome::LiveSubmitted(_) => "live_submitted",
        CopyOutcome::LiveConfirmed(_) => "live_confirmed",
        CopyOutcome::LiveFailed(_) => "live_failed",
        CopyOutcome::PaperSellObserved(_) => "paper_sell_observed",
        CopyOutcome::LiveSellSubmitted(_) => "live_sell_submitted",
        CopyOutcome::LiveSellFailed(_) => "live_sell_failed",
        CopyOutcome::Skipped { .. } => "skipped",
    };
    let skipped = json!({
        "task_id": 1, "signature": "s", "mint": null,
        "reason": { "kind": "not_buy_swap" }, "decided_at": Utc::now().to_rfc3339(),
    });
    let all = [
        outcome("paper_filled", paper_decision()),
        outcome("live_submitted", live_decision()),
        outcome("live_confirmed", live_decision()),
        outcome("live_failed", live_decision()),
        outcome("paper_sell_observed", sell_decision()),
        outcome("live_sell_submitted", sell_decision()),
        outcome("live_sell_failed", sell_decision()),
        outcome("skipped", skipped),
    ];
    for outcome in &all {
        assert_eq!(serialized(outcome, "outcome"), id(outcome));
        assert_text(&format!("copy-outcome-{}", kebab(id(outcome))));
    }
}
