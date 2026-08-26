//! The platform fee on a direct pool swap.
//!
//! A pool program has no fee hook — unlike Jupiter, nothing in the swap
//! instruction can pay us. So the engine adds its OWN SPL transfer to the same
//! transaction, which makes the fee atomic with the swap: either both happen or
//! neither does. Rate and destinations come from
//! [`crate::chains::solana::swaps::revenue`], shared with the Jupiter router so
//! there is exactly one definition of what we charge.
//!
//! Which side pays is decided from the MINTS ALONE, before any amount is known
//! ([`FeeSide::for_pair`]). That ordering is what keeps the arithmetic
//! non-circular:
//!
//! 1. side from the mints,
//! 2. input-side fee from `amount_in`, leaving `swap_amount_in`,
//! 3. pool quote on `swap_amount_in` -> `expected_out` -> `min_out`,
//! 4. output-side fee from `min_out` — the amount the chain guarantees is there.
//!
//! Step 4 uses `min_out`, never `expected_out`: the transfer is built before the
//! swap executes, and a fee sized off an optimistic estimate would fail the whole
//! transaction whenever the fill came in light.

use super::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::constants::{SOL_MINT, USDC_MINT};
use crate::chains::solana::solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use crate::chains::solana::swaps::revenue::{fee_reference_for_pair, platform_fee_amount};
use std::str::FromStr;

/// Which leg of the swap the platform fee is taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeSide {
    /// Taken from what the user puts in, before the pool sees it.
    Input,
    /// Taken from what the pool returns, after the swap instruction.
    Output,
    /// Neither mint is a reference mint — nothing can be collected.
    None,
}

impl FeeSide {
    /// Decide the fee side from the pair alone. Output is preferred; input is the
    /// fallback; a pair with no reference mint on either side pays nothing.
    pub fn for_pair(input_mint: &Pubkey, output_mint: &Pubkey) -> Self {
        let input = input_mint.to_string();
        let output = output_mint.to_string();
        if is_reference_mint(&output) {
            FeeSide::Output
        } else if is_reference_mint(&input) {
            FeeSide::Input
        } else {
            FeeSide::None
        }
    }
}

fn is_reference_mint(mint: &str) -> bool {
    mint == SOL_MINT || mint == USDC_MINT
}

/// A fully resolved platform fee: how much, in what, and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformFee {
    /// The leg the fee comes from.
    pub side: FeeSide,
    /// Raw units of [`Self::mint`]. Zero when `side` is [`FeeSide::None`], and
    /// also when the traded amount is too small for 0.5% to round to one unit.
    pub amount: u64,
    /// Mint the fee is denominated in (WSOL or USDC), if any.
    pub mint: Option<Pubkey>,
    /// Token account that receives it, if any.
    pub destination: Option<Pubkey>,
    /// Decimals of [`Self::mint`], needed by `transfer_checked`.
    pub decimals: u8,
}

impl PlatformFee {
    /// A fee that is not collected, because the pair has no reference mint.
    pub fn none() -> Self {
        Self {
            side: FeeSide::None,
            amount: 0,
            mint: None,
            destination: None,
            decimals: 0,
        }
    }

    /// Size the fee for one leg. `base` is `amount_in` for [`FeeSide::Input`] and
    /// the guaranteed `min_out` for [`FeeSide::Output`].
    pub fn resolve(
        side: FeeSide,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        base: u64,
    ) -> DirectSwapResult<Self> {
        if side == FeeSide::None {
            return Ok(Self::none());
        }
        let reference = fee_reference_for_pair(&input_mint.to_string(), &output_mint.to_string())
            .ok_or_else(|| DirectSwapError::InvalidRequest {
            detail: "fee side resolved without a reference mint on either leg".to_owned(),
        })?;
        let mint = Pubkey::from_str(reference.mint).map_err(|e| DirectSwapError::Build {
            detail: format!("fee mint is not a pubkey: {e}"),
        })?;
        let destination =
            Pubkey::from_str(reference.account).map_err(|e| DirectSwapError::Build {
                detail: format!("fee account is not a pubkey: {e}"),
            })?;
        Ok(Self {
            side,
            amount: platform_fee_amount(base),
            mint: Some(mint),
            destination: Some(destination),
            decimals: reference_mint_decimals(reference.mint),
        })
    }

    /// Whether this fee produces an instruction. A zero amount does not: an
    /// SPL transfer of zero is legal but wastes compute and log space.
    pub fn is_collectible(&self) -> bool {
        self.amount > 0 && self.mint.is_some() && self.destination.is_some()
    }

