//! Meteora DLMM (`LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo`) — a discretised
//! range AMM. Liquidity sits in fixed-width BINS, not on a continuous curve:
//! bin `id`'s price is `(1 + bin_step/10000)^id` in Q64.64, and a swap walks
//! bins the way Raydium CLMM/Orca Whirlpool walk ticks.
//!
//! # Layout, verified against a live pool
//!
//! `LbPair` is 904 bytes, checked against the live SOL/USDC pool
//! `5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6`: `token_x_mint`/`token_y_mint`
//! decode to exactly `So11111111111111111111111111111111111111112` and
//! `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`, `oracle` decodes to a pubkey
//! whose PDA re-derives from `["oracle", pool]`, `creator`/`base_key` decode to
//! the system program default (an old permissionless pool never sets them), and
//! `last_updated_at` decodes to a plausible Unix timestamp.
//!
//! ```text
//!   8 static_parameters (32B): base_factor u16@0 filter_period u16@2
//!     decay_period u16@4 reduction_factor u16@6 variable_fee_control u32@8
//!     max_volatility_accumulator u32@12 min_bin_id i32@16 max_bin_id i32@20
//!     protocol_share u16@24 base_fee_power_factor u8@26 function_type u8@27
//!     collect_fee_mode u8@28
//!  40 variable_parameters (32B): volatility_accumulator u32@0
//!     volatility_reference u32@4 index_reference i32@8 last_update_timestamp i64@16
//!  76 active_id i32     80 bin_step u16      82 status u8
//!  88 token_x_mint      120 token_y_mint
//! 152 reserve_x         184 reserve_y
//! 216 protocol_fee.amount_x u64   224 protocol_fee.amount_y u64
//! 552 oracle
//! 584 bin_array_bitmap [u64; 16]  (unused by this venue -- see below)
//! 712 last_updated_at i64
//! 848 creator
//! ```
//!
//! The account carries no on-chain Anchor discriminator check needed beyond
//! `LbPair`'s own `[33, 11, 49, 98, 181, 101, 177, 13]` (confirmed against the
//! programme's own IDL, fetched on chain per `adding-a-venue.md`); every offset
//! above was read back from the live 904-byte account, not computed from the
//! IDL's field order alone -- `reward_infos` sits between `protocol_fee` and
//! `oracle`, and its real size (288 bytes for two 144-byte `RewardInfo`s) was
//! confirmed by locating the oracle pubkey's exact byte offset in the raw
//! account rather than trusted from Rust `repr(C)` alignment arithmetic.
//!
//! # The instruction: `swap2`, not `swap`
//!
//! `sha256("global:swap2")[..8]` = `414b3f4ceb5b5b88`, confirmed against a live
//! transaction routed through Jupiter
//! (`3oivfHFnTrFRAxBD4d6zqLqiubQ1umSrgP3gApv2fCoMzgEd129bg2YsjCH4t5K94vmBsMqD5t2dKAAgftpuvfzZ`,
//! inner instruction at stack height 2) against the same SOL/USDC pool. That
//! transaction used `swap2` even though BOTH legs are plain SPL tokens, so
//! unlike Orca Whirlpool's `swap`/`swap2` split (legacy vs Token-2022) this
//! venue always emits `swap2` -- it is a strict superset of `swap` (it just
//! carries `token_x_program`/`token_y_program` explicitly plus a trailing
//! `memo_program` and a `remaining_accounts_info` arg) and is what real
//! integrators actually send. The 17-account instruction decoded:
//!
//! ```text
//! 0 lb_pair  1 bin_array_bitmap_extension  2 reserve_x  3 reserve_y
//! 4 user_token_in  5 user_token_out  6 token_x_mint  7 token_y_mint
//! 8 oracle  9 host_fee_in (ABSENT -> programme's own id)  10 user (signer)
//! 11 token_x_program  12 token_y_program  13 memo_program
//! 14 event_authority  15 program  16+ bin arrays (remaining, positional)
//! ```
//!
//! `event_authority` (`D1ZN9Wj1fRSUQfCjhvnu1hqDMT7hzjzBBpi12nVniYD6`) re-derives
//! from `["__event_authority"]` under the DLMM programme -- confirmed against
//! that same live transaction. Args: `amount_in u64`, `min_amount_out u64`,
//! `remaining_accounts_info: { slices: Vec<RemainingAccountsSlice> }` (Borsh: a
//! `u32` length prefix, no `Option` wrapper -- the live transaction's trailing
//! 4 bytes were `00000000`, an empty vec, not an `Option::None` tag as
//! `swap_v2`'s trailing byte is on Orca/Raydium CLMM). This venue always sends
//! an empty vec: that field only carries extra accounts for a Token-2022
//! TRANSFER HOOK mint, which this venue refuses outright (see below) rather
//! than guess at unbuilt hook accounts.
//!
//! # PDAs, all confirmed against the same live transaction or a live account
//!
//! * `bin_array_bitmap_extension` = `["bitmap", lb_pair]` -- confirmed
//!   (`DArpuuqJxNLRGQ8xq5ebZbobyjxSWWsPq8MqSZ2fUZLE`). It is an Anchor
//!   OPTIONAL account and MOST POOLS DO NOT HAVE ONE: it is created only where
//!   liquidity reaches past the ±512 array indices `LbPair.bin_array_bitmap`
//!   covers. `load()` therefore reads it with everything else, and the
//!   instruction names the real PDA only when it exists, spelling absence as
//!   the programme's own id -- the same convention `host_fee_in` uses. Naming
//!   the un-created PDA instead fails on chain with Anchor error 3007
//!   (`AccountOwnedByWrongProgram`), which is what the majority of pools would
//!   have done. The deepest SOL/USDC pool HAS an extension, so only a pool
//!   without one proves this; `meteora_dlmm_swaps_a_pool_that_has_no_bin_array_bitmap_extension`
//!   is that test. Its contents are never read (see "What this venue does not do").
//! * `oracle` = `["oracle", lb_pair]` -- confirmed (`59YuGWPunbchD2mbi9U7qvjWQKQReGeepn4ZSr9zz9Li`).
//! * bin array = `["bin_array", lb_pair, array_index.to_le_bytes()]`, `array_index`
//!   an **`i64`, little-endian** (not the big-endian `i32` Raydium CLMM's tick
//!   arrays use). `array_index = floor(bin_id / 70)`. Confirmed: the live
//!   transaction's trailing account `6MeamjT3xB2symUVrndFiu9bCU375m8vniQEpEwngyLM`
//!   re-derives at `array_index = -80` for `active_id = -5594`
//!   (`floor(-5594 / 70) == -80`).
//!
//! # `BinArray`, 10136 bytes
//!
//! ```text
//! 8 index i64   16 version u8   24 lb_pair   56 bins[70], 144 bytes each
//! ```
//! `56 + 70*144 + 8 == 10136`, the real fetched account length. Verified against
//! the live account above: `bins[6]` (bin id `-80*70 + 6 == -5594`, the pool's
//! own `active_id` at the same read) decodes a plausible Q64.64 price
//! (`1969412515201075439 / 2^64 ≈ 0.1067`, matching `(1.0004)^-5594 ≈ 0.1069`
//! computed independently from `bin_step = 4`) and non-zero `amount_y`. A `Bin`
//! entry: `amount_x u64@0 amount_y u64@8 price u128@16 liquidity_supply u128@32
//! ... open_order_amount u64@112 ... limit_order_ask_side u8@140`.
//!
//! # The fee: base + variable, both charged, one of them state-dependent
//!
//! `FEE_PRECISION = 1_000_000_000`, `MAX_FEE_RATE = 100_000_000` (10%),
//! `BASIS_POINT_MAX = 10_000` -- all IDL constants, cross-checked against the
//! live pool's own fields (`base_factor = 10000`, `bin_step = 4`,
//! `reduction_factor = 5000`, `protocol_share = 1000`, all plausible).
//!
//! ```text
//! base_fee  = base_factor * bin_step * 10 * 10^base_fee_power_factor
//! var_fee   = ceil(variable_fee_control * (volatility_accumulator * bin_step)^2 / 1e11)
//! total_fee = min(base_fee + var_fee, MAX_FEE_RATE)
//! fee(amount_with_fee) = ceil(amount_with_fee * total_fee / FEE_PRECISION)
//! ```
//!
//! **This formula was NOT independently re-derived from a replayed vault-delta
//! transaction** -- see "What could not be verified" below. It is transcribed
//! from Meteora's own open-source `dlmm-sdk` (`commons/src/extensions/lb_pair.rs`,
//! `get_base_fee`/`compute_variable_fee`/`compute_fee_from_amount`,
//! `MeteoraAg/dlmm-sdk` on GitHub, Apache-2.0), which is the same crate their
//! own CLI, indexer and integration tests use to quote against real mainnet
//! pools -- not a header file or a guess, but also not an independent replay.
//!
//! `volatility_accumulator` decays with wall-clock time and grows with bins
//! crossed WITHIN a swap:
//!
//! ```text
//! update_references (once, at load, using the CURRENT unix timestamp):
//!   elapsed = now - last_update_timestamp
//!   if elapsed >= filter_period:
//!     index_reference = active_id
//!     volatility_reference = elapsed < decay_period
//!       ? volatility_accumulator * reduction_factor / BASIS_POINT_MAX : 0
//!
//! update_volatility_accumulator (once per bin visited during the walk):
//!   delta = |index_reference - active_id|
//!   volatility_accumulator = min(volatility_reference + delta*BASIS_POINT_MAX,
//!                                 max_volatility_accumulator)
//! ```
//!
//! `update_references` needs the CURRENT wall clock, so it runs in `load()`
//! (the async half, which reads the `Clock` sysvar in the same
//! `get_multiple_accounts` batch as everything else) and its result --
//! `index_reference`/`volatility_reference` already advanced -- is baked into
//! the pure market. `update_volatility_accumulator` needs no further clock
//! access (only `active_id`, which changes only as the pure walk crosses
//! bins), so it runs inside `quote()`.
//!
//! `collect_fee_mode` (0 = InputOnly, always fee-on-input; 1 = OnlyY, fee is
//! taken from whichever side is X, in Y-equivalent terms -- i.e. fee-on-input
//! only when Y is being spent) decides which side of a bin's fill the fee is
//! taken from, mirroring Meteora DAMM v2's `collect_fee_mode` (see
//! `venues.md`). The live pool above reads `collect_fee_mode = 0`.
//!
//! # What this venue does not do
//!
//! * **It does not walk the on-chain bin-array bitmap.** `LbPair` carries a
//!   1024-bit internal bitmap (±512 array indices) plus an off-chain-extension
//!   bitmap beyond that, letting the real programme jump straight to the next
//!   NON-EMPTY array across a gap of genuinely empty ones. This venue instead
//!   fetches up to three CONSECUTIVE array indices out from the active one (the
//!   same "fetch candidates, keep what exists" contract Orca Whirlpool uses for
//!   its tick arrays) and refuses once it runs out of loaded bins. A pool whose
//!   liquidity is concentrated near the active price -- the overwhelmingly
//!   common shape -- is unaffected; a pool with a real, wide EMPTY gap right
//!   next to the active bin would be refused here as `InsufficientLiquidity`
//!   even though the bitmap says liquidity exists further out. Under-promising
//!   is the safe direction the skill requires; over-claiming past a gap the
//!   bitmap would have skipped is not attempted.
//! * **It does not fill limit-order liquidity.** A `Bin` layers market-making
//!   liquidity (`amount_x`/`amount_y`) with limit orders
//!   (`open_order_amount`/`processed_order_remaining_amount`); the real
//!   programme can fill both. This venue quotes MM liquidity only, which can
//!   only UNDER-state what a bin can fill, never over-state it.
//! * **It refuses a mint with a Token-2022 transfer-HOOK extension** (distinct
//!   from the transfer-FEE extension this venue does support via
//!   `token2022.rs`) with `PoolNotTradable`, because filling the hook's own
//!   extra accounts is unbuilt and `remaining_accounts_info` is always sent
//!   empty.
//!
//! # What could not be verified
//!
//! The skill's central method -- replaying a real swap's vault-delta against
//! this venue's own arithmetic -- could not be completed in this environment.
//! Meteora's public DLMM API (`dlmm-api.meteora.ag`) returned `404` on every
//! documented route at the time of writing, and Jupiter's quote API's hostname
//! did not resolve from this sandbox, so no low-volume DLMM pool could be
//! located to give a stable pre-swap snapshot; the deepest SOL/USDC pool
//! trades too often for the bin state fetched moments after a swap to be
//! trusted as that swap's exact pre-image (confirmed directly: fetching the
//! bin the one captured swap touched shows several OTHER bins already fully
//! one-sided, evidence of further trading in between). The account layout, the
//! discriminators, the account order and count, and every PDA seed above ARE
//! independently confirmed against live chain data; the FEE FORMULA is sourced
//! from Meteora's own SDK rather than reproduced against a transaction's own
//! balance deltas. The mandatory zero-slippage live simulation
//! (`direct_swaps_mainnet.rs`) is the exactness proof that IS available here
//! and does not depend on this gap.

