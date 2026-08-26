//! Raydium AMM v4 (`675kPX9M…`) — the original "Standard" constant-product pool.
//!
//! # Layout, verified against mainnet
//!
//! `AmmInfo` is 752 bytes:
//!
//! ```text
//!   0 status u64        32 coin_decimals u64   40 pc_decimals u64
//! 144 swap_fee?         (see the fee block below)
//! 192 need_take_pnl_coin 200 need_take_pnl_pc
//! 336 coin_vault        368 pc_vault
//! 400 coin_mint         432 pc_mint            464 lp_mint
//! 496 open_orders       528 market             560 market_program
//! 592 target_orders
//! ```
//!
//! The fee block at 128 is eight `u64`s: `min_separate_numerator/denominator`,
//! `trade_fee_numerator/denominator`, `pnl_numerator/denominator`,
//! `swap_fee_numerator/denominator`. The swap charges the LAST pair (offsets
//! 176/184), which is 25/10000 on a standard pool.
//!
//! # The OpenBook accounts
//!
//! The instruction's canonical shape lists eight OpenBook/Serum accounts and an
//! `amm_target_orders`. Modern Raydium routing does not use the order book, and
//! the programme accepts the 17-account form in which `amm_target_orders` is
//! dropped and `open_orders` plus all eight market accounts are passed as the
//! POOL address itself. That is the shape observed on live mainnet swaps of the
//! SOL/USDC pool, and it is what this venue builds: it needs no market lookup, so
//! a pool whose OpenBook market is dead still trades.
//!
//! # The curve
//!
//! Tradable reserves are `vault − need_take_pnl` per side. `need_take_pnl` is
//! profit already earmarked for the pool's owner and sitting in the vault; it is
//! not swappable, and quoting off the raw vault over-states both reserves.

use super::layout::{pubkey_at, token_account_amount, u64_at};
use super::math::{constant_product_out, fee_amount, price_impact_pct};
use crate::chains::solana::constants::RAYDIUM_LEGACY_AMM_PROGRAM_ID;
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
};
use async_trait::async_trait;
use std::str::FromStr;

/// Instruction tag for `swapBaseIn`. AMM v4 predates Anchor: the discriminator is
/// a single byte, not an eight-byte hash.
const SWAP_BASE_IN_TAG: u8 = 9;

/// The programme's single global authority PDA, derived once from a fixed seed.
const AUTHORITY_SEED: &[u8] = &[97, 109, 109, 32, 97, 117, 116, 104, 111, 114, 105, 116, 121]; // "amm authority"

/// Status values that permit swapping. 1 = Initialized, 6 = SwapOnly,
/// 7 = WaitingTrade.
const SWAPPABLE_STATUSES: [u64; 3] = [1, 6, 7];

/// Compute units an AMM v4 swap needs.
const COMPUTE_UNITS: u32 = 100_000;

/// The venue adapter.
pub struct RaydiumAmmV4Venue;

#[async_trait]
impl PoolVenue for RaydiumAmmV4Venue {
    fn program(&self) -> ProgramKind {
        ProgramKind::RaydiumLegacyAmm
    }

    fn program_id(&self) -> Pubkey {
        Pubkey::from_str(RAYDIUM_LEGACY_AMM_PROGRAM_ID)
            .expect("AMM v4 program id constant is valid")
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = AmmV4PoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "AMM v4 pool state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if !state.swap_enabled() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: format!("status {} does not permit swapping", state.status),
            });
        }

        let addresses = [state.coin_vault, state.pc_vault];
        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("AMM v4 vaults could not be read: {e}"),
            })?;

        let balance = |index: usize| -> DirectSwapResult<u64> {
            let account = accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "vault does not exist".to_owned(),
                },
            )?;
            token_account_amount(&account.data).ok_or(DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "vault is not a token account".to_owned(),
            })
        };

        Ok(Box::new(AmmV4Market {
            state,
            coin_balance: balance(0)?,
            pc_balance: balance(1)?,
        }))
    }
}

