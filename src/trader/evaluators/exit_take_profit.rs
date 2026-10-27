// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The take-profit ladder: which step is due, and what share of the holding it sells.

use crate::trader::policy::TakeProfitPolicy;

/// A step whose share of what is left reaches 100% within this tolerance sells
/// everything, so float rounding of the shares sold before it never leaves dust.
const FULL_EXIT_TOLERANCE_PCT: f64 = 1e-6;

/// Decimal sell shares that add up to exactly 100% can sum past it in floating point
/// (5.2 + 78.9 + 15.9 is 100.00000000000001), so a ladder's shares are checked against
/// the whole position within this tolerance.
pub const TAKE_PROFIT_SHARE_SUM_TOLERANCE_PCT: f64 = 1e-9;

/// The take-profit rule on plain numbers, shared by the live evaluator and the paper
/// copy book. `steps_taken` is how many steps the holding has already reserved, so the
/// step due is the one at that index; it fires once profit from `entry_price` reaches its
/// level. A price that is not finite and positive fires nothing. Returns
/// `Some(exit_percentage)` when it fires, where the share is of what the
/// holding still holds and the inner `None` sells everything.
///
/// Each step's `sell_pct` is a share of the position as opened. What is left when step k
/// is due is `100 − Σ sell_pct` of the steps before it, so step k sells
/// `sell_pct[k] / (100 − Σ_{j<k} sell_pct[j]) × 100` percent of the holding. A ladder of
/// 30/30/40 sells 30%, then 30/70 = 42.857% of the rest, then 40/40 = everything. A
/// ladder that sold everything, or whose step finds nothing left to sell, fires nothing
/// more; `Err` carries the detail of a ladder whose shares leave nothing for a step.
pub fn take_profit_step(
    entry_price: f64,
    current_price: f64,
    steps_taken: u32,
    policy: &TakeProfitPolicy,
) -> crate::trader::Result<Option<Option<f64>>> {
    let usable = |price: f64| price.is_finite() && price > 0.0;
    if !policy.enabled || !usable(entry_price) || !usable(current_price) {
        return Ok(None);
    }
    let index = steps_taken as usize;
    let Some(step) = policy.levels.get(index) else {
        return Ok(None);
    };
    if (current_price / entry_price - 1.0) * 100.0 < step.profit_pct {
        return Ok(None);
    }
    let sold_before: f64 = policy.levels[..index]
        .iter()
        .map(|level| level.sell_pct)
        .sum();
    let left_pct = 100.0 - sold_before;
    if !left_pct.is_finite()
        || left_pct <= 0.0
        || !step.sell_pct.is_finite()
        || step.sell_pct <= 0.0
    {
        return Err(crate::trader::Error::InvalidExitPolicy {
            detail: format!(
                "invalid take-profit ladder: step {} sells {}% with {left_pct}% of the position left",
                index + 1,
                step.sell_pct
            ),
        });
    }
    let share_of_left = step.sell_pct / left_pct * 100.0;
    Ok(Some(
        (share_of_left < 100.0 - FULL_EXIT_TOLERANCE_PCT).then_some(share_of_left),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TakeProfitLevel;

    fn ladder(steps: &[(f64, f64)]) -> TakeProfitPolicy {
        TakeProfitPolicy {
            enabled: true,
            levels: steps
                .iter()
                .map(|&(profit_pct, sell_pct)| TakeProfitLevel {
                    profit_pct,
                    sell_pct,
                })
                .collect(),
        }
    }

    #[test]
    fn a_one_step_ladder_sells_everything_at_its_level() {
        let policy = ladder(&[(20.0, 100.0)]);
        assert_eq!(take_profit_step(1.0, 1.2, 0, &policy).unwrap(), Some(None));
        assert_eq!(take_profit_step(1.0, 1.19, 0, &policy).unwrap(), None);
        assert_eq!(
            take_profit_step(1.0, 5.0, 1, &policy).unwrap(),
            None,
            "a taken step never fires again"
        );
    }

    #[test]
    fn each_step_sells_its_share_of_what_is_left() {
        let policy = ladder(&[(20.0, 30.0), (50.0, 30.0), (100.0, 40.0)]);
        let first = take_profit_step(1.0, 1.2, 0, &policy).unwrap().unwrap();
        assert_eq!(first, Some(30.0));
        let second = take_profit_step(1.0, 1.5, 1, &policy)
            .unwrap()
            .unwrap()
            .expect("a partial");
        assert!((second - 30.0 / 70.0 * 100.0).abs() < 1e-9, "{second}");
        assert_eq!(
            take_profit_step(1.0, 2.0, 2, &policy).unwrap(),
            Some(None),
            "the last 40 of 40 left sells everything"
        );
    }

    #[test]
    fn only_the_step_due_is_checked() {
        let policy = ladder(&[(20.0, 30.0), (50.0, 30.0)]);
        // Far past both levels with nothing taken: the first step fires alone.
        assert_eq!(
            take_profit_step(1.0, 3.0, 0, &policy).unwrap(),
            Some(Some(30.0))
        );
        // The second step waits for its own level.
        assert_eq!(take_profit_step(1.0, 1.3, 1, &policy).unwrap(), None);
    }

    #[test]
    fn a_ladder_that_leaves_a_runner_sells_a_partial_at_its_last_step() {
        let policy = ladder(&[(20.0, 50.0), (60.0, 25.0)]);
        let last = take_profit_step(1.0, 1.6, 1, &policy)
            .unwrap()
            .unwrap()
            .expect("a partial");
        assert!((last - 50.0).abs() < 1e-9, "{last}");
        assert_eq!(take_profit_step(1.0, 9.0, 2, &policy).unwrap(), None);
    }

    #[test]
    fn rounding_of_the_shares_before_a_step_never_leaves_dust() {
        let policy = ladder(&[(10.0, 33.3), (20.0, 33.3), (30.0, 33.4)]);
        assert_eq!(take_profit_step(1.0, 1.3, 2, &policy).unwrap(), Some(None));
    }

    #[test]
    fn the_ladder_measures_from_the_average_entry() {
        let policy = ladder(&[(50.0, 100.0)]);
        assert_eq!(take_profit_step(0.5, 0.75, 0, &policy).unwrap(), Some(None));
        assert_eq!(take_profit_step(1.0, 0.75, 0, &policy).unwrap(), None);
    }

    #[test]
    fn a_disabled_ladder_or_an_unusable_entry_fires_nothing() {
        let mut policy = ladder(&[(20.0, 100.0)]);
        for bad in [0.0, -1.0, f64::NAN] {
            assert_eq!(take_profit_step(bad, 2.0, 0, &policy).unwrap(), None);
        }
        policy.enabled = false;
        assert_eq!(take_profit_step(1.0, 100.0, 0, &policy).unwrap(), None);
    }

    #[test]
    fn an_unusable_current_price_fires_nothing() {
        let policy = ladder(&[(20.0, 100.0)]);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
            assert_eq!(
                take_profit_step(1.0, bad, 0, &policy).unwrap(),
                None,
                "price {bad}"
            );
        }
        assert_eq!(
            take_profit_step(f64::INFINITY, 2.0, 0, &policy).unwrap(),
            None,
            "an infinite entry"
        );
    }

    #[test]
    fn a_ladder_that_leaves_nothing_for_a_step_is_an_error() {
        let policy = ladder(&[(20.0, 100.0), (50.0, 20.0)]);
        assert!(take_profit_step(1.0, 2.0, 1, &policy).is_err());
    }
}
