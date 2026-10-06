// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! A container holding one lazily built value per chain.
//!
//! Process-wide state that belongs to a chain (a cache, a database handle, a
//! store) is declared as a `PerChain<T>` static instead of one shared value
//! keyed by address. Each chain's slot is built on first use by a named build
//! function with that chain's own capacity, so one chain's traffic never
//! evicts, replaces or clears another chain's entries.

use std::sync::OnceLock;

use crate::chains::ChainId;

/// One value per chain, built on first use with that chain's own capacity. A
/// chain that is never asked for allocates nothing.
pub struct PerChain<T> {
    slots: [OnceLock<T>; ChainId::COUNT],
    build: fn(ChainId) -> T,
}

impl<T> PerChain<T> {
    /// A container whose slots are built by `build` on first access.
    pub const fn new(build: fn(ChainId) -> T) -> Self {
        Self {
            slots: [const { OnceLock::new() }; ChainId::COUNT],
            build,
        }
    }

    /// The value for `chain`, building it on first access.
    pub fn get(&self, chain: ChainId) -> &T {
        self.slots[chain.index()].get_or_init(|| (self.build)(chain))
    }

    /// The slots built so far, in chain index order. A chain never asked for
    /// is skipped and stays unallocated.
    pub fn built(&self) -> impl Iterator<Item = (ChainId, &T)> + '_ {
        ChainId::ALL
            .iter()
            .filter_map(|&chain| self.slots[chain.index()].get().map(|value| (chain, value)))
    }
}

#[cfg(test)]
mod tests {
    use super::PerChain;
    use crate::chains::ChainId;

    fn chain_name(chain: ChainId) -> String {
        chain.as_str().to_owned()
    }

    #[test]
    fn a_slot_is_built_on_first_use_only() {
        let values: PerChain<String> = PerChain::new(chain_name);
        assert_eq!(values.built().count(), 0);
        assert_eq!(values.get(ChainId::Solana), "solana");
        let built: Vec<_> = values.built().collect();
        assert_eq!(built.len(), 1);
        assert_eq!(built[0].0, ChainId::Solana);
    }

    #[test]
    fn a_slot_is_built_once() {
        let values: PerChain<String> = PerChain::new(chain_name);
        let first: *const String = values.get(ChainId::Solana);
        let second: *const String = values.get(ChainId::Solana);
        assert_eq!(first, second);
    }
}
