//! New modular pool system for real-time price calculations
//!
//! This module provides a centralized pool service that watches up to 100+ tokens
//! and provides real-time prices derived from various DEX pools (Raydium, Orca, etc.).
//!
//! PUBLIC API (only these functions are exposed):
//! - start_pool_service() -> Initialize the pool service
//! - get_pool_price(mint) -> Get current price for a token
//! - get_available_tokens() -> Get list of tokens with available prices
//! - get_price_history(mint) -> Get price history for a token
//!
//! Chain-neutral: persistence (`database`), caching (`cache`) and service
//! lifecycle (`service`) live here, and the `PoolDescriptor` domain model
//! (`types`) is a chain-neutral value object — no `Pubkey`, no Solana vendor
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

use std::sync::Arc;
use tokio::sync::Notify;

mod api;
pub(crate) mod cache;

// Re-export db types for blacklist API
pub mod database;
pub use database as db;

pub mod service;
pub mod types;
pub mod utils;

pub use api::{get_available_tokens, get_cache_stats, get_pool_price, get_price_history};
pub use service::{
    get_debug_token_override, initialize_pool_components, is_pool_service_running,
    is_single_pool_mode_enabled, set_debug_token_override, start_helper_tasks, stop_pool_service,
};
pub use types::{CacheStats, PoolError, PoolMintVaultInfo, PriceResult, TokenPairInfo};
