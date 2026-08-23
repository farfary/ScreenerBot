//! Chain-neutral identities, metadata, and the active-chain seam.
//!
//! `ChainId` and the identity/metadata types live here. Solana-specific
//! metadata construction lives under `solana`. Operational shared code must
//! call [`active_chain`] rather than naming a `ChainId` variant at the call
//! site.

mod adapter;
mod error;
mod registry;
pub mod solana;
mod types;

pub use adapter::{adapter, ChainAdapter};
pub use error::ChainError;
pub use registry::ChainRegistry;
pub use types::{AccountId, AssetId, ChainId, ChainMetadata, NativeAsset, PoolId, TransactionId};

/// The chain this process operates on.
///
/// This is the single operational selection seam. With one supported chain
/// it returns Solana; call sites outside `src/chains` and the composition
/// root must not hardcode that choice.
pub const fn active_chain() -> ChainId {
    ChainRegistry::active_chain()
}
