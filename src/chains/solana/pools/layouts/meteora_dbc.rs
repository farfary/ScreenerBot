// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Meteora Dynamic Bonding Curve (`dbcij3LW…`) `VirtualPool` account.
//!
//! # Layout, verified against mainnet
//!
//! Read from the programme's on-chain Anchor IDL and cross-checked against live
//! pools; the IDL's `VirtualPool` discriminator (`d5e005d16245775c`) matches a
//! live pool account's first 8 bytes. 424 bytes:
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

use crate::chains::solana::layout::{pubkey_at, u128_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// `VirtualPool`'s own Anchor discriminator, confirmed against the on-chain
/// IDL and a live pool account's first 8 bytes.
pub(crate) const VIRTUAL_POOL_DISCRIMINATOR: [u8; 8] = [213, 224, 5, 209, 98, 69, 119, 92];

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
