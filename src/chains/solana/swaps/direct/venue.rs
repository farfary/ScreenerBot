//! The contract every direct-swap venue implements.
//!
//! A venue is one DEX program. It is split deliberately into two halves:
//!
//! * [`PoolVenue`] — the async half. It knows which accounts a pool needs and
//!   reads them from chain. This is the only part that touches RPC.
//! * [`PoolMarket`] — the PURE half. Given decoded bytes it answers what the pool
//!   trades, what a swap would return, and what the swap instruction looks like.
//!   No RPC, no clock, no config. That is what makes the offline test tier able
//!   to assert real numbers against real captured account data.
//!
//! Adding a DEX means implementing this pair in `venues/` and registering it in
//! `super::registry` — nothing else in the engine changes.

use super::error::DirectSwapResult;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::{
    account::Account, instruction::Instruction, pubkey::Pubkey,
};
use async_trait::async_trait;
use std::fmt::Debug;

/// What a venue's own math says a swap returns, before slippage and before the
/// platform fee. All amounts are raw units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VenueQuote {
    /// Amount routed into the pool.
    pub amount_in: u64,
    /// Amount the pool returns for it, by the venue's exact curve.
    pub expected_out: u64,
    /// The POOL's own trade fee taken on the input, in input raw units. This is
    /// the LP/protocol fee, entirely separate from our platform fee.
    pub lp_fee: u64,
    /// Price impact of this size, as a percentage (2.5 means 2.5%).
    pub price_impact_pct: f64,
}

/// The wallet-side accounts a venue needs to orient its swap instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapAccounts {
    /// Wallet that signs.
    pub owner: Pubkey,
    /// Mint being spent.
    pub input_mint: Pubkey,
    /// Mint being received.
    pub output_mint: Pubkey,
    /// Owner's token account for `input_mint`.
    pub input_token_account: Pubkey,
    /// Owner's token account for `output_mint`.
    pub output_token_account: Pubkey,
}

/// A decoded, quotable pool. Pure: every method is deterministic in the decoded
/// state, so the same inputs always produce the same instruction bytes.
pub trait PoolMarket: Send + Sync + Debug {
    /// Which DEX program this market belongs to.
    fn program(&self) -> ProgramKind;

    /// The pool account address.
    fn pool(&self) -> Pubkey;

    /// The two mints this pool trades, in the pool's own on-chain order.
    fn mints(&self) -> (Pubkey, Pubkey);

    /// The SPL token program that owns `mint` (legacy or Token-2022), if the
    /// mint is one of this pool's two.
    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey>;

    /// Decimals of `mint`, if the mint is one of this pool's two.
    fn decimals(&self, mint: &Pubkey) -> Option<u8>;

    /// Whether this pool can trade `input_mint` for `output_mint`. Checked before
    /// anything is built: a pool that came from a mint-keyed lookup is not
    /// guaranteed to hold the OTHER leg of the pair.
    fn trades(&self, input_mint: &Pubkey, output_mint: &Pubkey) -> bool {
        let (a, b) = self.mints();
        (*input_mint == a && *output_mint == b) || (*input_mint == b && *output_mint == a)
    }

    /// Exact output for `amount_in` of `input_mint`, by this venue's curve.
    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote>;

    /// The swap instruction. `min_out` is enforced on chain — it is the only
    /// protection the transaction has once it is submitted.
    fn swap_instruction(
        &self,
        accounts: &SwapAccounts,
        amount_in: u64,
        min_out: u64,
    ) -> DirectSwapResult<Instruction>;

    /// Compute units to request for this venue's swap. A venue that walks tick
    /// arrays needs far more than a constant-product one, and an under-request
    /// fails the transaction after the fee is paid.
    fn compute_units(&self) -> u32;
}

/// The async half: reads a pool's accounts and hands back a pure [`PoolMarket`].
#[async_trait]
pub trait PoolVenue: Send + Sync {
    /// The pool kind this venue serves.
    fn program(&self) -> ProgramKind;

    /// The on-chain program that owns pools of this kind.
    fn program_id(&self) -> Pubkey;

    /// Fetch whatever else this venue needs (config, vaults, tick arrays) and
    /// decode everything into a market. `pool_account` is already loaded so the
    /// dispatcher can identify the venue by owner without a second read.
    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>>;
}
