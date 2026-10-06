// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Meteora Dynamic Bonding Curve (`dbcij3LW…`) `VirtualPool` and `PoolConfig`
//! accounts.
//!
//! # Layout, verified against mainnet
//!
//! Both account layouts were read from the programme's own on-chain Anchor
//! IDL (`create_with_seed(find_program_address([], program), "anchor:idl",
//! program)`) and cross-checked field-by-field against a live pool
//! (`A95th9YTiZrGYRsZ4eBXLvMLpr7pfqPVWwY1LFQ8f27U`) and its config
//! (`7wr6arSoaxQEppcSakvouxpKF9bfcYLCRRn4HNMxj2cZ`): the IDL's own
//! discriminator for `VirtualPool` (`d5e005d16245775c`) matches the pool
//! account's first 8 bytes exactly, and `PoolConfig`'s (`1a6c0e7b74e6812b`)
//! matches the config account's.
//!
//! `VirtualPool`, 424 bytes:
//!
//! ```text
//!   0 discriminator        8 volatility_tracker (64B, unused here)
//!  72 config               104 creator            136 base_mint
//! 168 base_vault           200 quote_vault
//! 232 base_reserve u64     240 quote_reserve u64
//! 248 protocol_base_fee    256 protocol_quote_fee
//! 264 partner_base_fee     272 partner_quote_fee
//! 280 sqrt_price u128      296 activation_point u64
//! 304 pool_type u8         305 is_migrated u8
//! ```
//!
//! `base_reserve`/`quote_reserve` are the curve's own reserves. The vaults hold
//! them plus the uncollected protocol, partner and creator fees, which are not
//! liquidity. `quote_mint` is not stored here; it lives on the `PoolConfig` the
//! pool points at.
//!
//! `PoolConfig`, 1048 bytes:
//!
//! ```text
//!   0 discriminator         8 quote_mint          40 fee_claimer
//! 104 base_fee.cliff_fee_numerator u64  112 second_factor u64
//! 120 third_factor u64      128 first_factor u16   130 base_fee_mode u8
//! 136 dynamic_fee.initialized u8
//! 144 max_volatility_accumulator u32    148 variable_fee_control u32
//! 152 bin_step u16          154 filter_period u16  156 decay_period u16
//! 158 reduction_factor u16
//! 232 collect_fee_mode u8   235 token_decimal u8
//! 392 sqrt_start_price u128
//! 408 curve: [LiquidityDistributionConfig; 20], 32B each
//!       (sqrt_price u128, liquidity u128)
//! ```

use crate::chains::solana::layout::{pubkey_at, u128_at, u16_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// `VirtualPool`'s own Anchor discriminator, confirmed against the on-chain
/// IDL and a live pool account's first 8 bytes.
pub(crate) const VIRTUAL_POOL_DISCRIMINATOR: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];

/// `PoolConfig`'s own Anchor discriminator, confirmed the same way.
const POOL_CONFIG_DISCRIMINATOR: [u8; 8] = [26, 108, 14, 123, 116, 230, 129, 43];

/// Curve checkpoints a `PoolConfig` may carry.
const MAX_CURVE_POINTS: usize = 20;

/// The parts of a DBC `VirtualPool` that swapping and pricing read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualPoolState {
    pub pool: Pubkey,
    pub config: Pubkey,
    pub base_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub sqrt_price: u128,
    pub is_migrated: bool,
}

impl VirtualPoolState {
    /// Decode a `VirtualPool` account. Pure: no RPC, no cache, no clock. `None`
    /// unless the account carries the `VirtualPool` discriminator.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() < 306 || data[0..8] != VIRTUAL_POOL_DISCRIMINATOR {
            return None;
        }
        Some(Self {
            pool,
            config: pubkey_at(data, 72)?,
            base_mint: pubkey_at(data, 136)?,
            base_vault: pubkey_at(data, 168)?,
            quote_vault: pubkey_at(data, 200)?,
            base_reserve: u64_at(data, 232)?,
            quote_reserve: u64_at(data, 240)?,
            sqrt_price: u128_at(data, 280)?,
            is_migrated: u8_at(data, 305)? != 0,
        })
    }
}

