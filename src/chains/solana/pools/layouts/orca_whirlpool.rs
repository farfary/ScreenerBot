// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Orca Whirlpool (`whirLbMi…`) `Whirlpool`, `TickArray`, `DynamicTickArray` and
//! `Oracle` accounts.
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
//! # Tick arrays
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
//! Newer pools use a `DynamicTickArray` instead, and the programme accepts
//! either: `start_tick_index i32` at 8, `whirlpool` at 12, `tick_bitmap u128` at
//! 44, then 88 ticks from offset 60, each a tag byte (0 = uninitialised, 1 =
//! initialised) followed, only when initialised, by 112 bytes whose first two
//! fields are `liquidity_net: i128` and `liquidity_gross: u128`. Its length is
//! `148 + 112 * initialised_ticks`; the PUMP pool
//! `BofA2ViUSudPBTUms2KRuG6AHNeMawjNfwqTJDgx5BKW` passed an 8,548-byte array
//! (75 initialised ticks) that the fixed-only decoder refused.
//!
//! # `Oracle`
//!
//! The `Oracle` account layout itself (`whirlpool` at 8,
//! `trade_enable_timestamp` at 40, `adaptive_fee_constants` at 48,
//! `adaptive_fee_variables` at 82) comes from the on-chain IDL and was not
//! independently confirmed against a live adaptive-fee pool.

use crate::chains::solana::layout::{i128_at, i32_at, pubkey_at, u128_at, u16_at, u32_at, u64_at};
use crate::chains::solana::pools::layouts::clmm_ticks::InitializedTick;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Ticks stored per `TickArray` account. Orca's own constant, distinct from
/// Raydium CLMM's 60.
pub const TICK_ARRAY_SIZE: i32 = 88;

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

/// Exact length of a classic fixed-size tick array.
const FIXED_TICK_ARRAY_LEN: usize =
    TICKS_OFFSET + (TICK_ARRAY_SIZE as usize) * TICK_STATE_SIZE + 32;

/// `DynamicTickArray`: `tick_bitmap: u128` after the discriminator(8),
/// `start_tick_index`(4) and `whirlpool`(32).
const DYNAMIC_TICK_BITMAP_OFFSET: usize = 44;

/// `DynamicTickArray`: first tick tag byte, right after the bitmap.
const DYNAMIC_TICKS_OFFSET: usize = 60;

/// `DynamicTickData`: `liquidity_net: i128`, `liquidity_gross: u128`,
/// `fee_growth_outside_a/b: u128`, `reward_growths_outside: [u128; 3]`.
const DYNAMIC_TICK_DATA_SIZE: usize = 112;

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
            protocol_fee_owed_a: u64_at(data, 85)?,
            protocol_fee_owed_b: u64_at(data, 93)?,
            mint_a: pubkey_at(data, 101)?,
            vault_a: pubkey_at(data, 133)?,
            mint_b: pubkey_at(data, 181)?,
            vault_b: pubkey_at(data, 213)?,
        })
    }
}

/// Decode the initialised ticks out of a live tick-array account, in either of
/// the two layouts Orca's programme accepts (see module docs).
///
/// Returns `None` when the account matches neither layout -- a decode failure,
/// never a partially-wrong swap. Neither layout stores a tick's own index, so
/// `start` and `tick_spacing` (both already known from the pool state and the
/// derivation that produced this address) supply it instead.
pub fn decode_tick_array(
    data: &[u8],
    start: i32,
    tick_spacing: u16,
) -> Option<Vec<InitializedTick>> {
    if data.len() == FIXED_TICK_ARRAY_LEN {
        decode_fixed_tick_array(data, start, tick_spacing)
    } else {
        decode_dynamic_tick_array(data, start, tick_spacing)
    }
}

