//! The quote the engine hands back: pool math, slippage floor and platform fee
//! assembled into the numbers a caller can act on.
//!
//! Two output figures exist and they are not interchangeable:
//!
//! * `expected_out` / `min_out` — what the POOL returns. `min_out` is the value
//!   written into the swap instruction and enforced on chain.
//! * `expected_net_out` / `min_net_out` — what the WALLET keeps, after an
//!   output-side platform fee. This is what a P&L figure must use.
//!
//! Confusing the two overstates every sell by the fee.

use super::error::{DirectSwapError, DirectSwapResult};
use super::fee::{FeeSide, PlatformFee};
use super::intent::DirectSwapIntent;
use super::venue::VenueQuote;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::solana::swaps::revenue::BPS_DENOMINATOR;

/// Apply a slippage tolerance to an expected output, rounding DOWN.
///
/// Rounding down is the safe direction: the result is a floor the chain must
/// beat, so a rounding error can only make the swap slightly more permissive,
/// never reject a fill that was actually within tolerance.
pub fn min_out_after_slippage(expected_out: u64, slippage_bps: u16) -> u64 {
    let bps = (slippage_bps as u128).min(BPS_DENOMINATOR as u128);
    ((expected_out as u128) * ((BPS_DENOMINATOR as u128) - bps) / (BPS_DENOMINATOR as u128)) as u64
}

/// A priced direct swap, ready to be built into a transaction.
#[derive(Debug, Clone)]
pub struct DirectQuote {
    /// Pool the swap executes against.
    pub pool: Pubkey,
    /// DEX program of that pool.
    pub program: ProgramKind,
    /// Mint being spent.
    pub input_mint: Pubkey,
    /// Mint being received.
    pub output_mint: Pubkey,
    /// Total the wallet gives up, platform fee included.
    pub amount_in: u64,
    /// The part of `amount_in` that actually reaches the pool. Equals
    /// `amount_in` unless the fee is taken on the input leg.
    pub swap_amount_in: u64,
    /// Pool output for `swap_amount_in`.
    pub expected_out: u64,
    /// Floor written into the swap instruction and enforced on chain.
    pub min_out: u64,
    /// Expected amount the wallet keeps, after an output-side fee.
    pub expected_net_out: u64,
    /// Guaranteed amount the wallet keeps, after an output-side fee.
    pub min_net_out: u64,
    /// The platform fee for this swap.
    pub fee: PlatformFee,
    /// The pool's own trade fee on the input, in input raw units.
    pub lp_fee: u64,
    /// Price impact percentage of this size.
    pub price_impact_pct: f64,
    /// Slippage tolerance this quote was built with.
    pub slippage_bps: u16,
}

