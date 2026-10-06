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
//! - token_pools(chain, mint) / pool_protocol(chain, mint, pool) -> the
//!   chain's known pools of a token
//! - pricing_status(scope) -> the pricing pipelines' state, merged over the
//!   scope's chains
//!
//! Chain-neutral: persistence (`database`), caching (`cache`), service
//! lifecycle (`service`), periodic upkeep (`maintenance`) and the
//! `PricingDriver` contract (`driver`) live here, and the `PoolDescriptor`
//! domain model (`types`) is a chain-neutral value object — no `Pubkey`, no
//! Solana vendor type, anywhere in this module. Each chain's pool discovery,
//! account fetching, protocol recognition, decoding and price calculation is
//! its pricing driver, owned by the chain module and reached only through
//! `ChainRuntime::pricing_driver`. This module consumes the driver's output
//! (`PoolDescriptor` instances and published prices) but owns no program IDs,
//! account layouts or chain-specific decode logic itself.

mod api;
pub(crate) mod cache;
mod driver;
mod error;
mod maintenance;

// Re-export db types for blacklist API
pub mod database;
pub use database as db;

pub mod service;
pub mod types;
pub mod utils;

pub use api::{
    get_available_tokens, get_cache_stats, get_pool_price, pool_protocol, pricing_status,
    token_pools,
};
pub use driver::{
    sort_protocol_counts, AccountFetchStatus, PoolDirectoryStatus, PricingDriver, PricingInit,
    PricingStage, PricingStageMetrics, PricingStatus,
};
pub use error::{Error, Result};
pub use maintenance::start_maintenance_task;
pub use service::{
    initialize_pool_components, is_pool_service_running, is_single_pool_mode_enabled,
    pricing_stage_metrics, pricing_stage_ready, start_pricing_stage, stop_pool_service,
};
pub use types::{CacheStats, PoolMintVaultInfo, PriceResult, TokenPairInfo};
