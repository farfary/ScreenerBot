//! Wallet Balance Monitoring Module
//!
//! This module provides wallet balance monitoring with historical snapshots stored in SQLite database.
//! It monitors both SOL balance and token balances for the configured wallet address.
//!
//! Features:
//! - Background service that checks wallet balance every minute
//! - Delayed RPC calls to avoid overwhelming the global RPC client
//! - Historical snapshots stored in data/wallet.db
//! - Tracks both SOL and token balances
//! - Integration with existing RPC infrastructure
//! - Pure wallet monitoring without position management interference

mod cache;
mod dashboard;
mod database;
mod service;
mod types;

// Re-export public API
pub use service::{
    // Initialization & background service
    initialize_wallet_database,
    start_wallet_monitoring_service,
    // Snapshot queries
    get_recent_wallet_snapshots,
    get_snapshot_token_balances,
    get_snapshot_nft_balances,
    get_current_wallet_status,
    get_balance_at_time,
    // Monitoring stats
    get_wallet_monitor_stats,
    get_flow_cache_stats,
    // Dashboard cache management
    refresh_dashboard_cache,
    get_dashboard_cache_metrics,
    clear_dashboard_api_cache,
};

pub use database::get_wallet_service_metrics;
pub use dashboard::get_wallet_dashboard_data;
pub use cache::get_cached_wallet_snapshot_status;

// Re-export public types
pub use types::{
    CachePerformanceMetrics, DailyFlowPoint, DashboardCacheFreshness, DashboardCacheMetadata,
    DashboardDataSource, NftBalance, SnapshotTokenBalance, WalletBalancePoint, WalletDashboardData,
    WalletFlowCacheStats, WalletFlowMetrics, WalletMonitorStats, WalletNftOverview, WalletSnapshot,
    WalletSnapshotStatus, WalletSummarySnapshot, WalletTokenOverview,
};
