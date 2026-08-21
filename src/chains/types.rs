//! Chain identifiers, chain-qualified object identifiers, and metadata.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    chains::solana::constants::{SOL_DECIMALS, SOL_MINT},
    chains::ChainError,
};

/// A blockchain supported by this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainId {
    /// The Solana blockchain.
    Solana,
}

impl ChainId {
    /// Returns the stable lowercase storage and wire representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Solana => "solana",
        }
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChainId {
    type Err = ChainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "solana" => Ok(Self::Solana),
            _ => Err(ChainError::UnsupportedChain {
                value: value.to_owned(),
            }),
        }
    }
}

/// A chain-qualified asset address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "AddressIdWire")]
pub struct AssetId {
    chain: ChainId,
    address: String,
}

/// A chain-qualified account address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "AddressIdWire")]
pub struct AccountId {
    chain: ChainId,
    address: String,
}

/// A chain-qualified transaction hash or signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "TransactionIdWire")]
pub struct TransactionId {
    chain: ChainId,
    hash: String,
}

/// A chain-qualified pool address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "AddressIdWire")]
pub struct PoolId {
    chain: ChainId,
    address: String,
}

#[derive(Deserialize)]
struct AddressIdWire {
    chain: ChainId,
    address: String,
}

#[derive(Deserialize)]
struct TransactionIdWire {
    chain: ChainId,
    hash: String,
}

macro_rules! address_id {
    ($type:ident, $kind:literal) => {
        impl $type {
            /// Creates an identity after normalizing surrounding whitespace.
            pub fn new(chain: ChainId, address: impl Into<String>) -> Result<Self, ChainError> {
                let address = address.into().trim().to_owned();
                if address.is_empty() {
                    return Err(ChainError::EmptyIdentifier { kind: $kind });
                }
                Ok(Self { chain, address })
            }

            /// Returns the owning chain.
            pub const fn chain(&self) -> ChainId {
                self.chain
            }

            /// Returns the canonical generic-layer address string.
            pub fn address(&self) -> &str {
                &self.address
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", self.chain, self.address)
            }
        }
    };
}

address_id!(AssetId, "asset address");
address_id!(AccountId, "account address");
address_id!(PoolId, "pool address");

macro_rules! address_id_deserialization {
    ($type:ident) => {
        impl TryFrom<AddressIdWire> for $type {
            type Error = ChainError;

            fn try_from(value: AddressIdWire) -> Result<Self, Self::Error> {
                Self::new(value.chain, value.address)
            }
        }
    };
}

address_id_deserialization!(AssetId);
address_id_deserialization!(AccountId);
address_id_deserialization!(PoolId);

impl TransactionId {
    /// Creates an identity after normalizing surrounding whitespace.
    pub fn new(chain: ChainId, hash: impl Into<String>) -> Result<Self, ChainError> {
        let hash = hash.into().trim().to_owned();
        if hash.is_empty() {
            return Err(ChainError::EmptyIdentifier {
                kind: "transaction hash",
            });
        }
        Ok(Self { chain, hash })
    }

    /// Returns the owning chain.
    pub const fn chain(&self) -> ChainId {
        self.chain
    }

    /// Returns the canonical generic-layer transaction hash or signature.
    pub fn hash(&self) -> &str {
        &self.hash
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.chain, self.hash)
    }
}

impl TryFrom<TransactionIdWire> for TransactionId {
    type Error = ChainError;

    fn try_from(value: TransactionIdWire) -> Result<Self, Self::Error> {
        Self::new(value.chain, value.hash)
    }
}

/// Metadata for a chain's native asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct NativeAsset {
    /// Conventional asset symbol.
    pub symbol: &'static str,
    /// Number of fractional base-unit digits.
    pub decimals: u8,
    /// The chain's canonical native asset mint or address.
    pub address: &'static str,
}

/// The native asset descriptor for Solana.
pub const SOLANA_NATIVE_ASSET: NativeAsset = NativeAsset {
    symbol: "SOL",
    decimals: SOL_DECIMALS,
    address: SOL_MINT,
};

/// Read-only metadata for a supported blockchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ChainMetadata {
    /// Stable chain identifier.
    pub id: ChainId,
    /// Metadata for the native asset of this chain.
    pub native_asset: NativeAsset,
}

impl ChainMetadata {
    /// The metadata for Solana.
    pub const fn solana() -> Self {
        Self {
            id: ChainId::Solana,
            native_asset: SOLANA_NATIVE_ASSET,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_id_serialization_is_stably_lowercase() {
        assert_eq!(
            serde_json::to_string(&ChainId::Solana).unwrap(),
            "\"solana\""
        );
        assert_eq!("solana".parse::<ChainId>().unwrap(), ChainId::Solana);
        assert!(matches!(
            "SOLANA".parse::<ChainId>(),
            Err(ChainError::UnsupportedChain { .. })
        ));
    }

    #[test]
    fn identities_are_explicitly_chain_qualified() {
        let asset = AssetId::new(ChainId::Solana, " mint ").unwrap();
        assert_eq!(asset.chain(), ChainId::Solana);
        assert_eq!(asset.address(), "mint");
        assert_eq!(asset.to_string(), "solana:mint");
        assert_eq!(
            serde_json::to_value(asset).unwrap(),
            serde_json::json!({
                "chain": "solana", "address": "mint"
            })
        );
    }

    #[test]
    fn identities_reject_empty_values() {
        assert!(matches!(
            AssetId::new(ChainId::Solana, " \t"),
            Err(ChainError::EmptyIdentifier { .. })
        ));
        assert!(matches!(
            AccountId::new(ChainId::Solana, ""),
            Err(ChainError::EmptyIdentifier { .. })
        ));
        assert!(matches!(
            PoolId::new(ChainId::Solana, "\n"),
            Err(ChainError::EmptyIdentifier { .. })
        ));
        assert!(matches!(
            TransactionId::new(ChainId::Solana, " "),
            Err(ChainError::EmptyIdentifier { .. })
        ));
        assert!(serde_json::from_str::<AssetId>(r#"{"chain":"solana","address":" "}"#).is_err());
    }
}
