//! Orca Whirlpool (`whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`) — concentrated
//! liquidity.
//!
//! # Layout, verified against mainnet
//!
//! `Whirlpool` is 653 bytes, cross-checked against the live SOL/USDC pool
//! `Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE` (the deepest Orca pool on
//! mainnet): the mint/vault fields at the offsets below decode to exactly the
//! addresses that pool actually holds.
//!
//! ```text
//!   8 whirlpools_config    41 tick_spacing u16     45 fee_rate u16
//!  47 protocol_fee_rate u16 49 liquidity u128       65 sqrt_price u128
//!  81 tick_current_index i32
//!  85 protocol_fee_owed_a u64  93 protocol_fee_owed_b u64
//! 101 token_mint_a          133 token_vault_a
//! 181 token_mint_b          213 token_vault_b
//! ```
//!
//! `sqrt_price` brackets the tick math exactly (`get_sqrt_price_at_tick`, the
//! same function Raydium CLMM uses — Orca runs the identical Uniswap-v3-style
//! curve): for `tick_current_index = -22315`,
//! `sqrt_price = 6044928832152843188`,
//! `get_sqrt_price_at_tick(-22315) = 6044771392447776916 <= 6044928832152843188
//! < get_sqrt_price_at_tick(-22314) = 6045073623461811934`.
//!
//! # The instruction: `swap`, not `swap_v2`, when both legs are legacy SPL
//!
//! Live mainnet swaps of the pool above carry discriminator `f8c69e91e17587c8`
//! (`sha256("global:swap")[..8]`) over exactly 11 named accounts — the
//! `whirlpool_program` account the IDL lists twelfth is NOT passed:
//!
//! ```text
//! 0 token_program  1 token_authority(signer)  2 whirlpool
//! 3 token_owner_account_a  4 token_vault_a
//! 5 token_owner_account_b  6 token_vault_b
//! 7 tick_array_0  8 tick_array_1  9 tick_array_2
//! 10 oracle
//! ```
//!
//! `sqrt_price_limit = 0` in every observed live transaction regardless of
//! swap direction — the programme treats zero as "no limit" the same way
//! Raydium CLMM does, so `min_out` stays the only protection, exactly as
//! `venues.md` documents for CLMM.
//!
//! `swap_v2` (`2b04ed0b1ac91e62` — the same bytes as Raydium CLMM's, because
//! an Anchor discriminator hashes only the instruction NAME, not the
//! programme) carries `token_program_a`/`token_program_b`/`memo_program` plus
//! both mints ahead of the rest, 15 accounts total, and is required for a
//! Token-2022 leg. This venue picks it automatically when either mint's owner
//! is the Token-2022 programme; that branch is derived from the on-chain IDL
//! and not independently confirmed against a live `swap_v2` transaction — no
//! Token-2022 Whirlpool pool was found to test against in the time available.
//!
//! # Tick arrays have NO on-chain bitmap
//!
//! Unlike Raydium CLMM, `Whirlpool` carries no bitmap field at all. Which tick
//! arrays exist can only be learned by fetching the arithmetically-derived
//! candidates and seeing which ones the RPC actually returns — an account
//! that does not exist is simply left out of the swap, and the walk refuses
//! once it runs out of loaded ticks, the same "loaded tick arrays cover only
//! so much" contract Raydium CLMM already uses.
//!
//! `TickArray` is 9988 bytes: `start_tick_index i32` at 8, 88 `Tick` entries
//! of 113 bytes each starting at offset 12
//! (`12 + 88*113 + 32(whirlpool pubkey) == 9988`, the real fetched account
//! length), NOT Raydium's 60-tick, 168-byte-entry layout. A `Tick` entry has
//! NO explicit tick index field — `initialized: bool` at 0, `liquidity_net:
//! i128` at 1, `liquidity_gross: u128` at 17 — the index is
//! `start_tick_index + i * tick_spacing`, unlike Raydium's `TickState` which
//! stores its own `tick` field.
//!
//! **The tick-array PDA seed is the DECIMAL STRING of `start_tick_index`, not
//! its big-endian bytes.** Raydium CLMM uses `start_index.to_be_bytes()`;
//! Orca's own programme uses `start_tick_index.to_string().as_bytes()`. Live
//! transaction `3fGyaPRos4A7i1U2xhPG97vqea5Foa9Utd763qTuZMTxscd2ugfqKCujxvcACRpjWnX9NPd9y2BagubaUpo8v1Fk`
//! passed tick array `32wMhfqGgeaftnPacPR6pqBPL3agbd7to1oUsqo6y14F` for
//! `start_tick_index = -22528`; `["tick_array", pool, "-22528"]` derives that
//! exact address, while the big-endian-bytes seed derives a different one
//! entirely. Getting this wrong would have silently built an instruction that
//! always fails on deserialisation.
//!
//! The oracle PDA (`["oracle", pool]`) was verified the same way: it derives
//! to the exact `FoKYKtRpD25TKzBMndysKpgPqbj8AdLXjfpYHXn9PGTX` the same live
//! transaction passed.
//!
//! # The fee: `fee_rate` is exact for a classic pool, refused for an adaptive one
//!
//! `fee_rate` at offset 45 is over a denominator of 1_000_000, the same
//! convention as Raydium CLMM (400 == 0.04% on the SOL/USDC pool above).
//! `protocol_fee_rate` only SPLITS the already-collected trade fee between
//! Orca and the LPs — it does not add to what the trader pays, so it plays no
//! part in the quote, mirroring Raydium CLMM's `protocol_fee_rate`.
//!
//! Orca's newer "adaptive fee" pools can charge MORE than `fee_rate` under a
//! volatility-driven formula tracked in a separate `Oracle` PDA
//! (`["oracle", pool]` — the same account plain pools pass but leave
//! uninitialised). `Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE`'s own oracle
//! account does not exist on chain at all, confirming a classic pool's
//! `fee_rate` is the whole fee with nothing adaptive layered on top. When the
//! oracle account DOES exist and its `adaptive_fee_constants.
//! adaptive_fee_control_factor` is non-zero, this venue REFUSES the pool
//! (`PoolNotTradable`) rather than approximate the volatility surcharge —
//! reproducing it exactly would require replaying the pool's tick-crossing
//! history since the oracle's last update, which is not available from a
//! single account read. The `Oracle` account layout itself (`whirlpool` at 8,
//! `trade_enable_timestamp` at 40, `adaptive_fee_constants` at 48,
//! `adaptive_fee_variables` at 82) comes from the on-chain IDL and was not
//! independently confirmed against a live adaptive-fee pool.
//!
//! # What the quote can and cannot promise
//!
//! The quote walks the real tick-by-tick swap loop the programme itself runs,
//! delegated to `clmm_ticks::walk_ticks` — the same program-agnostic
//! constant-liquidity, Q64.64 step math Raydium CLMM's venue uses. A size that
//! would cross beyond the last tick array `load()` fetched is refused outright
//! rather than assumed constant past that point.

