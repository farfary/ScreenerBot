// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Time-based exit override.

use crate::trader::policy::TimePolicy;

/// The time-override rule on plain numbers, shared by the live evaluator and the
/// paper copy book: exit a position held at least `duration_seconds` whose P&L is
/// at or below `loss_threshold_pct` (negative, e.g. -40 = a 40% loss or worse).
/// `Err` carries the detail of an invalid configuration.
pub fn time_override_triggered(
    entry_price: f64,
    current_price: f64,
    held_seconds: i64,
    policy: &TimePolicy,
) -> crate::trader::Result<bool> {
    if !policy.enabled {
        return Ok(false);
    }
    policy.validate()?;
    let loss_threshold_pct = policy.loss_threshold_pct;
    let duration_seconds = policy.duration_seconds;
    if (held_seconds as f64) < duration_seconds || entry_price <= 0.0 || !entry_price.is_finite() {
        return Ok(false);
    }
    let pnl_pct = ((current_price - entry_price) / entry_price) * 100.0;
    Ok(pnl_pct <= loss_threshold_pct)
}
