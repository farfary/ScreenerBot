// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Exit sizing — how many raw tokens a full or a partial exit of a position sells.
//!
//! The one owner of the rule: the full close, the partial close and any preview of either
//! size their sale here, so what is quoted is what is sold.

use crate::chains::RawAmount;
use crate::positions::db::OtherOpenHeld;
use crate::swaps::calculate_partial_amount;

/// How many raw tokens a full close sells, and why it is below the wallet total when it is.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ExitSizing {
    pub(super) amount: u64,
    pub(super) note: Option<String>,
}

/// Sizes a full close from the wallet's `total` holding across its token accounts and the
/// `primary` account's balance, the one most routes spend from.
///
/// With no other open position of the mint the close sweeps the wallet: the primary balance
/// when the total is split across accounts, the total when the primary account is empty.
/// Tokens the mint's other open positions hold are not this position's to sell: with
/// `others` booked, the sale is at most the total less their holding, and while their
/// holding cannot be attributed it is at most `own_remaining`.
///
/// A `quote_asset` mint (the chain's native coin or a stablecoin) is cash the wallet holds
/// beside the position: the row books only what the bot bought, so the sale is at most
/// `own_remaining` and never sweeps the wallet.
pub(super) fn full_exit_amount(
    total: u64,
    primary: u64,
    own_remaining: RawAmount,
    others: OtherOpenHeld,
    quote_asset: bool,
) -> ExitSizing {
    let (swept, note) = if primary == 0 && total > 0 {
        (total, Some("primary_ata_empty_using_total".to_owned()))
    } else if total > primary && primary > 0 {
        (
            primary,
            Some(format!(
                "multi_account_total={total} primary={primary} shortfall={}, limiting_to_primary",
                total - primary
            )),
        )
    } else {
        (total, None)
    };
    let own = u64::try_from(own_remaining.raw()).unwrap_or(u64::MAX);
    if quote_asset {
        if own >= swept {
            return ExitSizing {
                amount: swept,
                note,
            };
        }
        return ExitSizing {
            amount: own,
            note: Some(format!(
                "wallet_total={total} swept={swept} quote_asset_limiting_to_booked={own}"
            )),
        };
    }
    let own_cap = match others {
        OtherOpenHeld::Booked(held) if held == RawAmount::ZERO => {
            return ExitSizing {
                amount: swept,
                note,
            }
        }
        OtherOpenHeld::Booked(held) => {
            u64::try_from(held.raw()).map_or(0, |held| total.saturating_sub(held))
        }
        OtherOpenHeld::Unattributable { .. } => own,
    };
    if own_cap >= swept {
        return ExitSizing {
            amount: swept,
            note,
        };
    }
    ExitSizing {
        amount: own_cap,
        note: Some(format!(
            "wallet_total={total} swept={swept} limiting_to_own_holding={own_cap}"
        )),
    }
}

