// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Raydium AMM v4 (`675kPX9M…`) `AmmInfo` account.
//!
//! # Layout, verified against mainnet
//!
//! `AmmInfo` is 752 bytes:
//!
//! ```text
//!   0 status u64        32 coin_decimals u64   40 pc_decimals u64
//! 144 swap_fee?         (see the fee block below)
//! 192 need_take_pnl_coin 200 need_take_pnl_pc
//! 336 coin_vault        368 pc_vault
//! 400 coin_mint         432 pc_mint            464 lp_mint
//! 496 open_orders       528 market             560 market_program
//! 592 target_orders
//! ```
//!
//! The fee block at 128 is eight `u64`s: `min_separate_numerator/denominator`,
//! `trade_fee_numerator/denominator`, `pnl_numerator/denominator`,
//! `swap_fee_numerator/denominator`. The swap charges the LAST pair (offsets
//! 176/184), which is 25/10000 on a standard pool.
//!
//! # Reserves
//!
//! Tradable reserves are `vault − need_take_pnl` per side. `need_take_pnl` is
//! profit already earmarked for the pool's owner and sitting in the vault; it is
//! not swappable, and reading the raw vault over-states both reserves.

use crate::chains::solana::layout::{pubkey_at, u64_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Status values that permit swapping. 1 = Initialized, 6 = SwapOnly,
/// 7 = WaitingTrade.
const SWAPPABLE_STATUSES: [u64; 3] = [1, 6, 7];

/// The parts of `AmmInfo` that swapping and pricing read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmmV4PoolState {
    pub pool: Pubkey,
    pub status: u64,
    pub coin_decimals: u8,
    pub pc_decimals: u8,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub need_take_pnl_coin: u64,
    pub need_take_pnl_pc: u64,
    pub coin_vault: Pubkey,
    pub pc_vault: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market: Pubkey,
    pub market_program: Pubkey,
    pub target_orders: Pubkey,
}

impl AmmV4PoolState {
    /// Decode an `AmmInfo` account. Pure: no RPC, no cache, no clock.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            status: u64_at(data, 0)?,
            coin_decimals: u64_at(data, 32)?.min(u8::MAX as u64) as u8,
            pc_decimals: u64_at(data, 40)?.min(u8::MAX as u64) as u8,
            swap_fee_numerator: u64_at(data, 176)?,
            swap_fee_denominator: u64_at(data, 184)?,
            need_take_pnl_coin: u64_at(data, 192)?,
            need_take_pnl_pc: u64_at(data, 200)?,
            coin_vault: pubkey_at(data, 336)?,
            pc_vault: pubkey_at(data, 368)?,
            coin_mint: pubkey_at(data, 400)?,
            pc_mint: pubkey_at(data, 432)?,
            open_orders: pubkey_at(data, 496)?,
            market: pubkey_at(data, 528)?,
            market_program: pubkey_at(data, 560)?,
            target_orders: pubkey_at(data, 592)?,
        })
    }

    /// Whether the pool's status permits swapping.
    pub fn swap_enabled(&self) -> bool {
        SWAPPABLE_STATUSES.contains(&self.status)
    }

    /// Tradable `(coin, pc)` reserves from the two vault balances: each vault less
    /// the profit earmarked out of it.
    pub fn tradable_reserves(&self, coin_balance: u64, pc_balance: u64) -> (u64, u64) {
        (
            coin_balance.saturating_sub(self.need_take_pnl_coin),
            pc_balance.saturating_sub(self.need_take_pnl_pc),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(status: u64) -> AmmV4PoolState {
        AmmV4PoolState {
            pool: Pubkey::new_unique(),
            status,
            coin_decimals: 9,
            pc_decimals: 6,
            swap_fee_numerator: 25,
            swap_fee_denominator: 10_000,
            need_take_pnl_coin: 500,
            need_take_pnl_pc: 700,
            coin_vault: Pubkey::new_unique(),
            pc_vault: Pubkey::new_unique(),
            coin_mint: Pubkey::new_unique(),
            pc_mint: Pubkey::new_unique(),
            open_orders: Pubkey::new_unique(),
            market: Pubkey::new_unique(),
            market_program: Pubkey::new_unique(),
            target_orders: Pubkey::new_unique(),
        }
    }

    #[test]
    fn only_the_swappable_statuses_are_tradable() {
        for status in [1u64, 6, 7] {
            assert!(
                state(status).swap_enabled(),
                "status {status} should permit swapping"
            );
        }
        for status in [0u64, 2, 3, 4, 5] {
            assert!(
                !state(status).swap_enabled(),
                "status {status} must not permit swapping"
            );
        }
    }

    #[test]
    fn tradable_reserves_exclude_the_earmarked_profit_and_never_underflow() {
        let s = state(6);
        assert_eq!(s.tradable_reserves(1_000, 2_000), (500, 1_300));
        assert_eq!(s.tradable_reserves(100, 100), (0, 0));
    }

    #[test]
    fn the_vaults_and_mints_are_read_at_their_own_offsets() {
        let mut data = vec![0u8; 752];
        let coin_vault = Pubkey::new_unique();
        let pc_vault = Pubkey::new_unique();
        let coin_mint = Pubkey::new_unique();
        let pc_mint = Pubkey::new_unique();
        data[336..368].copy_from_slice(coin_vault.as_ref());
        data[368..400].copy_from_slice(pc_vault.as_ref());
        data[400..432].copy_from_slice(coin_mint.as_ref());
        data[432..464].copy_from_slice(pc_mint.as_ref());

        let decoded = AmmV4PoolState::decode(Pubkey::new_unique(), &data).expect("decodes");
        assert_eq!(decoded.coin_vault, coin_vault);
        assert_eq!(decoded.pc_vault, pc_vault);
        assert_eq!(decoded.coin_mint, coin_mint);
        assert_eq!(decoded.pc_mint, pc_mint);
    }

    #[test]
    fn a_truncated_pool_account_decodes_to_none_instead_of_panicking() {
        assert!(AmmV4PoolState::decode(Pubkey::new_unique(), &[0u8; 300]).is_none());
    }
}
