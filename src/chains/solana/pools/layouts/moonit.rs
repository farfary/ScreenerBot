// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Moonit (`MoonCVVN…`) `CurveAccount` and global `ConfigAccount` accounts.
//!
//! # `CurveAccount`, verified byte-for-byte against two live pools
//!
//! Matches the on-chain IDL's `CurveAccount` struct exactly (409-byte
//! account, most of it unused reserved padding):
//!
//! ```text
//!  8 total_supply u64        16 curve_amount u64        24 mint pubkey
//! 56 decimals u8             57 collateral_currency u8  58 curve_type u8
//! 59 marketcap_threshold u64 67 marketcap_currency u8   68 migration_fee u64
//! 76 coef_b u32              80 bump u8                 81 migration_target u8
//! ```
//!
//! `curve_amount` was independently confirmed to equal the curve's own SPL
//! token account balance to the raw unit on both live pools. The deployed
//! programme's account carries one undocumented field beyond its own published
//! IDL, `price_increase` (`u16` @82); neither it nor `coef_b` takes part in the
//! `ConstantProductV1` price, so neither is decoded here.

use crate::chains::solana::layout::{pubkey_at, u16_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// `curve_type` byte for `ConstantProductV1`, the only curve whose formula is
/// verified here.
pub const CURVE_TYPE_CONSTANT_PRODUCT_V1: u8 = 1;
/// `collateral_currency` byte for SOL.
pub const COLLATERAL_CURRENCY_SOL: u8 = 0;

/// The reference implementation's global virtual reserves
/// (`@heliofi/launchpad-common`'s `ConstantProductCurveV1`), reproduced
/// exactly rather than fitted: raw token units (9 dp) and lamports. The direct
/// swap venue (`swaps/direct/venues/moonit.rs`) documents the four-trade replay
/// and the live on-chain proof.
pub const INITIAL_VIRTUAL_TOKEN_RESERVES: u128 = 1_073_000_000_000_000_000;
pub const INITIAL_VIRTUAL_COLLATERAL_RESERVES: u128 = 30_000_000_000;

/// The current virtual `(token, collateral)` reserves at `curve_position`, the
/// reference SDK's own `getCurrentReserves`: recomputed fresh from the token
/// position rather than tracked continuously, floor-rounded, exactly as the
/// live programme does. `None` once the position leaves no virtual tokens.
pub fn current_reserves(curve_position: u128) -> Option<(u128, u128)> {
    let vtr = INITIAL_VIRTUAL_TOKEN_RESERVES.checked_sub(curve_position)?;
    if vtr == 0 {
        return None;
    }
    let constant_product = INITIAL_VIRTUAL_TOKEN_RESERVES * INITIAL_VIRTUAL_COLLATERAL_RESERVES;
    Some((vtr, constant_product / vtr))
}

/// The parts of Moonit's `CurveAccount` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurveAccountState {
    pub pool: Pubkey,
    pub total_supply: u64,
    pub curve_amount: u64,
    pub mint: Pubkey,
    pub decimals: u8,
    pub collateral_currency: u8,
    pub curve_type: u8,
}

impl CurveAccountState {
    /// Decode a `CurveAccount`. Pure. Layout verified byte-for-byte against
    /// two live pools -- see the module docs.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() < 82 {
            return None;
        }
        Some(Self {
            pool,
            total_supply: u64_at(data, 8)?,
            curve_amount: u64_at(data, 16)?,
            mint: pubkey_at(data, 24)?,
            decimals: u8_at(data, 56)?,
            collateral_currency: u8_at(data, 57)?,
            curve_type: u8_at(data, 58)?,
        })
    }

    /// Tokens sold so far, the reference curve's own `curvePosition`.
    pub fn curve_position(&self) -> u128 {
        (self.total_supply as u128).saturating_sub(self.curve_amount as u128)
    }
}

/// The parts of Moonit's global `ConfigAccount` a swap needs -- fetched fresh
/// at `load()` time rather than hardcoded, even though the reference SDK
/// hardcodes its own copies of these same values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigAccountState {
    pub helio_fee: Pubkey,
    pub dex_fee: Pubkey,
    pub fee_bps: u16,
}

impl ConfigAccountState {
    /// Decode a `ConfigAccount`. Pure. Verified against the live account:
    /// `fee_bps = 100`, `dex_fee_share = 50`, and `helio_fee`/`dex_fee` match
    /// the addresses every replayed real trade paid.
    pub fn decode(data: &[u8]) -> Option<Self> {
        Some(Self {
            // migration_authority @8, backend_authority @40, config_authority @72
            helio_fee: pubkey_at(data, 104)?,
            dex_fee: pubkey_at(data, 136)?,
            fee_bps: u16_at(data, 168)?,
        })
    }
}
