// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Raydium CPMM (`CPMMoo8L…`) `PoolState` and `AmmConfig` accounts.
//!
//! # Layout, verified against mainnet
//!
//! `PoolState` is 637 bytes. Offsets used here were read back off a live pool
//! rather than taken from a header file:
//!
//! ```text
//!   8 amm_config      40 pool_creator    72 token_0_vault  104 token_1_vault
//! 136 lp_mint        168 token_0_mint   200 token_1_mint   232 token_0_program
//! 264 token_1_program 296 observation   328 auth_bump      329 status
//! 330 lp_decimals    331 mint_0_decimals 332 mint_1_decimals
//! 333 lp_supply      341 protocol_fee_0 349 protocol_fee_1
//! 357 fund_fee_0     365 fund_fee_1     373 open_time      381 recent_epoch
//! 389 creator_fee_on 390 enable_creator_fee
//! 397 creator_fee_0  405 creator_fee_1
//! ```
//!
//! `AmmConfig` is 236 bytes: `12 trade_fee_rate · 20 protocol_fee_rate ·
//! 28 fund_fee_rate · 36 create_pool_fee · 108 creator_fee_rate`, all over a
//! denominator of 1_000_000.

use crate::chains::solana::layout::{pubkey_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Bit index of the swap permission inside `PoolState::status`. The bit is a
/// DISABLE flag: clear means swapping is allowed.
const STATUS_BIT_SWAP_DISABLED: u8 = 2;

/// The parts of `PoolState` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpmmPoolState {
    pub pool: Pubkey,
    pub amm_config: Pubkey,
    pub vault_0: Pubkey,
    pub vault_1: Pubkey,
    pub mint_0: Pubkey,
    pub mint_1: Pubkey,
    pub program_0: Pubkey,
    pub program_1: Pubkey,
    pub observation: Pubkey,
    pub decimals_0: u8,
    pub decimals_1: u8,
    pub status: u8,
    pub open_time: u64,
    pub protocol_fees_0: u64,
    pub protocol_fees_1: u64,
    pub fund_fees_0: u64,
    pub fund_fees_1: u64,
    pub creator_fees_0: u64,
    pub creator_fees_1: u64,
    pub creator_fee_enabled: bool,
}

impl CpmmPoolState {
    /// Decode a CP-Swap pool account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            amm_config: pubkey_at(data, 8)?,
            vault_0: pubkey_at(data, 72)?,
            vault_1: pubkey_at(data, 104)?,
            mint_0: pubkey_at(data, 168)?,
            mint_1: pubkey_at(data, 200)?,
            program_0: pubkey_at(data, 232)?,
            program_1: pubkey_at(data, 264)?,
            observation: pubkey_at(data, 296)?,
            status: u8_at(data, 329)?,
            decimals_0: u8_at(data, 331)?,
            decimals_1: u8_at(data, 332)?,
            protocol_fees_0: u64_at(data, 341)?,
            protocol_fees_1: u64_at(data, 349)?,
            fund_fees_0: u64_at(data, 357)?,
            fund_fees_1: u64_at(data, 365)?,
            open_time: u64_at(data, 373)?,
            creator_fee_enabled: u8_at(data, 390)? != 0,
            creator_fees_0: u64_at(data, 397)?,
            creator_fees_1: u64_at(data, 405)?,
        })
    }

    /// Whether the swap permission bit is clear.
    pub fn swap_enabled(&self) -> bool {
        self.status & (1 << STATUS_BIT_SWAP_DISABLED) == 0
    }

    /// Whether the pool has reached its open time.
    pub fn is_open(&self, now_unix: u64) -> bool {
        now_unix >= self.open_time
    }
}

/// The fee rates from `AmmConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpmmFeeConfig {
    pub trade_fee_rate: u64,
    pub protocol_fee_rate: u64,
    pub fund_fee_rate: u64,
    pub creator_fee_rate: u64,
}

impl CpmmFeeConfig {
    /// Decode an `AmmConfig` account. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        Some(Self {
            trade_fee_rate: u64_at(data, 12)?,
            protocol_fee_rate: u64_at(data, 20)?,
            fund_fee_rate: u64_at(data, 28)?,
            creator_fee_rate: u64_at(data, 108)?,
        })
    }
}

/// A swappable pool state with distinct accounts, for tests of code that reads one.
#[cfg(test)]
pub(crate) fn sample_state() -> CpmmPoolState {
    CpmmPoolState {
        pool: Pubkey::new_unique(),
        amm_config: Pubkey::new_unique(),
        vault_0: Pubkey::new_unique(),
        vault_1: Pubkey::new_unique(),
        mint_0: Pubkey::new_unique(),
        mint_1: Pubkey::new_unique(),
        program_0: crate::chains::solana::spl_token::id(),
        program_1: crate::chains::solana::spl_token::id(),
        observation: Pubkey::new_unique(),
        decimals_0: 9,
        decimals_1: 6,
        status: 0,
        open_time: 0,
        protocol_fees_0: 0,
        protocol_fees_1: 0,
        fund_fees_0: 0,
        fund_fees_1: 0,
        creator_fees_0: 0,
        creator_fees_1: 0,
        creator_fee_enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_swap_status_bit_is_a_disable_flag_not_an_enable_flag() {
        let mut s = sample_state();
        assert!(s.swap_enabled(), "a zero status permits swapping");
        s.status = 0b100;
        assert!(!s.swap_enabled());
        s.status = 0b011;
        assert!(
            s.swap_enabled(),
            "deposit and withdraw bits must not block a swap"
        );
    }

    #[test]
    fn a_pool_before_its_open_time_is_not_tradable_yet() {
        let mut s = sample_state();
        s.open_time = 1_000;
        assert!(!s.is_open(999));
        assert!(s.is_open(1_000));
    }

    #[test]
    fn a_truncated_pool_account_decodes_to_none_instead_of_panicking() {
        assert!(CpmmPoolState::decode(Pubkey::new_unique(), &[0u8; 100]).is_none());
        assert!(CpmmFeeConfig::decode(&[0u8; 20]).is_none());
    }
}