use super::layout::{
    i32_at, mint_decimals, pubkey_at, token_account_amount, u16_at, u32_at, u64_at, u8_at,
};
use super::math::{mul_div_ceil, mul_div_floor};
use super::token2022::{transfer_fee_schedule, TransferFeeSchedule};
use crate::chains::solana::constants::{MEMO_PROGRAM_ID, METEORA_DLMM_PROGRAM_ID};
use crate::chains::solana::pools::types::ProgramKind;
use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
use crate::chains::solana::solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use crate::chains::solana::spl_token_2022::extension::{
    transfer_hook::TransferHook, BaseStateWithExtensions, StateWithExtensions,
};
use crate::chains::solana::spl_token_2022::state::Mint;
use crate::chains::solana::swaps::direct::error::{DirectSwapError, DirectSwapResult};
use crate::chains::solana::swaps::direct::venue::{
    PoolMarket, PoolVenue, SwapAccounts, VenueQuote,
};
use async_trait::async_trait;
use std::str::FromStr;

/// `sha256("global:swap2")[..8]`, confirmed against a live mainnet transaction
/// (see module docs).
const SWAP2: [u8; 8] = [0x41, 0x4b, 0x3f, 0x4c, 0xeb, 0x5b, 0x5b, 0x88];

/// Denominator the pool's total fee rate is expressed over.
const FEE_PRECISION: u128 = 1_000_000_000;

/// 10% -- the programme's own ceiling on `base_fee + variable_fee`.
const MAX_FEE_RATE: u128 = 100_000_000;

