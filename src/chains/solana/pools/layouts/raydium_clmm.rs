// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Raydium CLMM (`CAMMCzo5…`) `PoolState`, `AmmConfig`, `TickArrayState` and
//! `TickArrayBitmapExtension` accounts.
//!
//! # Layout, verified against mainnet
//!
//! `PoolState` is 1544 bytes:
//!
//! ```text
//!   9 amm_config      73 token_mint_0  105 token_mint_1
//! 137 token_vault_0  169 token_vault_1 201 observation_key
//! 233 mint_decimals_0 234 mint_decimals_1 235 tick_spacing u16
//! 237 liquidity u128 253 sqrt_price_x64 u128  269 tick_current i32
//! 389 status u8      904 tick_array_bitmap [u64; 16]
//! ```
//!
//! `AmmConfig` is 117 bytes: `43 protocol_fee_rate u32 · 47 trade_fee_rate u32 ·
//! 51 tick_spacing u16 · 53 fund_fee_rate u32`, over a denominator of 1_000_000.

use crate::chains::solana::layout::{
    i128_at, i32_at, pubkey_at, u128_at, u16_at, u32_at, u64_at, u8_at,
};
use crate::chains::solana::pools::layouts::clmm_ticks::InitializedTick;
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Bit index of the swap permission inside `PoolState::status`, a DISABLE flag.
const STATUS_BIT_SWAP_DISABLED: u8 = 2;

/// Ticks stored per tick-array account.
pub const TICK_ARRAY_SIZE: i32 = 60;

/// Bits in the pool's own bitmap, i.e. array indices −512..511.
pub(crate) const POOL_BITMAP_BITS: i32 = 1_024;

/// Blocks of 512 bits either side of the pool bitmap in the extension account.
pub(crate) const EXTENSION_BLOCKS: usize = 14;

/// Bits per extension block.
pub(crate) const EXTENSION_BLOCK_BITS: i32 = 512;

/// The parts of the CLMM `PoolState` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClmmPoolState {
    pub pool: Pubkey,
    pub amm_config: Pubkey,
    pub mint_0: Pubkey,
    pub mint_1: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub observation: Pubkey,
    pub decimals_0: u8,
    pub decimals_1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub status: u8,
}

impl ClmmPoolState {
    /// Decode a CLMM pool account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            amm_config: pubkey_at(data, 9)?,
            mint_0: pubkey_at(data, 73)?,
            mint_1: pubkey_at(data, 105)?,
            vault_0: pubkey_at(data, 137)?,
            vault_1: pubkey_at(data, 169)?,
            observation: pubkey_at(data, 201)?,
            decimals_0: u8_at(data, 233)?,
            decimals_1: u8_at(data, 234)?,
            tick_spacing: u16_at(data, 235)?,
            liquidity: u128_at(data, 237)?,
            sqrt_price_x64: u128_at(data, 253)?,
            tick_current: i32_at(data, 269)?,
            status: u8_at(data, 389)?,
        })
    }

    /// Whether the swap permission bit is clear.
    pub fn swap_enabled(&self) -> bool {
        self.status & (1 << STATUS_BIT_SWAP_DISABLED) == 0
    }
}

/// The fee rates from the CLMM `AmmConfig`. Stored as `u32`, unlike CP-Swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClmmFeeConfig {
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub fund_fee_rate: u32,
}

impl ClmmFeeConfig {
    /// Decode a CLMM `AmmConfig` account. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        Some(Self {
            protocol_fee_rate: u32_at(data, 43)?,
            trade_fee_rate: u32_at(data, 47)?,
            fund_fee_rate: u32_at(data, 53)?,
        })
    }
}

/// Bytes from the start of a `TickArrayState` account to its first `TickState`
/// entry: an 8-byte Anchor discriminator, a 32-byte `pool_id`, and the 4-byte
/// `start_tick_index`.
///
/// Verified against live mainnet bytes, NOT the offset originally guessed for
/// this module (`start_tick_index` at 8): the real `TickArrayState` carries a
/// `pool_id: Pubkey` field between the discriminator and `start_tick_index`
/// that the guess omitted. Confirmed two ways against tick array
/// `7KGRHr8gSwVqmJVv3sdnUEmKM3jRC551SMCt9ZxmCXsb` (start index -22920) and
/// `5LuEHwAuoPAEJunEvBcDnTwRGnXWtC7JQmAgeXzA44cV` (start index -22980), both
/// derived PDAs of the SOL/USDC pool above: the on-chain Anchor IDL (pulled per
/// `adding-a-venue.md`) declares this exact layout, and decoding both accounts
/// at this offset reproduces `start_tick_index + i * tick_spacing` for all 60
/// `tick` fields with `tick_spacing = 1`.
const TICKS_OFFSET: usize = 44;

/// One `TickState` entry's byte size. `168` was the size named in the
/// unverified starting hypothesis and IS correct — confirmed by IDL field
/// layout (i32 + i128 + u128 + u128 + u128 + u128*3 + u64*3 + u128 + u32*3 =
/// 168 bytes) and independently by live bytes: `TICKS_OFFSET + 60 * 168 + 1 +
/// 8 + 107 == 10240`, the exact account length fetched from chain.
const TICK_STATE_SIZE: usize = 168;

