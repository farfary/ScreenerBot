// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trailing stop loss implementation.

use crate::trader::policy::TrailingPolicy;

/// The trailing-stop rule on plain numbers, shared by the live evaluator and the
/// paper copy book. `Err` carries the detail of an impossible configuration.
///
/// ARM THE TRAIL OFF THE PEAK, NEVER OFF THE CURRENT PRICE. A trailing stop exists
/// to protect a profit the position has ALREADY made, so what arms it is how far the
/// price has run at its best since `entry_price` became the entry it is measured from
/// (a live position's basis peak, restarted by every buy that moves the average).
/// Measuring activation against the CURRENT price makes the trail un-arm itself as
/// the price falls -- exactly when it is supposed to act. With activation 20% and
/// distance 10%, a position that peaked at +25% and is retracing through +12% is
/// below the stop (peak * 0.9 = +12.5%) but under the activation on its CURRENT
/// profit, so the trail stayed silent and rode down to the stop loss.
///
/// The trail follows the distance in force at the peak: the band with the highest
/// threshold the peak profit has reached, else the base distance.
///
/// It fires only while the position is still in profit, so a retracement never
/// turns the trail into a loss exit.
pub fn trailing_stop_triggered(
    entry_price: f64,
    peak_price: f64,
    current_price: f64,
    policy: &TrailingPolicy,
) -> crate::trader::Result<bool> {
    if peak_price <= 0.0 || !policy.enabled {
        return Ok(false);
    }
    policy.validate()?;
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return Ok(false);
    }
    let peak_profit_pct = (peak_price / entry_price - 1.0) * 100.0;
    if peak_profit_pct < policy.activation_pct {
        return Ok(false);
    }
    let stop_price = peak_price * (1.0 - policy.distance_in_force(peak_profit_pct) / 100.0);
    let current_profit_pct = (current_price / entry_price - 1.0) * 100.0;
    Ok(current_price <= stop_price && current_profit_pct > 0.0)
}
