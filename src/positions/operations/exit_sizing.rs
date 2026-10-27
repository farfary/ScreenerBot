// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Exit sizing — how many raw tokens a full or a partial exit of a position sells.
//!
//! The one owner of the rule: the full close, the partial close and any preview of either
//! size their sale here, so what is quoted is what is sold.

use crate::chains::RawAmount;
use crate::positions::db::{get_other_open_held, OtherOpenHeld};
use crate::positions::types::Position;
use crate::positions::{Error, Result};

/// The part of a position one exit sells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExitShare {
    /// The whole position: a full close.
    All,
    /// This percentage of the position, strictly between 0 and 100: a partial exit.
    Part(f64),
}

/// The size of one exit and the wallet holding it was sized against.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ExitSale {
    /// Raw units the exit sells.
    pub(crate) amount: RawAmount,
    /// Why a full close sells less than the wallet's sweep, when it does.
    pub(crate) note: Option<String>,
    /// The wallet's holding of the mint across its token accounts, read for this sale.
    pub(crate) wallet_holding: RawAmount,
}

/// What one exit is sized from, as read from the wallet and the positions store.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExitReadings {
    All {
        total: u64,
        primary: u64,
        own_remaining: RawAmount,
        others: OtherOpenHeld,
        quote_asset: bool,
    },
    Part {
        holding: RawAmount,
        own_remaining: RawAmount,
        percentage: f64,
    },
}

/// Reads what an exit of `mint` from `wallet` is sized from and sizes `share` of `own`, the
/// open round of the mint when one exists.
///
/// A full close reads the wallet's holding across its token accounts, the primary
/// account's balance, the other open positions' holding and whether the mint is a quote
/// asset; a partial exit reads the wallet's holding. Both read `own`'s booked remainder,
/// and both fail when the wallet's holding cannot be read: an exit is never sized from the
/// position's own figures alone.
/// The full close, the partial exit and the manual sell preview all size through here, so
/// the preview quotes what execution sells.
pub(crate) async fn read_exit_sale(
    wallet: &str,
    mint: &str,
    own: Option<&Position>,
    share: ExitShare,
) -> Result<ExitSale> {
    let own_remaining = own.and_then(Position::held_amount).unwrap_or_default();
    let readings = match share {
        ExitShare::All => {
            let holding = crate::positions::settle::holding(wallet, mint).await?;
            let total = u64::try_from(holding.amount.raw()).map_err(|_| Error::AmountOverflow {
                mint: mint.to_owned(),
                operation: "sizing a full exit from the wallet holding",
            })?;
            // The primary account is what most routes spend from; an unread balance sizes
            // from the total.
            let primary = crate::chains::solana::assets::ata::get_token_balance(wallet, mint)
                .await
                .unwrap_or_default();
            ExitReadings::All {
                total,
                primary,
                own_remaining,
                others: get_other_open_held(mint, own.and_then(|p| p.id)).await?,
                quote_asset: crate::chains::adapter().is_quote_asset(mint),
            }
        }
        ExitShare::Part(percentage) => ExitReadings::Part {
            holding: crate::positions::settle::holding(wallet, mint)
                .await?
                .amount,
            own_remaining,
            percentage,
        },
    };
    Ok(size_exit(readings))
}

/// Sizes one exit from its readings through the full or the partial rule.
fn size_exit(readings: ExitReadings) -> ExitSale {
    match readings {
        ExitReadings::All {
            total,
            primary,
            own_remaining,
            others,
            quote_asset,
        } => {
            let ExitSizing { amount, note } =
                full_exit_amount(total, primary, own_remaining, others, quote_asset);
            ExitSale {
                amount: RawAmount::from(amount),
                note,
                wallet_holding: RawAmount::from(total),
            }
        }
        ExitReadings::Part {
            holding,
            own_remaining,
            percentage,
        } => ExitSale {
            amount: partial_exit_amount(own_remaining, holding, percentage),
            note: None,
            wallet_holding: holding,
        },
    }
}

