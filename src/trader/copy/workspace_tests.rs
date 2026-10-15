// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests the copy workspace: task comparison rows, default settings, and legacy paper summaries. Manual paper closes are in `manual_close`.

use super::*;
use crate::trader::copy::{ExitMode, SizingMode};

#[path = "workspace_tests/manual_close.rs"]
mod manual_close;

const ADDRESS: &str = "CopyTarget1111111111111111111111111111111111";

fn task(label: Option<&str>) -> CopyTask {
    CopyTask {
        id: 7,
        chain: crate::chains::ChainId::Solana,
        target_address: ADDRESS.to_owned(),
        label: label.map(str::to_owned),
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::BuyOnly,
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
        pause_reason: None,
        paused_at: None,
    }
}

#[test]
fn an_unnamed_task_compares_by_its_full_address() {
    let row = comparison(&task(None), CopyInsights::default());
    assert_eq!(row.label, None);
    assert_eq!(row.target_address, ADDRESS);

    let json = serde_json::to_value(&row).expect("comparison serializes");
    assert!(json.get("name").is_none(), "no pre-shortened display name");
    assert_eq!(json["target_address"], ADDRESS);
}

#[test]
fn a_named_task_compares_by_its_label() {
    let row = comparison(&task(Some("Momentum desk")), CopyInsights::default());
    assert_eq!(row.label.as_deref(), Some("Momentum desk"));
    assert_eq!(row.target_address, ADDRESS);
}

#[test]
fn legacy_workspace_values_use_only_saved_paper_estimates() {
    let now = Utc::now();
    let position = PaperPosition {
        task_id: 7,
        mint: "legacy-mint".to_owned(),
        token_amount: 10.0,
        raw_token_amount: None,
        token_decimals: None,
        cost_basis_native: 2.0,
        invested_native: 2.5,
        realized_proceeds_native: 1.0,
        realized_cost_native: 0.5,
        buys: 2,
        sells: 1,
        last_price_native: Some(0.3),
        last_price_at: Some(now),
        opened_at: now,
        closed_at: None,
        peak_price_native: None,
    };

    let summary = legacy_paper_summary(&[position], 2.5);
    assert_eq!(summary.spent_native, 2.5);
    assert_eq!(summary.realized_pnl_native, 0.5);
    assert_eq!(summary.holdings[0].last_recorded_price_native, Some(0.3));
    assert_eq!(
        summary.holdings[0].remaining_pnl_at_last_recorded_price_native,
        Some(1.0)
    );
}

#[test]
fn copy_defaults_fee_rate_uses_the_chain_adapter_policy() {
    crate::config::utils::install_default_config();
    let defaults = super::defaults();
    assert_eq!(
        defaults.swap_fee_pct,
        f64::from(crate::chains::platform_swap_fee_bps()) / 100.0
    );
}