/// Denominator `protocol_share`, `reduction_factor` and bin-step ratios use.
const BASIS_POINT_MAX: u128 = 10_000;

/// Q64.64: bin price and every amount-from-price conversion is scaled by
/// `2^SCALE_OFFSET`.
const SCALE_OFFSET: u32 = 64;

/// Bins per `BinArray` account. Distinct from Raydium CLMM's 60 and Orca
/// Whirlpool's 88.
const MAX_BIN_PER_ARRAY: i32 = 70;

/// Bytes from the start of a `BinArray` account to its first `Bin` entry: an
/// 8-byte Anchor discriminator, `index: i64`, `version: u8`, 7 bytes padding,
/// `lb_pair: Pubkey`.
const BINS_OFFSET: usize = 56;

/// One `Bin` entry's byte size, verified against a live account (see module
/// docs): `56 + 70*144 + 8 == 10136`, the real fetched `BinArray` length.
const BIN_SIZE: usize = 144;

/// Bin arrays fetched (and, when they exist, named) per swap direction,
/// mirroring the `TICK_ARRAYS_PER_SWAP` convention Raydium CLMM/Orca
/// Whirlpool already use.
const BIN_ARRAYS_PER_SWAP: usize = 3;

/// Compute units a DLMM swap needs. A live Jupiter-routed transaction's own
/// inner `swap2` call consumed ~33k units crossing a single bin; this leaves
/// headroom for a few bin crossings before `compute.rs`'s own 30% margin is
/// applied on top.
const COMPUTE_UNITS: u32 = 400_000;

const BIN_ARRAY_SEED: &[u8] = b"bin_array";
const ORACLE_SEED: &[u8] = b"oracle";
const BITMAP_SEED: &[u8] = b"bitmap";
const CLOCK_SYSVAR: &str = "SysvarC1ock11111111111111111111111111111111";

/// The venue adapter.
pub struct MeteoraDlmmVenue;

#[async_trait]
impl PoolVenue for MeteoraDlmmVenue {
    fn program(&self) -> ProgramKind {
        ProgramKind::MeteoraDlmm
    }

    fn program_id(&self) -> Pubkey {
        dlmm_program_id()
    }

    async fn load(
        &self,
        pool: &Pubkey,
        pool_account: &Account,
    ) -> DirectSwapResult<Box<dyn PoolMarket>> {
        let state = LbPairState::decode(*pool, &pool_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: format!(
                    "LbPair state did not match the expected layout ({} bytes)",
                    pool_account.data.len()
                ),
            }
        })?;

        if state.status != 0 {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "the pair's status byte is not Enabled".to_owned(),
            });
        }

        let program = dlmm_program_id();
        let active_array_index = bin_id_to_array_index(state.active_id);

        let mut array_indices: Vec<i64> = Vec::new();
        for zero_for_one in [true, false] {
            for index in candidate_array_indices(active_array_index, zero_for_one) {
                if !array_indices.contains(&index) {
                    array_indices.push(index);
                }
            }
        }
        let array_addresses: Vec<Pubkey> = array_indices
            .iter()
            .map(|index| bin_array_address(&program, pool, *index))
            .collect();

        let fixed_addresses = [
            state.reserve_x,
            state.reserve_y,
            state.token_x_mint,
            state.token_y_mint,
            Pubkey::from_str(CLOCK_SYSVAR).expect("clock sysvar address is valid"),
            bitmap_extension_address(&program, pool),
        ];
        let mut addresses = fixed_addresses.to_vec();
        addresses.extend(array_addresses.iter().copied());

        let accounts = get_rpc_client()
            .get_multiple_accounts(&addresses)
            .await
            .map_err(|e| DirectSwapError::AccountUnavailable {
                address: *pool,
                detail: format!("LbPair accounts could not be read: {e}"),
            })?;

        let required = |index: usize| -> DirectSwapResult<&Account> {
            accounts.get(index).and_then(Option::as_ref).ok_or(
                DirectSwapError::AccountUnavailable {
                    address: addresses[index],
                    detail: "account does not exist".to_owned(),
                },
            )
        };

        let reserve_x_balance = token_account_amount(&required(0)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "reserve_x is not a token account".to_owned(),
            }
        })?;
        let reserve_y_balance = token_account_amount(&required(1)?.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "reserve_y is not a token account".to_owned(),
            }
        })?;
        let mint_x_account = required(2)?;
        let mint_y_account = required(3)?;
        let token_program_x = mint_x_account.owner;
        let token_program_y = mint_y_account.owner;
        let decimals_x = mint_decimals(&mint_x_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_x_mint is not a mint account".to_owned(),
            }
        })?;
        let decimals_y = mint_decimals(&mint_y_account.data).ok_or_else(|| {
            DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "token_y_mint is not a mint account".to_owned(),
            }
        })?;

        // A transfer-HOOK mint needs extra accounts this venue does not build
        // (`remaining_accounts_info` is always sent empty). Refuse rather than
        // send an instruction the hook will reject.
        if mint_has_transfer_hook(mint_x_account) || mint_has_transfer_hook(mint_y_account) {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "one of this pool's mints carries a Token-2022 transfer hook, which this \
                         venue does not build accounts for"
                    .to_owned(),
            });
        }

        let transfer_fee_x = transfer_fee_schedule(mint_x_account);
        let transfer_fee_y = transfer_fee_schedule(mint_y_account);

        let clock_account = required(4)?;
        let now =
            i64_at(&clock_account.data, 32).ok_or_else(|| DirectSwapError::PoolUndecodable {
                pool: *pool,
                detail: "the clock sysvar did not match the expected layout".to_owned(),
            })?;

        let raw_v_params =
            LbPairState::decode_v_parameters(&pool_account.data).ok_or_else(|| {
                DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: "v_parameters did not match the expected layout".to_owned(),
                }
            })?;

        // `update_references`: a wall-clock-dependent step, so it runs here in
        // the async half rather than in the pure `quote()`.
        let v_params = raw_v_params.update_references(now, state.active_id, &state.parameters);

        // `bin_array_bitmap_extension` is an OPTIONAL account, and most pools
        // never create one -- it exists only where liquidity reaches past the
        // +/-512 array indices `LbPair`'s own bitmap covers. Whether it is
        // there decides how the instruction names it, so it is read here with
        // everything else rather than assumed.
        let has_bitmap_extension = matches!(accounts.get(5), Some(Some(_)));

        // Every bin array the batch actually returned, decoded into its bins.
        // An array the RPC did not return is not a load error -- the swap may
        // never reach it, mirroring Orca Whirlpool's "no bitmap" contract even
        // though DLMM does publish one (see module docs on why it is not used).
        let mut bins: Vec<DecodedBin> = Vec::new();
        let mut available_array_indices: Vec<i64> = Vec::new();
        for (offset, address) in array_addresses.iter().enumerate() {
            let Some(Some(account)) = accounts.get(fixed_addresses.len() + offset) else {
                continue;
            };
            let index = array_indices[offset];
            available_array_indices.push(index);
            if let Some(decoded) = decode_bin_array(&account.data, index) {
                bins.extend(decoded);
            } else {
                return Err(DirectSwapError::PoolUndecodable {
                    pool: *pool,
                    detail: format!("bin array {address} did not match the expected layout"),
                });
            }
        }
        bins.sort_by_key(|bin| bin.id);
        bins.dedup_by_key(|bin| bin.id);

        if available_array_indices.is_empty() {
            return Err(DirectSwapError::PoolNotTradable {
                pool: *pool,
                detail: "not one bin array around the active bin exists on chain".to_owned(),
            });
        }

        Ok(Box::new(DlmmMarket {
            state,
            v_params,
            token_program_x,
            token_program_y,
            decimals_x,
            decimals_y,
            reserve_x_balance,
            reserve_y_balance,
            transfer_fee_x,
            transfer_fee_y,
            bins,
            available_array_indices,
            has_bitmap_extension,
        }))
    }
}

