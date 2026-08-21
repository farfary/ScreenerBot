//! Read-only registry of metadata for chains supported by this build.

use crate::chains::{ChainId, ChainMetadata};

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
            chains: [ChainMetadata::solana()],
        }
    }

    /// Returns metadata for a supported chain.
    pub fn get(&self, id: ChainId) -> Option<&ChainMetadata> {
        self.chains.iter().find(|metadata| metadata.id == id)
    }

    /// Iterates over supported chain metadata.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &ChainMetadata> {
        self.chains.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::chains::{solana::constants::SOL_MINT, ChainId, ChainRegistry, SOLANA_NATIVE_ASSET};

    #[test]
    fn registry_exposes_solana_metadata() {
        let registry = ChainRegistry::new();
        let solana = registry.get(ChainId::Solana).unwrap();

        assert_eq!(registry.iter().len(), 1);
        assert_eq!(solana.native_asset, SOLANA_NATIVE_ASSET);
        assert_eq!(solana.native_asset.address, SOL_MINT);
    }
}
