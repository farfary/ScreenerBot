// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure rules over a round's raw token amounts, free of I/O: what is dust, what part of a
//! holding a round owns, and how a closed position follows its round when a fill lands after
//! its close.

use crate::chains::RawAmount;

use super::booking::DcaFill;
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
    /// The closed position held more than dust while its mint has another open position: the
    /// holding and its cost went to that position as an add, and this one stays closed.
    HandedOver,
}

/// The holding and cost a closed position hands to the open position of its mint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Handover {
    /// The tokens the closed position still held.
    pub tokens: RawAmount,
    /// The part of the closed position's invested total those tokens carry, in proportion
    /// to what it acquired.
    pub cost_native: f64,
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

/// Like [`follow_round`], for a closed position whose mint has another open position: one
/// mint has one open position, so a holding above dust is handed to that position instead of
/// reopening this one. The handed-over tokens leave this position's acquisition and their
/// share of its invested total leaves its cost, so it stays closed with its P&L restated
/// over what it kept. At or below dust it follows [`follow_round`] and hands nothing over.
pub fn hand_over_round(
    position: &mut Position,
    held: RawAmount,
) -> Result<(FollowOutcome, Option<Handover>)> {
    if !is_closed(position) {
        return Ok((FollowOutcome::Unchanged, None));
    }
    let Some(acquired) = position.acquired_amount() else {
        return Err(Error::AmountOverflow {
            mint: position.mint.clone(),
            operation: "handing the round over",
        });
    };
    if is_dust(held, acquired) {
        return Ok((follow_round(position, held)?, None));
    }
    let held = held.min(acquired);
    let share = held.raw() as f64 / acquired.raw() as f64;
    let cost_native = position.total_size_native * share;
    position.total_size_native -= cost_native;
    position.remaining_token_amount = Some(RawAmount::ZERO);
    position.total_exited_amount = acquired.checked_sub(held).unwrap_or(RawAmount::ZERO);
    let (pnl, pnl_percent) = realized_pnl(position);
    position.pnl = Some(pnl);
    position.pnl_percent = Some(pnl_percent);
    Ok((
        FollowOutcome::HandedOver,
        Some(Handover {
            tokens: held,
            cost_native,
        }),
    ))
}

/// A part of one fill: the tokens it acquired and what they cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    pub tokens: RawAmount,
    pub cost_native: f64,
}

/// How a buy that lands on a closed position divides between that position and the open
/// position of its mint, by what [`hand_over_leg`] decided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegSplit {
    /// The part of the buy booked on the closed position: tokens it acquired that the
    /// wallet no longer holds, at the buy's own price.
    pub kept: Leg,
    /// The part of the buy the wallet still holds, handed over at the buy's own price.
    pub handed_leg: Leg,
    /// The closed position's earlier tokens the wallet still holds beyond the buy, handed
    /// over at the closed position's average cost before the buy.
    pub handed_earlier: Leg,
}

impl LegSplit {
    /// The whole of `leg` stays on the position it was bought for.
    fn kept_whole(leg: Leg) -> Self {
        let nothing = Leg {
            tokens: RawAmount::ZERO,
            cost_native: 0.0,
        };
        Self {
            kept: leg,
            handed_leg: nothing,
            handed_earlier: nothing,
        }
    }

    /// Everything handed to the open position, the buy's part and the earlier part
    /// together. `None` when the sum overflows the raw amount range.
    pub fn handed(&self) -> Option<Handover> {
        Some(Handover {
            tokens: self
                .handed_leg
                .tokens
                .checked_add(self.handed_earlier.tokens)?,
            cost_native: self.handed_leg.cost_native + self.handed_earlier.cost_native,
        })
    }
}

/// `part` as a fraction of `whole`, zero for an empty whole.
fn fraction(part: RawAmount, whole: RawAmount) -> f64 {
    if whole == RawAmount::ZERO {
        0.0
    } else {
        part.raw() as f64 / whole.raw() as f64
    }
}

