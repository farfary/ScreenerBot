// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The moving-average cost of a holding: a sale releases cost at the average in force, a buy
//! adds what it cost.

use std::cmp::Ordering;

use crate::chains::RawAmount;

use super::ledger::DUST;
use super::types::{EntryRecord, ExitRecord};

/// What the tokens a holding still holds cost, under the moving-average rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HeldCost {
    cost: f64,
}

impl HeldCost {
    /// Nothing held.
    pub(crate) fn new() -> Self {
        Self { cost: 0.0 }
    }

    /// `held_whole` tokens held at `average` native per whole token.
    pub(crate) fn at_average(average: f64, held_whole: f64) -> Self {
        Self {
            cost: average * held_whole,
        }
    }

    /// A buy that cost `spent` joins the held cost.
    pub(crate) fn acquire(&mut self, spent: f64) {
        self.cost += spent;
    }

    /// A sale of `fraction` of what is held releases that fraction of the held cost, so the
    /// average of what stays is unchanged. The fraction is clamped to `[0, 1]`. Returns the
    /// cost released.
    pub(crate) fn release(&mut self, fraction: f64) -> f64 {
        let released = self.cost * fraction.clamp(0.0, 1.0);
        self.cost = (self.cost - released).max(0.0);
        released
    }

    /// Everything held is gone.
    pub(crate) fn clear(&mut self) {
        self.cost = 0.0;
    }

    /// The cost of what is held.
    pub(crate) fn cost(&self) -> f64 {
        self.cost
    }

    /// The average cost per whole token of `held_whole` tokens. `None` unless more than dust
    /// is held and the average is a positive finite number.
    pub(crate) fn average(&self, held_whole: f64) -> Option<f64> {
        if !(held_whole > DUST && held_whole.is_finite()) {
            return None;
        }
        let average = self.cost / held_whole;
        (average.is_finite() && average > 0.0).then_some(average)
    }
}

/// The average after a buy of `bought_whole` for `spent` that leaves `held_after_whole` held,
/// from the average in force before it. A buy partly gone by the time the holding is read
/// adds only the cost of its part still held. `None` for a buy or holding that is not a
/// positive finite amount, a cost that is not a positive finite number, or earlier tokens
/// still held with no usable average before the buy.
pub(crate) fn average_after_acquisition(
    average_before: f64,
    bought_whole: f64,
    spent: f64,
    held_after_whole: f64,
) -> Option<f64> {
    if !(bought_whole > DUST
        && bought_whole.is_finite()
        && spent > 0.0
        && spent.is_finite()
        && held_after_whole > DUST
        && held_after_whole.is_finite())
    {
        return None;
    }
    let kept_before = (held_after_whole - bought_whole).max(0.0);
    if kept_before > DUST && !(average_before > 0.0 && average_before.is_finite()) {
        return None;
    }
    let bought_held = held_after_whole - kept_before;
    let mut cost = HeldCost::at_average(average_before, kept_before);
    cost.acquire(spent * bought_held / bought_whole);
    cost.average(held_after_whole)
}

/// The cost basis and paid fees a position's swap records give under the moving-average
/// rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Replayed {
    /// The average in force after the last buy; `None` when no buy left a usable average.
    pub average: Option<f64>,
    /// The fees of every buy, `None` when a buy's fee is not recorded.
    pub entry_fees_raw: Option<u64>,
    /// The fees of every sale, `None` when a sale's fee is not recorded.
    pub exit_fees_raw: Option<u64>,
}

/// One swap record in replay order.
enum Swap<'a> {
    Buy(&'a EntryRecord),
    Sale(&'a ExitRecord),
}

impl Swap<'_> {
    fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        match self {
            Swap::Buy(entry) => entry.timestamp,
            Swap::Sale(exit) => exit.timestamp,
        }
    }
}

/// Replays a position's buys and sales in timestamp order, buys first at an equal timestamp:
/// a buy acquires its cost and sets the average, a sale releases cost in proportion to what it
/// sold of the holding. `None` when a sale sells more than the records hold before it, or an
/// amount overflows: the records do not describe the holding.
pub(crate) fn replay(
    entries: &[EntryRecord],
    exits: &[ExitRecord],
    decimals: u8,
) -> Option<Replayed> {
    let mut swaps: Vec<Swap<'_>> = entries
        .iter()
        .map(Swap::Buy)
        .chain(exits.iter().map(Swap::Sale))
        .collect();
    swaps.sort_by(|a, b| match a.timestamp().cmp(&b.timestamp()) {
        Ordering::Equal => match (a, b) {
            (Swap::Buy(_), Swap::Sale(_)) => Ordering::Less,
            (Swap::Sale(_), Swap::Buy(_)) => Ordering::Greater,
            _ => Ordering::Equal,
        },
        order => order,
    });

    let mut held = RawAmount::ZERO;
    let mut cost = HeldCost::new();
    let mut average = None;
    for swap in swaps {
        match swap {
            Swap::Buy(entry) => {
                held = held.checked_add(entry.amount)?;
                cost.acquire(entry.native_spent);
                average = cost.average(held.to_whole_units(decimals));
            }
            Swap::Sale(exit) => {
                if exit.amount > held {
                    return None;
                }
                if held > RawAmount::ZERO {
                    cost.release(exit.amount.raw() as f64 / held.raw() as f64);
                }
                held = held.checked_sub(exit.amount)?;
                if held == RawAmount::ZERO {
                    cost.clear();
                }
            }
        }
    }

    let (entry_fees_raw, exit_fees_raw) = recorded_fees(entries, exits);
    Some(Replayed {
        average,
        entry_fees_raw,
        exit_fees_raw,
    })
}

