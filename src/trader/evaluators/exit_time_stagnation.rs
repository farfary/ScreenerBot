// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The stagnation exit: close a holding that has gone nowhere for too long.

use crate::trader::policy::TimeStagnationPolicy;

/// The stagnation rule on plain numbers, shared by the live evaluator and the paper copy
/// book: exit a holding held at least `duration_seconds` whose P&L from `entry_price` is
/// within `band_pct` either side of zero, both edges included. It frees capital stuck in
/// a position that neither wins nor loses; a holding outside the band is left to the
/// other rules. A negative holding time or a price that is not finite and positive fires
/// nothing. `Err` carries the detail of an invalid configuration.
pub fn stagnation_triggered(
    entry_price: f64,
    current_price: f64,
    held_seconds: i64,
    policy: &TimeStagnationPolicy,
) -> crate::trader::Result<bool> {
    if !policy.enabled {
        return Ok(false);
    }
    let (duration_seconds, band_pct) = (policy.duration_seconds, policy.band_pct);
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(crate::trader::Error::InvalidExitPolicy {
            detail: format!("invalid stagnation exit duration: {duration_seconds} seconds"),
        });
    }
    if !band_pct.is_finite() || band_pct <= 0.0 {
        return Err(crate::trader::Error::InvalidExitPolicy {
            detail: format!("invalid stagnation exit band: {band_pct}%"),
        });
    }
    let usable = |price: f64| price.is_finite() && price > 0.0;
    if held_seconds < 0 || !usable(entry_price) || !usable(current_price) {
        return Ok(false);
    }
    if (held_seconds as f64) < duration_seconds {
        return Ok(false);
    }
    // The band is compared as prices, so a mark exactly on its edge is inside it rather
    // than a rounding error outside.
    let floor = entry_price * (1.0 - band_pct / 100.0);
    let ceiling = entry_price * (1.0 + band_pct / 100.0);
    Ok((floor..=ceiling).contains(&current_price))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTES_240: i64 = 240 * 60;

    /// Four hours inside ±5%.
    fn policy() -> TimeStagnationPolicy {
        TimeStagnationPolicy {
            enabled: true,
            duration_seconds: MINUTES_240 as f64,
            band_pct: 5.0,
        }
    }

    #[test]
    fn it_fires_once_held_long_enough_inside_the_band() {
        let policy = policy();
        for current in [1.0, 1.05, 0.95, 1.02, 0.97] {
            assert!(
                stagnation_triggered(1.0, current, MINUTES_240, &policy).unwrap(),
                "{current}"
            );
        }
        assert!(
            !stagnation_triggered(1.0, 1.0, MINUTES_240 - 60, &policy).unwrap(),
            "a minute short of the holding time it waits"
        );
    }

    #[test]
    fn a_holding_outside_the_band_is_left_alone() {
        let policy = policy();
        for current in [1.0501, 0.9499, 1.5, 0.5] {
            assert!(
                !stagnation_triggered(1.0, current, MINUTES_240 * 2, &policy).unwrap(),
                "{current}"
            );
        }
    }

    #[test]
    fn a_disabled_rule_never_fires() {
        let policy = TimeStagnationPolicy {
            enabled: false,
            ..policy()
        };
        assert!(!stagnation_triggered(1.0, 1.0, MINUTES_240, &policy).unwrap());
    }

    #[test]
    fn unusable_inputs_fire_nothing() {
        let policy = policy();
        for (entry, current, held) in [
            (0.0, 1.0, MINUTES_240),
            (f64::NAN, 1.0, MINUTES_240),
            (1.0, 0.0, MINUTES_240),
            (1.0, f64::INFINITY, MINUTES_240),
            (1.0, 1.0, -1),
        ] {
            assert!(
                !stagnation_triggered(entry, current, held, &policy).unwrap(),
                "{entry} {current} {held}"
            );
        }
    }

    #[test]
    fn an_invalid_configuration_is_an_error() {
        for (duration_seconds, band_pct) in [
            (0.0, 5.0),
            (-60.0, 5.0),
            (f64::NAN, 5.0),
            (3600.0, 0.0),
            (3600.0, -1.0),
            (3600.0, f64::INFINITY),
        ] {
            let policy = TimeStagnationPolicy {
                enabled: true,
                duration_seconds,
                band_pct,
            };
            assert!(
                stagnation_triggered(1.0, 1.0, MINUTES_240, &policy).is_err(),
                "{duration_seconds}/{band_pct}"
            );
        }
    }
}
