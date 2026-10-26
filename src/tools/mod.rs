// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tools module for ScreenerBot
//!
//! Contains utility tools for token operations.
//!
//! ## Available Tools
//! - `swap_executor` - Execute swaps with custom keypairs (no position tracking)
//! - `multi_wallet` - Multi-wallet trading tools (buy/sell/consolidate)
//! - `trade_watcher` - Monitor external wallet trades and trigger actions
//!
//! ## Database
//! - `database` - Persistent storage for tool sessions and operations

pub mod database;
mod error;
pub mod multi_wallet;
pub mod swap_executor;
pub mod trade_watcher;
mod types;

pub use error::{Error, Result};

// Re-export common types
pub use types::{
    DelayConfig, DistributionStrategy, SizingConfig, ToolResult, ToolStatus, WalletMode,
};

// Re-export database types
pub use database::{
    get_tool_favorites, increment_tool_favorite_use, remove_tool_favorite, update_tool_favorite,
    upsert_tool_favorite, ToolFavoriteRow,
};

// Re-export swap executor
pub use swap_executor::{execute_tool_swap, tool_buy, tool_sell, ToolSwapResult};

// Re-export multi-wallet types and operations. Single-transfer/ATA-close
// mechanics live at crate::chains::solana::assets::transfer, not here.
pub use multi_wallet::{
    collect_sol,
    execute_consolidation,
    // Operations
    execute_multi_buy,
    execute_multi_sell,
    fund_wallets,
    ConsolidateConfig,
    ConsolidationResult,
    // Configuration types
    MultiBuyConfig,
    MultiSellConfig,
    // Result types
    SessionProgress,
    SessionResult,
    SessionStatus as MultiWalletSessionStatus,
    WalletOpResult,
    WalletPlan,
};

// Re-export trade watcher types and functions
pub use trade_watcher::{
    // Database operations
    add_watched_token,
    // Monitor control
    clear_tracked_signatures,
    delete_watched_token,
    get_active_watched_tokens,
    // Pool search
    get_best_pool,
    get_trade_monitor_status,
    get_watched_tokens,
    is_trade_monitor_running,
    refresh_own_wallets,
    search_pools,
    search_pools_by_source,
    search_pools_with_min_liquidity,
    start_trade_monitor,
    stop_trade_monitor,
    update_watched_token_status,
    update_watched_token_tracking,
    // Types
    DetectedTrade,
    PoolInfo,
    PoolSource,
    TradeMonitorStatus,
    WatchType,
    WatchedToken,
    WatchedTokenConfig,
};
