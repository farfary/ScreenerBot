// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool system for real-time price calculations
//!
//! A centralized pool service that watches tokens and provides real-time
//! prices derived from DEX pools. Every price, list and history is kept per
//! chain, and every public function takes the chain (or, for an aggregate,
//! the `ChainScope`) from its caller:
//! - get_pool_price(chain, mint) -> current fresh price for a token
//! - get_available_tokens(chain) -> tokens of that chain with fresh prices
//! - get_cache_stats(scope) -> cache counts summed over the scope's chains
//!
//! Chain-neutral: persistence (`database`), caching (`cache`), service
//! lifecycle (`service`) and periodic upkeep (`maintenance`) live here, and
//! the `PoolDescriptor` domain model (`types`) is a chain-neutral value object — no `Pubkey`, no Solana vendor
//! type, anywhere in this module. Solana-specific pool discovery, RPC account
//! fetching, protocol recognition, DEX byte decoding and price calculation
//! (which dispatches on the concrete `ProgramKind` and reads `Pubkey`-keyed
//! RPC account bundles) live under `crate::chains::solana::pools` — this
//! module consumes its output (`PoolDescriptor` instances, built through
//! explicit conversions at that boundary) but owns no Solana program IDs,
//! account layouts or Pubkey-shaped decode logic itself. Chain-specific
//! discovery/fetcher/calculator types (`PoolDiscovery`, `AccountData`,
//! `PriceCalculator`) are NOT re-exported here — callers that need them
//! import `crate::chains::solana::pools` directly, so this module's public
//! surface stays chain-neutral. Solana swap instruction building/execution
//! lives under `crate::chains::solana::swaps`.

mod api;
pub(crate) mod cache;
mod error;
mod maintenance;

// Re-export db types for blacklist API
pub mod database;
pub use database as db;

pub mod service;
pub mod types;
pub mod utils;

pub use api::{get_available_tokens, get_cache_stats, get_pool_price};
pub use error::{Error, Result};
pub use maintenance::start_maintenance_task;
pub use service::{
    initialize_pool_components, is_pool_service_running, is_single_pool_mode_enabled,
    stop_pool_service,
};
pub use types::{CacheStats, PoolMintVaultInfo, PriceResult, TokenPairInfo};
