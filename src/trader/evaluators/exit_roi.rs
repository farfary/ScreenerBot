// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Return on Investment (ROI) based exit strategy.

use crate::positions::Position;
use crate::trader::evaluators::exit_rules::{ExitRule, RuleOutcome};
use crate::trader::policy::RoiPolicy;
use crate::trader::types::TradeDecision;

/// The ROI-target rule on plain numbers, shared by the live evaluator and the
/// paper copy book.
pub fn roi_target_reached(entry_price: f64, current_price: f64, policy: &RoiPolicy) -> bool {
    policy.enabled
        && entry_price.is_finite()
        && entry_price > 0.0
        && (current_price / entry_price - 1.0) * 100.0 >= policy.target_profit_pct
}

/// Check if a position should be exited based on ROI target
pub async fn check_roi_exit(
    position: &Position,
    current_price: f64,
    policy: &RoiPolicy,
) -> crate::trader::Result<Option<TradeDecision>> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(crate::positions::Error::InvalidPrice {
            mint: position.mint.clone(),
            price: current_price,
        }
        .into());
    }
    if !roi_target_reached(position.average_entry_price, current_price, policy) {
        return Ok(None);
    }
    Ok(Some(super::exit::rule_decision(
        position,
        current_price,
        &RuleOutcome {
            rule: ExitRule::TakeProfit,
            exit_percentage: None,
        },
    )))
}