/// Decode the initialised ticks out of a live `TickArrayState` account.
///
/// Returns `None` when the account does not belong to `pool` or is too short
/// to hold a full array -- a decode failure, never a partially-wrong swap.
pub fn decode_tick_array(pool: &Pubkey, data: &[u8]) -> Option<Vec<InitializedTick>> {
    if pubkey_at(data, 8)? != *pool {
        return None;
    }
    let mut ticks = Vec::new();
    for i in 0..(TICK_ARRAY_SIZE as usize) {
        let offset = TICKS_OFFSET + i * TICK_STATE_SIZE;
        let tick = i32_at(data, offset)?;
        let liquidity_net = i128_at(data, offset + 4)?;
        let liquidity_gross = u128_at(data, offset + 20)?;
        if liquidity_gross != 0 {
            ticks.push(InitializedTick {
                tick,
                liquidity_net,
            });
        }
    }
    Some(ticks)
}

/// A pool's initialised-tick-array bitmap: the pool's own 1024 bits plus, when
/// the extension account exists, the blocks either side of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickArrayBitmap {
    pub(crate) pool: [u64; 16],
    pub(crate) positive: Vec<[u64; 8]>,
    pub(crate) negative: Vec<[u64; 8]>,
}

impl TickArrayBitmap {
    /// Read the pool's own bitmap out of `PoolState` at offset 904.
    pub fn from_pool_state(data: &[u8]) -> Option<Self> {
        let mut pool = [0u64; 16];
        for (index, word) in pool.iter_mut().enumerate() {
            *word = u64_at(data, 904 + index * 8)?;
        }
        Some(Self {
            pool,
            positive: Vec::new(),
            negative: Vec::new(),
        })
    }

    /// Attach a `TickArrayBitmapExtension` account, if the pool has one.
    ///
    /// Layout: discriminator, `pool_id`, then 14 positive blocks of `[u64; 8]`
    /// followed by 14 negative blocks. A wrong `pool_id` is ignored rather than
    /// trusted — the extension is a PDA, but reading someone else's bitmap would
    /// send the swap to tick arrays that belong to another pool.
    pub fn with_extension(mut self, pool_id: &Pubkey, data: &[u8]) -> Self {
        if pubkey_at(data, 8) != Some(*pool_id) {
            return self;
        }
        let base = 40;
        let block = |offset: usize| -> Option<[u64; 8]> {
            let mut words = [0u64; 8];
            for (index, word) in words.iter_mut().enumerate() {
                *word = u64_at(data, offset + index * 8)?;
            }
            Some(words)
        };
        for i in 0..EXTENSION_BLOCKS {
            match block(base + i * 64) {
                Some(words) => self.positive.push(words),
                None => return self,
            }
        }
        for i in 0..EXTENSION_BLOCKS {
            match block(base + EXTENSION_BLOCKS * 64 + i * 64) {
                Some(words) => self.negative.push(words),
                None => return self,
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_tick_array_reads_only_the_entries_with_gross_liquidity() {
        let pool = Pubkey::new_unique();
        let mut data = vec![0u8; TICKS_OFFSET + (TICK_ARRAY_SIZE as usize) * TICK_STATE_SIZE + 200];
        data[8..40].copy_from_slice(&pool.to_bytes());
        data[40..44].copy_from_slice(&(-60i32).to_le_bytes());

        // Slot 0: a real initialised tick.
        let slot0 = TICKS_OFFSET;
        data[slot0..slot0 + 4].copy_from_slice(&(-60i32).to_le_bytes());
        data[slot0 + 4..slot0 + 20].copy_from_slice(&(12_345i128).to_le_bytes());
        data[slot0 + 20..slot0 + 36].copy_from_slice(&(999_999u128).to_le_bytes());

        // Slot 1: left at all zero -- an unused slot, not tick 0.
        // Slot 2: a negative liquidity_net, still initialised.
        let slot2 = TICKS_OFFSET + 2 * TICK_STATE_SIZE;
        data[slot2..slot2 + 4].copy_from_slice(&(-58i32).to_le_bytes());
        data[slot2 + 4..slot2 + 20].copy_from_slice(&(-500i128).to_le_bytes());
        data[slot2 + 20..slot2 + 36].copy_from_slice(&(1u128).to_le_bytes());

        let ticks = decode_tick_array(&pool, &data).expect("a full-length array decodes");
        assert_eq!(ticks.len(), 2, "the untouched zero slot must not appear");
        assert_eq!(
            ticks[0],
            InitializedTick {
                tick: -60,
                liquidity_net: 12_345
            }
        );
        assert_eq!(
            ticks[1],
            InitializedTick {
                tick: -58,
                liquidity_net: -500
            }
        );
    }

    #[test]
    fn decode_tick_array_refuses_an_account_belonging_to_another_pool() {
        let pool = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();
        let mut data = vec![0u8; TICKS_OFFSET + (TICK_ARRAY_SIZE as usize) * TICK_STATE_SIZE];
        data[8..40].copy_from_slice(&stranger.to_bytes());
        assert!(decode_tick_array(&pool, &data).is_none());
    }

    #[test]
    fn decode_tick_array_refuses_a_truncated_account_rather_than_reading_short() {
        let pool = Pubkey::new_unique();
        let mut data = vec![0u8; TICKS_OFFSET + 10];
        data[8..40].copy_from_slice(&pool.to_bytes());
        assert!(decode_tick_array(&pool, &data).is_none());
    }

    #[test]
    fn a_short_pool_account_has_no_bitmap_rather_than_a_wrong_one() {
        assert!(TickArrayBitmap::from_pool_state(&[0u8; 500]).is_none());
    }
}
