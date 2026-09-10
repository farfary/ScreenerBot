use super::*;
use crate::trader::copy::{ExitMode, SizingMode};

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
        max_sol_per_trade: 0.2,
        max_sol_per_token: 1.0,
        total_budget_sol: 5.0,
        min_target_trade_sol: None,
        max_target_trade_sol: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn a_partial_patch_changes_only_the_fields_it_names() {
    let merged = merge_task_patch(&stored(), serde_json::json!({ "enabled": false })).unwrap();
    assert!(!merged.enabled);
    assert_eq!(merged.label.as_deref(), Some("Kept"));
    assert_eq!(merged.exit_mode, ExitMode::Mirror);
    assert_eq!(merged.total_budget_sol, 5.0);
}

#[test]
fn a_patch_with_an_unknown_or_mistyped_field_is_refused() {
    assert!(merge_task_patch(&stored(), serde_json::json!({ "enabld": false })).is_err());
    assert!(merge_task_patch(&stored(), serde_json::json!({ "enabled": "no" })).is_err());
    assert!(merge_task_patch(&stored(), serde_json::json!([1])).is_err());
}
