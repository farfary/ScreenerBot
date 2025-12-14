//! Trade Watcher Tool
//!
//! Monitor token trades from external wallets and trigger automatic actions:
//! - Buy when external wallet sells (counter-trade)
//! - Sell when external wallet buys (follow-trade)
//! - Notify only (just send alerts)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use screenerbot::tools::trade_watcher::{
//!     search_pools, start_trade_monitor, stop_trade_monitor, is_trade_monitor_running,
//! };
//!
//! // Search for pools to select one for watching
//! let pools = search_pools("TokenMintAddress...").await?;
//! println!("Found {} pools", pools.len());
//!
//! // Start the trade monitor
//! start_trade_monitor().await;
//!
//! // Check status
//! let running = is_trade_monitor_running().await;
//!
//! // Stop when done
//! stop_trade_monitor().await;
//! ```
//!
//! ## Configuration
//!
//! Watched tokens are stored in the tools database. Use the database functions
//! to add, remove, or modify watched tokens:
//!
//! ```rust,ignore
//! use screenerbot::tools::database::{
//!     add_watched_token, get_watched_tokens, update_watched_token_status,
//!     delete_watched_token, WatchedTokenConfig,
//! };
//!
//! // Add a new watched token
//! let config = WatchedTokenConfig {
//!     mint: "TokenMint...".to_string(),
//!     symbol: Some("TOKEN".to_string()),
//!     pool_address: "PoolAddress...".to_string(),
//!     pool_source: "geckoterminal".to_string(),
//!     pool_dex: Some("raydium".to_string()),
//!     pool_pair: Some("TOKEN/SOL".to_string()),
//!     pool_liquidity: Some(100000.0),
//!     watch_type: "buy_on_sell".to_string(),
//!     trigger_amount_sol: Some(1.0),
//!     action_amount_sol: Some(0.5),
//!     slippage_bps: Some(500),
//! };
//! let id = add_watched_token(&config)?;
//! ```

mod monitor;
mod pool_selector;
mod types;

// Re-export types
pub use types::{DetectedTrade, PoolInfo, PoolSource, TradeMonitorStatus, WatchType};

// Re-export pool selector functions
pub use pool_selector::{get_best_pool, search_pools, search_pools_by_source, search_pools_with_min_liquidity};

// Re-export monitor functions
pub use monitor::{
    clear_tracked_signatures, get_trade_monitor_status, is_trade_monitor_running,
    refresh_own_wallets, start_trade_monitor, stop_trade_monitor,
};

// Re-export database types and functions for convenience
pub use crate::tools::database::{
    add_watched_token, delete_watched_token, get_active_watched_tokens, get_watched_tokens,
    update_watched_token_status, update_watched_token_tracking, WatchedToken, WatchedTokenConfig,
};