/// The static (admin-set) fee parameters, unpacked from `LbPair.parameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticParameters {
    pub base_factor: u16,
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub variable_fee_control: u32,
    pub max_volatility_accumulator: u32,
    pub protocol_share: u16,
    pub base_fee_power_factor: u8,
    pub collect_fee_mode: u8,
}

/// The dynamic (market-driven) fee parameters, unpacked from `LbPair.v_parameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariableParameters {
    pub volatility_accumulator: u32,
    pub volatility_reference: u32,
    pub index_reference: i32,
    pub last_update_timestamp: i64,
}

impl VariableParameters {
    /// The wall-clock-dependent half of the on-chain `update_references`,
    /// applied ONCE per swap using the current time and the pool's
    /// `active_id` at load time. Pure given both.
    /// `pub` so the offline test tier can reproduce exactly what `load()`
    /// does against a captured fixture, rather than re-deriving the logic.
    pub fn update_references(
        &self,
        now: i64,
        active_id: i32,
        s_params: &StaticParameters,
    ) -> VariableParameters {
        let mut next = *self;
        let elapsed = now.saturating_sub(self.last_update_timestamp);
        if elapsed >= s_params.filter_period as i64 {
            next.index_reference = active_id;
            next.volatility_reference = if elapsed < s_params.decay_period as i64 {
                ((self.volatility_accumulator as u64) * (s_params.reduction_factor as u64)
                    / (BASIS_POINT_MAX as u64)) as u32
            } else {
                0
            };
        }
        next
    }
}

/// The parts of the `LbPair` account a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LbPairState {
    pub pool: Pubkey,
    pub parameters: StaticParameters,
    pub active_id: i32,
    pub bin_step: u16,
    pub status: u8,
    pub token_x_mint: Pubkey,
    pub token_y_mint: Pubkey,
    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,
    pub oracle: Pubkey,
}

impl LbPairState {
    /// Decode an `LbPair` account. Pure: no RPC, no cache, no clock. See module
    /// docs for how every offset here was confirmed against a live account.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        let parameters = StaticParameters {
            base_factor: u16_at(data, 8)?,
            filter_period: u16_at(data, 10)?,
            decay_period: u16_at(data, 12)?,
            reduction_factor: u16_at(data, 14)?,
            variable_fee_control: u32_at(data, 16)?,
            max_volatility_accumulator: u32_at(data, 20)?,
            protocol_share: u16_at(data, 32)?,
            base_fee_power_factor: u8_at(data, 34)?,
            collect_fee_mode: u8_at(data, 36)?,
        };
        Some(Self {
            pool,
            parameters,
            active_id: i32_at(data, 76)?,
            bin_step: u16_at(data, 80)?,
            status: u8_at(data, 82)?,
            token_x_mint: pubkey_at(data, 88)?,
            token_y_mint: pubkey_at(data, 120)?,
            reserve_x: pubkey_at(data, 152)?,
            reserve_y: pubkey_at(data, 184)?,
            oracle: pubkey_at(data, 552)?,
        })
    }

    /// Unpack `v_parameters` separately -- callers combine this with
    /// `update_references` before building a market.
    pub fn decode_v_parameters(data: &[u8]) -> Option<VariableParameters> {
        Some(VariableParameters {
            volatility_accumulator: u32_at(data, 40)?,
            volatility_reference: u32_at(data, 44)?,
            index_reference: i32_at(data, 48)?,
            last_update_timestamp: i64_at(data, 56)?,
        })
    }
}

/// A single bin's swappable (market-making only, see module docs) liquidity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodedBin {
    id: i32,
    amount_x: u64,
    amount_y: u64,
}

/// A decoded, quotable DLMM pair.
#[derive(Debug, Clone)]
pub struct DlmmMarket {
    state: LbPairState,
    /// `v_parameters` AFTER `update_references` has been applied against the
    /// clock read at load time; `index_reference` still needs `active_id`
    /// filled in per-call, done in `quote()`.
    v_params: VariableParameters,
    token_program_x: Pubkey,
    token_program_y: Pubkey,
    decimals_x: u8,
    decimals_y: u8,
    reserve_x_balance: u64,
    reserve_y_balance: u64,
    transfer_fee_x: Option<TransferFeeSchedule>,
    transfer_fee_y: Option<TransferFeeSchedule>,
    /// Bins from every bin array `load()` actually fetched, sorted ascending.
    bins: Vec<DecodedBin>,
    /// Array indices that ACTUALLY EXIST on chain, out of the consecutive
    /// candidates `load()` derived. The only record of which of the
    /// (up to three) positional bin-array slots the instruction may name.
    available_array_indices: Vec<i64>,
    /// Whether this pool's `bin_array_bitmap_extension` account exists.
    ///
    /// It is an Anchor OPTIONAL account, and on chain most pools have none:
    /// of five live SOL-paired DLMM pools sampled, three had no extension.
    /// An absent optional account is signalled by passing the PROGRAMME ID in
    /// its slot -- the same convention this venue already uses for
    /// `host_fee_in`. Naming the un-created PDA instead makes the programme
    /// fail to deserialise it, on the majority of pools, after the priority
    /// fee is paid.
    has_bitmap_extension: bool,
}