/// The fees of every recorded buy and of every recorded sale, each `None` when a record of
/// that side carries no fee.
pub(crate) fn recorded_fees(
    entries: &[EntryRecord],
    exits: &[ExitRecord],
) -> (Option<u64>, Option<u64>) {
    (
        sum_fees(entries.iter().map(|entry| entry.fees_raw)),
        sum_fees(exits.iter().map(|exit| exit.fees_raw)),
    )
}

/// The sum of recorded fees, `None` when any is not recorded.
fn sum_fees(mut fees: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    fees.try_fold(0_u64, |total, fee| Some(total.saturating_add(fee?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn close_to(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() < 1e-12
    }

    fn buy(seconds: i64, amount: u128, spent: f64, fee: Option<u64>) -> EntryRecord {
        EntryRecord {
            id: None,
            position_id: 1,
            timestamp: Utc::now() + Duration::seconds(seconds),
            amount: RawAmount::new(amount),
            price: 0.0,
            native_spent: spent,
            transaction_signature: format!("buy-{seconds}"),
            is_dca: seconds > 0,
            fees_raw: fee,
        }
    }

    fn sale(seconds: i64, amount: u128, fee: Option<u64>) -> ExitRecord {
        ExitRecord {
            id: None,
            position_id: 1,
            timestamp: Utc::now() + Duration::seconds(seconds),
            amount: RawAmount::new(amount),
            price: 0.0,
            native_received: 0.0,
            transaction_signature: format!("sale-{seconds}"),
            is_partial: true,
            percentage: 0.0,
            fees_raw: fee,
        }
    }

    #[test]
    fn a_sale_leaves_the_average_unchanged() {
        let mut cost = HeldCost::at_average(1.0, 1.0);
        let released = cost.release(0.4);
        assert!(close_to(released, 0.4));
        assert!(close_to(cost.average(0.6).unwrap(), 1.0));
    }

    #[test]
    fn a_dca_after_a_sale_averages_the_cost_of_the_tokens_still_held() {
        // 1.0 token bought for 1.0, 0.4 sold, 0.4 bought for 0.8: 0.6 + 0.8 for 1.0 token.
        let average = average_after_acquisition(1.0, 0.4, 0.8, 1.0).unwrap();
        assert!(close_to(average, 1.4), "average {average}");
    }

    #[test]
    fn buying_from_nothing_held_averages_the_buy_alone() {
        let average = average_after_acquisition(0.0, 2.0, 0.5, 2.0).unwrap();
        assert!(close_to(average, 0.25));
    }

    #[test]
    fn a_buy_partly_gone_when_read_adds_the_cost_of_its_held_part() {
        // Half of a 2.0-token buy for 1.0 is still held: 0.5 of its cost for 1.0 token.
        let average = average_after_acquisition(3.0, 2.0, 1.0, 1.0).unwrap();
        assert!(close_to(average, 0.5));
    }

    #[test]
    fn degenerate_inputs_give_no_average() {
        assert_eq!(average_after_acquisition(1.0, 0.0, 1.0, 1.0), None);
        assert_eq!(average_after_acquisition(1.0, 1.0, 0.0, 1.0), None);
        assert_eq!(average_after_acquisition(1.0, 1.0, f64::NAN, 1.0), None);
        assert_eq!(average_after_acquisition(1.0, 1.0, 1.0, 0.0), None);
        assert_eq!(
            average_after_acquisition(1.0, f64::INFINITY, 1.0, 1.0),
            None
        );
        assert_eq!(average_after_acquisition(0.0, 1.0, 1.0, 2.0), None);
        assert_eq!(HeldCost::new().average(1.0), None);
        assert_eq!(HeldCost::at_average(1.0, 1.0).average(0.0), None);
    }

    #[test]
    fn a_replay_releases_cost_at_the_average_and_sums_every_fee() {
        let entries = [
            buy(0, 1_000_000, 1.0, Some(5_000)),
            buy(2, 400_000, 0.8, Some(5_000)),
        ];
        let exits = [sale(1, 400_000, Some(5_000))];
        let replayed = replay(&entries, &exits, 6).unwrap();
        assert!(close_to(replayed.average.unwrap(), 1.4));
        assert_eq!(replayed.entry_fees_raw, Some(10_000));
        assert_eq!(replayed.exit_fees_raw, Some(5_000));
    }

    #[test]
    fn a_replay_after_everything_was_sold_averages_the_next_buy_alone() {
        let entries = [buy(0, 1_000, 1.0, Some(1)), buy(2, 1_000, 3.0, Some(1))];
        let exits = [sale(1, 1_000, Some(1))];
        let replayed = replay(&entries, &exits, 3).unwrap();
        assert!(close_to(replayed.average.unwrap(), 3.0));
    }

    #[test]
    fn a_replay_with_an_unrecorded_fee_reports_that_side_unknown() {
        let entries = [buy(0, 1_000, 1.0, Some(5_000))];
        let exits = [sale(1, 500, None)];
        let replayed = replay(&entries, &exits, 3).unwrap();
        assert_eq!(replayed.entry_fees_raw, Some(5_000));
        assert_eq!(replayed.exit_fees_raw, None);
    }

    #[test]
    fn a_sale_beyond_what_the_records_hold_refuses_the_replay() {
        let entries = [buy(1, 1_000, 1.0, Some(1))];
        let exits = [sale(0, 500, Some(1))];
        assert_eq!(replay(&entries, &exits, 3), None);
    }
}