use super::clmm_ticks::{ticks_ahead, walk_ticks, InitializedTick};
use super::layout::{
    i128_at, i32_at, mint_decimals, pubkey_at, token_account_amount, u128_at, u16_at,
};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::{MEMO_PROGRAM_ID, ORCA_WHIRLPOOL_PROGRAM_ID};
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

/// `sha256("global:swap")[..8]`, confirmed against live mainnet swaps of the
/// SOL/USDC pool.
const SWAP: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// `sha256("global:swap_v2")[..8]`. Required whenever either leg is
/// Token-2022; not independently confirmed against a live `swap_v2` mainnet
/// transaction (see module docs).
const SWAP_V2: [u8; 8] = [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62];

/// Denominator `fee_rate` is expressed over.
const FEE_RATE_DENOMINATOR: u64 = 1_000_000;

/// Ticks stored per `TickArray` account. Orca's own constant, distinct from
/// Raydium CLMM's 60.
const TICK_ARRAY_SIZE: i32 = 88;

/// Bytes from the start of a `TickArray` account to its first `Tick` entry:
/// an 8-byte Anchor discriminator, then `start_tick_index: i32`.
const TICKS_OFFSET: usize = 12;

/// One `Tick` entry's byte size (`bytemuck`, packed, no padding):
/// `initialized: bool`(1) + `liquidity_net: i128`(16) + `liquidity_gross:
/// u128`(16) + `fee_growth_outside_a: u128`(16) + `fee_growth_outside_b:
/// u128`(16) + `reward_growths_outside: [u128; 3]`(48) = 113 bytes. Verified
/// against a live account: `TICKS_OFFSET + 88*113 + 32 == 9988`, the real
/// fetched `TickArray` length.
const TICK_STATE_SIZE: usize = 113;

/// Tick arrays a swap instruction carries. Orca's own client passes three,
/// matching Raydium CLMM's convention.
const TICK_ARRAYS_PER_SWAP: usize = 3;

/// Compute units a Whirlpool swap needs. Higher than a constant-product venue
/// because the programme may load and cross several tick arrays.
const COMPUTE_UNITS: u32 = 200_000;