/// Books a buy `fill` on `position` when the buy may land after the position closed and its
/// mint has another open position. `held` is the wallet's holding attributable to the
/// position with the buy counted (see [`attributable_held`]).
///
/// An open position books the whole buy. A closed position whose holding is at most dust of
/// everything it acquired, the buy included, books the whole buy and follows
/// [`follow_round`]. Otherwise the holding goes to the open position, each part at its own
/// cost: the part of the buy still held at the buy's price, and any of the closed position's
/// earlier tokens still held beyond the buy at its average cost before the buy. The part of
/// the buy no longer held is booked on the closed position as acquired and exited at the
/// buy's price, so the closed position's books change by exactly that part and its P&L is
/// restated over what it kept.
pub(crate) fn hand_over_leg(
    position: &mut Position,
    fill: &DcaFill,
    held: RawAmount,
) -> Result<(FollowOutcome, LegSplit)> {
    let whole = Leg {
        tokens: fill.tokens_bought,
        cost_native: fill.native_spent,
    };
    if !is_closed(position) {
        position.book_dca(fill)?;
        return Ok((FollowOutcome::Unchanged, LegSplit::kept_whole(whole)));
    }
    let overflow = || Error::AmountOverflow {
        mint: position.mint.clone(),
        operation: "handing a late buy over",
    };
    let before = position.acquired_amount().ok_or_else(overflow)?;
    let acquired = before
        .checked_add(fill.tokens_bought)
        .ok_or_else(overflow)?;
    if is_dust(held, acquired) {
        position.book_dca(fill)?;
        return Ok((follow_round(position, held)?, LegSplit::kept_whole(whole)));
    }

    let held = held.min(acquired);
    let handed_leg_tokens = held.min(fill.tokens_bought);
    let handed_leg_cost = fill.native_spent * fraction(handed_leg_tokens, fill.tokens_bought);
    let earlier_tokens = held
        .checked_sub(handed_leg_tokens)
        .unwrap_or(RawAmount::ZERO);
    let earlier_cost = position.total_size_native * fraction(earlier_tokens, before);
    let kept = Leg {
        tokens: fill
            .tokens_bought
            .checked_sub(handed_leg_tokens)
            .unwrap_or(RawAmount::ZERO),
        cost_native: fill.native_spent - handed_leg_cost,
    };

    let exited = before
        .checked_sub(earlier_tokens)
        .unwrap_or(RawAmount::ZERO)
        .checked_add(kept.tokens)
        .ok_or_else(overflow)?;
    position.total_size_native += kept.cost_native - earlier_cost;
    if kept.tokens > RawAmount::ZERO {
        position.dca_count += 1;
        position.last_dca_time = Some(fill.dca_time);
    }
    position.remaining_token_amount = Some(RawAmount::ZERO);
    position.total_exited_amount = exited;
    let (pnl, pnl_percent) = realized_pnl(position);
    position.pnl = Some(pnl);
    position.pnl_percent = Some(pnl_percent);
    Ok((
        FollowOutcome::HandedOver,
        LegSplit {
            kept,
            handed_leg: Leg {
                tokens: handed_leg_tokens,
                cost_native: handed_leg_cost,
            },
            handed_earlier: Leg {
                tokens: earlier_tokens,
                cost_native: earlier_cost,
            },
        },
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

    /// A position that bought `acquired` raw units for `cost` native and sold them all for
    /// `received`, closed and verified.
    fn closed(acquired: u128, cost: f64, received: f64) -> Position {
        let mut position: Position = serde_json::from_str(
            r#"{
            "id": 1,
            "mint": "mint",
            "symbol": "SYM",
            "name": "Token",
            "entry_price": 0.001,
            "entry_time": "2026-01-01T00:00:00Z",
            "exit_price": 0.0006,
            "exit_time": "2026-01-02T00:00:00Z",
            "position_type": "buy",
            "entry_size_native": 0.0,
            "total_size_native": 0.0,
            "price_highest": 0.001,
            "price_lowest": 0.0006,
            "entry_transaction_signature": "entry",
            "exit_transaction_signature": "exit",
            "token_amount": null,
            "effective_entry_price": 0.001,
            "effective_exit_price": 0.0006,
            "native_received": null,
            "profit_target_min": null,
            "profit_target_max": null,
            "liquidity_tier": null,
            "transaction_entry_verified": true,
            "transaction_exit_verified": true,
            "entry_fee_raw": null,
            "exit_fee_raw": null,
            "current_price": null,
            "current_price_updated": null,
            "phantom_remove": false,
            "phantom_confirmations": 0,
            "phantom_first_seen": null,
            "synthetic_exit": false,
            "closed_reason": null,
            "pnl": null,
            "pnl_percent": null,
            "unrealized_pnl": null,
            "unrealized_pnl_percent": null,
            "remaining_token_amount": "0",
            "total_exited_amount": "0",
            "average_exit_price": null,
            "partial_exit_count": 0,
            "dca_count": 0,
            "average_entry_price": 0.001,
            "last_dca_time": null,
            "origin": { "kind": "manual" },
            "management": "user_only"
            }"#,
        )
        .expect("position fixture deserializes");
        position.entry_size_native = cost;
        position.total_size_native = cost;
        position.token_amount = Some(raw(acquired));
        position.total_exited_amount = raw(acquired);
        position.native_received = Some(received);
        position.pnl = Some(received - cost);
        position
    }

    fn late_buy(tokens: u128, native: f64) -> DcaFill {
        DcaFill {
            tokens_bought: raw(tokens),
            native_spent: native,
            dca_time: chrono::Utc::now(),
        }
    }

    fn close_to(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() < 1e-12
    }

    /// A buy at half the closed position's price lands after its full exit; the wallet still
    /// holds exactly the buy beside the open position. The buy moves at its own cost, so the
    /// closed position's realized P&L is what its own trades made.
    #[test]
    fn a_late_buy_still_held_moves_at_its_own_cost_and_leaves_the_closed_books_unchanged() {
        let mut position = closed(1_000, 1.0, 0.6);
        let (outcome, split) =
            hand_over_leg(&mut position, &late_buy(1_000, 0.5), raw(1_000)).unwrap();

        assert_eq!(outcome, FollowOutcome::HandedOver);
        assert_eq!(split.kept.tokens, raw(0));
        assert!(close_to(split.kept.cost_native, 0.0));
        assert_eq!(split.handed_leg.tokens, raw(1_000));
        assert!(close_to(split.handed_leg.cost_native, 0.5));
        assert_eq!(split.handed_earlier.tokens, raw(0));
        let handed = split.handed().unwrap();
        assert_eq!(handed.tokens, raw(1_000));
        assert!(close_to(handed.cost_native, 0.5));

        assert!(close_to(position.total_size_native, 1.0));
        assert!(close_to(position.pnl.unwrap(), 0.6 - 1.0));
        assert_eq!(position.acquired_amount(), Some(raw(1_000)));
        assert_eq!(position.remaining_token_amount, Some(raw(0)));
        assert_eq!(position.dca_count, 0, "the closed position recorded no add");
    }

    /// Part of the buy left the wallet: that part stays on the closed position as acquired
    /// and exited at the buy's price, and only the held part moves.
    #[test]
    fn the_part_of_a_late_buy_no_longer_held_stays_on_the_closed_position_at_its_own_cost() {
        let mut position = closed(1_000, 1.0, 0.6);
        let (outcome, split) =
            hand_over_leg(&mut position, &late_buy(1_000, 0.5), raw(400)).unwrap();

        assert_eq!(outcome, FollowOutcome::HandedOver);
        assert_eq!(split.handed_leg.tokens, raw(400));
        assert!(close_to(split.handed_leg.cost_native, 0.2));
        assert_eq!(split.kept.tokens, raw(600));
        assert!(close_to(split.kept.cost_native, 0.3));
        assert!(close_to(
            split.kept.cost_native + split.handed_leg.cost_native,
            0.5
        ));
        assert!(close_to(position.total_size_native, 1.3));
        assert!(close_to(position.pnl.unwrap(), 0.6 - 1.3));
        assert_eq!(position.acquired_amount(), Some(raw(1_600)));
        assert_eq!(position.dca_count, 1);
    }

    /// The wallet holds more than the buy: the closed position's own earlier tokens are
    /// still there, and only they move at its average cost before the buy.
    #[test]
    fn earlier_tokens_held_beyond_the_buy_move_at_the_pre_buy_average_cost() {
        let mut position = closed(1_000, 1.0, 0.0);
        let (outcome, split) =
            hand_over_leg(&mut position, &late_buy(1_000, 0.5), raw(1_250)).unwrap();

        assert_eq!(outcome, FollowOutcome::HandedOver);
        assert_eq!(split.handed_leg.tokens, raw(1_000));
        assert!(close_to(split.handed_leg.cost_native, 0.5));
        assert_eq!(split.handed_earlier.tokens, raw(250));
        assert!(close_to(split.handed_earlier.cost_native, 0.25));
        assert_eq!(split.kept.tokens, raw(0));
        assert!(close_to(position.total_size_native, 0.75));
        assert_eq!(position.acquired_amount(), Some(raw(750)));
    }

    /// A holding that is dust of everything acquired hands nothing over: the closed
    /// position books the whole buy and stays closed.
    #[test]
    fn a_dust_holding_keeps_the_whole_late_buy_on_the_closed_position() {
        let mut position = closed(1_000_000, 1.0, 0.6);
        let (outcome, split) =
            hand_over_leg(&mut position, &late_buy(1_000_000, 0.5), raw(2_000)).unwrap();

        assert_eq!(outcome, FollowOutcome::StaysClosed);
        assert_eq!(split.kept.tokens, raw(1_000_000));
        assert!(close_to(split.kept.cost_native, 0.5));
        assert_eq!(split.handed().unwrap().tokens, raw(0));
        assert!(close_to(position.total_size_native, 1.5));
        assert_eq!(position.dca_count, 1);
    }
}
