// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain-neutral identities, metadata, and the active-chain seam.
//!
//! `ChainId` and the identity/metadata types live here. Solana-specific
//! metadata construction lives under `solana`. Operational shared code must
//! call [`active_chain`] rather than naming a `ChainId` variant at the call
//! site.

mod adapter;
mod error;
mod execution;
mod registry;
mod runtime;
pub mod solana;
mod types;

pub use adapter::{adapter, adapter_for, ChainAdapter};
pub use error::{Error, Result};
pub use execution::ExecutionFailure;
pub use registry::{enabled_chains, install_enabled_runtimes, runtime_for, ChainRegistry};
pub use runtime::ChainRuntime;
pub use types::{AccountId, AssetId, ChainId, ChainMetadata, NativeAsset, PoolId, TransactionId};

/// The chain this process operates on: the single enabled chain, frozen from
/// `[chains]` config at first read. With one supported chain and
/// default config that is Solana. This is the single operational selection
/// seam, deleted when every caller passes its subject's chain.
pub fn active_chain() -> ChainId {
    registry::enabled_chains()
        .first()
        .copied()
        .expect("config load refuses a config with no enabled chain")
}
