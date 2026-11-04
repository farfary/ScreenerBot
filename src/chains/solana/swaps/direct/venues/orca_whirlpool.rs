// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Orca Whirlpool (`whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`) — concentrated
//! liquidity.
//!
//! The `Whirlpool`, `TickArray`, `DynamicTickArray` and `Oracle` layouts, their
//! byte-offset tables and how they were verified live in
//! `crate::chains::solana::pools::layouts::orca_whirlpool`.
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
//! Unlike Raydium CLMM, `Whirlpool` carries no bitmap field at all, and it
//! needs none: the programme's `SparseSwapTickSequenceBuilder` (used by both
//! `swap` and `swap_v2`) derives the three start indices a swap walks from the
//! current tick alone — `[0, -1, -2]` arrays away on a→b, `[0, 1, 2]` on b→a,
//! or `[1, 2, 3]` when `tick_current + tick_spacing` already reaches the next
//! array — and drops any start outside the valid tick range. For each start it
//! takes the initialised account, OR the un-created PDA passed in its place,
//! which it treats as an all-zero array; the first start with neither ends
//! the sequence. So every slot names the derived PDA whether or not the
//! account exists: an absent array holds no initialised tick, and liquidity is
//! constant across it.
//!
//! Inside that sequence the programme steps to the next initialised tick, and
//! when none is left, to the sequence's own end — the last array's start on
//! a→b, its start plus `88 × tick_spacing - 1` on b→a, the global min or max
//! tick when the last array holds it — with liquidity unchanged. Input still
//! left there fails the swap (`TickArraySequenceInvalidIndex`), so the quote
//! walks the same ticks, ends at the same place and refuses the same sizes
//! (`swap_tick_array_starts`, `sequence_end_tick`).
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
//! single account read.
//!
//! # What the quote can and cannot promise
//!
//! The quote walks the real tick-by-tick swap loop the programme itself runs,
//! delegated to `clmm_ticks::walk_ticks` — the same program-agnostic
//! constant-liquidity, Q64.64 step math Raydium CLMM's venue uses. A size that
//! would travel past the end of the three-array sequence is refused outright,
//! exactly where the programme itself would fail it.

use super::clmm_ticks::{ticks_ahead, walk_ticks, MAX_TICK, MIN_TICK};
use super::token2022::{refuse_transfer_hook, transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::{MEMO_PROGRAM_ID, ORCA_WHIRLPOOL_PROGRAM_ID};
use crate::chains::solana::layout::{mint_decimals, token_account_amount};
use crate::chains::solana::pools::layouts::clmm_ticks::InitializedTick;
use crate::chains::solana::pools::layouts::orca_whirlpool::{
    decode_tick_array, oracle_has_active_adaptive_fee, WhirlpoolState, TICK_ARRAY_SIZE,
};
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    AccountReader, PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
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
        reader: &dyn AccountReader,
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

        // The arrays either direction's swap walks, fetched for their
        // initialised ticks; one that does not exist holds none.
        let mut tick_array_starts: Vec<i32> = Vec::new();
        for zero_for_one in [true, false] {
            for start in
                swap_tick_array_starts(state.tick_current, state.tick_spacing, zero_for_one)
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

        let accounts = reader.read_accounts(&addresses).await?;

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
        refuse_transfer_hook(pool, &[mint_a_account, mint_b_account])?;
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

        // Every tick array the batch returned, decoded into its initialised
        // ticks. An array that does not exist is the programme's zeroed
        // array, not a load error.
        let mut ticks: Vec<InitializedTick> = Vec::new();
        for (offset, address) in tick_array_addresses.iter().enumerate() {
            let Some(Some(account)) = accounts.get(fixed_addresses.len() + offset) else {
                continue;
            };
            let start = tick_array_starts[offset];
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
        }))
    }
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
    /// Initialised ticks from the tick arrays of both directions' swap
    /// sequences, sorted ascending: every tick a swap from the current tick
    /// can reach, and nothing beyond it.
    ticks: Vec<InitializedTick>,
}

impl WhirlpoolMarket {
    /// Build a market directly from decoded parts, for this module's unit tests.
    #[cfg(test)]
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

    /// The three tick-array accounts the instruction names: the PDA of every
    /// start in the swap's sequence, whether or not the account exists (the
    /// programme reads an un-created PDA as a zeroed array; see module docs).
    ///
    /// Near the edge of the tick range the sequence holds fewer than three
    /// valid starts; the last one is repeated to fill the required positional
    /// slots, which the programme de-duplicates by key.
    fn tick_array_accounts(&self, zero_for_one: bool) -> Vec<Pubkey> {
        let program = whirlpool_program_id();
        let mut starts = swap_tick_array_starts(
            self.state.tick_current,
            self.state.tick_spacing,
            zero_for_one,
        );

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

    /// Walk the swap over exactly the ticks the programme can reach: the
    /// initialised ticks inside the sequence's arrays, then the sequence's
    /// own end, which carries no liquidity change.
    fn walk(&self, zero_for_one: bool, amount_in: u64) -> DirectSwapResult<(u64, u64, u128)> {
        let starts = swap_tick_array_starts(
            self.state.tick_current,
            self.state.tick_spacing,
            zero_for_one,
        );
        let (Some(&first), Some(&last)) = (starts.first(), starts.last()) else {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the current tick leaves no tick array to swap through".to_owned(),
            });
        };
        let span = TICK_ARRAY_SIZE * (self.state.tick_spacing as i32);
        let (low, high) = (first.min(last), first.max(last).saturating_add(span));

