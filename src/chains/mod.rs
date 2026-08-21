//! Chain-neutral identities and metadata.
//!
//! This module is the sole owner of chain identity primitives. Chain-specific
//! clients and capabilities intentionally do not live here yet.

mod error;
mod registry;
pub mod solana;
mod types;

pub use error::ChainError;
pub use registry::ChainRegistry;
pub use types::{
    AccountId, AssetId, ChainId, ChainMetadata, NativeAsset, PoolId, TransactionId,
    SOLANA_NATIVE_ASSET,
};