/// The parts of `AmmInfo` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmV4PoolState {
    pub pool: Pubkey,
    pub status: u64,
    pub coin_decimals: u8,
    pub pc_decimals: u8,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub need_take_pnl_coin: u64,
    pub need_take_pnl_pc: u64,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market: Pubkey,
    pub market_program: Pubkey,
    pub target_orders: Pubkey,
}

impl AmmV4PoolState {
    /// Decode an `AmmInfo` account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            status: u64_at(data, 0)?,
            coin_decimals: u64_at(data, 32)?.min(u8::MAX as u64) as u8,
            pc_decimals: u64_at(data, 40)?.min(u8::MAX as u64) as u8,
            swap_fee_numerator: u64_at(data, 176)?,
            swap_fee_denominator: u64_at(data, 184)?,
            need_take_pnl_coin: u64_at(data, 192)?,
            need_take_pnl_pc: u64_at(data, 200)?,
            coin_vault: pubkey_at(data, 336)?,
            pc_vault: pubkey_at(data, 368)?,
            coin_mint: pubkey_at(data, 400)?,
            pc_mint: pubkey_at(data, 432)?,
            open_orders: pubkey_at(data, 496)?,
            market: pubkey_at(data, 528)?,
            market_program: pubkey_at(data, 560)?,
            target_orders: pubkey_at(data, 592)?,
        })
    }

    /// Whether the pool's status permits swapping.
    pub fn swap_enabled(&self) -> bool {
        SWAPPABLE_STATUSES.contains(&self.status)
    }
}

/// A decoded, quotable AMM v4 pool.
#[derive(Debug, Clone)]
pub struct AmmV4Market {
    state: AmmV4PoolState,
    coin_balance: u64,
    pc_balance: u64,
}

impl AmmV4Market {
    /// Build a market directly from decoded parts, for the offline test tier.
    pub fn new(state: AmmV4PoolState, coin_balance: u64, pc_balance: u64) -> Self {
        Self {
            state,
            coin_balance,
            pc_balance,
        }
    }

    /// Swappable reserves: vaults less the profit earmarked out of them.
    pub fn reserves(&self) -> (u64, u64) {
        (
            self.coin_balance
                .saturating_sub(self.state.need_take_pnl_coin),
            self.pc_balance.saturating_sub(self.state.need_take_pnl_pc),
        )
    }

    /// Whether `mint` is the coin (base) side.
    fn is_coin(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.coin_mint {
            Some(true)
        } else if *mint == self.state.pc_mint {
            Some(false)
        } else {
            None
        }
    }
}

impl PoolMarket for AmmV4Market {
    fn program(&self) -> ProgramKind {
        ProgramKind::RaydiumLegacyAmm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.coin_mint, self.state.pc_mint)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        // AMM v4 predates Token-2022 and only ever holds legacy SPL mints.
        self.is_coin(mint)
            .map(|_| crate::chains::solana::spl_token::id())
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_coin(mint).map(|coin| {
            if coin {
                self.state.coin_decimals
            } else {
                self.state.pc_decimals
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_coin = self
            .is_coin(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        let (coin_reserve, pc_reserve) = self.reserves();
        let (reserve_in, reserve_out) = if input_is_coin {
            (coin_reserve, pc_reserve)
        } else {
            (pc_reserve, coin_reserve)
        };
        if reserve_in == 0 || reserve_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "a vault side is empty once earmarked profit is excluded".to_owned(),
            });
        }

        let lp_fee = fee_amount(
            amount_in,
            self.state.swap_fee_numerator,
            self.state.swap_fee_denominator,
        );
        let swappable = amount_in.saturating_sub(lp_fee);
        let expected_out = constant_product_out(reserve_in, reserve_out, swappable);

        Ok(VenueQuote {
            amount_in,
            expected_out,
            lp_fee,
            price_impact_pct: price_impact_pct(reserve_in, reserve_out, amount_in, expected_out),
        })
    }