const TICK_ARRAY_SEED: &[u8] = b"tick_array";
const ORACLE_SEED: &[u8] = b"oracle";

/// The venue adapter.
pub struct OrcaWhirlpoolVenue;

#[async_trait]
impl PoolVenue for OrcaWhirlpoolVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::OrcaWhirlpool
    }

    fn program_id(&self) -> Pubkey {
        whirlpool_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = WhirlpoolState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "Whirlpool state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if state.liquidity == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: *pool,
                amount_in: 0,
                detail: "the pool has no liquidity in range at the current tick".to_owned(),
            });
        }

        let program = whirlpool_program_id();
        let oracle = oracle_address(&program, pool);

        // No pool-level bitmap exists for a Whirlpool -- unlike Raydium CLMM,
        // the only way to learn which tick arrays are initialised is to fetch
        // the arithmetically-derived candidates and see which the RPC
        // actually returns.
        let mut tick_array_starts: Vec<i32> = Vec::new();
        for zero_for_one in [true, false] {
            for start in
                candidate_tick_array_starts(state.tick_current, state.tick_spacing, zero_for_one)
            {
                if !tick_array_starts.contains(&start) {
                    tick_array_starts.push(start);
                }
            }
        }
        let tick_array_addresses: Vec<Pubkey> = tick_array_starts
            .iter()
            .map(|start| tick_array_address(&program, pool, *start))
            .collect();

        let fixed_addresses = [
            state.vault_a,
            state.vault_b,
            state.mint_a,
            state.mint_b,
            oracle,
        ];
        let mut addresses = fixed_addresses.to_vec();
        addresses.extend(tick_array_addresses.iter().copied());

        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("Whirlpool accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let vault_a_balance = token_account_amount(&required(0)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_vault_a is not a token account".to_owned(),
            }
        })?;
        let vault_b_balance = token_account_amount(&required(1)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_vault_b is not a token account".to_owned(),
            }
        })?;
        let mint_a_account = required(2)?;
        let mint_b_account = required(3)?;
        let token_program_a = mint_a_account.owner;
        let token_program_b = mint_b_account.owner;
        // The Whirlpool account carries no decimals of its own, so they come
        // from the mint accounts read in this same batch -- never from a
        // cache, because these numbers end up in a `min_out`.
        let decimals_a = mint_decimals(&mint_a_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_mint_a is not a mint account".to_owned(),
            }
        })?;
        let decimals_b = mint_decimals(&mint_b_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_mint_b is not a mint account".to_owned(),
            }
        })?;
        let transfer_fee_a = transfer_fee_schedule(mint_a_account);
        let transfer_fee_b = transfer_fee_schedule(mint_b_account);

        // The oracle account is optional: a classic, non-adaptive-fee pool
        // never initialises it. When it DOES exist and carries a non-zero
        // adaptive fee control factor, this venue refuses rather than
        // approximate the volatility surcharge (see module docs).
        if let Some(Some(oracle_account)) = accounts.get(4) {
            let inert = oracle_has_active_adaptive_fee(&oracle_account.data)
                .map(|active| !active)
                .unwrap_or(false);
            if !inert {
                return Err(DirectSwapError::PoolNotTradable {
                    pool: *pool,
                    detail: "this pool's oracle account either drives an adaptive fee or does \
                             not decode -- fee_rate alone cannot be trusted as the whole fee"
                        .to_owned(),
                });
            }
        }

        // Every tick array actually returned by the batch, decoded into its
        // initialised ticks. An array the RPC did not return is simply not
        // there to decode -- it is not a load error, because the swap may
        // never reach it.
        let mut ticks: Vec<InitializedTick> = Vec::new();
        let mut available_tick_array_starts: Vec<i32> = Vec::new();
        for (offset, address) in tick_array_addresses.iter().enumerate() {
            let Some(Some(account)) = accounts.get(fixed_addresses.len() + offset) else {
                continue;
            };
            let start = tick_array_starts[offset];
            available_tick_array_starts.push(start);
            if let Some(decoded) = decode_tick_array(&account.data, start, state.tick_spacing) {
                ticks.extend(decoded);
            } else {
                return Err(DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: format!("tick array {address} did not match the expected layout"),
                });
            }
        }
        ticks.sort_by_key(|tick| tick.tick);
        ticks.dedup_by_key(|tick| tick.tick);

        if available_tick_array_starts.is_empty() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "not one tick array around the current tick exists on chain".to_owned(),
            });
        }

        Ok(Box::new(WhirlpoolMarket {
            state,
            oracle,
            token_program_a,
            token_program_b,
            decimals_a,
            decimals_b,
            vault_a_balance,
            vault_b_balance,
            transfer_fee_a,
            transfer_fee_b,
            ticks,
            available_tick_array_starts,
        }))
    }
}

