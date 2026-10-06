// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Read-only registry of metadata for chains supported by this build.
//!
//! The registry is the owner of which chains this build supports. Active-chain
//! selection is derived from that set: with one registered chain, that chain
//! is active.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::chains::runtime::ChainRuntime;
use crate::chains::{adapter_for, solana, ChainId, ChainMetadata, Error, Result};

/// A fixed registry of supported blockchain metadata.
#[derive(Debug, Clone)]
pub struct ChainRegistry {
    chains: [ChainMetadata; 1],
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainRegistry {
    /// Creates the registry with every chain supported by this build.
    pub fn new() -> Self {
        Self {
            chains: [solana::chain_metadata()],
        }
    }

    /// Returns metadata for a supported chain.
    pub fn get(&self, id: ChainId) -> Option<&ChainMetadata> {
        self.chains.iter().find(|metadata| metadata.id == id)
    }

    /// Metadata for the active chain.
    pub fn active(&self) -> &ChainMetadata {
        self.get(Self::active_chain())
            .expect("the active chain is registered")
    }

    /// Iterates over supported chain metadata.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ChainMetadata> {
        self.chains.iter()
    }

    /// The chain this process operates on — the single enabled chain (see
    /// [`crate::chains::active_chain`]).
    pub fn active_chain() -> ChainId {
        crate::chains::active_chain()
    }
}

/// The chains enabled in `[chains]` config, frozen at first read. Enable and
/// disable are restart-gated settings: the frozen set changes only with a
/// process restart. A config that was never loaded (unit tests) reads as its
/// default, which enables every supported chain.
pub fn enabled_chains() -> &'static [ChainId] {
    ENABLED_CHAINS
        .get_or_init(|| {
            let chains =
                crate::config::try_with_config(|cfg| cfg.chains.clone()).unwrap_or_default();
            let mut enabled = Vec::new();
            if chains.is_enabled(ChainId::Solana) {
                enabled.push(ChainId::Solana);
            }
            enabled
        })
        .as_slice()
}

/// The single enabled chain whose adapter accepts `address`.
///
/// No accepting chain is [`Error::UnrecognizedAddress`]; more than one is
/// [`Error::AmbiguousChain`] — the caller's input does not say which chain it
/// means, so none is chosen.
pub fn chain_for_address(address: &str) -> Result<ChainId> {
    let candidates: Vec<ChainId> = enabled_chains()
        .iter()
        .copied()
        .filter(|&chain| adapter_for(chain).validate_address(address).is_ok())
        .collect();
    match candidates.as_slice() {
        [] => Err(Error::UnrecognizedAddress {
            value: address.to_owned(),
        }),
        [only] => Ok(*only),
        _ => Err(Error::AmbiguousChain { candidates }),
    }
}

/// Install the runtime of every enabled chain, once per boot, from the
/// composition root (`crate::run::services`). A disabled chain gets no
/// runtime, no process seams and no services.
pub fn install_enabled_runtimes() {
    let mut runtimes: HashMap<ChainId, Arc<dyn ChainRuntime>> = HashMap::new();
    for &chain in enabled_chains() {
        match chain {
            ChainId::Solana => {
                runtimes.insert(chain, solana::runtime::runtime());
                solana::runtime::install_process_seams();
            }
        }
    }
    let _ = RUNTIMES.set(runtimes);
}

/// The installed runtime for `chain`; `None` for a chain that is not enabled
/// or before the composition root has run.
pub fn runtime_for(chain: ChainId) -> Option<Arc<dyn ChainRuntime>> {
    RUNTIMES.get()?.get(&chain).cloned()
}

static ENABLED_CHAINS: OnceLock<Vec<ChainId>> = OnceLock::new();
static RUNTIMES: OnceLock<HashMap<ChainId, Arc<dyn ChainRuntime>>> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::{chain_for_address, enabled_chains, install_enabled_runtimes, runtime_for};
    use crate::chains::Error;
    use crate::chains::{solana, solana::constants::SOL_MINT, ChainId, ChainRegistry};

    #[test]
    fn registry_exposes_solana_metadata() {
        let registry = ChainRegistry::new();
        let solana_meta = registry.get(ChainId::Solana).unwrap();

        assert_eq!(registry.iter().len(), 1);
        assert_eq!(ChainRegistry::active_chain(), ChainId::Solana);
        assert_eq!(crate::chains::active_chain(), ChainId::Solana);
        assert_eq!(registry.active().id, ChainId::Solana);
        assert_eq!(solana_meta.native_asset, solana::NATIVE_ASSET);
        assert_eq!(solana_meta.native_asset.address, SOL_MINT);
    }

    #[test]
    fn enabled_chains_default_to_solana_without_config() {
        // Test binaries never load config; the default ChainsConfig enables
        // every supported chain.
        assert_eq!(enabled_chains(), [ChainId::Solana]);
    }

    #[test]
    fn an_address_resolves_to_the_single_chain_that_accepts_it() {
        assert_eq!(chain_for_address(SOL_MINT).unwrap(), ChainId::Solana);
        assert!(matches!(
            chain_for_address("not an address"),
            Err(Error::UnrecognizedAddress { .. })
        ));
    }

    #[test]
    fn every_enabled_chain_has_an_installed_runtime() {
        install_enabled_runtimes();
        for &chain in enabled_chains() {
            let runtime = runtime_for(chain)
                .unwrap_or_else(|| panic!("no runtime for the enabled chain {chain:?}"));
            assert_eq!(runtime.id(), chain);
            assert!(!runtime.swap_routers().is_empty());
        }
    }
}
