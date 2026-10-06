// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Meteora DLMM (`LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo`) `LbPair` and
//! `BinArray` accounts.
//!
//! # `LbPair`, verified against a live pool
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
//! 584 bin_array_bitmap [u64; 16]  (not decoded)
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
//! # `BinArray`, 10136 bytes
//!
//! ```text
//! 8 index i64   16 version u8   24 lb_pair   56 bins[70], 144 bytes each
//! ```
//! `56 + 70*144 + 8 == 10136`, the real fetched account length. Verified against
//! the live bin array `6MeamjT3xB2symUVrndFiu9bCU375m8vniQEpEwngyLM` (array
//! index -80 of the pool above): `bins[6]` (bin id `-80*70 + 6 == -5594`, the pool's
//! own `active_id` at the same read) decodes a plausible Q64.64 price
//! (`1969412515201075439 / 2^64 ≈ 0.1067`, matching `(1.0004)^-5594 ≈ 0.1069`
//! computed independently from `bin_step = 4`) and non-zero `amount_y`. A `Bin`
//! entry: `amount_x u64@0 amount_y u64@8 price u128@16 liquidity_supply u128@32
//! ... open_order_amount u64@112 ... limit_order_ask_side u8@140`.

use crate::chains::solana::layout::{i32_at, i64_at, pubkey_at, u16_at, u32_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Bins per `BinArray` account. Distinct from Raydium CLMM's 60 and Orca
/// Whirlpool's 88.
pub const MAX_BIN_PER_ARRAY: i32 = 70;

/// Bytes from the start of a `BinArray` account to its first `Bin` entry: an
/// 8-byte Anchor discriminator, `index: i64`, `version: u8`, 7 bytes padding,
/// `lb_pair: Pubkey`.
const BINS_OFFSET: usize = 56;

/// One `Bin` entry's byte size, verified against a live account (see module
/// docs): `56 + 70*144 + 8 == 10136`, the real fetched `BinArray` length.
const BIN_SIZE: usize = 144;

/// The dynamic (market-driven) fee parameters, unpacked from `LbPair.v_parameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariableParameters {
    pub volatility_accumulator: u32,
    pub volatility_reference: u32,
    pub index_reference: i32,
    pub last_update_timestamp: i64,
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

/// A single bin's swappable liquidity: the market-making amounts only, never
/// the limit-order amounts a `Bin` also carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedBin {
    pub id: i32,
    pub amount_x: u64,
    pub amount_y: u64,
}

/// Decode the bins out of a live `BinArray` account. `None` when the account
/// is too short -- a decode failure, never a partially-wrong swap.
pub fn decode_bin_array(data: &[u8], array_index: i64) -> Option<Vec<DecodedBin>> {
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