/// The parts of the `Whirlpool` account a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhirlpoolState {
    pub pool: Pubkey,
    pub tick_spacing: u16,
    pub fee_rate: u16,
    pub liquidity: u128,
    pub sqrt_price: u128,
    pub tick_current: i32,
    pub protocol_fee_owed_a: u64,
    pub protocol_fee_owed_b: u64,
    pub mint_a: Pubkey,
    pub vault_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_b: Pubkey,
}

impl WhirlpoolState {
    /// Decode a `Whirlpool` account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            tick_spacing: u16_at(data, 41)?,
            fee_rate: u16_at(data, 45)?,
            liquidity: u128_at(data, 49)?,
            sqrt_price: u128_at(data, 65)?,
            tick_current: i32_at(data, 81)?,
            protocol_fee_owed_a: super::layout::u64_at(data, 85)?,
            protocol_fee_owed_b: super::layout::u64_at(data, 93)?,
            mint_a: pubkey_at(data, 101)?,
            vault_a: pubkey_at(data, 133)?,
            mint_b: pubkey_at(data, 181)?,
            vault_b: pubkey_at(data, 213)?,
        })
    }
}

/// Whether an `Oracle` account's adaptive fee is active. `None` when the
/// account is too short to carry the field at all, which is NOT the same
/// answer as "inert": an oracle that exists but does not decode is a layout
/// we do not understand, and a `false` there would quote a pool whose real
/// fee may exceed `fee_rate`. The caller refuses on `None` for that reason.
fn oracle_has_active_adaptive_fee(data: &[u8]) -> Option<bool> {
    super::layout::u32_at(data, 54).map(|factor| factor != 0)
}

/// A decoded, quotable Whirlpool.
#[derive(Debug, Clone)]
pub struct WhirlpoolMarket {
    state: WhirlpoolState,
    oracle: Pubkey,
    /// Owner of `mint_a`. The Whirlpool state does not record a token program
    /// per side, so it comes from the mint account itself -- and it must,
    /// because a Token-2022 mint derives a different ATA address than a
    /// legacy one.
    token_program_a: Pubkey,
    /// Owner of `mint_b`.
    token_program_b: Pubkey,
    decimals_a: u8,
    decimals_b: u8,
    vault_a_balance: u64,
    vault_b_balance: u64,
    transfer_fee_a: Option<TransferFeeSchedule>,
    transfer_fee_b: Option<TransferFeeSchedule>,
    /// Initialised ticks from every tick array `load()` fetched, sorted
    /// ascending. Only ticks the swap could plausibly reach were fetched --
    /// this is a bound on how far the walk can speak for, not a full pool
    /// state.
    ticks: Vec<InitializedTick>,
    /// Start indices of the tick arrays that ACTUALLY EXIST on chain, out of
    /// the candidates `load()` derived. A Whirlpool has no bitmap, so this
    /// list is the only record of which of the three positional tick-array
    /// accounts the instruction may name: passing a PDA nothing has ever
    /// initialised fails the programme's own deserialisation, after the
    /// priority fee is paid.
    available_tick_array_starts: Vec<i32>,
}

