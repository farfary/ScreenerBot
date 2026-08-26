//! Wallet-side token accounts for a direct swap.
//!
//! Two rules here were each a real failure mode of the previous builder:
//!
//! * The token program comes from the POOL's decoded state, not from a guess and
//!   not from a second RPC read of the mint. A Token-2022 mint derives a
//!   different ATA address than a legacy one, so guessing produces an account the
//!   swap instruction cannot use.
//! * Creation is IDEMPOTENT. The old code did `get_account` first and created the
//!   ATA only if the read said it was missing — a race against every other
//!   transaction in flight, and one extra RPC round trip per leg. The
//!   idempotent instruction makes the question moot on chain.

use super::error::{DirectSwapError, DirectSwapResult};
use super::venue::PoolMarket;
use crate::chains::solana::solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use crate::chains::solana::spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account_idempotent,
};

/// The owner's token accounts for both legs of a swap, with the token program
/// each mint actually belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalletLegs {
    /// Owner's token account for the input mint.
    pub input_account: Pubkey,
    /// Token program owning the input mint.
    pub input_program: Pubkey,
    /// Owner's token account for the output mint.
    pub output_account: Pubkey,
    /// Token program owning the output mint.
    pub output_program: Pubkey,
}

impl WalletLegs {
    /// Derive both legs from the pool's own view of its mints.
    pub fn resolve(
        owner: &Pubkey,
        market: &dyn PoolMarket,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
    ) -> DirectSwapResult<Self> {
        let input_program = market.token_program(input_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: market.pool(),
                input_mint: *input_mint,
                output_mint: *output_mint,
            }
        })?;
        let output_program = market.token_program(output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: market.pool(),
                input_mint: *input_mint,
                output_mint: *output_mint,
            }
        })?;

        Ok(Self {
            input_account: get_associated_token_address_with_program_id(
                owner,
                input_mint,
                &input_program,
            ),
            input_program,
            output_account: get_associated_token_address_with_program_id(
                owner,
                output_mint,
                &output_program,
            ),
            output_program,
        })
    }

    /// Instructions that guarantee both accounts exist by the time the swap runs.
    /// Safe to include unconditionally — each is a no-op if the account is there.
    pub fn ensure_instructions(
        &self,
        payer: &Pubkey,
        owner: &Pubkey,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
    ) -> Vec<Instruction> {
        vec![
            create_associated_token_account_idempotent(
                payer,
                owner,
                input_mint,
                &self.input_program,
            ),
            create_associated_token_account_idempotent(
                payer,
                owner,
                output_mint,
                &self.output_program,
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::pools::types::ProgramKind;
    use crate::chains::solana::swaps::direct::venue::{SwapAccounts, VenueQuote};
    use crate::chains::solana::{spl_token, spl_token_2022};

    #[derive(Debug)]
    struct FakeMarket {
        pool: Pubkey,
        mint_a: Pubkey,
        mint_b: Pubkey,
    }

    impl PoolMarket for FakeMarket {
        fn program(&self) -> ProgramKind {
            ProgramKind::RaydiumCpmm
        }
        fn pool(&self) -> Pubkey {
            self.pool
        }
        fn mints(&self) -> (Pubkey, Pubkey) {
            (self.mint_a, self.mint_b)
        }
        fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
            if *mint == self.mint_a {
                Some(spl_token::id())
            } else if *mint == self.mint_b {
                Some(spl_token_2022::id())
            } else {
                None
            }
        }
        fn decimals(&self, _mint: &Pubkey) -> Option<u8> {
            Some(6)
        }
        fn quote(&self, _input_mint: &Pubkey, _amount_in: u64) -> DirectSwapResult<VenueQuote> {
            unreachable!("not exercised by account resolution")
        }
        fn swap_instruction(
            &self,
            _accounts: &SwapAccounts,
            _amount_in: u64,
            _min_out: u64,
        ) -> DirectSwapResult<Instruction> {
            unreachable!("not exercised by account resolution")
        }
        fn compute_units(&self) -> u32 {
            0
        }
    }

    fn market() -> FakeMarket {
        FakeMarket {
            pool: Pubkey::new_unique(),
            mint_a: Pubkey::new_unique(),
            mint_b: Pubkey::new_unique(),
        }
    }

    #[test]
    fn each_leg_uses_the_token_program_the_pool_reports_for_that_mint() {
        let m = market();
        let owner = Pubkey::new_unique();
        let legs = WalletLegs::resolve(&owner, &m, &m.mint_a, &m.mint_b).unwrap();

        assert_eq!(legs.input_program, spl_token::id());
        assert_eq!(
            legs.output_program,
            spl_token_2022::id(),
            "a Token-2022 leg must not be derived against the legacy program"
        );
        assert_eq!(
            legs.output_account,
            get_associated_token_address_with_program_id(&owner, &m.mint_b, &spl_token_2022::id())
        );
        assert_ne!(
            legs.output_account,
            get_associated_token_address_with_program_id(&owner, &m.mint_b, &spl_token::id()),
            "the two programs derive different addresses -- guessing would target the wrong account"
        );
    }

    #[test]
    fn a_mint_the_pool_does_not_hold_is_a_pair_mismatch_not_a_derived_address() {
        let m = market();
        let owner = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();
        assert!(matches!(
            WalletLegs::resolve(&owner, &m, &m.mint_a, &stranger),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }

    #[test]
    fn both_creations_are_idempotent_so_they_are_safe_to_always_include() {
        let m = market();
        let owner = Pubkey::new_unique();
        let legs = WalletLegs::resolve(&owner, &m, &m.mint_a, &m.mint_b).unwrap();
        let ixs = legs.ensure_instructions(&owner, &owner, &m.mint_a, &m.mint_b);

        assert_eq!(ixs.len(), 2);
        for ix in &ixs {
            assert_eq!(
                ix.data,
                vec![1u8],
                "CreateIdempotent discriminator, not Create"
            );
        }
    }
}
