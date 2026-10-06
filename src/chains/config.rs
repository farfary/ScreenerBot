// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain-indexed reads of `[chains]`. Every arm is exhaustive over `ChainId`:
//! a new chain does not compile until its settings are wired here, and no
//! method chooses a chain on the caller's behalf.

use crate::chains::ChainId;
use crate::config::ChainsConfig;

/// The chain whose settings the config layout before `[chains]` kept in global
/// sections (`[rpc]`, `[swaps]`, the Jupiter/Raptor endpoint monitors). A fact
/// about files written by earlier releases, read only by the relocation of that
/// layout; never a default for anything else.
pub(crate) const PRE_CHAINS_LAYOUT_CHAIN: ChainId = ChainId::Solana;

impl ChainsConfig {
    /// Whether `chain` runs in this process.
    pub fn is_enabled(&self, chain: ChainId) -> bool {
        match chain {
            ChainId::Solana => self.solana.enabled,
        }
    }
}
