//! Tools module for ScreenerBot
//!
//! Contains utility tools like volume aggregator for token operations.
//!
//! ## Available Tools
//! - `ata_cleanup` - Scan and close empty ATAs to reclaim rent
//! - `volume_aggregator` - Generate trading volume using multiple wallets
//! - `swap_executor` - Execute swaps with custom keypairs (no position tracking)
//!
//! ## Database
//! - `database` - Persistent storage for tool sessions and operations

pub mod ata_cleanup;
pub mod database;
pub mod swap_executor;
mod types;
pub mod volume_aggregator;

// Re-export common types
pub use types::{DelayConfig, DistributionStrategy, SizingConfig, ToolResult, ToolStatus, WalletMode};

// Re-export database types
pub use database::{
    FailedAtaRow, ToolFavoriteRow, VaAnalyticsSummary, VaSessionRow, VaSwapRow,
    get_recent_va_sessions, get_va_sessions_analytics, init_tools_db,
    get_tool_favorites, upsert_tool_favorite, remove_tool_favorite,
    update_tool_favorite, increment_tool_favorite_use,
};

// Re-export swap executor
pub use swap_executor::{execute_tool_swap, tool_buy, tool_sell, ToolSwapResult};

// Re-export volume aggregator
pub use volume_aggregator::{
    calculate_amount, calculate_amount_clamped, calculate_delay,
    SessionStatus, StrategyExecutor, TransactionStatus,
    VolumeAggregator, VolumeConfig, VolumeSession, VolumeTransaction,
};

// Re-export ATA cleanup types and functions
pub use ata_cleanup::{
    AtaCleanupResult, AtaCleanupStats, AtaInfo, AtaSession, AtaClosure, FailedAta,
    cleanup_empty_atas, clear_failed_ata_cache, close_ata, get_ata_status,
    get_cleanup_stats, get_failed_ata_count, scan_closeable_atas, scan_empty_atas,
    scan_wallet_atas, start_ata_cleanup_service, trigger_immediate_cleanup,
    // Backward compatibility aliases
    get_ata_cleanup_statistics, trigger_immediate_ata_cleanup, get_ata_cleanup_stats,
};