/// One `LiquidityDistributionConfig` checkpoint: the curve's price at this
/// point and the constant liquidity of the segment ENDING here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CurvePoint {
    pub(crate) sqrt_price: u128,
    pub(crate) liquidity: u128,
}

/// The parts of a DBC `PoolConfig` a swap needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolConfigState {
    pub quote_mint: Pubkey,
    pub cliff_fee_numerator: u64,
    pub second_factor: u64,
    pub third_factor: u64,
    pub first_factor: u16,
    pub base_fee_mode: u8,
    pub dynamic_fee_initialized: bool,
    pub collect_fee_mode: u8,
    pub sqrt_start_price: u128,
    pub(crate) curve: Vec<CurvePoint>,
}

impl PoolConfigState {
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 1048 || data[0..8] != POOL_CONFIG_DISCRIMINATOR {
            return None;
        }
        let mut curve = Vec::with_capacity(MAX_CURVE_POINTS);
        for i in 0..MAX_CURVE_POINTS {
            let offset = 408 + i * 32;
            let sqrt_price = u128_at(data, offset)?;
            let liquidity = u128_at(data, offset + 16)?;
            if sqrt_price == 0 {
                break;
            }
            curve.push(CurvePoint {
                sqrt_price,
                liquidity,
            });
        }
        Some(Self {
            quote_mint: pubkey_at(data, 8)?,
            cliff_fee_numerator: crate::chains::solana::layout::u64_at(data, 104)?,
            second_factor: crate::chains::solana::layout::u64_at(data, 112)?,
            third_factor: crate::chains::solana::layout::u64_at(data, 120)?,
            first_factor: u16_at(data, 128)?,
            base_fee_mode: u8_at(data, 130)?,
            dynamic_fee_initialized: u8_at(data, 136)? != 0,
            collect_fee_mode: u8_at(data, 232)?,
            sqrt_start_price: u128_at(data, 392)?,
            curve,
        })
    }

    /// The real, non-zero curve checkpoints this config carries -- exposed for
    /// the offline test tier to assert they are genuine segments, not padding.
    pub fn curve_points(&self) -> Vec<(u128, u128)> {
        self.curve
            .iter()
            .map(|p| (p.sqrt_price, p.liquidity))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_data() -> Vec<u8> {
        let mut data = vec![0u8; 424];
        data[0..8].copy_from_slice(&VIRTUAL_POOL_DISCRIMINATOR);
        data
    }

    #[test]
    fn every_field_is_read_at_its_own_offset() {
        let mut data = pool_data();
        let config = Pubkey::new_unique();
        let base_mint = Pubkey::new_unique();
        let base_vault = Pubkey::new_unique();
        let quote_vault = Pubkey::new_unique();
        data[72..104].copy_from_slice(config.as_ref());
        data[136..168].copy_from_slice(base_mint.as_ref());
        data[168..200].copy_from_slice(base_vault.as_ref());
        data[200..232].copy_from_slice(quote_vault.as_ref());
        data[232..240].copy_from_slice(&11u64.to_le_bytes());
        data[240..248].copy_from_slice(&22u64.to_le_bytes());
        data[280..296].copy_from_slice(&33u128.to_le_bytes());
        data[305] = 1;

        let state = VirtualPoolState::decode(Pubkey::new_unique(), &data).expect("decodes");
        assert_eq!(state.config, config);
        assert_eq!(state.base_mint, base_mint);
        assert_eq!(state.base_vault, base_vault);
        assert_eq!(state.quote_vault, quote_vault);
        assert_eq!(state.base_reserve, 11);
        assert_eq!(state.quote_reserve, 22);
        assert_eq!(state.sqrt_price, 33);
        assert!(state.is_migrated);
    }

    #[test]
    fn an_account_without_the_discriminator_or_too_short_is_not_a_pool() {
        let mut data = pool_data();
        data[0] ^= 1;
        assert!(VirtualPoolState::decode(Pubkey::new_unique(), &data).is_none());
        assert!(VirtualPoolState::decode(Pubkey::new_unique(), &pool_data()[..305]).is_none());
    }
}
