// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Copy task type validation and compatibility tests.

use super::*;

#[test]
fn watch_budget_pause_exposes_a_distinct_dashboard_reason() {
    let reason = CopyPauseReason::WatchBudgetExceeded {
        page_budget: 8,
        signatures_checked: 800,
    };
    assert_eq!(
        serde_json::to_value(reason).unwrap(),
        serde_json::json!({
            "kind": "watch_budget_exceeded",
            "page_budget": 8,
            "signatures_checked": 800
        })
    );
}

#[test]
fn helius_pause_exposes_a_distinct_dashboard_reason() {
    assert_eq!(
        serde_json::to_value(CopyPauseReason::HeliusUnavailable).unwrap(),
        serde_json::json!({"kind": "helius_unavailable"})
    );
}

#[test]
fn processing_pause_exposes_a_distinct_dashboard_reason() {
    assert_eq!(
        serde_json::to_value(CopyPauseReason::WatchProcessingFailed).unwrap(),
        serde_json::json!({"kind": "watch_processing_failed"})
    );
}

fn input() -> CopyTaskInput {
    CopyTaskInput {
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.05 },
        exit_mode: ExitMode::BuyOnly,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 0.1,
        max_native_per_token: 0.5,
        total_budget_native: 2.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: true,
        slippage_pct: 2.0,
        require_filter_pass: None,
        copy_stable_quoted: None,
    }
}

fn validate(input: CopyTaskInput) -> Result<CopyTask, CopySkip> {
    input.into_task(ChainId::Solana, Utc::now())
}

#[test]
fn a_task_that_could_never_place_a_copy_is_refused() {
    let minimum_native = crate::trader::constants::MIN_TRADE_SIZE_NATIVE;
    assert!(validate(input()).is_ok());
    let tiny_size = CopyTaskInput {
        sizing: SizingMode::Fixed { sol: 0.0005 },
        ..input()
    };
    assert_eq!(
        validate(tiny_size),
        Err(CopySkip::BelowMinimumSize {
            minimum_sol: minimum_native
        })
    );
    let tiny_cap = CopyTaskInput {
        sizing: SizingMode::RatioOfTarget { pct: 10.0 },
        max_native_per_trade: 0.0005,
        ..input()
    };
    assert_eq!(
        validate(tiny_cap),
        Err(CopySkip::BelowMinimumSize {
            minimum_sol: minimum_native
        })
    );
}

#[test]
fn slippage_below_the_copy_minimum_is_refused() {
    let low = CopyTaskInput {
        slippage_pct: MIN_COPY_SLIPPAGE_PCT / 2.0,
        ..input()
    };
    assert!(matches!(
        validate(low),
        Err(CopySkip::InvalidSlippage { .. })
    ));
    let lowest = CopyTaskInput {
        slippage_pct: MIN_COPY_SLIPPAGE_PCT,
        ..input()
    };
    assert!(validate(lowest).is_ok());
}

#[test]
fn legacy_skip_without_telemetry_still_decodes() {
    let json = r#"{
            "outcome":"skipped",
            "task_id":1,
            "signature":"target-signature",
            "mint":"mint",
            "reason":{"kind":"task_disabled"},
            "decided_at":"2026-01-01T00:00:00Z"
        }"#;
    let outcome: CopyOutcome = serde_json::from_str(json).unwrap();
    let CopyOutcome::Skipped { telemetry, .. } = outcome else {
        panic!("expected skipped outcome")
    };
    assert_eq!(telemetry, None);
}

#[test]
fn quote_metadata_keeps_old_json_and_round_trips_raw_values_above_f64_precision() {
    let old = serde_json::json!({
        "router_id": "fixture",
        "initial_at": "2026-01-01T00:00:00Z",
        "landing_at": "2026-01-01T00:00:01Z",
        "requested_delay_ms": 0,
        "queue_duration_ms": 0,
        "minimum_output_amount": "1",
        "final_output_amount": "9007199254741010",
        "epoch": 0,
        "network_fee_estimated": true,
        "platform_fee_included_in_output": true
    });
    let legacy: PaperQuoteMetadata = serde_json::from_value(old).unwrap();
    assert_eq!(legacy.input_amount, None);
    assert_eq!(legacy.token_decimals, None);
    assert_eq!(legacy.final_output_amount.to_string(), "9007199254741010");
    let encoded = serde_json::to_value(&legacy).unwrap();
    assert!(encoded.get("input_amount").is_none());
    assert!(encoded.get("token_decimals").is_none());
}