/// A `DynamicTickArray`: variable length, one tag byte per tick, and a bitmap
/// that must agree with those tags.
///
/// Every structural fact is checked rather than assumed -- the stored start
/// index, each tag against its bitmap bit, `liquidity_gross >= |liquidity_net|`
/// for every initialised tick, and that the ticks consume the account exactly.
/// A layout that drifted by one field fails at least one of those.
fn decode_dynamic_tick_array(
    data: &[u8],
    start: i32,
    tick_spacing: u16,
) -> Option<Vec<InitializedTick>> {
    if i32_at(data, 8)? != start {
        return None;
    }
    let bitmap = u128_at(data, DYNAMIC_TICK_BITMAP_OFFSET)?;
    let mut offset = DYNAMIC_TICKS_OFFSET;
    let mut ticks = Vec::new();
    for i in 0..(TICK_ARRAY_SIZE as usize) {
        let tag = *data.get(offset)?;
        offset += 1;
        let flagged = bitmap & (1u128 << i) != 0;
        match (tag, flagged) {
            (0, false) => {}
            (1, true) => {
                let liquidity_net = i128_at(data, offset)?;
                let liquidity_gross = u128_at(data, offset + 16)?;
                if liquidity_gross < liquidity_net.unsigned_abs() {
                    return None;
                }
                ticks.push(InitializedTick {
                    tick: start + (i as i32) * (tick_spacing as i32),
                    liquidity_net,
                });
                offset += DYNAMIC_TICK_DATA_SIZE;
            }
            _ => return None,
        }
    }
    (offset == data.len()).then_some(ticks)
}

/// A classic fixed-size `TickArray`.
fn decode_fixed_tick_array(
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

/// Whether an `Oracle` account's adaptive fee is active. `None` when the
/// account is too short to carry the field at all, which is NOT the same
/// answer as "inert": an oracle that exists but does not decode is a layout
/// we do not understand, and a `false` there would quote a pool whose real
/// fee may exceed `fee_rate`. The caller refuses on `None` for that reason.
pub fn oracle_has_active_adaptive_fee(data: &[u8]) -> Option<bool> {
    u32_at(data, 54).map(|factor| factor != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dynamic array with initialised ticks at slots 2 and 40.
    fn dynamic_tick_array(start: i32, corrupt_bitmap: bool) -> Vec<u8> {
        let mut data = vec![0u8; DYNAMIC_TICKS_OFFSET];
        data[8..12].copy_from_slice(&start.to_le_bytes());
        let mut bitmap: u128 = (1 << 2) | (1 << 40);
        if corrupt_bitmap {
            bitmap |= 1 << 7;
        }
        data[DYNAMIC_TICK_BITMAP_OFFSET..DYNAMIC_TICKS_OFFSET]
            .copy_from_slice(&bitmap.to_le_bytes());
        for i in 0..TICK_ARRAY_SIZE as usize {
            if i == 2 || i == 40 {
                data.push(1);
                let net: i128 = if i == 2 { 5_000 } else { -5_000 };
                let mut tick = vec![0u8; DYNAMIC_TICK_DATA_SIZE];
                tick[0..16].copy_from_slice(&net.to_le_bytes());
                tick[16..32].copy_from_slice(&5_000u128.to_le_bytes());
                data.extend_from_slice(&tick);
            } else {
                data.push(0);
            }
        }
        data
    }

    #[test]
    fn a_dynamic_tick_array_decodes_and_a_malformed_one_is_refused() {
        let data = dynamic_tick_array(-880, false);
        assert_eq!(data.len(), 148 + 2 * DYNAMIC_TICK_DATA_SIZE);
        let ticks = decode_tick_array(&data, -880, 10).expect("well-formed dynamic array");
        assert_eq!(ticks.len(), 2);
        assert_eq!((ticks[0].tick, ticks[0].liquidity_net), (-860, 5_000));
        assert_eq!((ticks[1].tick, ticks[1].liquidity_net), (-480, -5_000));

        assert!(
            decode_tick_array(&data, 0, 10).is_none(),
            "wrong start index"
        );
        assert!(
            decode_tick_array(&dynamic_tick_array(-880, true), -880, 10).is_none(),
            "a bitmap that disagrees with the tags"
        );
        assert!(
            decode_tick_array(&data[..data.len() - 1], -880, 10).is_none(),
            "a truncated account"
        );
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
