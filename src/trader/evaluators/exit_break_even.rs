// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The break-even stop: once a holding has run far enough into profit, never let it fall back below its entry.

use crate::trader::policy::BreakEvenPolicy;

/// The break-even rule on plain numbers, shared by the live evaluator and the paper copy
/// book. It arms once the best price since the entry it is measured from (a live
/// position's basis peak) is `trigger_pct` or more above `entry_price`, and then fires
/// when the current price is at or below `entry_price × (1 + offset_pct / 100)`.
///
/// Unlike the trailing stop it also fires below the entry: a price that gapped through
/// the floor into a loss still sells, because the floor is the one thing this rule
/// promises. A price that is not finite and positive fires nothing. `Err` carries the
/// detail of an impossible configuration: a margin that is not below the arming profit
/// would sell the moment the stop arms.
pub fn break_even_triggered(
    entry_price: f64,
    peak_price: f64,
    current_price: f64,
    policy: &BreakEvenPolicy,
) -> crate::trader::Result<bool> {
    if !policy.enabled {
        return Ok(false);
    }
    let (trigger_pct, offset_pct) = (policy.trigger_pct, policy.offset_pct);
    if !trigger_pct.is_finite()
        || !offset_pct.is_finite()
        || trigger_pct <= 0.0
        || offset_pct < 0.0
        || offset_pct >= trigger_pct
    {
        return Err(crate::trader::Error::InvalidExitPolicy {
            detail: format!(
                "invalid break-even stop config: offset_pct ({offset_pct}%) must be at least 0 and less than trigger_pct ({trigger_pct}%)"
            ),
        });
    }
    let usable = |price: f64| price.is_finite() && price > 0.0;
    if !usable(entry_price) || !usable(peak_price) || !usable(current_price) {
        return Ok(false);
    }
    // Both thresholds are compared as prices, so a peak or mark exactly on a configured
    // percentage is on it rather than a rounding error short of it.
    if peak_price < entry_price * (1.0 + trigger_pct / 100.0) {
        return Ok(false);
    }
    Ok(current_price <= entry_price * (1.0 + offset_pct / 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arms at +15%, keeps +1% above the entry.
    fn policy() -> BreakEvenPolicy {
        BreakEvenPolicy {
            enabled: true,
            trigger_pct: 15.0,
            offset_pct: 1.0,
        }
    }

    #[test]
    fn it_arms_on_the_peak_and_fires_at_the_kept_margin() {
        let policy = policy();
        assert!(break_even_triggered(1.0, 1.15, 1.01, &policy).unwrap());
        assert!(break_even_triggered(1.0, 1.30, 1.005, &policy).unwrap());
        assert!(
            !break_even_triggered(1.0, 1.15, 1.02, &policy).unwrap(),
            "above the kept margin it holds"
        );
        assert!(
            !break_even_triggered(1.0, 1.14, 1.0, &policy).unwrap(),
            "a peak under the arming profit never arms it"
        );
    }

    #[test]
    fn an_armed_stop_fires_below_the_entry() {
        assert!(break_even_triggered(1.0, 1.2, 0.9, &policy()).unwrap());
    }

    #[test]
    fn a_zero_margin_fires_at_the_entry() {
        let policy = BreakEvenPolicy {
            offset_pct: 0.0,
            ..policy()
        };
        assert!(break_even_triggered(1.0, 1.2, 1.0, &policy).unwrap());
        assert!(!break_even_triggered(1.0, 1.2, 1.0001, &policy).unwrap());
    }

    #[test]
    fn a_disabled_stop_never_fires() {
        let policy = BreakEvenPolicy {
            enabled: false,
            ..policy()
        };
        assert!(!break_even_triggered(1.0, 2.0, 0.5, &policy).unwrap());
    }

    #[test]
    fn unusable_prices_fire_nothing() {
        let policy = policy();
        for (entry, peak, current) in [
            (0.0, 1.2, 0.9),
            (f64::NAN, 1.2, 0.9),
            (1.0, 0.0, 0.9),
            (1.0, f64::INFINITY, 0.9),
            (1.0, 1.2, 0.0),
            (1.0, 1.2, -1.0),
            (1.0, 1.2, f64::NAN),
        ] {
            assert!(
                !break_even_triggered(entry, peak, current, &policy).unwrap(),
                "{entry} {peak} {current}"
            );
        }
    }

    #[test]
    fn an_impossible_configuration_is_an_error() {
        for (trigger_pct, offset_pct) in [
            (15.0, 15.0),
            (15.0, 20.0),
            (15.0, -1.0),
            (0.0, 0.0),
            (f64::NAN, 1.0),
            (15.0, f64::INFINITY),
        ] {
            let policy = BreakEvenPolicy {
                enabled: true,
                trigger_pct,
                offset_pct,
            };
            assert!(
                break_even_triggered(1.0, 1.2, 0.9, &policy).is_err(),
                "{trigger_pct}/{offset_pct}"
            );
        }
    }
}
