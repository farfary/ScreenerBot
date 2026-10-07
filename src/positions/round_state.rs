// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure arithmetic over a round's raw token amounts, free of I/O and of any chain.

use crate::chains::RawAmount;

/// One part in this many of what was acquired is dust.
const DUST_DIVISOR: u128 = 1_000;

/// Raw units that are dust whatever was acquired.
const DUST_FLOOR_RAW: u128 = 10;

/// True when `held` is at most one thousandth of `acquired`, or at most ten raw units: a
/// balance this small is leftover, not a holding.
pub fn is_dust(held: RawAmount, acquired: RawAmount) -> bool {
    held.raw() <= (acquired.raw() / DUST_DIVISOR).max(DUST_FLOOR_RAW)
}

/// The part of the wallet's holding a round can own: what the other open rounds of the mint
/// do not hold, never more than the round acquired.
pub fn attributable_held(
    wallet_held: RawAmount,
    held_by_other_open_rows: RawAmount,
    acquired: RawAmount,
) -> RawAmount {
    wallet_held
        .checked_sub(held_by_other_open_rows)
        .unwrap_or(RawAmount::ZERO)
        .min(acquired)
}

/// The raw amount a buy of `entry_size_native` at `entry_price` (native per whole token)
/// acquires, the inverse of the entry price. `None` without the token's decimals or for a
/// size or price that is not a positive finite number.
pub fn expected_acquisition(
    entry_size_native: f64,
    entry_price: f64,
    decimals: Option<u8>,
) -> Option<RawAmount> {
    let decimals = decimals?;
    if !(entry_size_native.is_finite()
        && entry_size_native > 0.0
        && entry_price.is_finite()
        && entry_price > 0.0)
    {
        return None;
    }
    let raw = (entry_size_native / entry_price * 10_f64.powi(i32::from(decimals))).floor();
    RawAmount::from_integral_f64(raw)
}

/// Whether the wallet's holding attributable to a round that expects to acquire `expected`
/// is dust. `None` when the expected acquisition is unknown, or is itself within the dust
/// floor: the attributable holding is capped at `expected`, so it would read as dust whatever
/// the wallet holds.
pub fn attributable_is_dust(
    wallet_held: RawAmount,
    held_by_other_open_rows: RawAmount,
    expected: Option<RawAmount>,
) -> Option<bool> {
    let expected = expected.filter(|expected| expected.raw() > DUST_FLOOR_RAW)?;
    Some(is_dust(
        attributable_held(wallet_held, held_by_other_open_rows, expected),
        expected,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dust(held: u128, acquired: u128) -> bool {
        is_dust(RawAmount::new(held), RawAmount::new(acquired))
    }

    #[test]
    fn one_thousandth_of_the_acquired_amount_is_dust_and_one_unit_more_is_not() {
        assert!(dust(1_000, 1_000_000));
        assert!(!dust(1_001, 1_000_000));
    }

    #[test]
    fn ten_raw_units_are_dust_whatever_was_acquired() {
        assert!(dust(0, 0));
        assert!(dust(10, 0));
        assert!(dust(10, 5_000));
        assert!(!dust(11, 5_000));
    }

    #[test]
    fn the_threshold_holds_at_the_top_of_the_amount_range() {
        assert!(dust(u128::MAX / 1_000, u128::MAX));
        assert!(!dust(u128::MAX / 1_000 + 1, u128::MAX));
    }

    fn raw(value: u128) -> RawAmount {
        RawAmount::new(value)
    }

    #[test]
    fn the_holding_of_other_open_rounds_is_not_attributable() {
        assert_eq!(
            attributable_held(raw(1_500), raw(1_000), raw(2_000)),
            raw(500)
        );
        assert_eq!(attributable_held(raw(900), raw(1_000), raw(2_000)), raw(0));
    }

    #[test]
    fn a_round_is_never_attributed_more_than_it_acquired() {
        assert_eq!(
            attributable_held(raw(5_000), raw(0), raw(2_000)),
            raw(2_000)
        );
    }

    #[test]
    fn the_expected_acquisition_inverts_the_entry_price() {
        assert_eq!(
            expected_acquisition(0.5, 0.0625, Some(6)),
            Some(raw(8_000_000))
        );
        assert_eq!(expected_acquisition(1.0, 0.5, Some(0)), Some(raw(2)));
    }

    #[test]
    fn the_expected_acquisition_is_unknown_without_decimals_or_a_valid_quote() {
        assert_eq!(expected_acquisition(0.2, 0.004, None), None);
        for (size, price) in [
            (0.0, 0.004),
            (-0.2, 0.004),
            (0.2, 0.0),
            (f64::NAN, 0.004),
            (0.2, f64::INFINITY),
        ] {
            assert_eq!(expected_acquisition(size, price, Some(6)), None);
        }
    }

    #[test]
    fn a_residue_up_to_a_thousandth_of_the_expected_acquisition_is_attributable_dust() {
        let expected = Some(raw(50_000_000));
        assert_eq!(
            attributable_is_dust(raw(50_000), raw(0), expected),
            Some(true)
        );
        assert_eq!(
            attributable_is_dust(raw(50_001), raw(0), expected),
            Some(false)
        );
    }

    #[test]
    fn the_holding_of_other_open_rounds_is_subtracted_before_the_dust_test() {
        let expected = Some(raw(50_000_000));
        assert_eq!(
            attributable_is_dust(raw(10_050_000), raw(10_000_000), expected),
            Some(true)
        );
        assert_eq!(
            attributable_is_dust(raw(10_050_001), raw(10_000_000), expected),
            Some(false)
        );
    }

    #[test]
    fn an_unknown_expected_acquisition_decides_nothing() {
        assert_eq!(attributable_is_dust(raw(0), raw(0), None), None);
        assert_eq!(
            attributable_is_dust(raw(0), raw(0), expected_acquisition(0.2, 0.004, None)),
            None
        );
    }

    #[test]
    fn an_expected_acquisition_within_the_dust_floor_decides_nothing() {
        for expected in [0, 1, DUST_FLOOR_RAW] {
            for held in [0, 1, DUST_FLOOR_RAW, 1_000_000] {
                assert_eq!(
                    attributable_is_dust(raw(held), raw(0), Some(raw(expected))),
                    None,
                    "held {held} against expected {expected}"
                );
            }
        }
        let just_above = Some(raw(DUST_FLOOR_RAW + 1));
        assert_eq!(attributable_is_dust(raw(0), raw(0), just_above), Some(true));
        assert_eq!(
            attributable_is_dust(raw(DUST_FLOOR_RAW + 1), raw(0), just_above),
            Some(false)
        );
    }
}