impl DlmmMarket {
    /// Build a market directly from decoded parts, for the offline test tier.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: LbPairState,
        v_params: VariableParameters,
        token_program_x: Pubkey,
        token_program_y: Pubkey,
        decimals: (u8, u8),
        reserve_x_balance: u64,
        reserve_y_balance: u64,
        transfer_fee_x: Option<TransferFeeSchedule>,
        transfer_fee_y: Option<TransferFeeSchedule>,
        bins: Vec<(i32, u64, u64)>,
        available_array_indices: Vec<i64>,
        has_bitmap_extension: bool,
    ) -> Self {
        let mut bins: Vec<DecodedBin> = bins
            .into_iter()
            .map(|(id, amount_x, amount_y)| DecodedBin {
                id,
                amount_x,
                amount_y,
            })
            .collect();
        bins.sort_by_key(|bin| bin.id);
        bins.dedup_by_key(|bin| bin.id);
        Self {
            state,
            v_params,
            token_program_x,
            token_program_y,
            decimals_x: decimals.0,
            decimals_y: decimals.1,
            reserve_x_balance,
            reserve_y_balance,
            transfer_fee_x,
            transfer_fee_y,
            bins,
            available_array_indices,
            has_bitmap_extension,
        }
    }

    fn is_side_x(&self, mint: &Pubkey) -> Option<bool> {
        if *mint == self.state.token_x_mint {
            Some(true)
        } else if *mint == self.state.token_y_mint {
            Some(false)
        } else {
            None
        }
    }

    fn transfer_fee(&self, side_x: bool) -> Option<&TransferFeeSchedule> {
        if side_x {
            self.transfer_fee_x.as_ref()
        } else {
            self.transfer_fee_y.as_ref()
        }
    }

    fn fee_on_input(&self, swap_for_y: bool) -> bool {
        match self.state.parameters.collect_fee_mode {
            1 => !swap_for_y,
            // 0 (InputOnly) and any unrecognised value: always fee-on-input,
            // the same conservative fallback Orca's adaptive-fee refusal
            // pattern uses elsewhere -- here it is also simply the correct
            // reading of mode 0.
            _ => true,
        }
    }

    /// Total pool fee rate (base + variable), capped at `MAX_FEE_RATE`, given
    /// the CURRENT volatility accumulator (which changes as the walk crosses
    /// bins).
    fn total_fee_rate(&self, volatility_accumulator: u32) -> u128 {
        let p = &self.state.parameters;
        let base_fee = (p.base_factor as u128)
            * (self.state.bin_step as u128)
            * 10
            * 10u128.pow(p.base_fee_power_factor as u32);
        let variable_fee = if p.variable_fee_control > 0 {
            let square = (volatility_accumulator as u128) * (self.state.bin_step as u128);
            let square = square.saturating_mul(square);
            let v_fee = (p.variable_fee_control as u128).saturating_mul(square);
            (v_fee + 99_999_999_999) / 100_000_000_000
        } else {
            0
        };
        (base_fee + variable_fee).min(MAX_FEE_RATE)
    }

    /// `ceil(amount_with_fee * rate / FEE_PRECISION)`.
    fn fee_from_amount(rate: u128, amount_with_fee: u64) -> Option<u64> {
        mul_div_ceil(amount_with_fee as u128, rate, FEE_PRECISION)?
            .try_into()
            .ok()
    }

    /// `ceil(amount_excluding_fee * rate / (FEE_PRECISION - rate))` -- the
    /// inverse used when only PART of a requested amount lands in a bin (the
    /// bin drains before the whole input is placed) and the fee on that
    /// smaller, exact amount must be recomputed rather than pro-rated.
    fn fee_from_excluded_amount(rate: u128, amount_excluding_fee: u64) -> Option<u64> {
        let denominator = FEE_PRECISION.checked_sub(rate)?;
        if denominator == 0 {
            return None;
        }
        mul_div_ceil(amount_excluding_fee as u128, rate, denominator)?
            .try_into()
            .ok()
    }

    /// One bin's exact-in fill against its market-making liquidity only (see
    /// module docs on limit orders). Returns
    /// `(amount_in_consumed, amount_out, fee)`.
    fn quote_bin(
        &self,
        bin: &DecodedBin,
        price: u128,
        amount_in: u64,
        swap_for_y: bool,
        fee_on_input: bool,
        rate: u128,
    ) -> DirectSwapResult<(u64, u64, u64)> {
        let mm_amount_out_cap = if swap_for_y {
            bin.amount_y
        } else {
            bin.amount_x
        };

        let mut trading_fee: u64 = 0;
        let mut excluded_fee_amount_in = amount_in;
        if fee_on_input {
            let fee = Self::fee_from_amount(rate, amount_in).ok_or(quote_math_error())?;
            trading_fee = fee;
            excluded_fee_amount_in = amount_in.saturating_sub(fee);
        }

        let (amount_in_mm, amount_left, out_amount) = if mm_amount_out_cap == 0 {
            (0, excluded_fee_amount_in, 0)
        } else {
            let max_amount_in = get_amount_in(mm_amount_out_cap, price, swap_for_y, Rounding::Up)
                .ok_or(quote_math_error())?;
            if excluded_fee_amount_in >= max_amount_in {
                (
                    max_amount_in,
                    excluded_fee_amount_in.saturating_sub(max_amount_in),
                    mm_amount_out_cap,
                )
            } else {
                let out = get_amount_out(excluded_fee_amount_in, price, swap_for_y, Rounding::Down)
                    .ok_or(quote_math_error())?;
                (excluded_fee_amount_in, 0, out)
            }
        };

        let mut included_fee_amount_in = amount_in;
        if amount_left > 0 {
            // The bin drained before the whole request landed -- only
            // `amount_in_mm` of the excluded-fee amount was actually used, so
            // the fee must be recomputed on that exact figure.
            if fee_on_input {
                let fee =
                    Self::fee_from_excluded_amount(rate, amount_in_mm).ok_or(quote_math_error())?;
                trading_fee = fee;
                included_fee_amount_in = amount_in_mm.saturating_add(fee);
            } else {
                included_fee_amount_in = amount_in_mm;
            }
        }

        let mut excluded_fee_amount_out = out_amount;
        if !fee_on_input {
            let fee = Self::fee_from_amount(rate, out_amount).ok_or(quote_math_error())?;
            trading_fee = fee;
            excluded_fee_amount_out = out_amount.saturating_sub(fee);
        }

        Ok((included_fee_amount_in, excluded_fee_amount_out, trading_fee))
    }

    /// Walk bins from `active_id`, consuming `amount_in` (already net of any
    /// input-mint Token-2022 transfer fee). Returns
    /// `(total_out, total_fee_in_walk_units)`; `fee_on_input` decides whether
    /// the second figure is denominated in input or output raw units.
    fn walk(&self, swap_for_y: bool, amount_in: u64) -> DirectSwapResult<(u64, u64, bool)> {
        let fee_on_input = self.fee_on_input(swap_for_y);
        let mut active_id = self.state.active_id;
        // `index_reference` was already advanced (or deliberately held) by
        // `update_references` at load time using the real wall clock; the
        // real programme holds it fixed for the rest of a single swap, so it
        // is read once here and never re-derived mid-walk.
        let index_reference = self.v_params.index_reference;
        let mut amount_left = amount_in;
        let mut total_out: u64 = 0;
        let mut total_fee: u64 = 0;

        loop {
            if amount_left == 0 {
                break;
            }
            let Some(bin) = self.bins.iter().find(|b| b.id == active_id) else {
                return Err(DirectSwapError::InsufficientLiquidity {
                    pool: self.state.pool,
                    amount_in,
                    detail: "the size travels further than the loaded bin arrays cover".to_owned(),
                });
            };

            let mm_cap = if swap_for_y {
                bin.amount_y
            } else {
                bin.amount_x
            };
            if mm_cap > 0 {
                let delta = (index_reference as i64 - active_id as i64).unsigned_abs();
                let volatility_accumulator = ((self.v_params.volatility_reference as u64)
                    .saturating_add(delta.saturating_mul(BASIS_POINT_MAX as u64)))
                .min(self.state.parameters.max_volatility_accumulator as u64)
                    as u32;
                let rate = self.total_fee_rate(volatility_accumulator);
                let price =
                    get_price_from_id(active_id, self.state.bin_step).ok_or(quote_math_error())?;

                let (consumed, out, fee) =
                    self.quote_bin(bin, price, amount_left, swap_for_y, fee_on_input, rate)?;
                if consumed > 0 {
                    amount_left = amount_left.saturating_sub(consumed);
                    total_out = total_out.saturating_add(out);
                    total_fee = total_fee.saturating_add(fee);
                }
            }

            if amount_left > 0 {
                let next_id = if swap_for_y {
                    active_id.checked_sub(1)
                } else {
                    active_id.checked_add(1)
                }
                .ok_or(quote_math_error())?;
                active_id = next_id;
            }
        }

        Ok((total_out, total_fee, fee_on_input))
    }
}

