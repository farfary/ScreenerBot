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
}