impl DirectQuote {
    /// Assemble a quote from the venue's curve result.
    ///
    /// `venue_quote` must have been produced for `swap_amount_in`, i.e. AFTER an
    /// input-side fee was deducted — see the ordering note in [`super::fee`].
    pub fn assemble(
        intent: &DirectSwapIntent,
        program: ProgramKind,
        swap_amount_in: u64,
        venue_quote: VenueQuote,
        input_fee: PlatformFee,
    ) -> DirectSwapResult<Self> {
        if venue_quote.expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: intent.pool,
                amount_in: intent.amount_in,
                detail: "the pool returns nothing for this size".to_owned(),
            });
        }

        let min_out = min_out_after_slippage(venue_quote.expected_out, intent.slippage_bps);
        if min_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: intent.pool,
                amount_in: intent.amount_in,
                detail: "slippage tolerance reduces the guaranteed output to zero".to_owned(),
            });
        }

        // The output-side fee is sized off `min_out` — the amount the chain
        // guarantees will be in the account when the transfer executes.
        let fee = match input_fee.side {
            FeeSide::Output => PlatformFee::resolve(
                FeeSide::Output,
                &intent.input_mint,
                &intent.output_mint,
                min_out,
            )?,
            _ => input_fee,
        };

        let (expected_net_out, min_net_out) = match fee.side {
            FeeSide::Output => (
                venue_quote.expected_out.saturating_sub(fee.amount),
                min_out.saturating_sub(fee.amount),
            ),
            _ => (venue_quote.expected_out, min_out),
        };

        if min_net_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: intent.pool,
                amount_in: intent.amount_in,
                detail: "the platform fee consumes the entire guaranteed output".to_owned(),
            });
        }

        Ok(Self {
            pool: intent.pool,
            program,
            input_mint: intent.input_mint,
            output_mint: intent.output_mint,
            amount_in: intent.amount_in,
            swap_amount_in,
            expected_out: venue_quote.expected_out,
            min_out,
            expected_net_out,
            min_net_out,
            fee,
            lp_fee: venue_quote.lp_fee,
            price_impact_pct: venue_quote.price_impact_pct,
            slippage_bps: intent.slippage_bps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::constants::SOL_MINT;
    use crate::chains::solana::swaps::direct::intent::wsol_mint;
    use std::str::FromStr;

    fn venue_quote(expected_out: u64) -> VenueQuote {
        VenueQuote {
            amount_in: 5_000_000,
            expected_out,
            lp_fee: 12_500,
            price_impact_pct: 0.2,
        }
    }

    fn buy_intent() -> DirectSwapIntent {
        DirectSwapIntent {
            pool: Pubkey::new_unique(),
            owner: Pubkey::new_unique(),
            input_mint: wsol_mint(),
            output_mint: Pubkey::new_unique(),
            amount_in: 5_000_000,
            slippage_bps: 100,
        }
    }

    #[test]
    fn min_out_rounds_down_and_zero_slippage_is_the_identity() {
        assert_eq!(min_out_after_slippage(1_000_000, 0), 1_000_000);
        assert_eq!(min_out_after_slippage(1_000_000, 100), 990_000);
        assert_eq!(min_out_after_slippage(9_999, 1), 9_998, "rounds down");
    }

    #[test]
    fn min_out_cannot_overflow_or_go_negative_at_extremes() {
        assert_eq!(min_out_after_slippage(u64::MAX, 10_000), 0);
        assert_eq!(
            min_out_after_slippage(u64::MAX, u16::MAX),
            0,
            "slippage above 100% is clamped, never wrapped"
        );
    }

    #[test]
    fn a_buy_keeps_the_full_pool_output_because_the_fee_was_taken_on_the_input() {
        let intent = buy_intent();
        let input_fee =
            PlatformFee::resolve(FeeSide::Input, &intent.input_mint, &intent.output_mint, 5_000_000)
                .unwrap();
        let swap_in = intent.amount_in - input_fee.amount;

        let quote = DirectQuote::assemble(
            &intent,
            ProgramKind::RaydiumCpmm,
            swap_in,
            venue_quote(1_000_000),
            input_fee,
        )
        .unwrap();

        assert_eq!(quote.amount_in, 5_000_000);
        assert_eq!(quote.swap_amount_in, 4_975_000, "0.5% never reaches the pool");
        assert_eq!(quote.fee.amount, 25_000);
        assert_eq!(quote.min_out, 990_000);
        assert_eq!(
            quote.min_net_out, 990_000,
            "an input-side fee does not reduce what the wallet receives"
        );
    }

    #[test]
    fn a_sell_nets_the_output_fee_out_of_what_the_wallet_keeps() {
        let intent = DirectSwapIntent {
            input_mint: Pubkey::new_unique(),
            output_mint: Pubkey::from_str(SOL_MINT).unwrap(),
            ..buy_intent()
        };
        let side = FeeSide::for_pair(&intent.input_mint, &intent.output_mint);
        assert_eq!(side, FeeSide::Output);
        let placeholder = PlatformFee::resolve(side, &intent.input_mint, &intent.output_mint, 0).unwrap();

        let quote = DirectQuote::assemble(
            &intent,
            ProgramKind::RaydiumCpmm,
            intent.amount_in,
            venue_quote(1_000_000),
            placeholder,
        )
        .unwrap();

        assert_eq!(quote.swap_amount_in, intent.amount_in, "nothing is held back");
        assert_eq!(quote.min_out, 990_000, "the chain enforces the pool output");
        assert_eq!(quote.fee.amount, 4_950, "0.5% of the GUARANTEED output");
        assert_eq!(quote.min_net_out, 985_050);
        assert_eq!(quote.expected_net_out, 995_050);
    }

    #[test]
    fn a_pool_that_returns_nothing_is_an_liquidity_failure_not_a_zero_quote() {
        let intent = buy_intent();
        let fee = PlatformFee::none();
        assert!(matches!(
            DirectQuote::assemble(
                &intent,
                ProgramKind::RaydiumCpmm,
                intent.amount_in,
                venue_quote(0),
                fee
            ),
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }

    #[test]
    fn a_quote_whose_floor_rounds_to_zero_is_refused_rather_than_sent_unprotected() {
        let intent = DirectSwapIntent {
            slippage_bps: 5_000,
            ..buy_intent()
        };
        let fee = PlatformFee::none();
        assert!(matches!(
            DirectQuote::assemble(
                &intent,
                ProgramKind::RaydiumCpmm,
                intent.amount_in,
                venue_quote(1),
                fee
            ),
            Err(DirectSwapError::InsufficientLiquidity { .. })
        ));
    }
}
