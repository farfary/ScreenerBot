//! Tools module for ScreenerBot
//!
//! Contains utility tools like volume aggregator for token operations.
//!
//! ## Available Tools
//! - `ata_cleanup` - Scan and close empty ATAs to reclaim rent
//! - `volume_aggregator` - Generate trading volume using multiple wallets
//! - `swap_executor` - Execute swaps with custom keypairs (no position tracking)
//! - `multi_wallet` - Multi-wallet trading tools (buy/sell/consolidate)
//! - `trade_watcher` - Monitor external wallet trades and trigger actions
//!
//! ## Database
//! - `database` - Persistent storage for tool sessions and operations

pub mod ata_cleanup;
pub mod database;
pub mod multi_wallet;
pub mod swap_executor;
pub mod trade_watcher;
mod types;
pub mod volume_aggregator;

// Re-export common types
pub use types::{
    DelayConfig, DistributionStrategy, SizingConfig, ToolResult, ToolStatus, WalletMode,
};

// Re-export database types
pub use database::{
    get_recent_va_sessions, get_tool_favorites, get_va_sessions_analytics,
    increment_tool_favorite_use, init_tools_db, remove_tool_favorite, update_tool_favorite,
    upsert_tool_favorite, FailedAtaRow, ToolFavoriteRow, VaAnalyticsSummary, VaSessionRow,
    VaSwapRow,
};

// Re-export swap executor
pub use swap_executor::{execute_tool_swap, tool_buy, tool_sell, ToolSwapResult};

// Re-export volume aggregator
pub use volume_aggregator::{
    calculate_amount, calculate_amount_clamped, calculate_delay, SessionStatus, StrategyExecutor,
    TransactionStatus, VolumeAggregator, VolumeConfig, VolumeSession, VolumeTransaction,
};

// Re-export ATA cleanup types and functions
pub use ata_cleanup::{
    cleanup_empty_atas,
    clear_failed_ata_cache,
    close_ata,
    // Backward compatibility aliases
    get_ata_cleanup_statistics,
    get_ata_cleanup_stats,
    get_ata_status,
    get_cleanup_stats,
    get_failed_ata_count,
    scan_closeable_atas,
    scan_empty_atas,
    scan_wallet_atas,
    start_ata_cleanup_service,
    trigger_immediate_ata_cleanup,
    trigger_immediate_cleanup,
    AtaCleanupResult,
    AtaCleanupStats,
    AtaClosure,
    AtaInfo,
    AtaSession,
    FailedAta,
};

// Re-export multi-wallet types and operations
pub use multi_wallet::{
    close_ata as close_token_ata,
    collect_sol,
    execute_consolidation,
    // Operations
    execute_multi_buy,
    execute_multi_sell,
    fund_wallets,
    // Transfer utilities
    transfer_sol,
    transfer_token,
    ConsolidateConfig,
    // Configuration types
    MultiBuyConfig,
    MultiSellConfig,
    // Result types
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