fn quote_math_error() -> DirectSwapError {
    DirectSwapError::QuoteMath {
        detail: "DLMM bin/fee arithmetic overflowed".to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rounding {
    Up,
    Down,
}

/// `(1 + bin_step/10000)^id` in Q64.64, ported from the programme's own
/// `get_price_from_id` (`commons/src/math/price_math.rs` in Meteora's public
/// `dlmm-sdk`, itself calling `u64x64_math::pow`). Matches the live SOL/USDC
/// pool's own stored bin price to within the precision of an independent
/// spot check (see module docs); this venue does not trust the STORED price
/// field (which the programme itself only lazily populates, `0` until first
/// touched) and always derives it fresh.
fn get_price_from_id(id: i32, bin_step: u16) -> Option<u128> {
    let one = 1u128 << SCALE_OFFSET;
    let bps = ((bin_step as u128) << SCALE_OFFSET) / (BASIS_POINT_MAX as u128);
    let base = one.checked_add(bps)?;
    pow_q64(base, id)
}

/// Q64.64 `base^exp`, `exp` possibly negative, via the programme's own
/// repeated-squaring-with-inversion trick (`u64x64_math::pow`): squaring a
/// Q64.64 value doubles its bit width, so the base is inverted first whenever
/// it is `>= 1.0` to keep every intermediate product inside 128 bits.
fn pow_q64(base: u128, exp: i32) -> Option<u128> {
    const MAX_EXPONENTIAL: u32 = 0x80000;
    let one = 1u128 << SCALE_OFFSET;
    if exp == 0 {
        return Some(one);
    }
    let mut invert = exp.is_negative();
    let exp: u32 = if invert {
        exp.unsigned_abs()
    } else {
        exp as u32
    };
    if exp >= MAX_EXPONENTIAL {
        return None;
    }

    let mut squared_base = base;
    let mut result = one;
    if squared_base >= result {
        squared_base = u128::MAX.checked_div(squared_base)?;
        invert = !invert;
    }

    let mut remaining = exp;
    let mut bit = 0x1u32;
    while remaining > 0 && bit != 0 {
        if exp & bit > 0 {
            result = (result.checked_mul(squared_base)?) >> SCALE_OFFSET;
        }
        squared_base = (squared_base.checked_mul(squared_base)?) >> SCALE_OFFSET;
        remaining &= !bit;
        bit <<= 1;
        if bit == 0 || bit > MAX_EXPONENTIAL {
            break;
        }
    }

    if result == 0 {
        return None;
    }
    if invert {
        Some(u128::MAX / result)
    } else {
        Some(result)
    }
}

/// Output for `amount_in` at `price`, ported from `Bin::get_amount_out`.
fn get_amount_out(
    amount_in: u64,
    price: u128,
    swap_for_y: bool,
    rounding: Rounding,
) -> Option<u64> {
    let denominator = 1u128 << SCALE_OFFSET;
    let raw = if swap_for_y {
        match rounding {
            Rounding::Up => mul_div_ceil(price, amount_in as u128, denominator),
            Rounding::Down => mul_div_floor(price, amount_in as u128, denominator),
        }
    } else {
        match rounding {
            Rounding::Up => mul_div_ceil(amount_in as u128, denominator, price),
            Rounding::Down => mul_div_floor(amount_in as u128, denominator, price),
        }
    }?;
    raw.try_into().ok()
}

/// Input needed to receive exactly `amount_out` at `price`, ported from
/// `Bin::get_amount_in`.
fn get_amount_in(
    amount_out: u64,
    price: u128,
    swap_for_y: bool,
    rounding: Rounding,
) -> Option<u64> {
    let denominator = 1u128 << SCALE_OFFSET;
    let raw = if swap_for_y {
        match rounding {
            Rounding::Up => mul_div_ceil(amount_out as u128, denominator, price),
            Rounding::Down => mul_div_floor(amount_out as u128, denominator, price),
        }
    } else {
        match rounding {
            Rounding::Up => mul_div_ceil(amount_out as u128, price, denominator),
            Rounding::Down => mul_div_floor(amount_out as u128, price, denominator),
        }
    }?;
    raw.try_into().ok()
}

impl PoolMarket for DlmmMarket {
    fn program(&self) -> ProgramKind {
        ProgramKind::MeteoraDlmm
    }

    fn pool(&self) -> Pubkey {
        self.state.pool
    }

    fn mints(&self) -> (Pubkey, Pubkey) {
        (self.state.token_x_mint, self.state.token_y_mint)
    }

    fn token_program(&self, mint: &Pubkey) -> Option<Pubkey> {
        self.is_side_x(mint).map(|side_x| {
            if side_x {
                self.token_program_x
            } else {
                self.token_program_y
            }
        })
    }

    fn decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.is_side_x(mint).map(|side_x| {
            if side_x {
                self.decimals_x
            } else {
                self.decimals_y
            }
        })
    }

    fn quote(&self, input_mint: &Pubkey, amount_in: u64) -> DirectSwapResult<VenueQuote> {
        let swap_for_y = self
            .is_side_x(input_mint)
            .ok_or(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: *input_mint,
                output_mint: Pubkey::default(),
            })?;

        let received_by_pool =
            super::token2022::net_of_fee(self.transfer_fee(swap_for_y), amount_in);
        if received_by_pool == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the input transfer fee consumes the whole amount at this size".to_owned(),
            });
        }

        let (gross_out, walk_fee, fee_on_input) = self.walk(swap_for_y, received_by_pool)?;
        if gross_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "fees consume the whole input at this size".to_owned(),
            });
        }

        let expected_out = super::token2022::net_of_fee(self.transfer_fee(!swap_for_y), gross_out);
        if expected_out == 0 {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output transfer fee consumes the whole amount at this size".to_owned(),
            });
        }
        let output_vault_balance = if swap_for_y {
            self.reserve_y_balance
        } else {
            self.reserve_x_balance
        };
        if expected_out >= output_vault_balance {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "the output exceeds what the pool's vault holds".to_owned(),
            });
        }

        // `VenueQuote::lp_fee`'s contract is input-raw-units. When the pool
        // charged the OUTPUT side instead (`collect_fee_mode == OnlyY` and
        // this leg's input is X), convert at this fill's own realised rate,
        // the same approach Meteora DAMM v2's venue uses for the identical
        // situation.
        let lp_fee = if fee_on_input {
            walk_fee
        } else if gross_out == 0 {
            0
        } else {
            (((walk_fee as u128) * (received_by_pool as u128)) / (gross_out as u128)) as u64
        };

        let price_before = (get_price_from_id(self.state.active_id, self.state.bin_step)
            .unwrap_or(0) as f64)
            / (2.0_f64).powi(64);
        let realised = if received_by_pool > 0 {
            (gross_out as f64) / (received_by_pool as f64)
        } else {
            0.0
        };
        let price_impact_pct = if price_before > 0.0 {
            let spot = if swap_for_y {
                price_before
            } else if price_before > 0.0 {
                1.0 / price_before
            } else {
                0.0
            };
            if spot > 0.0 {
                ((spot - realised).abs() / spot * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            }
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
        let input_is_x =
            self.is_side_x(&accounts.input_mint)
                .ok_or_else(|| DirectSwapError::PairNotInPool {
                    pool: self.state.pool,
                    input_mint: accounts.input_mint,
                    output_mint: accounts.output_mint,
                })?;
        let output_is_x = self.is_side_x(&accounts.output_mint).ok_or_else(|| {
            DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            }
        })?;
        if input_is_x == output_is_x {
            return Err(DirectSwapError::PairNotInPool {
                pool: self.state.pool,
                input_mint: accounts.input_mint,
                output_mint: accounts.output_mint,
            });
        }
        let swap_for_y = input_is_x;

        let bin_arrays = self.bin_array_accounts(swap_for_y);
        if bin_arrays.is_empty() {
            return Err(DirectSwapError::InsufficientLiquidity {
                pool: self.state.pool,
                amount_in,
                detail: "no bin array in the swap direction exists on chain".to_owned(),
            });
        }

        let program = dlmm_program_id();
        let mut data = Vec::with_capacity(24);
        data.extend_from_slice(&SWAP2);
        data.extend_from_slice(&amount_in.to_le_bytes());
        data.extend_from_slice(&min_out.to_le_bytes());
        // remaining_accounts_info.slices: Vec<..> = empty (u32 length prefix,
        // no Option tag -- confirmed against a live transaction, see module
        // docs). Only needed for a transfer-hook mint, which `load()` refuses.
        data.extend_from_slice(&0u32.to_le_bytes());

        let mut metas = vec![
            AccountMeta::new(self.state.pool, false),
            // bin_array_bitmap_extension: present -> the real PDA, writable;
            // absent -> the programme's own id, which is how Anchor spells
            // `None` for an optional account (see the field's own docs).
            if self.has_bitmap_extension {
                AccountMeta::new(bitmap_extension_address(&program, &self.state.pool), false)
            } else {
                AccountMeta::new_readonly(program, false)
            },
            AccountMeta::new(self.state.reserve_x, false),
            AccountMeta::new(self.state.reserve_y, false),
            AccountMeta::new(accounts.input_token_account, false),
            AccountMeta::new(accounts.output_token_account, false),
            AccountMeta::new_readonly(self.state.token_x_mint, false),
            AccountMeta::new_readonly(self.state.token_y_mint, false),
            AccountMeta::new(self.state.oracle, false),
            // host_fee_in: absent -> the programme's own id, confirmed
            // against a live transaction (see module docs).
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(accounts.owner, true),
            AccountMeta::new_readonly(self.token_program_x, false),
            AccountMeta::new_readonly(self.token_program_y, false),
            AccountMeta::new_readonly(memo_program_id(), false),
            AccountMeta::new_readonly(event_authority_address(&program), false),
            AccountMeta::new_readonly(program, false),
        ];
        for array in bin_arrays {
            metas.push(AccountMeta::new(array, false));
        }

        Ok(Instruction {
            program_id: program,
            accounts: metas,
            data,
        })
    }

    fn compute_units(&self) -> u32 {
        COMPUTE_UNITS
    }
}

