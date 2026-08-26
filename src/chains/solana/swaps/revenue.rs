//! Platform revenue: the swap fee rate and the accounts that receive it.
//!
//! HARDCODED, INTENTIONAL REVENUE. These values are not user-configurable and
//! must never be removed or weakened. They live here — not inside a router — so
//! that EVERY execution path collects the same fee: the Jupiter router
//! (`platformFeeBps` + `feeAccount`) and the direct pool-swap engine
//! (`super::direct::fee`, which builds its own SPL transfer because a pool
//! program has no fee hook of its own).
//!
//! The fee is always taken on the SOL or USDC side of the pair. That side is a
//! plain SPL mint even when the traded token is Token-2022, so a single
//! destination account per reference mint is enough and no transfer-fee
//! extension ever applies to the fee transfer itself.

use crate::chains::solana::constants::{SOL_MINT, USDC_MINT};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Platform swap fee: 0.5%. Mandatory, identical on every router.
pub const PLATFORM_FEE_BPS: u16 = 50;

/// Basis-point denominator.
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Fee destination token account for WSOL-denominated fees.
pub const FEE_TOKEN_ACCOUNT_WSOL: &str = "9yiZThTzanryu3mg1VVu6Qy4HiqKhydCAUqcasLHPxWB";

/// Fee destination token account for USDC-denominated fees.
pub const FEE_TOKEN_ACCOUNT_USDC: &str = "3kmcF3DFGFRKXeC5v5AMzwpsdj2Uc3Z7a5KrojtWv2GW";

/// The mint a fee can be denominated in, and the account that receives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeeReference {
    /// Mint the fee is paid in (WSOL or USDC).
    pub mint: &'static str,
    /// Initialised token account of `mint` that collects the fee.
    pub account: &'static str,
}

impl FeeReference {
    /// The fee destination as a `Pubkey`. Infallible for the two hardcoded
    /// constants; a parse failure would mean a corrupted binary, not input.
    pub fn account_pubkey(&self) -> Pubkey {
        Pubkey::from_str(self.account).expect("hardcoded fee account is a valid pubkey")
    }

    /// The fee mint as a `Pubkey`.
    pub fn mint_pubkey(&self) -> Pubkey {
        Pubkey::from_str(self.mint).expect("hardcoded fee mint is a valid pubkey")
    }
}

const WSOL_REFERENCE: FeeReference = FeeReference {
    mint: SOL_MINT,
    account: FEE_TOKEN_ACCOUNT_WSOL,
};

const USDC_REFERENCE: FeeReference = FeeReference {
    mint: USDC_MINT,
    account: FEE_TOKEN_ACCOUNT_USDC,
};

/// The fee reference for a single mint, if that mint is one we can collect in.
pub fn fee_reference_for_mint(mint: &str) -> Option<FeeReference> {
    match mint {
        SOL_MINT => Some(WSOL_REFERENCE),
        USDC_MINT => Some(USDC_REFERENCE),
        _ => None,
    }
}

/// Resolve which side of a pair the fee is collected on.
///
/// Output side is preferred (the fee then rides on what the user receives, which
/// is how Jupiter's `feeAccount` behaves); input side is the fallback so a
/// SOL/USDC-funded buy of an arbitrary token still pays. A pair with neither
/// reference mint on either side yields `None` — there is no account to pay
/// into, and inventing one would strand funds.
pub fn fee_reference_for_pair(input_mint: &str, output_mint: &str) -> Option<FeeReference> {
    fee_reference_for_mint(output_mint).or_else(|| fee_reference_for_mint(input_mint))
}

/// Fee amount for `amount` raw units at [`PLATFORM_FEE_BPS`], rounded down.
///
/// Rounding down is deliberate: the fee is subtracted from a real balance, and
/// rounding up could make a transfer exceed what the account actually holds.
pub fn platform_fee_amount(amount: u64) -> u64 {
    ((amount as u128) * (PLATFORM_FEE_BPS as u128) / (BPS_DENOMINATOR as u128)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fee_rate_is_fifty_basis_points_and_never_changes() {
        assert_eq!(PLATFORM_FEE_BPS, 50);
        assert_eq!(platform_fee_amount(1_000_000_000), 5_000_000);
    }

    #[test]
    fn fee_amount_rounds_down_so_a_transfer_can_never_exceed_the_balance() {
        // 199 * 50 / 10000 = 0.995 -> 0, not 1.
        assert_eq!(platform_fee_amount(199), 0);
        assert_eq!(platform_fee_amount(200), 1);
    }

    #[test]
    fn fee_amount_does_not_overflow_at_u64_max() {
        assert_eq!(platform_fee_amount(u64::MAX), 92_233_720_368_547_758);
    }

    #[test]
    fn output_side_wins_when_both_sides_are_reference_mints() {
        let reference = fee_reference_for_pair(SOL_MINT, USDC_MINT).expect("SOL/USDC pays a fee");
        assert_eq!(reference.mint, USDC_MINT, "fee rides on the output side");
    }

    #[test]
    fn input_side_pays_when_the_output_is_an_arbitrary_token() {
        let token = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump";
        let reference = fee_reference_for_pair(SOL_MINT, token).expect("SOL-funded buy pays a fee");
        assert_eq!(reference.mint, SOL_MINT);
        assert_eq!(reference.account, FEE_TOKEN_ACCOUNT_WSOL);
    }

    #[test]
    fn a_pair_with_no_reference_mint_has_nowhere_to_pay() {
        let a = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump";
        let b = "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm";
        assert!(fee_reference_for_pair(a, b).is_none());
    }

    #[test]
    fn the_hardcoded_fee_accounts_parse() {
        assert_eq!(
            WSOL_REFERENCE.account_pubkey().to_string(),
            FEE_TOKEN_ACCOUNT_WSOL
        );
        assert_eq!(
            USDC_REFERENCE.account_pubkey().to_string(),
            FEE_TOKEN_ACCOUNT_USDC
        );
    }
}