/// How many raw tokens a full close sells, and why it is below the wallet total when it is.
#[derive(Debug, PartialEq, Eq)]
struct ExitSizing {
    amount: u64,
    note: Option<String>,
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
fn full_exit_amount(
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
/// holding: never more than the wallet has, never more than the position owns. An empty
/// wallet sells nothing.
fn partial_exit_amount(remaining: RawAmount, on_chain: RawAmount, percentage: f64) -> RawAmount {
    calculate_partial_amount(on_chain.min(remaining), percentage)
}

/// The share of `total_amount` for `percentage`, truncated toward zero and never above the
/// total. Zero for an empty total or a NaN or nonpositive percentage; the whole total at 100%
/// or more.
fn calculate_partial_amount(total_amount: RawAmount, percentage: f64) -> RawAmount {
    if total_amount == RawAmount::ZERO || percentage.is_nan() || percentage <= 0.0 {
        return RawAmount::ZERO;
    }
    if percentage >= 100.0 {
        return total_amount;
    }
    let partial = (total_amount.raw() as f64 * percentage / 100.0).trunc();
    RawAmount::from_integral_f64(partial).map_or(total_amount, |partial| partial.min(total_amount))
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
        let chain = RawAmount::new;
        assert_eq!(
            partial_exit_amount(remaining, chain(1_000), 50.0),
            RawAmount::new(500)
        );
        assert_eq!(
            partial_exit_amount(remaining, chain(600), 50.0),
            RawAmount::new(300)
        );
        assert_eq!(
            partial_exit_amount(remaining, chain(5_000), 50.0),
            RawAmount::new(500)
        );
        // An empty wallet sells nothing; the booked remainder is never the base on its own.
        assert_eq!(
            partial_exit_amount(remaining, RawAmount::ZERO, 50.0),
            RawAmount::ZERO
        );
        assert_eq!(
            partial_exit_amount(RawAmount::ZERO, chain(1_000), 50.0),
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
                    let sold = partial_exit_amount(
                        RawAmount::from(remaining),
                        RawAmount::from(on_chain),
                        percentage,
                    );
                    assert!(sold <= RawAmount::from(remaining));
                    assert!(sold <= RawAmount::from(on_chain));
                }
            }
        }
    }

    #[test]
    fn a_partial_amount_is_the_requested_share_of_the_balance() {
        let balance = RawAmount::new(1_000);
        assert_eq!(calculate_partial_amount(balance, 25.0), RawAmount::new(250));
        assert_eq!(calculate_partial_amount(balance, 50.0), RawAmount::new(500));
        assert_eq!(calculate_partial_amount(balance, 99.0), RawAmount::new(990));
    }

    #[test]
    fn a_full_percentage_sells_the_entire_balance() {
        // Exactly 100 must return the balance itself, not a rounded product — selling
        // 999_999_999 of 1_000_000_000 units leaves dust that blocks the account close.
        let balance = RawAmount::new(1_000_000_000);
        assert_eq!(calculate_partial_amount(balance, 100.0), balance);
        assert_eq!(calculate_partial_amount(balance, 150.0), balance);
    }

    #[test]
    fn a_partial_amount_can_never_exceed_the_balance() {
        // The wallet cannot sell what it does not hold; an over-sized amount fails the
        // swap outright rather than partially filling.
        for pct in [100.0, 100.000_001, 1_000.0, f64::INFINITY] {
            assert!(
                calculate_partial_amount(RawAmount::new(12_345), pct) <= RawAmount::new(12_345),
                "percentage {pct} produced more than the balance"
            );
        }
    }

    #[test]
    fn a_zero_balance_or_non_positive_percentage_sells_nothing() {
        assert_eq!(
            calculate_partial_amount(RawAmount::ZERO, 50.0),
            RawAmount::ZERO
        );
        for pct in [0.0, -25.0, f64::NAN] {
            assert_eq!(
                calculate_partial_amount(RawAmount::new(1_000), pct),
                RawAmount::ZERO,
                "percentage {pct}"
            );
        }
    }

    #[test]
    fn a_partial_amount_truncates_rather_than_rounding_up() {
        // Truncating keeps the result inside the balance for every input. Rounding up on
        // the last percent would try to sell one unit more than is held.
        assert_eq!(
            calculate_partial_amount(RawAmount::new(7), 50.0),
            RawAmount::new(3)
        );
        assert_eq!(
            calculate_partial_amount(RawAmount::new(3), 99.9),
            RawAmount::new(2)
        );
    }

    #[test]
    fn a_dust_sized_share_of_a_small_balance_is_zero() {
        // The caller must treat 0 as "do not submit": `partial_close_position` refuses a
        // zero exit amount rather than sending a swap that cannot fill.
        assert_eq!(
            calculate_partial_amount(RawAmount::new(10), 1.0),
            RawAmount::ZERO
        );
    }

    #[test]
    fn a_partial_amount_stays_exact_above_the_u64_range() {
        assert_eq!(
            calculate_partial_amount(RawAmount::new(1u128 << 70), 50.0),
            RawAmount::new(1u128 << 69)
        );
        assert_eq!(
            calculate_partial_amount(RawAmount::MAX, 100.0),
            RawAmount::MAX
        );
        let just_below_100 = f64::from_bits(100f64.to_bits() - 1);
        assert_eq!(
            calculate_partial_amount(RawAmount::MAX, just_below_100),
            RawAmount::new(u128::MAX >> 75 << 75)
        );
    }

    /// The share the manual sell preview quotes for a requested percentage: a partial exit
    /// below 100%, the full close at 100% or with none.
    fn preview_share(percentage: Option<f64>) -> ExitShare {
        match percentage {
            Some(pct) if pct < 100.0 => ExitShare::Part(pct),
            _ => ExitShare::All,
        }
    }

    /// Over a grid of wallet total, primary account, booked remainder and percentage, the
    /// sale the preview sizes is the sale execution sizes: both read through
    /// `read_exit_sale`, whose sizing half is `size_exit` over the full or the partial rule.
    #[test]
    fn the_preview_and_execution_size_the_same_sale() {
        let grid = [0_u64, 1, 7, 423_386, 1_000_000_000, u64::MAX];
        let percentages = [
            None,
            Some(0.5),
            Some(25.0),
            Some(50.0),
            Some(99.9),
            Some(100.0),
        ];
        for total in grid {
            for primary in grid.iter().copied().filter(|primary| *primary <= total) {
                for own_booked in grid {
                    for quote_asset in [false, true] {
                        for percentage in percentages {
                            let own_remaining = RawAmount::from(own_booked);
                            let readings = match preview_share(percentage) {
                                ExitShare::All => ExitReadings::All {
                                    total,
                                    primary,
                                    own_remaining,
                                    others: booked(0),
                                    quote_asset,
                                },
                                ExitShare::Part(percentage) => ExitReadings::Part {
                                    holding: RawAmount::from(total),
                                    own_remaining,
                                    percentage,
                                },
                            };
                            let previewed = size_exit(readings);
                            let executed = match percentage {
                                Some(pct) if pct < 100.0 => {
                                    partial_exit_amount(own_remaining, RawAmount::from(total), pct)
                                }
                                _ => RawAmount::from(
                                    full_exit_amount(
                                        total,
                                        primary,
                                        own_remaining,
                                        booked(0),
                                        quote_asset,
                                    )
                                    .amount,
                                ),
                            };
                            assert_eq!(
                                previewed.amount, executed,
                                "total={total} primary={primary} booked={own_booked} quote={quote_asset} pct={percentage:?}"
                            );
                            assert!(previewed.amount <= RawAmount::from(total).max(own_remaining));
                        }
                    }
                }
            }
        }
    }

    /// The full close, the partial exit and the manual sell preview of a position read and
    /// size through `read_exit_sale` alone; none of them applies a sizing rule itself, and
    /// the exits read no balance of their own. The preview reads the wallet balance only for
    /// a token no open position holds.
    #[test]
    fn every_exit_and_its_preview_size_through_the_one_reader() {
        for (file, source, reads_wallet) in [
            ("close.rs", include_str!("close.rs"), false),
            ("partial_close.rs", include_str!("partial_close.rs"), false),
            (
                "webserver/routes/trader/manual.rs",
                include_str!("../../webserver/routes/trader/manual.rs"),
                true,
            ),
        ] {
            let code = source.split("#[cfg(test)]").next().unwrap();
            assert!(code.contains("read_exit_sale("), "{file}");
            for bypass in [
                "full_exit_amount(",
                "partial_exit_amount(",
                "get_other_open_held(",
                "get_token_balance(",
            ] {
                assert!(!code.contains(bypass), "{file} calls {bypass}");
            }
            if !reads_wallet {
                assert!(!code.contains("get_total_token_balance("), "{file}");
            }
        }
    }

    /// A partial exit reads the wallet's holding or fails: the read is never optional, so
    /// a failed one never sizes the sale from the booked remainder alone.
    #[test]
    fn a_partial_exit_fails_when_the_wallet_cannot_be_read() {
        let source = include_str!("exit_sizing.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        let start = code.find("ExitShare::Part(percentage) =>").unwrap();
        let arm = &code[start..start + code[start..].find("},").unwrap()];
        assert!(
            arm.contains("settle::holding(wallet, mint).await?"),
            "{arm}"
        );
        assert!(!arm.contains(".ok()"), "{arm}");
        assert!(!arm.contains("unwrap_or"), "{arm}");
    }
}
