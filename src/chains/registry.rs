//! Read-only registry of metadata for chains supported by this build.
//!
//! The registry is the owner of which chains this build supports. Active-chain
//! selection is derived from that set: with one registered chain, that chain
//! is active.

use crate::chains::{solana, ChainId, ChainMetadata};

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
    /// The chain this build operates on.
    ///
    /// Shared operational code must call [`crate::chains::active_chain`] (or
    /// this method) instead of naming a `ChainId` variant at the call site.
    pub const fn active_chain() -> ChainId {
        ChainId::Solana
    }

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
}

#[cfg(test)]
mod tests {
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
}