impl WhirlpoolMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: WhirlpoolState,
        oracle: Pubkey,
        token_program_a: Pubkey,
        token_program_b: Pubkey,
        decimals: (u8, u8),
        vault_a_balance: u64,
        vault_b_balance: u64,
        transfer_fee_a: Option<TransferFeeSchedule>,
        transfer_fee_b: Option<TransferFeeSchedule>,
        mut ticks: Vec<InitializedTick>,
        available_tick_array_starts: Vec<i32>,
    ) -> Self {
        ticks.sort_by_key(|tick| tick.tick);
        ticks.dedup_by_key(|tick| tick.tick);
        Self {
            state,
            oracle,
            token_program_a,
            token_program_b,
            decimals_a: decimals.0,
            decimals_b: decimals.1,
            vault_a_balance,
            vault_b_balance,
            transfer_fee_a,
            transfer_fee_b,
            ticks,
            available_tick_array_starts,
        }
    }

    /// Whether `mint` is the pool's `mint_a` side.
    fn is_side_a(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.mint_a {
            Some(true)
        } else if *mint == self.state.mint_b {
            Some(false)
        } else {
            None
        }
    }

    fn transfer_fee(&self, side_a: bool) -> Option<&TransferFeeSchedule> {
        if side_a {
            self.transfer_fee_a.as_ref()
        } else {
            self.transfer_fee_b.as_ref()
        }
    }

    /// The reserve a side can actually pay out: the vault balance minus the
    /// protocol's own uncollected fee sitting inside it. Both Raydium and
    /// Meteora hold uncollected fees in the vault the same way; quoting off
    /// the raw balance would over-state what the pool can pay.
    fn reserve(&self, side_a: bool) -> u64 {
        if side_a {
            self.vault_a_balance
                .saturating_sub(self.state.protocol_fee_owed_a)
        } else {
            self.vault_b_balance
                .saturating_sub(self.state.protocol_fee_owed_b)
        }
    }

    /// The reserve the OUTPUT side can pay, so a quote never promises more
    /// than the pool physically holds net of its own earmarked fee.
    fn output_reserve(&self, input_is_a: bool) -> u64 {
        self.reserve(!input_is_a)
    }

    /// The three tick-array accounts the instruction names, in the order the
    /// programme walks them.
    ///
    /// `swap` takes tick_array_0/1/2 as REQUIRED positional accounts, not as
    /// optional trailing ones, so all three slots must be filled with an
    /// account that exists. A Whirlpool publishes no bitmap, so the only
    /// record of which candidates are real is what `load()` actually read
    /// back; candidates it did not get are dropped here and the last real
    /// array is repeated to fill the remaining slots. Repeating is safe and
    /// is what Orca's own client does: the programme re-reads the same
    /// account, and the walk has already refused any size that travels
    /// further than the ticks we hold.
    fn tick_array_accounts(&self, zero_for_one: bool) -> Vec<Pubkey> {
        let program = whirlpool_program_id();
        let mut starts: Vec<i32> = candidate_tick_array_starts(
            self.state.tick_current,
            self.state.tick_spacing,
            zero_for_one,
        )
        .into_iter()
        .filter(|start| self.available_tick_array_starts.contains(start))
        .collect();

        let Some(last) = starts.last().copied() else {
            return Vec::new();
        };
        while starts.len() < TICK_ARRAYS_PER_SWAP {
            starts.push(last);
        }

        starts
            .into_iter()
            .take(TICK_ARRAYS_PER_SWAP)
            .map(|start| tick_array_address(&program, &self.state.pool, start))
            .collect()
    }

    fn walk(&self, zero_for_one: bool, amount_in: u64) -> DirectSwapResult<(u64, u64, u128)> {
        let candidates = ticks_ahead(&self.ticks, self.state.tick_current, zero_for_one);
        walk_ticks(
            self.state.pool,
            &candidates,
            self.state.liquidity,
            self.state.sqrt_price,
            self.state.fee_rate as u64,
            FEE_RATE_DENOMINATOR,
            zero_for_one,
            amount_in,
        )
    }
}

