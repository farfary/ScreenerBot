// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests the task comparison row - a task's own label and its full target address reach the dashboard unshortened.

use super::*;
use crate::trader::copy::{ExitMode, SizingMode};

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