        let mut candidates: Vec<InitializedTick> =
            ticks_ahead(&self.ticks, self.state.tick_current, zero_for_one)
                .into_iter()
                .filter(|tick| tick.tick >= low && tick.tick < high)
                .collect();
        let end = sequence_end_tick(last, self.state.tick_spacing, zero_for_one);
        if candidates.last().map(|tick| tick.tick) != Some(end) {
            candidates.push(InitializedTick {
                tick: end,
                liquidity_net: 0,
            });
        }

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

        // Empty only when the current tick leaves no valid start in this
        // direction, so there is nothing to name in the three required slots.
        let tick_arrays = self.tick_array_accounts(input_is_a);
        if tick_arrays.len() != TICK_ARRAYS_PER_SWAP {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the current tick leaves no tick array to swap through".to_owned(),
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

/// The tick-array starts a swap in this direction walks, in order: the
/// programme's own `get_start_tick_indexes`. On b→a the sequence begins one
/// array further up when `tick_current + tick_spacing` already reaches the
/// next array, because no tick above the current one is left in its own.
/// Starts outside the valid range are dropped, so the edge of the tick range
/// yields fewer than three.
pub fn swap_tick_array_starts(
    tick_current: i32,
    tick_spacing: u16,
    zero_for_one: bool,
) -> Vec<i32> {
    let span = TICK_ARRAY_SIZE * (tick_spacing as i32);
    if span == 0 {
        return Vec::new();
    }
    let base = tick_array_start_index(tick_current, tick_spacing);
    let offsets: [i32; TICK_ARRAYS_PER_SWAP] = if zero_for_one {
        [0, -1, -2]
    } else if tick_current.saturating_add(tick_spacing as i32) >= base.saturating_add(span) {
        [1, 2, 3]
    } else {
        [0, 1, 2]
    };
    let lowest_valid = tick_array_start_index(MIN_TICK, tick_spacing);
    offsets
        .iter()
        .filter_map(|offset| base.checked_add(offset.checked_mul(span)?))
        .filter(|start| (lowest_valid..=MAX_TICK).contains(start))
        .collect()
}

/// Where the programme's walk stops when no initialised tick is left in the
/// sequence whose last array starts at `last_start`: that array's first tick
/// on a→b, its last tick on b→a, or the global min or max tick when the array
/// holds it.
pub fn sequence_end_tick(last_start: i32, tick_spacing: u16, zero_for_one: bool) -> i32 {
    let span = TICK_ARRAY_SIZE * (tick_spacing as i32);
    if zero_for_one {
        if last_start <= MIN_TICK {
            MIN_TICK
        } else {
            last_start
        }
    } else if last_start.saturating_add(span) > MAX_TICK {
        MAX_TICK
    } else {
        last_start + span - 1
    }
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
        )
    }

    /// The input that moves the price from `m`'s current tick to `tick` at
    /// the pool's liquidity, ignoring fees.
    fn input_to(m: &WhirlpoolMarket, zero_for_one: bool, tick: i32) -> u128 {
        super::super::clmm_ticks::input_for_move(
            zero_for_one,
            m.state.liquidity,
            m.state.sqrt_price,
            get_sqrt_price_at_tick(tick).unwrap(),
        )
        .unwrap()
    }

