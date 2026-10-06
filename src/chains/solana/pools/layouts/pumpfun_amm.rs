// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pump.fun AMM (`pAMMBay6…`) `Pool`, `GlobalConfig` and fee programme
//! `FeeConfig` accounts.
//!
//! # Layout, verified against mainnet
//!
//! `Pool` is 301 bytes on chain, of which the first 245 are meaningful:
//!
//! ```text
//!   8 pool_bump   9 index   11 creator   43 base_mint   75 quote_mint
//! 107 lp_mint    139 pool_base_token_account   171 pool_quote_token_account
//! 203 lp_supply  211 coin_creator
//! ```

use crate::chains::solana::layout::{pubkey_at, u128_at, u32_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// The three rates pump charges, all in basis points of the QUOTE mint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PumpFees {
    pub lp_bps: u64,
    pub protocol_bps: u64,
    pub creator_bps: u64,
}

/// The parts of pump's `GlobalConfig` this venue needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalConfig {
    pub lp_fee_bps: u64,
    pub protocol_fee_bps: u64,
    pub creator_fee_bps: u64,
    pub disable_flags: u8,
    pub fee_recipients: [Pubkey; 8],
    pub buyback_fee_recipients: [Pubkey; 8],
}

impl GlobalConfig {
    /// Decode a `GlobalConfig` account. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut fee_recipients = [Pubkey::default(); 8];
        for (index, slot) in fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 57 + index * 32)?;
        }
        let mut buyback_fee_recipients = [Pubkey::default(); 8];
        for (index, slot) in buyback_fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 643 + index * 32)?;
        }
        Some(Self {
            buyback_fee_recipients,
            lp_fee_bps: u64_at(data, 40)?,
            protocol_fee_bps: u64_at(data, 48)?,
            creator_fee_bps: u64_at(data, 313)?,
            disable_flags: u8_at(data, 56)?,
            fee_recipients,
        })
    }

    /// Whether any disable flag is set. The programme uses this byte as a
    /// bitmask of things that are switched OFF, so anything non-zero is a reason
    /// to refuse rather than to guess which bit meant what.
    pub fn swaps_disabled(&self) -> bool {
        self.disable_flags != 0
    }

    /// The first initialised protocol fee recipient. Live swaps rotate between
    /// the slots; any initialised one is accepted by the programme.
    pub fn first_fee_recipient(&self) -> Option<Pubkey> {
        self.fee_recipients
            .iter()
            .find(|key| **key != Pubkey::default())
            .copied()
    }

    /// The first initialised buyback fee recipient. Live swaps pick freely
    /// among the eight slots.
    pub fn first_buyback_recipient(&self) -> Option<Pubkey> {
        self.buyback_fee_recipients
            .iter()
            .find(|key| **key != Pubkey::default())
            .copied()
    }

    /// The flat rates, used when the tier table cannot be read.
    pub fn flat_fees(&self) -> PumpFees {
        PumpFees {
            lp_bps: self.lp_fee_bps,
            protocol_bps: self.protocol_fee_bps,
            creator_bps: self.creator_fee_bps,
        }
    }
}

/// Pump's market-cap-keyed fee table, out of the fee programme's `FeeConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeTierTable {
    pub(crate) tiers: Vec<(u128, PumpFees)>,
}

impl FeeTierTable {
    /// Decode the `fee_tiers` vector of a `FeeConfig` account.
    ///
    /// Layout: discriminator, `bump`, `admin`, `flat_fees` (three `u64`), then a
    /// borsh `Vec<FeeTier>` where each tier is a `u128` lamport threshold
    /// followed by the same three `u64` rates.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut offset = 8 + 1 + 32 + 24;
        let count = u32_at(data, offset)? as usize;
        offset += 4;
        // A table this long is a decode failure, not a real config.
        if count > 1_024 {
            return None;
        }
        let mut tiers = Vec::with_capacity(count);
        for _ in 0..count {
            let threshold = u128_at(data, offset)?;
            let fees = PumpFees {
                lp_bps: u64_at(data, offset + 16)?,
                protocol_bps: u64_at(data, offset + 24)?,
                creator_bps: u64_at(data, offset + 32)?,
            };
            tiers.push((threshold, fees));
            offset += 40;
        }
        if tiers.is_empty() {
            return None;
        }
        Some(Self { tiers })
    }

    /// The most expensive tier, used when the market cap cannot be computed.
    pub fn most_expensive(&self) -> PumpFees {
        self.tiers[0].1
    }
}

/// The parts of the pump-swap `Pool` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PumpAmmPoolState {
    pub pool: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_token_account: Pubkey,
    pub quote_token_account: Pubkey,
    pub coin_creator: Pubkey,
    pub is_cashback_coin: bool,
    /// Virtual quote reserves of a boosted pool. Pricing only — the payout is
    /// still capped at what the real vault holds.
    pub virtual_quote_reserves: u64,
}

impl PumpAmmPoolState {
    /// Decode a pump-swap pool account. Pure.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        Some(Self {
            pool,
            base_mint: pubkey_at(data, 43)?,
            quote_mint: pubkey_at(data, 75)?,
            base_token_account: pubkey_at(data, 139)?,
            quote_token_account: pubkey_at(data, 171)?,
            coin_creator: pubkey_at(data, 211)?,
            is_cashback_coin: u8_at(data, 244)? != 0,
            virtual_quote_reserves: u64_at(data, 245)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cashback_flag_and_virtual_reserve_are_read_from_the_bytes_past_the_idl() {
        let mut data = vec![0u8; 301];
        data[244] = 1;
        data[245..253].copy_from_slice(&17_584_505_358u64.to_le_bytes());
        let decoded = PumpAmmPoolState::decode(Pubkey::new_unique(), &data).expect("decodes");
        assert!(decoded.is_cashback_coin);
        assert_eq!(decoded.virtual_quote_reserves, 17_584_505_358);
    }

    #[test]
    fn a_table_claiming_an_absurd_number_of_tiers_is_a_decode_failure() {
        let mut data = vec![0u8; 4_073];
        data[65..69].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(FeeTierTable::decode(&data).is_none());
    }

    #[test]
    fn a_truncated_fee_config_is_a_decode_failure_rather_than_a_panic() {
        assert!(FeeTierTable::decode(&[0u8; 20]).is_none());
    }
}
