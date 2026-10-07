// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure rules over a round's raw token amounts, free of I/O: what is dust, what part of a
//! holding a round owns, and how a closed position follows its round when a fill lands after
//! its close.

use crate::chains::RawAmount;

use super::pnl::realized_pnl;
use super::types::Position;
use super::{Error, Result};

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
/// is dust. `held_by_other_open_rows` is `None` while what the other open rounds hold is
/// unknown: a holding that is dust even with nothing subtracted is dust whatever they hold,
/// and any other holding may be the round's own, so it decides nothing. `None` also when the
/// expected acquisition is unknown, or is itself within the dust floor: the attributable
/// holding is capped at `expected`, so it would read as dust whatever the wallet holds.
pub fn attributable_is_dust(
    wallet_held: RawAmount,
    held_by_other_open_rows: Option<RawAmount>,
    expected: Option<RawAmount>,
) -> Option<bool> {
    let expected = expected.filter(|expected| expected.raw() > DUST_FLOOR_RAW)?;
    match held_by_other_open_rows {
        Some(others) => Some(is_dust(
            attributable_held(wallet_held, others, expected),
            expected,
        )),
        None => is_dust(
            attributable_held(wallet_held, RawAmount::ZERO, expected),
            expected,
        )
        .then_some(true),
    }
}

/// What [`follow_round`] did with a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FollowOutcome {
    /// The position is not closed; nothing follows.
    Unchanged,
    /// The closed position holds more than dust, so it is open again.
    Reopened,
    /// The closed position holds dust at most and stays closed, its realized P&L restated.
    StaysClosed,
}

/// True when `position` is closed: its exit time is set and its exit verified.
pub fn is_closed(position: &Position) -> bool {
    position.exit_time.is_some() && position.transaction_exit_verified
}

/// The full-exit signature of `position` whose sale still has to be verified and booked: an
/// exit not yet verified, or the sale in flight when the position was written off. The
/// write-off marks the exit verified without booking that sale, so the sale is verified as a
/// late fill; booking it turns the write-off into a real close, and a sale the chain proves
/// failed or never landed is dropped from the row, so neither qualifies again.
pub fn exit_awaiting_verification(position: &Position) -> Option<&str> {
    let signature = position.exit_transaction_signature.as_deref()?;
    (!position.transaction_exit_verified || position.synthetic_exit).then_some(signature)
}

/// Makes a closed position follow its round once a late fill's own leg is booked on it.
/// `held` is the wallet's holding attributable to the position (see [`attributable_held`]).
///
/// Above dust of what the position acquired, the position reopens: its closing state is
/// cleared, it holds `held` and the rest of what it acquired counts as exited. Every booked
/// leg stays, and the management, archive flag and price columns are never touched. At or
/// below dust it stays closed with nothing held and its P&L taken from the booked legs. An
/// open position is left unchanged.
pub fn follow_round(position: &mut Position, held: RawAmount) -> Result<FollowOutcome> {
    if !is_closed(position) {
        return Ok(FollowOutcome::Unchanged);
    }
    let Some(acquired) = position.acquired_amount() else {
        return Err(Error::AmountOverflow {
            mint: position.mint.clone(),
            operation: "following the round",
        });
    };

    if is_dust(held, acquired) {
        position.remaining_token_amount = Some(RawAmount::ZERO);
        position.total_exited_amount = acquired;
        let (pnl, pnl_percent) = realized_pnl(position);
        position.pnl = Some(pnl);
        position.pnl_percent = Some(pnl_percent);
        return Ok(FollowOutcome::StaysClosed);
    }

    // The sale that closed the position is now one of its partial exits; a write-off sold
    // nothing.
    if !position.synthetic_exit {
        position.partial_exit_count += 1;
    }
    let held = held.min(acquired);
    position.remaining_token_amount = Some(held);
    position.total_exited_amount = acquired.checked_sub(held).unwrap_or(RawAmount::ZERO);
    position.exit_time = None;
    position.exit_price = None;
    position.effective_exit_price = None;
    position.average_exit_price = None;
    position.pnl = None;
    position.pnl_percent = None;
    position.closed_reason = None;
    position.transaction_exit_verified = false;
    position.synthetic_exit = false;
    // A kept exit signature would be verified again as a full exit of the reopened position.
    position.exit_transaction_signature = None;
    Ok(FollowOutcome::Reopened)
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
            attributable_is_dust(raw(50_000), Some(raw(0)), expected),
            Some(true)
        );
        assert_eq!(
            attributable_is_dust(raw(50_001), Some(raw(0)), expected),
            Some(false)
        );
    }

    #[test]
    fn the_holding_of_other_open_rounds_is_subtracted_before_the_dust_test() {
        let expected = Some(raw(50_000_000));
        assert_eq!(
            attributable_is_dust(raw(10_050_000), Some(raw(10_000_000)), expected),
            Some(true)
        );
        assert_eq!(
            attributable_is_dust(raw(10_050_001), Some(raw(10_000_000)), expected),
            Some(false)
        );
    }

    /// While the other rounds' holding is unknown, only a holding that is dust on its own is
    /// dust: a zero holding needs no attribution, and anything more may be the round's own.
    #[test]
    fn an_unknown_other_holding_decides_only_a_holding_that_is_dust_alone() {
        let expected = Some(raw(50_000_000));
        assert_eq!(attributable_is_dust(raw(0), None, expected), Some(true));
        assert_eq!(
            attributable_is_dust(raw(50_000), None, expected),
            Some(true)
        );
        assert_eq!(attributable_is_dust(raw(50_001), None, expected), None);
        assert_eq!(attributable_is_dust(raw(10_050_000), None, expected), None);
        assert_eq!(attributable_is_dust(raw(0), None, None), None);
    }

    #[test]
    fn an_unknown_expected_acquisition_decides_nothing() {
        assert_eq!(attributable_is_dust(raw(0), Some(raw(0)), None), None);
        assert_eq!(
            attributable_is_dust(raw(0), Some(raw(0)), expected_acquisition(0.2, 0.004, None)),
            None
        );
    }

    #[test]
    fn an_expected_acquisition_within_the_dust_floor_decides_nothing() {
        for expected in [0, 1, DUST_FLOOR_RAW] {
            for held in [0, 1, DUST_FLOOR_RAW, 1_000_000] {
                assert_eq!(
                    attributable_is_dust(raw(held), Some(raw(0)), Some(raw(expected))),
                    None,
                    "held {held} against expected {expected}"
                );
            }
        }
        let just_above = Some(raw(DUST_FLOOR_RAW + 1));
        assert_eq!(
            attributable_is_dust(raw(0), Some(raw(0)), just_above),
            Some(true)
        );
        assert_eq!(
            attributable_is_dust(raw(DUST_FLOOR_RAW + 1), Some(raw(0)), just_above),
            Some(false)
        );
    }
}