impl PoolMarket for WhirlpoolMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::OrcaWhirlpool
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.mint_a, self.state.mint_b)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_side_a(mint).map(|side_a| {
            if side_a {
                self.token_program_a
            } else {
                self.token_program_b
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_side_a(mint).map(|side_a| {
            if side_a {
                self.decimals_a
            } else {
                self.decimals_b
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let input_is_a = self
            .is_side_a(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        let received_by_pool =
            super::token2022::net_of_fee(self.transfer_fee(input_is_a), amount_in);
        if received_by_pool == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the input transfer fee consumes the whole amount at this size".to_owned(),
            });
        }

        // a_to_b (selling A) moves the price down: zero_for_one == input_is_a.
        let (gross_out, lp_fee, sqrt_next) = self.walk(input_is_a, received_by_pool)?;
        if gross_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "fees consume the whole input at this size".to_owned(),
            });
        }

        let expected_out = super::token2022::net_of_fee(self.transfer_fee(!input_is_a), gross_out);
        if expected_out == 0 || expected_out >= self.output_reserve(input_is_a) {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output exceeds what the pool's vault holds".to_owned(),
            });
        }

        // Impact is the move in the SQUARED price, which is the price itself.
        let before = (self.state.sqrt_price as f64) / (2.0_f64).powi(64);
        let after = (sqrt_next as f64) / (2.0_f64).powi(64);
        let price_impact_pct = if before > 0.0 {
            ((before * before - after * after).abs() / (before * before) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        Ok(VenueQuote {
            amount_in,
            expected_out,
            lp_fee,
            price_impact_pct,
        })
    }

    fn swap_instruction(
        &self,
        accounts: &SwapAccounts,
        amount_in: u64,
        min_out: u64,
    ) -> DirectSwapResult<Instruction> {
        let input_is_a =
            self.is_side_a(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_a = self.is_side_a(&accounts.output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            }
        })?;
        if input_is_a == output_is_a {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }

        // Empty means the array holding the current tick does not exist on
        // chain in this direction, so there is nothing valid to name in the
        // three required slots.
        let tick_arrays = self.tick_array_accounts(input_is_a);
        if tick_arrays.len() != TICK_ARRAYS_PER_SWAP {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "no tick array in the swap direction exists on chain".to_owned(),
            });
        }

        let mut data = Vec::with_capacity(42);
        let uses_token_2022 = self.token_program_a == crate::chains::solana::spl_token_2022::id()
            || self.token_program_b == crate::chains::solana::spl_token_2022::id();

        if uses_token_2022 {
            data.extend_from_slice(&SWAP_V2);
        } else {
            data.extend_from_slice(&SWAP);
        }
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());
        // sqrt_price_limit = 0: live transactions use this regardless of
        // direction, and `min_out` is the real protection.
        data.extend_from_slice(&0u128.to_le_bytes());
        // amount_specified_is_input: the amount above is what goes IN.
        data.push(1);
        // a_to_b: true when selling the A side.
        data.push(if input_is_a { 1 } else { 0 });
        if uses_token_2022 {
            // `remaining_accounts_info: Option<RemainingAccountsInfo>` = None.
            data.push(0);
        }

        let metas = if uses_token_2022 {
            let mut m = vec![
                AccountMeta::new_readonly(self.token_program_a, false),
                AccountMeta::new_readonly(self.token_program_b, false),
                AccountMeta::new_readonly(memo_program_id(), false),
                AccountMeta::new_readonly(accounts.owner, true),
                AccountMeta::new(self.state.pool, false),
                AccountMeta::new_readonly(self.state.mint_a, false),
                AccountMeta::new_readonly(self.state.mint_b, false),
                AccountMeta::new(owner_account_a(accounts, input_is_a), false),
                AccountMeta::new(self.state.vault_a, false),
                AccountMeta::new(owner_account_b(accounts, input_is_a), false),
                AccountMeta::new(self.state.vault_b, false),
            ];
            for array in tick_arrays.iter().take(TICK_ARRAYS_PER_SWAP) {
                m.push(AccountMeta::new(*array, false));
            }
            m.push(AccountMeta::new(self.oracle, false));
            m
        } else {
            let mut m = vec![
                AccountMeta::new_readonly(crate::chains::solana::spl_token::id(), false),
                AccountMeta::new_readonly(accounts.owner, true),
                AccountMeta::new(self.state.pool, false),
                AccountMeta::new(owner_account_a(accounts, input_is_a), false),
                AccountMeta::new(self.state.vault_a, false),
                AccountMeta::new(owner_account_b(accounts, input_is_a), false),
                AccountMeta::new(self.state.vault_b, false),
            ];
            for array in tick_arrays.iter().take(TICK_ARRAYS_PER_SWAP) {
                m.push(AccountMeta::new(*array, false));
            }
            m.push(AccountMeta::new_readonly(self.oracle, false));
            m
        };

        Ok(Instruction {
            program_id: whirlpool_program_id(),
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

/// The wallet's own token account paired with `mint_a`, whichever side of the
/// swap it plays.
fn owner_account_a(accounts: &SwapAccounts, input_is_a: bool) -> Pubkey {
    if input_is_a {
        accounts.input_token_account
    } else {
        accounts.output_token_account
    }
}

/// The wallet's own token account paired with `mint_b`.
fn owner_account_b(accounts: &SwapAccounts, input_is_a: bool) -> Pubkey {
    if input_is_a {
        accounts.output_token_account
    } else {
        accounts.input_token_account
    }
}

/// The start index of the tick array containing `tick`. Floor division, not
/// truncation, for the same reason Raydium CLMM's does: `-1 / 88` truncates
/// to `0`, which would put a negative tick just below a boundary into the
/// array above it.
pub fn tick_array_start_index(tick: i32, tick_spacing: u16) -> i32 {
    let span = TICK_ARRAY_SIZE * (tick_spacing as i32);
    if span == 0 {
        return 0;
    }
    let mut index = tick / span;
    if tick < 0 && tick % span != 0 {
        index -= 1;
    }
    index * span
}

/// The tick-array starts a swap in this direction might touch, walking
/// outward from the array holding the current tick. There is no bitmap to
/// consult -- `load()` fetches these candidates and keeps only the ones that
/// actually exist.
pub fn candidate_tick_array_starts(
    tick_current: i32,
    tick_spacing: u16,
    zero_for_one: bool,
) -> Vec<i32> {
    let span = TICK_ARRAY_SIZE * (tick_spacing as i32);
    if span == 0 {
        return Vec::new();
    }
    let step = if zero_for_one { -span } else { span };
    let mut starts = Vec::with_capacity(TICK_ARRAYS_PER_SWAP);
    let mut start = tick_array_start_index(tick_current, tick_spacing);
    for _ in 0..TICK_ARRAYS_PER_SWAP {
        starts.push(start);
        let Some(next) = start.checked_add(step) else {
            break;
        };
        start = next;
    }
    starts
}

/// The PDA of a tick array account. The seed is the DECIMAL STRING of
/// `start_index`, not big-endian bytes -- confirmed against a live
/// transaction (see module docs); Raydium CLMM's own tick arrays use
/// big-endian bytes instead, so this is not a shared helper.
pub fn tick_array_address(program: &Pubkey, pool: &Pubkey, start_index: i32) -> Pubkey {
    Pubkey::find_program_address(
        &[
            TICK_ARRAY_SEED,
            pool.as_ref(),
            start_index.to_string().as_bytes(),
        ],
        program,
    )
    .0
}

/// The PDA of a pool's oracle account.
pub fn oracle_address(program: &Pubkey, pool: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[ORACLE_SEED, pool.as_ref()], program).0
}

/// Decode the initialised ticks out of a live `TickArray` account.
///
/// Returns `None` when the account is too short to hold a full array -- a
/// decode failure, never a partially-wrong swap. Unlike Raydium's
/// `TickArrayState`, a `Tick` entry carries no `pool_id` to cross-check and
/// no explicit tick index, so `start` and `tick_spacing` (both already known
/// from the pool state and the derivation that produced this address) supply
/// the index instead.
pub fn decode_tick_array(
    data: &[u8],
    start: i32,
    tick_spacing: u16,
) -> Option<Vec<InitializedTick>> {
    let mut ticks = Vec::new();
    for i in 0..(TICK_ARRAY_SIZE as usize) {
        let offset = TICKS_OFFSET + i * TICK_STATE_SIZE;
        let initialized = *data.get(offset)?;
        if initialized == 0 {
            continue;
        }
        let liquidity_net = i128_at(data, offset + 1)?;
        let tick = start + (i as i32) * (tick_spacing as i32);
        ticks.push(InitializedTick {
            tick,
            liquidity_net,
        });
    }
    Some(ticks)
}

fn whirlpool_program_id() -> Pubkey {
    Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM_ID).expect("Whirlpool program id constant is valid")
}