impl DlmmMarket {
    /// The bin-array accounts the instruction names, in walk order, out of
    /// what `load()` actually confirmed exists on chain. See module docs on
    /// why this is consecutive-candidate fetching rather than a bitmap walk.
    fn bin_array_accounts(&self, swap_for_y: bool) -> Vec<Pubkey> {
        let program = dlmm_program_id();
        let active_array_index = bin_id_to_array_index(self.state.active_id);
        let starts: Vec<i64> = candidate_array_indices(active_array_index, swap_for_y)
            .into_iter()
            .filter(|index| self.available_array_indices.contains(index))
            .collect();
        starts
            .into_iter()
            .take(BIN_ARRAYS_PER_SWAP)
            .map(|index| bin_array_address(&program, &self.state.pool, index))
            .collect()
    }
}

/// Floor division: `-1 / 70` truncates to `0` in Rust, which would put a
/// negative bin just below a boundary into the array above it -- the same
/// trap Raydium CLMM's and Orca Whirlpool's own tick-array-index math avoid.
fn bin_id_to_array_index(bin_id: i32) -> i64 {
    let span = MAX_BIN_PER_ARRAY;
    let mut index = bin_id / span;
    if bin_id < 0 && bin_id % span != 0 {
        index -= 1;
    }
    index as i64
}

/// The (up to `BIN_ARRAYS_PER_SWAP`) consecutive array indices a swap in this
/// direction might touch, walking outward from the array holding the active
/// bin.
fn candidate_array_indices(active_array_index: i64, swap_for_y: bool) -> Vec<i64> {
    let step: i64 = if swap_for_y { -1 } else { 1 };
    let mut indices = Vec::with_capacity(BIN_ARRAYS_PER_SWAP);
    let mut index = active_array_index;
    for _ in 0..BIN_ARRAYS_PER_SWAP {
        indices.push(index);
        let Some(next) = index.checked_add(step) else {
            break;
        };
        index = next;
    }
    indices
}

/// Decode the bins out of a live `BinArray` account. `None` when the account
/// is too short -- a decode failure, never a partially-wrong swap.
fn decode_bin_array(data: &[u8], array_index: i64) -> Option<Vec<DecodedBin>> {
    let mut bins = Vec::with_capacity(MAX_BIN_PER_ARRAY as usize);
    for i in 0..(MAX_BIN_PER_ARRAY as usize) {
        let offset = BINS_OFFSET + i * BIN_SIZE;
        let amount_x = u64_at(data, offset)?;
        let amount_y = u64_at(data, offset + 8)?;
        let id = (array_index * MAX_BIN_PER_ARRAY as i64) as i32 + i as i32;
        bins.push(DecodedBin {
            id,
            amount_x,
            amount_y,
        });
    }
    Some(bins)
}

fn mint_has_transfer_hook(mint_account: &Account) -> bool {
    if mint_account.owner != crate::chains::solana::spl_token_2022::id() {
        return false;
    }
    let Ok(state) = StateWithExtensions::<Mint>::unpack(&mint_account.data) else {
        return false;
    };
    state.get_extension::<TransferHook>().is_ok()
}

fn i64_at(data: &[u8], offset: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        data.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

/// Bin array PDA: `["bin_array", lb_pair, array_index.to_le_bytes()]`,
/// `array_index` an `i64` -- confirmed against a live transaction (module docs).
pub fn bin_array_address(program: &Pubkey, pool: &Pubkey, array_index: i64) -> Pubkey {
    Pubkey::find_program_address(
        &[BIN_ARRAY_SEED, pool.as_ref(), &array_index.to_le_bytes()],
        program,
    )
    .0
}

/// Oracle PDA: `["oracle", lb_pair]` -- confirmed against a live account.
pub fn oracle_address(program: &Pubkey, pool: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[ORACLE_SEED, pool.as_ref()], program).0
}

/// Bin-array bitmap extension PDA: `["bitmap", lb_pair]` -- confirmed against
/// a live account.
pub fn bitmap_extension_address(program: &Pubkey, pool: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[BITMAP_SEED, pool.as_ref()], program).0
}

/// `["__event_authority"]` -- confirmed against a live transaction.
pub fn event_authority_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], program).0
}

fn dlmm_program_id() -> Pubkey {
    Pubkey::from_str(METEORA_DLMM_PROGRAM_ID).expect("DLMM program id constant is valid")
}

