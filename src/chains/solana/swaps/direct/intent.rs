//! What the caller asked for: an amount of one mint, swapped for another, in a
//! specific pool.
//!
//! The intent is expressed as a MINT PAIR, never as buy/sell. A buy/sell shape
//! silently assumes the other leg is SOL, which is why the previous builder could
//! only do SOL<->TOKEN and mis-oriented every pool whose SOL side was `token_1`.
//! With an explicit pair, SOL->TOKEN, TOKEN->SOL, USDC->TOKEN, TOKEN->USDC,
//! SOL->USDC and USDC->SOL are all the same code path, and the venue decides the
//! orientation by matching mints against the pool's own.
//!
//! Native SOL is always handled for the caller: WSOL on the input leg is wrapped
//! from the wallet's native balance, WSOL on the output leg is unwrapped back to
//! it, and the temporary WSOL account is closed in the same transaction. Callers
//! think in SOL; only the pool sees WSOL.

use super::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::LazyLock;

/// The wrapped-SOL mint, decoded once. `is_wsol` runs on every quote for every
/// pool, so comparing 32 bytes rather than base58-encoding a `Pubkey` into a
/// `String` on each call matters on this hot path.
static WSOL_MINT: LazyLock<Pubkey> =
    LazyLock::new(|| Pubkey::from_str(SOL_MINT).expect("SOL_MINT constant is a valid pubkey"));

/// Hard ceiling on slippage the engine will accept, in basis points (50%).
/// Anything above this is a mistake, not a preference: it authorises a swap to
/// return almost nothing and still succeed.
pub const MAX_SLIPPAGE_BPS: u16 = 5_000;

/// A swap of `amount_in` raw units of `input_mint` for `output_mint`, executed
/// directly against `pool`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectSwapIntent {
    /// The pool account to trade against.
    pub pool: Pubkey,
    /// Wallet that pays, signs, and receives.
    pub owner: Pubkey,
    /// Mint being spent.
    pub input_mint: Pubkey,
    /// Mint being received.
    pub output_mint: Pubkey,
    /// Amount of `input_mint` to spend, in raw units. This is the TOTAL the
    /// wallet gives up, platform fee included.
    pub amount_in: u64,
    /// Slippage tolerance in basis points (100 = 1%).
    pub slippage_bps: u16,
}

impl DirectSwapIntent {
    /// Reject a request that cannot produce a sane swap. Called before any RPC
    /// so an obviously wrong intent never costs a round trip.
    pub fn validate(&self) -> DirectSwapResult<()> {
        if self.amount_in == 0 {
            return Err(DirectSwapError::InvalidRequest {
                detail: "amount_in is zero".to_owned(),
            });
        }
        if self.input_mint == self.output_mint {
            return Err(DirectSwapError::InvalidRequest {
                detail: format!("input and output mint are both {}", self.input_mint),
            });
        }
        if self.slippage_bps > MAX_SLIPPAGE_BPS {
            return Err(DirectSwapError::InvalidRequest {
                detail: format!(
                    "slippage {} bps exceeds the {MAX_SLIPPAGE_BPS} bps ceiling",
                    self.slippage_bps
                ),
            });
        }
        Ok(())
    }

    /// Whether native SOL must be wrapped to fund this swap.
    pub fn wraps_native(&self) -> bool {
        is_wsol(&self.input_mint)
    }

    /// Whether the proceeds must be unwrapped back to native SOL.
    pub fn unwraps_native(&self) -> bool {
        is_wsol(&self.output_mint)
    }

    /// Whether WSOL is involved at all, meaning a temporary WSOL account is
    /// created and closed inside the transaction.
    pub fn touches_wsol(&self) -> bool {
        self.wraps_native() || self.unwraps_native()
    }
}

/// The wrapped-SOL mint as a `Pubkey`.
pub fn wsol_mint() -> Pubkey {
    *WSOL_MINT
}

/// Whether a mint is wrapped SOL.
pub fn is_wsol(mint: &Pubkey) -> bool {
    *mint == *WSOL_MINT
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent() -> DirectSwapIntent {
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
    fn a_well_formed_intent_validates() {
        assert!(intent().validate().is_ok());
    }

    #[test]
    fn a_zero_amount_is_refused_before_any_rpc() {
        let mut i = intent();
        i.amount_in = 0;
        assert!(matches!(
            i.validate(),
            Err(DirectSwapError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn swapping_a_mint_for_itself_is_refused() {
        let mut i = intent();
        i.output_mint = i.input_mint;
        assert!(matches!(
            i.validate(),
            Err(DirectSwapError::InvalidRequest { .. })
        ));
    }

    #[test]
    fn slippage_above_the_ceiling_is_refused_rather_than_clamped() {
        let mut i = intent();
        i.slippage_bps = MAX_SLIPPAGE_BPS + 1;
        assert!(matches!(
            i.validate(),
            Err(DirectSwapError::InvalidRequest { .. })
        ));
        i.slippage_bps = MAX_SLIPPAGE_BPS;
        assert!(i.validate().is_ok(), "the ceiling itself is allowed");
    }

    #[test]
    fn wsol_on_the_input_wraps_and_wsol_on_the_output_unwraps() {
        let mut i = intent();
        assert!(i.wraps_native() && !i.unwraps_native() && i.touches_wsol());

        std::mem::swap(&mut i.input_mint, &mut i.output_mint);
        assert!(!i.wraps_native() && i.unwraps_native() && i.touches_wsol());
    }

    #[test]
    fn a_pair_without_sol_touches_no_wsol_account() {
        let i = DirectSwapIntent {
            input_mint: Pubkey::new_unique(),
            output_mint: Pubkey::new_unique(),
            ..intent()
        };
        assert!(!i.touches_wsol());
    }
}