fn memo_program_id() -> Pubkey {
    Pubkey::from_str(MEMO_PROGRAM_ID).expect("memo program id constant is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::swaps::direct::venues::clmm_ticks::get_sqrt_price_at_tick;

    fn market(
        liquidity: u128,
        tick_current: i32,
        tick_spacing: u16,
        fee_rate: u16,
        ticks: Vec<InitializedTick>,
    ) -> WhirlpoolMarket {
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();
        let state = WhirlpoolState {
            pool: Pubkey::new_unique(),
            tick_spacing,
            fee_rate,
            liquidity,
            sqrt_price: get_sqrt_price_at_tick(tick_current).expect("tick in range"),
            tick_current,
            protocol_fee_owed_a: 0,
            protocol_fee_owed_b: 0,
            mint_a,
            vault_a: Pubkey::new_unique(),
            mint_b,
            vault_b: Pubkey::new_unique(),
        };
        // Every candidate array counts as present: these tests exercise the
        // walk and the quote, not which accounts the instruction names.
        let mut available: Vec<i32> = Vec::new();
        for zero_for_one in [true, false] {
            for start in candidate_tick_array_starts(tick_current, tick_spacing, zero_for_one) {
                if !available.contains(&start) {
                    available.push(start);
                }
            }
        }
        WhirlpoolMarket::new(
            state,
            Pubkey::new_unique(),
            crate::chains::solana::spl_token::id(),
            crate::chains::solana::spl_token::id(),
            (9, 6),
            u64::MAX,
            u64::MAX,
            None,
            None,
            ticks,
            available,
        )
    }

    #[test]
    fn a_swap_entirely_within_one_liquidity_region_never_crosses_a_tick() {
        let far_tick = InitializedTick {
            tick: -10_000,
            liquidity_net: 0,
        };
        let m = market(1_000_000_000_000, 0, 4, 0, vec![far_tick]);
        let (out, fee, sqrt_end) = m.walk(true, 1_000_000).expect("stays in range");
        assert!(out > 0, "a real swap must produce something");
        assert_eq!(fee, 0, "the fee rate here is zero");
        assert!(
            sqrt_end < m.state.sqrt_price,
            "selling side A must move the price down"
        );
    }

    #[test]
    fn crossing_a_tick_where_liquidity_drops_yields_less_than_constant_liquidity_would() {
        let crossed = InitializedTick {
            tick: -20,
            liquidity_net: 400_000_000_000,
        };
        let safety_net = InitializedTick {
            tick: -100_000,
            liquidity_net: 0,
        };
        let initial_liquidity = 1_000_000_000_000u128;
        let m = market(initial_liquidity, 0, 4, 0, vec![crossed, safety_net]);

        let amount_to_cross = super::super::clmm_ticks::input_for_move(
            true,
            initial_liquidity,
            m.state.sqrt_price,
            get_sqrt_price_at_tick(-20).unwrap(),
        )
        .unwrap();
        let amount_in = (amount_to_cross + 5_000_000_000) as u64;

        let (out, _fee, sqrt_end) = m
            .walk(true, amount_in)
            .expect("covered by the safety net tick");
        let naive_out = super::super::clmm_ticks::output_for_move(
            true,
            initial_liquidity,
            m.state.sqrt_price,
            sqrt_end,
        )
        .unwrap();

        assert!(
            out < naive_out,
            "crossing into thinner liquidity must yield less than a constant-liquidity \
             estimate would have promised: real {out} vs naive {naive_out}"
        );
    }

    #[test]
    fn a_swap_that_exhausts_every_loaded_tick_array_refuses_rather_than_guesses() {
        let only_tick = InitializedTick {
            tick: -20,
            liquidity_net: 100,
        };
        let m = market(1_000_000_000_000, 0, 4, 0, vec![only_tick]);
        let amount_to_cross = super::super::clmm_ticks::input_for_move(
            true,
            1_000_000_000_000,
            m.state.sqrt_price,
            get_sqrt_price_at_tick(-20).unwrap(),
        )
        .unwrap();
        let amount_in = (amount_to_cross * 10) as u64;

        let err = m
            .walk(true, amount_in)
            .expect_err("a size beyond every loaded tick must not be quoted");
        match err {
            DirectSwapError::InsufficientLiquidity { detail, .. } => {
                assert!(
                    detail.contains("loaded tick arrays"),
                    "the refusal must name the real limit, got: {detail}"
                );
            }
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn the_tick_array_pda_seed_is_a_decimal_string_not_big_endian_bytes() {
        // Live transaction
        // 3fGyaPRos4A7i1U2xhPG97vqea5Foa9Utd763qTuZMTxscd2ugfqKCujxvcACRpjWnX9NPd9y2BagubaUpo8v1Fk
        // passed this exact address for start_tick_index = -22528 on pool
        // Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE.
        let program = Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM_ID).unwrap();
        let pool = Pubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE").unwrap();
        let expected = Pubkey::from_str("32wMhfqGgeaftnPacPR6pqBPL3agbd7to1oUsqo6y14F").unwrap();
        assert_eq!(tick_array_address(&program, &pool, -22528), expected);
    }

    #[test]
    fn the_oracle_pda_matches_a_live_transactions_account() {
        let program = Pubkey::from_str(ORCA_WHIRLPOOL_PROGRAM_ID).unwrap();
        let pool = Pubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE").unwrap();
        let expected = Pubkey::from_str("FoKYKtRpD25TKzBMndysKpgPqbj8AdLXjfpYHXn9PGTX").unwrap();
        assert_eq!(oracle_address(&program, &pool), expected);
    }

    #[test]
    fn a_tick_array_start_index_floors_towards_negative_infinity() {
        assert_eq!(tick_array_start_index(-22_315, 4), -22_528);
        assert_eq!(tick_array_start_index(0, 4), 0);
        assert_eq!(tick_array_start_index(-1, 4), -352);
    }

    #[test]
    fn an_active_adaptive_fee_control_factor_is_detected() {
        let mut data = vec![0u8; 254];
        data[54..58].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(oracle_has_active_adaptive_fee(&data), Some(true));
    }

    #[test]
    fn a_zero_adaptive_fee_control_factor_is_inert() {
        let data = vec![0u8; 254];
        assert_eq!(oracle_has_active_adaptive_fee(&data), Some(false));
    }

    #[test]
    fn an_oracle_too_short_to_decode_is_unknown_rather_than_inert() {
        // The load path refuses on this, because an oracle we cannot read
        // might be charging more than `fee_rate`.
        assert_eq!(oracle_has_active_adaptive_fee(&[0u8; 10]), None);
    }
}
