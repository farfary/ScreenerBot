// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain-neutral identities, metadata, and the active-chain seam.
//!
//! `ChainId` and the identity/metadata types live here. Solana-specific
//! metadata construction lives under `solana`. Operational shared code must
//! call [`active_chain`] rather than naming a `ChainId` variant at the call
//! site.

mod adapter;
mod amount;
mod config;
mod error;
mod execution;
mod per_chain;
mod registry;
mod runtime;
mod scope;
pub mod solana;
mod types;

pub use adapter::{adapter, adapter_for, ChainAdapter};
pub use amount::{AmountParseError, RawAmount};
pub use error::{Error, Result};
pub use execution::ExecutionFailure;
pub use per_chain::PerChain;
pub use registry::{
    chain_for_address, enabled_chains, install_enabled_runtimes, runtime_for, ChainRegistry,
};
pub use runtime::{ChainRuntime, TokenAccountFacts};
pub use scope::ChainScope;
pub use types::{AccountId, AssetId, ChainId, ChainMetadata, NativeAsset, PoolId, TransactionId};

pub(crate) use config::PRE_CHAINS_LAYOUT_CHAIN;

use crate::paths::{chain_db_path, DbKind};
use std::path::PathBuf;

/// Legacy tokens-database path (`tokens.db`); it lives on the chain seam
/// because only the seam may name the chain, and it is deleted when its last
/// caller passes the chain explicitly.
pub fn get_tokens_db_path() -> PathBuf {
    chain_db_path(DbKind::Tokens, ChainId::Solana)
}

/// Legacy pools-database path (`pools.db`); it lives on the chain seam
/// because only the seam may name the chain, and it is deleted when its last
/// caller passes the chain explicitly.
pub fn get_pools_db_path() -> PathBuf {
    chain_db_path(DbKind::Pools, ChainId::Solana)
}

/// Legacy OHLCV-database path (`ohlcvs.db`); it lives on the chain seam
/// because only the seam may name the chain, and it is deleted when its last
/// caller passes the chain explicitly.
pub fn get_ohlcvs_db_path() -> PathBuf {
    chain_db_path(DbKind::Ohlcvs, ChainId::Solana)
}

/// Legacy RPC-stats-database path (`rpc_stats.db`); it lives on the chain
/// seam because only the seam may name the chain, and it is deleted when its
/// last caller passes the chain explicitly.
pub fn get_rpc_stats_db_path() -> PathBuf {
    chain_db_path(DbKind::RpcStats, ChainId::Solana)
}

/// Legacy chain stamped into new rows of the shared-file stores whose domain
/// has not threaded its subject's chain through yet (events, actions, RPC
/// stats, strategy performance, the tools session tables, AI decision
/// history): the single enabled chain. It lives on the chain seam because
/// only the seam may name the chain, and it is deleted when its last caller
/// passes the chain explicitly.
pub fn legacy_row_chain() -> ChainId {
    active_chain()
}

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
