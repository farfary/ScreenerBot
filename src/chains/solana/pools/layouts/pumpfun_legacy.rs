// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pump.fun legacy (`6EF8rrec…`) `BondingCurve` and `Global` accounts.
//!
//! # Layout, verified against mainnet
//!
//! `BondingCurve` is 150 (migrated) or 256 (full) bytes, of which the first 115
//! are meaningful and identical in both sizes:
//!
//! ```text
//!  0..8   discriminator (17 b7 f8 37 60 d8 ac 60)
//!  8      virtual_token_reserves u64
//! 16      virtual_sol_reserves u64
//! 24      real_token_reserves u64
//! 32      real_sol_reserves u64
//! 40      token_total_supply u64
//! 48      complete bool
//! 49      creator pubkey
//! 81      is_mayhem_mode bool
//! 82      is_cashback_coin bool
//! 83      quote_mint pubkey   -- the DEFAULT pubkey for a native-SOL curve
//! ```
//!
//! Confirmed against the on-chain IDL (account `AYgC53tU5BbP2NAnv5nConJxAdpQZctvmZK88pu69xRs`)
//! and a real curve's bytes: `creator` at 49 derives the correct `creator_vault`
//! PDA for a live trade (`3yWiFWMJmcrRiaVJatHnz4o3D9qJ83xhkL6uSdcVBBor` from
//! creator `7ufmve7ZSFCzuNcKRunYrGtyb2Ka1MXzkWwf7jZhVsmL`, bump 254).

use crate::chains::solana::layout::{pubkey_at, u64_at, u8_at};
use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// `BondingCurve`'s own discriminator.
const BONDING_CURVE_DISCRIMINATOR: [u8; 8] = [0x17, 0xb7, 0xf8, 0x37, 0x60, 0xd8, 0xac, 0x60];

/// The parts of a pump.fun legacy `BondingCurve` a swap needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BondingCurve {
    pub pool: Pubkey,
    pub mint: Pubkey,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
    pub creator: Pubkey,
    pub is_mayhem_mode: bool,
    pub is_cashback_coin: bool,
    pub quote_mint: Pubkey,
}

impl BondingCurve {
    /// Decode a bonding curve account. Pure. `mint` is not stored in the
    /// account (the discovery layer already knows it, the same as the price
    /// decoder's contract), so the caller supplies it -- but a direct-swap
    /// venue is looked up by POOL ADDRESS, never by mint, so this decode takes
    /// it from the caller's own record via `PumpFunLegacyVenue::load`'s
    /// dispatcher instead: the pool account carries no mint field, so `mint`
    /// here is filled in from context by the loader, not decoded from bytes.
    pub fn decode(pool: Pubkey, data: &[u8]) -> Option<Self> {
        if data.len() < 115 || data[0..8] != BONDING_CURVE_DISCRIMINATOR {
            return None;
        }
        Some(Self {
            pool,
            // Filled in by the loader once the mint is known; zero here is a
            // decode placeholder never used before it is overwritten.
            mint: Pubkey::default(),
            virtual_token_reserves: u64_at(data, 8)?,
            virtual_sol_reserves: u64_at(data, 16)?,
            real_token_reserves: u64_at(data, 24)?,
            real_sol_reserves: u64_at(data, 32)?,
            token_total_supply: u64_at(data, 40)?,
            complete: u8_at(data, 48)? != 0,
            creator: pubkey_at(data, 49)?,
            is_mayhem_mode: u8_at(data, 81)? != 0,
            is_cashback_coin: u8_at(data, 82)? != 0,
            quote_mint: pubkey_at(data, 83)?,
        })
    }

    pub fn creator_set(&self) -> bool {
        self.creator != Pubkey::default()
    }
}

/// The fee recipient arrays out of pump.fun legacy's `Global` account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalFeeRecipients {
    pub fee_recipients: [Pubkey; 7],
    pub buyback_fee_recipients: [Pubkey; 8],
}

impl GlobalFeeRecipients {
    /// Decode the two recipient arrays out of `Global`. Pure.
    pub fn decode(data: &[u8]) -> Option<Self> {
        let mut fee_recipients = [Pubkey::default(); 7];
        for (index, slot) in fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 162 + index * 32)?;
        }
        let mut buyback_fee_recipients = [Pubkey::default(); 8];
        for (index, slot) in buyback_fee_recipients.iter_mut().enumerate() {
            *slot = pubkey_at(data, 741 + index * 32)?;
        }
        Some(Self {
            fee_recipients,
            buyback_fee_recipients,
        })
    }

    /// The first initialised protocol fee recipient. Live swaps rotate between
    /// the seven slots; any initialised one is accepted by the programme.
    pub fn first_fee_recipient(&self) -> Option<Pubkey> {
        self.fee_recipients
            .iter()
            .find(|key| **key != Pubkey::default())
            .copied()
    }

    /// Two DISTINCT initialised buyback fee recipients, for the two trailing
    /// accounts every trade instruction demands beyond its own IDL. Every live
    /// trade replayed while building this venue used a different pair from
    /// these eight slots; only the second position ever received lamports, but
    /// the first must still name a valid, distinct recipient.
    pub fn two_buyback_recipients(&self) -> Option<(Pubkey, Pubkey)> {
        let mut distinct = self
            .buyback_fee_recipients
            .iter()
            .filter(|key| **key != Pubkey::default());
        let first = *distinct.next()?;
        let second = *distinct.next()?;
        Some((first, second))
    }
}