    fn swap_instruction(
        &self,
        accounts: &SwapAccounts,
        amount_in: u64,
        min_out: u64,
    ) -> DirectSwapResult<Instruction> {
        let input_is_coin =
            self.is_coin(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_coin =
            self.is_coin(&accounts.output_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        if input_is_coin == output_is_coin {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        let authority = Pubkey::find_program_address(&[AUTHORITY_SEED], &amm_v4_program_id()).0;

        let mut data = Vec::with_capacity(17);
        data.push(SWAP_BASE_IN_TAG);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());

        // The vaults are passed in the pool's OWN order (coin then pc), not in
        // swap direction -- the programme reads the direction from which user
        // account is the source. Reordering them here silently inverts the trade.
        let mut metas = vec![
            AccountMeta::new_readonly(crate::chains::solana::spl_token::id(), false),
            AccountMeta::new(self.state.pool, false),
            AccountMeta::new_readonly(authority, false),
            AccountMeta::new(self.state.pool, false),
            AccountMeta::new(self.state.coin_vault, false),
            AccountMeta::new(self.state.pc_vault, false),
        ];
        // The eight OpenBook slots, filled with the pool address. See the module
        // header: the programme tolerates this and it removes the market lookup.
        for _ in 0..8 {
            metas.push(AccountMeta::new(self.state.pool, false));
        }
        metas.push(AccountMeta::new(accounts.input_token_account, false));
        metas.push(AccountMeta::new(accounts.output_token_account, false));
        metas.push(AccountMeta::new_readonly(accounts.owner, true));

        Ok(Instruction {
            program_id: amm_v4_program_id(),
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

fn amm_v4_program_id() -> Pubkey {
    Pubkey::from_str(RAYDIUM_LEGACY_AMM_PROGRAM_ID).expect("AMM v4 program id constant is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AmmV4PoolState {
        AmmV4PoolState {
            pool: Pubkey::new_unique(),
            status: 6,
            coin_decimals: 9,
            pc_decimals: 6,
            swap_fee_numerator: 25,
            swap_fee_denominator: 10_000,
            need_take_pnl_coin: 0,
            need_take_pnl_pc: 0,
            coin_vault: Pubkey::new_unique(),
            pc_vault: Pubkey::new_unique(),
            coin_mint: Pubkey::new_unique(),
            pc_mint: Pubkey::new_unique(),
            open_orders: Pubkey::new_unique(),
            market: Pubkey::new_unique(),
            market_program: Pubkey::new_unique(),
            target_orders: Pubkey::new_unique(),
        }
    }

    fn market() -> AmmV4Market {
        AmmV4Market::new(state(), 1_000_000_000_000, 1_000_000_000_000)
    }

    #[test]
    fn the_authority_seed_derives_raydiums_published_authority() {
        let authority = Pubkey::find_program_address(&[AUTHORITY_SEED], &amm_v4_program_id()).0;
        assert_eq!(
            authority.to_string(),
            "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
            "this is the authority every live AMM v4 swap passes at index 2"
        );
    }

    #[test]
    fn only_the_swappable_statuses_are_tradable() {
        for status in [1u64, 6, 7] {
            let s = AmmV4PoolState { status, ..state() };
            assert!(s.swap_enabled(), "status {status} should permit swapping");
        }
        for status in [0u64, 2, 3, 4, 5] {
            let s = AmmV4PoolState { status, ..state() };
            assert!(
                !s.swap_enabled(),
                "status {status} must not permit swapping"
            );
        }
    }

    #[test]
    fn reserves_exclude_the_profit_earmarked_inside_the_vaults() {
        let s = AmmV4PoolState {
            need_take_pnl_coin: 500,
            need_take_pnl_pc: 700,
            ..state()
        };
        let m = AmmV4Market::new(s, 1_000, 2_000);
        assert_eq!(m.reserves(), (500, 1_300));
    }

    #[test]
    fn a_quote_charges_the_pools_own_swap_fee_rate() {
        let m = market();
        let (coin, _) = m.mints();
        let q = m.quote(&coin, 1_000_000_000).expect("quote");
        assert_eq!(q.lp_fee, 2_500_000, "25/10000 of the input");
        assert!(q.expected_out < 997_500_000);
        assert!(q.expected_out > 995_000_000);
    }

    #[test]
    fn a_pool_with_a_zero_fee_denominator_charges_nothing_rather_than_dividing_by_zero() {
        let s = AmmV4PoolState {
            swap_fee_denominator: 0,
            ..state()
        };
        let m = AmmV4Market::new(s, 1_000_000_000_000, 1_000_000_000_000);
        let q = m.quote(&s.coin_mint, 1_000).expect("quote");
        assert_eq!(q.lp_fee, 0);
    }

    #[test]
    fn the_instruction_has_the_seventeen_account_shape_live_swaps_use() {
        let m = market();
        let (coin, pc) = m.mints();
        let owner = Pubkey::new_unique();
        let src = Pubkey::new_unique();
        let dst = Pubkey::new_unique();
        let ix = m
            .swap_instruction(
                &SwapAccounts {
                    owner,
                    input_mint: coin,
                    output_mint: pc,
                    input_token_account: src,
                    output_token_account: dst,
                },
                5_000_000,
                4_900_000,
            )
            .expect("builds");

        assert_eq!(ix.program_id, amm_v4_program_id());
        assert_eq!(ix.accounts.len(), 17);
        assert_eq!(
            ix.accounts[0].pubkey,
            crate::chains::solana::spl_token::id()
        );
        assert_eq!(ix.accounts[1].pubkey, m.state.pool);
        assert_eq!(ix.accounts[4].pubkey, m.state.coin_vault);
        assert_eq!(ix.accounts[5].pubkey, m.state.pc_vault);
        for i in 6..14 {
            assert_eq!(
                ix.accounts[i].pubkey, m.state.pool,
                "the OpenBook slots are filled with the pool itself"
            );
        }
        assert_eq!(ix.accounts[14].pubkey, src);
        assert_eq!(ix.accounts[15].pubkey, dst);
        assert_eq!(ix.accounts[16].pubkey, owner);
        assert!(ix.accounts[16].is_signer);
    }

    #[test]
    fn the_instruction_data_is_the_tag_then_amount_then_minimum() {
        let m = market();
        let (coin, pc) = m.mints();
        let ix = m
            .swap_instruction(
                &SwapAccounts {
                    owner: Pubkey::new_unique(),
                    input_mint: coin,
                    output_mint: pc,
                    input_token_account: Pubkey::new_unique(),
                    output_token_account: Pubkey::new_unique(),
                },
                5_000_000,
                4_900_000,
            )
            .expect("builds");

        assert_eq!(ix.data.len(), 17);
        assert_eq!(ix.data[0], SWAP_BASE_IN_TAG);
        assert_eq!(
            u64::from_le_bytes(ix.data[1..9].try_into().unwrap()),
            5_000_000
        );
        assert_eq!(
            u64::from_le_bytes(ix.data[9..17].try_into().unwrap()),
            4_900_000
        );
    }

    #[test]
    fn reversing_the_pair_keeps_the_vaults_in_pool_order_and_swaps_the_user_accounts() {
        let m = market();
        let (coin, pc) = m.mints();
        let src = Pubkey::new_unique();
        let dst = Pubkey::new_unique();
        let ix = m
            .swap_instruction(
                &SwapAccounts {
                    owner: Pubkey::new_unique(),
                    input_mint: pc,
                    output_mint: coin,
                    input_token_account: dst,
                    output_token_account: src,
                },
                1_000,
                900,
            )
            .expect("builds");

        assert_eq!(
            ix.accounts[4].pubkey, m.state.coin_vault,
            "vault order is fixed by the pool, never by direction"
        );
        assert_eq!(ix.accounts[5].pubkey, m.state.pc_vault);
        assert_eq!(
            ix.accounts[14].pubkey, dst,
            "the user source follows direction"
        );
        assert_eq!(ix.accounts[15].pubkey, src);
    }

    #[test]
    fn a_truncated_pool_account_decodes_to_none_instead_of_panicking() {
        assert!(AmmV4PoolState::decode(Pubkey::new_unique(), &[0u8; 300]).is_none());
    }
}
