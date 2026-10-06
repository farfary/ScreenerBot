// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Meteora DAMM v2 (`cpamdpZC…`, the programme Meteora calls `cp-amm`) `Pool`
//! account.
//!
//! # Layout, verified against mainnet
//!
//! `Pool` is 1112 bytes. Every offset below was read back off live pools and
//! cross-checked against the programme's own on-chain Anchor IDL:
//!
//! ```text
//!   8 pool_fees.base_fee (32 bytes, a mode-tagged union)
//!  48 protocol_fee_percent   50 referral_fee_percent   54 compounding_fee_bps
//!  56 dynamic_fee (initialized u8, then the volatility state)
//! 168 token_a_mint  200 token_b_mint  232 token_a_vault  264 token_b_vault
//! 360 liquidity u128 (Q64.64)         392 protocol_a_fee  400 protocol_b_fee
//! 424 sqrt_min_price 440 sqrt_max_price 456 sqrt_price (all Q64.64)
//! 472 activation_point  480 activation_type  481 pool_status
//! 482 token_a_flag      483 token_b_flag     484 collect_fee_mode
//! ```

use crate::chains::solana::layout::{pubkey_at, u128_at, u16_at, u32_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// The pool's stored dynamic-fee state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DynamicFee {
    pub initialized: bool,
    pub variable_fee_control: u32,
    pub bin_step: u16,
    pub volatility_accumulator: u128,
}

impl DynamicFee {
    /// Decode the dynamic-fee block at `Pool::pool_fees.dynamic_fee`.
    pub fn decode(data: &[u8], offset: usize) -> Option<Self> {
        Some(Self {
            initialized: u8_at(data, offset)? != 0,
            variable_fee_control: u32_at(data, offset + 12)?,
            bin_step: u16_at(data, offset + 16)?,
            volatility_accumulator: u128_at(data, offset + 64)?,
        })
    }
}

/// The base-fee union, already resolved to the one variant its tag selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseFee {
    /// A fee that starts at `cliff` and steps DOWN over time. `linear` selects
    /// between subtracting `reduction_factor` per period and compounding a
    /// `reduction_factor` basis-point cut per period.
    TimeScheduler {
        cliff: u64,
        periods: u16,
        period_frequency: u64,
        reduction_factor: u64,
        linear: bool,
    },
    /// A fee that steps UP with trade size, for a bounded window after the pool
    /// activates. Only the leg that spends token B is rate-limited.
    RateLimiter {
        cliff: u64,
        fee_increment_bps: u16,
        max_limiter_duration: u32,
        max_fee_bps: u32,
        reference_amount: u64,
    },
    /// Every other tag, including the market-cap scheduler: the cliff is the
    /// highest fee any scheduler charges, so quoting at it can only under-state
    /// the output.
    Cliff { cliff: u64 },
}

impl BaseFee {
    /// Decode the 32-byte union at `Pool::pool_fees.base_fee`.
    pub fn decode(data: &[u8], offset: usize) -> Option<Self> {
        let cliff = u64_at(data, offset)?;
        let mode = u8_at(data, offset + 8)?;
        Some(match mode {
            0 | 1 => Self::TimeScheduler {
                cliff,
                periods: u16_at(data, offset + 14)?,
                period_frequency: u64_at(data, offset + 16)?,
                reduction_factor: u64_at(data, offset + 24)?,
                linear: mode == 0,
            },
            2 => Self::RateLimiter {
                cliff,
                fee_increment_bps: u16_at(data, offset + 14)?,
                max_limiter_duration: u32_at(data, offset + 16)?,
                max_fee_bps: u32_at(data, offset + 20)?,
                reference_amount: u64_at(data, offset + 24)?,
            },
            _ => Self::Cliff { cliff },
        })
    }
}

/// The parts of the cp-amm `Pool` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DammPoolState {
    pub pool: Pubkey,
    pub base_fee: BaseFee,
    pub dynamic_fee: DynamicFee,
    pub collect_fee_mode: u8,
    pub compounding_fee_bps: u16,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub liquidity: u128,
    pub protocol_fee_a: u64,
    pub protocol_fee_b: u64,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub activation_type: u8,
    pub pool_status: u8,
}

impl DammPoolState {
    /// Decode a cp-amm pool account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            base_fee: BaseFee::decode(data, 8)?,
            dynamic_fee: DynamicFee::decode(data, 56)?,
            compounding_fee_bps: u16_at(data, 54)?,
            mint_a: pubkey_at(data, 168)?,
            mint_b: pubkey_at(data, 200)?,
            vault_a: pubkey_at(data, 232)?,
            vault_b: pubkey_at(data, 264)?,
            liquidity: u128_at(data, 360)?,
            protocol_fee_a: u64_at(data, 392)?,
            protocol_fee_b: u64_at(data, 400)?,
            sqrt_min_price: u128_at(data, 424)?,
            sqrt_max_price: u128_at(data, 440)?,
            sqrt_price: u128_at(data, 456)?,
            activation_point: u64_at(data, 472)?,
            activation_type: u8_at(data, 480)?,
            pool_status: u8_at(data, 481)?,
            collect_fee_mode: u8_at(data, 484)?,
        })
    }
}