/// Sizes a partial exit of `percentage` of the position.
///
/// The base is the lower of the position's booked `remaining` and the wallet's `on_chain`
/// holding: never more than the wallet has, never more than the position owns. An
/// unreadable or empty chain balance (`None` or zero) falls back to the booked remainder,
/// so a lookup failure never blocks an exit.
pub(super) fn partial_exit_amount(
    remaining: RawAmount,
    on_chain: Option<u64>,
    percentage: f64,
) -> RawAmount {
    let base = match on_chain {
        Some(on_chain) if on_chain > 0 => RawAmount::from(on_chain).min(remaining),
        _ => remaining,
    };
    calculate_partial_amount(base, percentage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn booked(raw: u128) -> OtherOpenHeld {
        OtherOpenHeld::Booked(RawAmount::new(raw))
    }

    const UNATTRIBUTABLE: OtherOpenHeld = OtherOpenHeld::Unattributable { position_id: 7 };

    fn amount(total: u64, primary: u64, own: u128, others: OtherOpenHeld) -> u64 {
        full_exit_amount(total, primary, RawAmount::new(own), others, false).amount
    }

    fn quote_amount(total: u64, primary: u64, own: u128, others: OtherOpenHeld) -> u64 {
        full_exit_amount(total, primary, RawAmount::new(own), others, true).amount
    }

    #[test]
    fn a_close_with_no_other_open_position_sweeps_the_wallet_as_before() {
        assert_eq!(amount(2_000, 2_000, 1_000, booked(0)), 2_000);
        assert_eq!(amount(2_000, 1_500, 1_000, booked(0)), 1_500);
        assert_eq!(amount(2_000, 0, 1_000, booked(0)), 2_000);
        assert_eq!(amount(0, 0, 1_000, booked(0)), 0);
    }

    #[test]
    fn a_close_leaves_what_the_other_open_positions_hold() {
        assert_eq!(amount(2_000, 2_000, 1_000, booked(600)), 1_400);
        assert_eq!(amount(2_000, 1_200, 1_000, booked(600)), 1_200);
        assert_eq!(amount(2_000, 2_000, 1_000, booked(2_000)), 0);
        assert_eq!(amount(2_000, 2_000, 1_000, booked(5_000)), 0);
        assert_eq!(amount(2_000, 2_000, 1_000, booked(u128::MAX)), 0);
    }

    #[test]
    fn a_close_beside_a_holding_that_cannot_be_attributed_sells_at_most_its_own_remainder() {
        assert_eq!(amount(2_000, 2_000, 1_000, UNATTRIBUTABLE), 1_000);
        assert_eq!(amount(2_000, 800, 1_000, UNATTRIBUTABLE), 800);
        assert_eq!(amount(900, 900, 1_000, UNATTRIBUTABLE), 900);
        assert_eq!(amount(2_000, 2_000, 0, UNATTRIBUTABLE), 0);
    }

    #[test]
    fn a_capped_sale_says_why() {
        let sizing = full_exit_amount(2_000, 2_000, RawAmount::new(1_000), booked(600), false);
        assert_eq!(sizing.amount, 1_400);
        assert!(sizing
            .note
            .is_some_and(|note| note.contains("limiting_to_own_holding=1400")));
        assert_eq!(
            full_exit_amount(2_000, 2_000, RawAmount::new(1_000), booked(0), false).note,
            None
        );
        let quote = full_exit_amount(2_000, 2_000, RawAmount::new(1_000), booked(0), true);
        assert!(quote
            .note
            .is_some_and(|note| note.contains("quote_asset_limiting_to_booked=1000")));
    }

    /// A wallet holding 1,000 units of a stablecoin as cash beside a 0.423386-unit bot
    /// position sells the position, never the cash; a memecoin with the same inputs sweeps.
    #[test]
    fn a_quote_asset_close_sells_only_the_booked_remainder() {
        assert_eq!(
            quote_amount(1_000_000_000, 1_000_000_000, 423_386, booked(0)),
            423_386
        );
        assert_eq!(
            amount(1_000_000_000, 1_000_000_000, 423_386, booked(0)),
            1_000_000_000
        );
        assert_eq!(
            quote_amount(1_000_000_000, 1_000_000_000, 423_386, booked(600)),
            423_386
        );
        assert_eq!(
            quote_amount(1_000_000_000, 1_000_000_000, 423_386, UNATTRIBUTABLE),
            423_386
        );
        assert_eq!(quote_amount(1_000_000_000, 1_000_000_000, 0, booked(0)), 0);
    }

    #[test]
    fn a_quote_asset_close_still_respects_the_primary_account_and_the_wallet_total() {
        assert_eq!(quote_amount(1_000, 300, 423, booked(0)), 300);
        assert_eq!(quote_amount(200, 200, 423, booked(0)), 200);
        assert_eq!(quote_amount(1_000, 0, 423, booked(0)), 423);
        assert_eq!(quote_amount(0, 0, 423, booked(0)), 0);
    }

    /// Over a grid of wallet and booked amounts: a quote-asset close never sells more than
    /// the row books nor more than the wallet holds, and a memecoin close is unchanged by
    /// the quote-asset rule whenever the row owns at least the wallet's sweep.
    #[test]
    fn full_exit_bounds_hold_over_a_grid() {
        let grid = [0_u64, 1, 7, 423_386, 1_000_000, 1_000_000_000, u64::MAX];
        for total in grid {
            for primary in grid.iter().copied().filter(|primary| *primary <= total) {
                for own in grid {
                    let quote = quote_amount(total, primary, u128::from(own), booked(0));
                    assert!(quote <= own, "total={total} primary={primary} own={own}");
                    assert!(quote <= total, "total={total} primary={primary} own={own}");
                    let swept = amount(total, primary, u128::from(own), booked(0));
                    assert!(quote <= swept);
                    if own >= swept {
                        assert_eq!(quote, swept);
                    }
                }
            }
        }
    }

    #[test]
    fn a_partial_exit_sizes_against_the_lower_of_booked_and_chain() {
        let remaining = RawAmount::new(1_000);
        assert_eq!(
            partial_exit_amount(remaining, Some(1_000), 50.0),
            RawAmount::new(500)
        );
        assert_eq!(
            partial_exit_amount(remaining, Some(600), 50.0),
            RawAmount::new(300)
        );
        assert_eq!(
            partial_exit_amount(remaining, Some(5_000), 50.0),
            RawAmount::new(500)
        );
        assert_eq!(
            partial_exit_amount(remaining, Some(0), 50.0),
            RawAmount::new(500)
        );
        assert_eq!(
            partial_exit_amount(remaining, None, 50.0),
            RawAmount::new(500)
        );
        assert_eq!(
            partial_exit_amount(RawAmount::ZERO, Some(1_000), 50.0),
            RawAmount::ZERO
        );
    }

    #[test]
    fn a_partial_exit_never_sells_more_than_the_position_or_the_wallet_holds() {
        let grid = [0_u64, 1, 7, 423_386, 1_000_000_000, u64::MAX];
        let percentages = [0.0, 0.5, 1.0, 25.0, 50.0, 99.0, 99.9];
        for remaining in grid {
            for on_chain in grid {
                for percentage in percentages {
                    let sold =
                        partial_exit_amount(RawAmount::from(remaining), Some(on_chain), percentage);
                    assert!(sold <= RawAmount::from(remaining));
                    if on_chain > 0 {
                        assert!(sold <= RawAmount::from(on_chain));
                    }
                }
            }
        }
    }
}