    fn refusal_detail(result: DirectSwapResult<(u64, u64, u128)>) -> String {
        match result.expect_err("the size must be refused") {
            DirectSwapError::InsufficientLiquidity { detail, .. } => detail,
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn a_swap_entirely_within_one_liquidity_region_never_crosses_a_tick() {
        let m = market(1_000_000_000_000, 0, 4, 0, Vec::new());
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
        let initial_liquidity = 1_000_000_000_000u128;
        let m = market(initial_liquidity, 0, 4, 0, vec![crossed]);
        let amount_in = (input_to(&m, true, -20) + 5_000_000_000) as u64;

        let (out, _fee, sqrt_end) = m
            .walk(true, amount_in)
            .expect("the size ends inside the swap's tick arrays");
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
    fn a_swap_with_no_initialised_tick_ahead_walks_to_the_end_of_its_arrays() {
        // A live JUP/SOL pool held three existing but empty arrays below its
        // tick: liquidity is constant across them, and the programme swaps
        // through them to the sequence's end.
        let m = market(7_811_957_552, 10_014, 16, 0, Vec::new());
        let starts = swap_tick_array_starts(10_014, 16, true);
        assert_eq!(starts, vec![9_856, 8_448, 7_040]);
        let end = sequence_end_tick(7_040, 16, true);

        let inside = (input_to(&m, true, end) - 1) as u64;
        let (_, _, sqrt_end) = m.walk(true, inside).expect("ends before the sequence does");
        assert!(sqrt_end > get_sqrt_price_at_tick(end).unwrap());

        let beyond = (input_to(&m, true, end) + 1_000) as u64;
        assert!(refusal_detail(m.walk(true, beyond)).contains("loaded tick arrays"));
    }

    #[test]
    fn the_b_to_a_walk_ends_on_the_last_tick_of_its_third_array() {
        let m = market(1_000_000_000_000, 0, 4, 0, Vec::new());
        assert_eq!(swap_tick_array_starts(0, 4, false), vec![0, 352, 704]);
        let end = sequence_end_tick(704, 4, false);
        assert_eq!(end, 1_055);

        let inside = (input_to(&m, false, end) - 1) as u64;
        m.walk(false, inside)
            .expect("ends before the sequence does");
        let beyond = (input_to(&m, false, end) + 1_000) as u64;
        assert!(refusal_detail(m.walk(false, beyond)).contains("loaded tick arrays"));
    }

    #[test]
    fn a_tick_beyond_the_swaps_arrays_never_extends_the_walk() {
        // A tick the programme cannot reach in this swap must not be crossed
        // in the quote, or the quote speaks for liquidity the swap never sees.
        let unreachable = InitializedTick {
            tick: -10_000,
            liquidity_net: 0,
        };
        let m = market(1_000_000_000_000, 0, 4, 0, vec![unreachable]);
        let beyond = (input_to(&m, true, -704) * 2) as u64;
        assert!(refusal_detail(m.walk(true, beyond)).contains("loaded tick arrays"));
    }

    #[test]
    fn an_initialised_tick_on_the_sequence_end_is_crossed_once() {
        let edge = InitializedTick {
            tick: -704,
            liquidity_net: 0,
        };
        let m = market(1_000_000_000_000, 0, 4, 0, vec![edge]);
        let inside = (input_to(&m, true, -704) - 1) as u64;
        m.walk(true, inside).expect("ends before the sequence does");
        let beyond = (input_to(&m, true, -704) + 1_000) as u64;
        assert!(refusal_detail(m.walk(true, beyond)).contains("loaded tick arrays"));
    }

    #[test]
    fn the_b_to_a_sequence_shifts_when_no_tick_above_is_left_in_the_current_array() {
        // Array 0 covers ticks 0..=351 at spacing 4; from tick 348 the next
        // tick up is 352, already in the next array.
        assert_eq!(swap_tick_array_starts(347, 4, false), vec![0, 352, 704]);
        assert_eq!(swap_tick_array_starts(348, 4, false), vec![352, 704, 1_056]);
        assert_eq!(swap_tick_array_starts(348, 4, true), vec![0, -352, -704]);
        assert_eq!(swap_tick_array_starts(-1, 4, false), vec![0, 352, 704]);
        assert_eq!(swap_tick_array_starts(-5, 4, false), vec![-352, 0, 352]);
    }

    #[test]
    fn the_sequence_stops_at_the_edge_of_the_tick_range() {
        let span = TICK_ARRAY_SIZE * 64;
        let lowest = tick_array_start_index(MIN_TICK, 64);
        assert_eq!(
            swap_tick_array_starts(MIN_TICK + 10, 64, true),
            vec![lowest],
            "nothing below the array holding the min tick is a valid start"
        );
        assert_eq!(sequence_end_tick(lowest, 64, true), MIN_TICK);

        let highest = tick_array_start_index(MAX_TICK, 64);
        let starts = swap_tick_array_starts(MAX_TICK - 10, 64, false);
        assert_eq!(starts, vec![highest]);
        assert_eq!(sequence_end_tick(highest, 64, false), MAX_TICK);
        assert_eq!(sequence_end_tick(highest - span, 64, false), highest - 1);
    }

    #[test]
    fn every_slot_names_its_derived_array_whether_or_not_it_exists() {
        let m = market(1_000_000_000_000, -26_090, 8, 0, Vec::new());
        let program = whirlpool_program_id();
        let expected: Vec<Pubkey> = [-26_752, -26_048, -25_344]
            .into_iter()
            .map(|start| tick_array_address(&program, &m.state.pool, start))
            .collect();
        assert_eq!(m.tick_array_accounts(false), expected);
    }

    #[test]
    fn the_edge_of_the_range_repeats_its_one_array_into_every_slot() {
        let m = market(1_000_000_000_000, MAX_TICK - 10, 64, 0, Vec::new());
        let accounts = m.tick_array_accounts(false);
        assert_eq!(accounts.len(), TICK_ARRAYS_PER_SWAP);
        assert!(accounts.iter().all(|account| *account == accounts[0]));
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
}