fn memo_program_id() -> Pubkey {
    Pubkey::from_str(MEMO_PROGRAM_ID).expect("memo program id constant is valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(active_id: i32, bin_step: u16, base_factor: u16) -> LbPairState {
        LbPairState {
            pool: Pubkey::new_unique(),
            parameters: StaticParameters {
                base_factor,
                filter_period: 30,
                decay_period: 600,
                reduction_factor: 5000,
                variable_fee_control: 0,
                max_volatility_accumulator: 300_000,
                protocol_share: 1000,
                base_fee_power_factor: 0,
                collect_fee_mode: 0,
            },
            active_id,
            bin_step,
            status: 0,
            token_x_mint: Pubkey::new_unique(),
            token_y_mint: Pubkey::new_unique(),
            reserve_x: Pubkey::new_unique(),
            reserve_y: Pubkey::new_unique(),
            oracle: Pubkey::new_unique(),
        }
    }

    fn market(
        active_id: i32,
        bin_step: u16,
        base_factor: u16,
        bins: Vec<(i32, u64, u64)>,
    ) -> DlmmMarket {
        let s = state(active_id, bin_step, base_factor);
        let v_params = VariableParameters {
            volatility_accumulator: 0,
            volatility_reference: 0,
            index_reference: active_id,
            last_update_timestamp: 1,
        };
        let mut available: Vec<i64> = Vec::new();
        let array_index = bin_id_to_array_index(active_id);
        for swap_for_y in [true, false] {
            for index in candidate_array_indices(array_index, swap_for_y) {
                if !available.contains(&index) {
                    available.push(index);
                }
            }
        }
        DlmmMarket::new(
            s,
            v_params,
            crate::chains::solana::spl_token::id(),
            crate::chains::solana::spl_token::id(),
            (9, 6),
            u64::MAX,
            u64::MAX,
            None,
            None,
            bins,
            available,
            false,
        )
    }

    #[test]
    fn price_at_bin_zero_is_exactly_one() {
        assert_eq!(get_price_from_id(0, 4), Some(1u128 << 64));
    }

    #[test]
    fn price_falls_as_bin_id_falls_and_rises_as_it_rises() {
        let base = get_price_from_id(0, 100).unwrap();
        let up = get_price_from_id(10, 100).unwrap();
        let down = get_price_from_id(-10, 100).unwrap();
        assert!(up > base);
        assert!(down < base);
        // Symmetric bin steps up/down should be reciprocal to a tight
        // tolerance (integer rounding in the repeated-squaring algorithm).
        let product = (up as f64 / (1u128 << 64) as f64) * (down as f64 / (1u128 << 64) as f64);
        assert!((product - 1.0).abs() < 1e-6, "product was {product}");
    }

    #[test]
    fn a_swap_within_one_bin_never_crosses() {
        let m = market(0, 4, 10000, vec![(0, 1_000_000_000, 1_000_000_000)]);
        let out = m.quote(&m.state.token_x_mint, 1_000_000).expect("in range");
        assert!(out.expected_out > 0);
        assert!(out.lp_fee > 0, "base_factor=10000 must charge something");
    }

    #[test]
    fn selling_x_for_y_consumes_amount_y_capacity_and_moves_price_down() {
        let m = market(
            0,
            4,
            0,
            vec![(0, 1_000_000_000, 100), (-1, 1_000_000_000, 1_000_000_000)],
        );
        // The active bin only has 100 raw units of Y; a bigger sell must
        // cross into bin -1.
        let q = m
            .quote(&m.state.token_x_mint, 10_000)
            .expect("crosses one bin");
        assert!(
            q.expected_out > 100,
            "must have crossed past the 100-unit bin"
        );
    }

    #[test]
    fn a_swap_that_exhausts_every_loaded_bin_refuses_rather_than_guesses() {
        let m = market(0, 4, 0, vec![(0, 100, 100)]);
        let err = m
            .quote(&m.state.token_x_mint, 1_000_000_000)
            .expect_err("size beyond every loaded bin must not be quoted");
        match err {
            DirectSwapError::InsufficientLiquidity { detail, .. } => {
                assert!(detail.contains("loaded bin arrays"), "got: {detail}");
            }
            other => panic!("expected InsufficientLiquidity, got {other:?}"),
        }
    }

    #[test]
    fn a_bin_array_index_floors_towards_negative_infinity() {
        assert_eq!(bin_id_to_array_index(-5594), -80);
        assert_eq!(bin_id_to_array_index(0), 0);
        assert_eq!(bin_id_to_array_index(-1), -1);
        assert_eq!(bin_id_to_array_index(69), 0);
        assert_eq!(bin_id_to_array_index(70), 1);
    }

    #[test]
    fn the_bin_array_pda_matches_a_live_transactions_account() {
        // Live transaction
        // 3oivfHFnTrFRAxBD4d6zqLqiubQ1umSrgP3gApv2fCoMzgEd129bg2YsjCH4t5K94vmBsMqD5t2dKAAgftpuvfzZ
        // passed this exact address for array_index = -80 on pool
        // 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6.
        let program = Pubkey::from_str(METEORA_DLMM_PROGRAM_ID).unwrap();
        let pool = Pubkey::from_str("5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6").unwrap();
        let expected = Pubkey::from_str("6MeamjT3xB2symUVrndFiu9bCU375m8vniQEpEwngyLM").unwrap();
        assert_eq!(bin_array_address(&program, &pool, -80), expected);
    }

    #[test]
    fn the_oracle_and_bitmap_pdas_match_a_live_pools_accounts() {
        let program = Pubkey::from_str(METEORA_DLMM_PROGRAM_ID).unwrap();
        let pool = Pubkey::from_str("5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6").unwrap();
        assert_eq!(
            oracle_address(&program, &pool),
            Pubkey::from_str("59YuGWPunbchD2mbi9U7qvjWQKQReGeepn4ZSr9zz9Li").unwrap()
        );
        assert_eq!(
            bitmap_extension_address(&program, &pool),
            Pubkey::from_str("DArpuuqJxNLRGQ8xq5ebZbobyjxSWWsPq8MqSZ2fUZLE").unwrap()
        );
    }

    #[test]
    fn the_event_authority_pda_matches_a_live_transaction() {
        let program = Pubkey::from_str(METEORA_DLMM_PROGRAM_ID).unwrap();
        assert_eq!(
            event_authority_address(&program),
            Pubkey::from_str("D1ZN9Wj1fRSUQfCjhvnu1hqDMT7hzjzBBpi12nVniYD6").unwrap()
        );
    }

    #[test]
    fn a_higher_base_factor_charges_more() {
        let cheap = market(0, 4, 1, vec![(0, 1_000_000_000, 1_000_000_000)]);
        let pricey = market(0, 4, 10000, vec![(0, 1_000_000_000, 1_000_000_000)]);
        let q_cheap = cheap.quote(&cheap.state.token_x_mint, 1_000_000).unwrap();
        let q_pricey = pricey.quote(&pricey.state.token_x_mint, 1_000_000).unwrap();
        assert!(q_pricey.lp_fee > q_cheap.lp_fee);
        assert!(q_pricey.expected_out < q_cheap.expected_out);
    }

    #[test]
    fn a_zero_amount_mint_mismatch_is_pair_not_in_pool() {
        let m = market(0, 4, 10000, vec![(0, 1_000_000_000, 1_000_000_000)]);
        let stranger = Pubkey::new_unique();
        assert!(matches!(
            m.quote(&stranger, 1_000),
            Err(DirectSwapError::PairNotInPool { .. })
        ));
    }
}
