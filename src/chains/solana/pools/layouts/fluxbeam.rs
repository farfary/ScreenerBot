// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! FluxBeam AMM (`FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X`) `SwapV1` account,
//! the `spl-token-swap` layout with ONE extra leading byte the vanilla struct
//! does not have.
//!
//! # Layout, verified against mainnet
//!
//! `SwapV1` is 324 bytes:
//!
//! ```text
//!  0 version u8        1 is_initialized bool   2 bump_seed u8
//!  3 token_program_id (legacy, vestigial -- see below)
//! 35 token_a_vault     67 token_b_vault        99 pool_mint
//! 131 token_a_mint     163 token_b_mint        195 pool_fee_account
//! 227 fees: trade_fee_numerator/denominator, owner_trade_fee_numerator/
//!     denominator, owner_withdraw_fee_numerator/denominator, host_fee_numerator/
//!     denominator -- eight u64s, 64 bytes
//! 291 curve_type u8    292 curve_parameters (32 bytes, unused here)
//! ```
//! `3 + 1 + 1 + 32*8 + 64 + 1 + 32 == 324`, the real account length, confirmed
//! against pool `7uajENggf2MaiZ5XGff91uoVsch1y5QN3bqjisv7eP6V` (a SOL /
//! Token-2022 pool) AND `5oA8PtzRTkvw8qfY6GWaaEGBfXscut25JtZq5XHLCHwi`. Both
//! decode `version == 1`, `is_initialized == true`, and `pool_fee_account`
//! matching the exact address a real swap passed as its fee account. The
//! existing PRICE decoder at `pools/decoders/fluxbeam_amm.rs` reads
//! `token_a_vault@35`, `token_b_vault@67`, `token_a_mint@131`,
//! `token_b_mint@163` -- all four CONFIRMED correct here independently.
//!
//! `token_program_id@3` is a single, pool-wide field left over from the vanilla
//! struct. On the confirmed Token-2022 pool it read the Token-2022 programme
//! id even though `token_a` (SOL) is a LEGACY mint -- it does not describe
//! either side reliably, so it is not decoded.
//!
//! No status/pause bit exists anywhere in these 324 bytes (every byte between
//! the header and the curve is accounted for above).

use crate::chains::solana::layout::{pubkey_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// The parts of a `SwapV1` account a swap needs. See the module docs for the
/// full byte-offset table and how it was confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxbeamPoolState {
    pub pool: Pubkey,
    pub is_initialized: bool,
    pub vault_a: Pubkey,
    pub vault_b: Pubkey,
    pub pool_mint: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub fee_account: Pubkey,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub owner_trade_fee_numerator: u64,
    pub owner_trade_fee_denominator: u64,
    pub curve_type: u8,
}

impl FluxbeamPoolState {
    /// Decode a `SwapV1` account. Pure: no RPC, no cache, no clock. Layout
    /// confirmed byte-for-byte against two live pools -- see the module docs.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() != 324 {
            return None;
        }
        Some(Self {
            pool,
            is_initialized: u8_at(data, 1)? != 0,
            vault_a: pubkey_at(data, 35)?,
            vault_b: pubkey_at(data, 67)?,
            pool_mint: pubkey_at(data, 99)?,
            mint_a: pubkey_at(data, 131)?,
            mint_b: pubkey_at(data, 163)?,
            fee_account: pubkey_at(data, 195)?,
            trade_fee_numerator: u64_at(data, 227)?,
            trade_fee_denominator: u64_at(data, 235)?,
            owner_trade_fee_numerator: u64_at(data, 243)?,
            owner_trade_fee_denominator: u64_at(data, 251)?,
            curve_type: u8_at(data, 291)?,
        })
    }
}
