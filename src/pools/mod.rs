/// New modular pool system for real-time price calculations
///
/// This module provides a centralized pool service that watches up to 100+ tokens
/// and provides real-time prices derived from various DEX pools (Raydium, Orca, etc.).
///
/// PUBLIC API (only these functions are exposed):
/// - start_pool_service() -> Initialize the pool service
/// - get_pool_price(mint) -> Get current price for a token
/// - get_available_tokens() -> Get list of tokens with available prices
/// - get_price_history(mint) -> Get price history for a token
use std::sync::Arc;
use tokio::sync::Notify;

mod analyzer;
mod api;
mod cache;
mod calculator;
mod db;
mod discovery;
mod fetcher;

pub mod decoders;
pub mod service;
pub mod swap;
pub mod types;
pub mod utils;

pub use api::{
    check_price_history_quality, get_available_tokens, get_cache_stats, get_extended_price_history,
    get_pool_price, get_price_history, get_price_history_stats, get_token_pools,
    load_token_history_into_cache, PriceHistoryStats,
};
pub use discovery::{
    get_canonical_pool_address, is_dexscreener_discovery_enabled,
    is_geckoterminal_discovery_enabled, is_raydium_discovery_enabled, PoolDiscovery,
};
pub use fetcher::AccountData;
pub use service::{
    get_account_fetcher, get_debug_token_override, get_pool_analyzer, get_pool_discovery,
    get_price_calculator, initialize_pool_components, is_pool_service_running,
    is_single_pool_mode_enabled, set_debug_token_override, start_helper_tasks, stop_pool_service,
};
pub use types::{PoolError, PriceResult};