    /// The transfer that moves the fee. `source` is the user's token account for
    /// [`Self::mint`]; `authority` is the wallet that owns it.
    ///
    /// Always `transfer_checked` — it re-verifies mint and decimals on chain, so
    /// a wrong `source` account cannot silently move the wrong asset. The fee
    /// mint is always WSOL or USDC, both legacy SPL, so the program is fixed.
    pub fn transfer_instruction(
        &self,
        source: &Pubkey,
        authority: &Pubkey,
    ) -> DirectSwapResult<Option<Instruction>> {
        if !self.is_collectible() {
            return Ok(None);
        }
        let (mint, destination) = match (self.mint, self.destination) {
            (Some(mint), Some(destination)) => (mint, destination),
            _ => return Ok(None),
        };
        let instruction = crate::chains::solana::spl_token::instruction::transfer_checked(
            &crate::chains::solana::spl_token::id(),
            source,
            &mint,
            &destination,
            authority,
            &[],
            self.amount,
            self.decimals,
        )
        .map_err(|e| DirectSwapError::Build {
            detail: format!("platform fee transfer could not be built: {e}"),
        })?;
        Ok(Some(instruction))
    }
}

/// Decimals of a reference mint. Both are fixed on mainnet: WSOL 9, USDC 6.
fn reference_mint_decimals(mint: &str) -> u8 {
    match mint {
        USDC_MINT => 6,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::swaps::revenue::{FEE_TOKEN_ACCOUNT_USDC, FEE_TOKEN_ACCOUNT_WSOL};

    fn wsol() -> Pubkey {
        Pubkey::from_str(SOL_MINT).unwrap()
    }

    fn usdc() -> Pubkey {
        Pubkey::from_str(USDC_MINT).unwrap()
    }

    #[test]
    fn buying_a_token_with_sol_takes_the_fee_from_the_input() {
        let token = Pubkey::new_unique();
        assert_eq!(FeeSide::for_pair(&wsol(), &token), FeeSide::Input);
    }

    #[test]
    fn selling_a_token_for_sol_takes_the_fee_from_the_output() {
        let token = Pubkey::new_unique();
        assert_eq!(FeeSide::for_pair(&token, &wsol()), FeeSide::Output);
    }

    #[test]
    fn a_token_to_token_pair_has_no_side_to_collect_on() {
        let a = Pubkey::new_unique();
        let b = Pubkey::new_unique();
        assert_eq!(FeeSide::for_pair(&a, &b), FeeSide::None);
        let fee = PlatformFee::resolve(FeeSide::None, &a, &b, 1_000_000_000).unwrap();
        assert_eq!(fee.amount, 0);
        assert!(!fee.is_collectible());
        assert!(fee.transfer_instruction(&a, &b).unwrap().is_none());
    }

    #[test]
    fn an_input_side_fee_on_sol_is_half_a_percent_paid_in_wsol() {
        let token = Pubkey::new_unique();
        let fee = PlatformFee::resolve(FeeSide::Input, &wsol(), &token, 5_000_000).unwrap();
        assert_eq!(fee.amount, 25_000, "0.5% of 0.005 SOL");
        assert_eq!(fee.mint, Some(wsol()));
        assert_eq!(
            fee.destination.map(|d| d.to_string()).as_deref(),
            Some(FEE_TOKEN_ACCOUNT_WSOL)
        );
        assert_eq!(fee.decimals, 9);
    }

    #[test]
    fn an_output_side_fee_on_usdc_is_paid_in_usdc_at_six_decimals() {
        let token = Pubkey::new_unique();
        let fee = PlatformFee::resolve(FeeSide::Output, &token, &usdc(), 10_000_000).unwrap();
        assert_eq!(fee.amount, 50_000, "0.5% of 10 USDC");
        assert_eq!(fee.decimals, 6);
        assert_eq!(
            fee.destination.map(|d| d.to_string()).as_deref(),
            Some(FEE_TOKEN_ACCOUNT_USDC)
        );
    }

    #[test]
    fn a_dust_trade_rounds_the_fee_to_zero_and_emits_no_transfer() {
        let token = Pubkey::new_unique();
        let fee = PlatformFee::resolve(FeeSide::Input, &wsol(), &token, 100).unwrap();
        assert_eq!(fee.amount, 0);
        assert!(fee
            .transfer_instruction(&Pubkey::new_unique(), &Pubkey::new_unique())
            .unwrap()
            .is_none());
    }

    #[test]
    fn the_fee_transfer_is_checked_and_targets_the_hardcoded_account() {
        let token = Pubkey::new_unique();
        let source = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let fee = PlatformFee::resolve(FeeSide::Input, &wsol(), &token, 5_000_000).unwrap();
        let ix = fee
            .transfer_instruction(&source, &authority)
            .unwrap()
            .expect("a non-zero fee must produce a transfer");

        assert_eq!(ix.program_id, crate::chains::solana::spl_token::id());
        // TransferChecked: [source, mint, destination, authority]
        assert_eq!(ix.accounts[0].pubkey, source);
        assert_eq!(ix.accounts[1].pubkey, wsol(), "mint is checked on chain");
        assert_eq!(
            ix.accounts[2].pubkey.to_string(),
            FEE_TOKEN_ACCOUNT_WSOL,
            "fee lands in the platform account, never anywhere else"
        );
        assert_eq!(ix.accounts[3].pubkey, authority);
        assert!(ix.accounts[3].is_signer);
        assert_eq!(ix.data[0], 12, "SPL TransferChecked discriminator");
    }
}
