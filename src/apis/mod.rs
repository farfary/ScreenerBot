//! Centralized API clients for external data sources
//!
//! Global singleton pattern ensures:
//! - Single instance of each API client across the entire bot
//! - True global rate limiting per API (not per-instance)
//! - Centralized stats tracking for all API calls
//!
//! Usage:
//! ```rust
//! use crate::apis::get_api_manager;
//!
//! let apis = get_api_manager();
//! apis.dexscreener.fetch_token_pools(mint).await?;
//! ```

// Base utilities
pub mod client;
pub mod manager;
pub mod stats;

// API client modules (each in its own subdirectory)
pub mod coingecko;
pub mod defillama;
pub mod dexscreener;
pub mod geckoterminal;
pub mod jupiter;
pub mod llm;
pub mod rugcheck;
pub mod sol_price;
pub mod solana_tracker;

// Re-exports for convenience
pub use client::{HttpClient, RateLimiter};
pub use manager::{get_api_manager, ApiManager, ApiManagerStats};
pub use stats::{ApiStats, ApiStatsTracker};

// Client type re-exports
pub use coingecko::CoinGeckoClient;
pub use defillama::DefiLlamaClient;
pub use dexscreener::DexScreenerClient;
pub use geckoterminal::GeckoTerminalClient;
pub use jupiter::JupiterClient;
pub use rugcheck::RugcheckClient;
pub use solana_tracker::SolanaTrackerClient;