/// Catalog key of the label for each skip kind. The match is exhaustive, so a
/// new variant fails to compile until it is mapped here and in `SKIP_LABELS`
/// (pages/copy/format.js).
fn skip_label_key(skip: &CopySkip) -> &'static str {
    match skip {
        CopySkip::NotBuySwap => "copy-skip-not-buy-swap",
        CopySkip::NotSellSwap => "copy-skip-not-sell-swap",
        CopySkip::ExitModeDisabled => "copy-skip-exit-mode-disabled",
        CopySkip::ForceStopped => "copy-skip-force-stopped",
        CopySkip::LatencyKillSwitch { .. } => "copy-skip-latency-kill-switch",
        CopySkip::ClaimReconciledAbandoned => "copy-skip-claim-reconciled-abandoned",
        CopySkip::CopyPositionNotFound => "copy-skip-copy-position-not-found",
        CopySkip::PositionUserOnly => "copy-skip-position-user-only",
        CopySkip::PositionManagementMismatch => "copy-skip-position-management-mismatch",
        CopySkip::TaskDisabled => "copy-skip-task-disabled",
        CopySkip::ModeTransitionRequired => "copy-skip-mode-transition-required",
        CopySkip::LiveConfirmationRequired => "copy-skip-live-confirmation-required",
        CopySkip::UnsupportedSizingMode => "copy-skip-unsupported-sizing-mode",
        CopySkip::SelfCopy => "copy-skip-self-copy",
        CopySkip::TargetBelowMinimum { .. } => "copy-skip-target-below-minimum",
        CopySkip::TargetAboveMaximum { .. } => "copy-skip-target-above-maximum",
        CopySkip::AlreadyBought => "copy-skip-already-bought",
        CopySkip::Blacklisted => "copy-skip-blacklisted",
        CopySkip::FilterRequired => "copy-skip-filter-required",
        CopySkip::EntryBlocked { .. } => "copy-skip-entry-blocked",
        CopySkip::BudgetExhausted => "copy-skip-budget-exhausted",
        CopySkip::TokenCapReached => "copy-skip-token-cap-reached",
        CopySkip::BelowMinimumSize { .. } => "copy-skip-below-minimum-size",
        CopySkip::InvalidSizing => "copy-skip-invalid-sizing",
        CopySkip::InvalidSlippage { .. } => "copy-skip-invalid-slippage",
        CopySkip::InvalidExitPolicy => "copy-skip-invalid-exit-policy",
        CopySkip::InvalidPrice => "copy-skip-invalid-price",
        CopySkip::StaleObservation { .. } => "copy-skip-stale-observation",
        CopySkip::UnknownObservationTime => "copy-skip-unknown-observation-time",
        CopySkip::PaperQuoteUnavailable => "copy-skip-paper-quote-unavailable",
        CopySkip::PaperQuoteUnfillable => "copy-skip-paper-quote-unfillable",
        CopySkip::PaperMinimumOutputNotMet => "copy-skip-paper-minimum-output-not-met",
        CopySkip::PaperQueueFull => "copy-skip-paper-queue-full",
        CopySkip::PaperExecutionUnavailable => "copy-skip-paper-execution-unavailable",
        CopySkip::StableQuoteDisabled => "copy-skip-stable-quote-disabled",
        CopySkip::StableQuotePriceUnavailable => "copy-skip-stable-quote-price-unavailable",
        CopySkip::SwapMixedQuote => "copy-skip-swap-mixed-quote",
        CopySkip::SwapMultipleTokens => "copy-skip-swap-multiple-tokens",
        CopySkip::SwapUnreadable => "copy-skip-swap-unreadable",
    }
}

#[test]
fn copy_skip_labels_exist_in_the_catalog() {
    let skips = [
        CopySkip::NotBuySwap,
        CopySkip::NotSellSwap,
        CopySkip::ExitModeDisabled,
        CopySkip::ForceStopped,
        CopySkip::LatencyKillSwitch {
            average_ms: 0,
            threshold_ms: 0,
        },
        CopySkip::ClaimReconciledAbandoned,
        CopySkip::CopyPositionNotFound,
        CopySkip::PositionUserOnly,
        CopySkip::PositionManagementMismatch,
        CopySkip::TaskDisabled,
        CopySkip::ModeTransitionRequired,
        CopySkip::LiveConfirmationRequired,
        CopySkip::UnsupportedSizingMode,
        CopySkip::SelfCopy,
        CopySkip::TargetBelowMinimum { minimum_sol: 0.0 },
        CopySkip::TargetAboveMaximum { maximum_sol: 0.0 },
        CopySkip::AlreadyBought,
        CopySkip::Blacklisted,
        CopySkip::FilterRequired,
        CopySkip::EntryBlocked {
            block: crate::trader::admission::EntryBlock::ForceStopped,
        },
        CopySkip::BudgetExhausted,
        CopySkip::TokenCapReached,
        CopySkip::BelowMinimumSize { minimum_sol: 0.0 },
        CopySkip::InvalidSizing,
        CopySkip::InvalidSlippage { maximum_pct: 0.0 },
        CopySkip::InvalidExitPolicy,
        CopySkip::InvalidPrice,
        CopySkip::StaleObservation {
            arrival_ms: 0,
            threshold_ms: 0,
        },
        CopySkip::UnknownObservationTime,
        CopySkip::PaperQuoteUnavailable,
        CopySkip::PaperQuoteUnfillable,
        CopySkip::PaperMinimumOutputNotMet,
        CopySkip::PaperQueueFull,
        CopySkip::PaperExecutionUnavailable,
        CopySkip::StableQuoteDisabled,
        CopySkip::StableQuotePriceUnavailable,
        CopySkip::SwapMixedQuote,
        CopySkip::SwapMultipleTokens,
        CopySkip::SwapUnreadable,
    ];
    for skip in &skips {
        let value = serde_json::to_value(skip).unwrap();
        let id = value["kind"].as_str().expect("skip kind");
        let key = skip_label_key(skip);
        assert_eq!(
            key,
            format!("copy-skip-{}", id.replace('_', "-")),
            "key does not follow the serialized id {id}"
        );
        assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
    }
}
