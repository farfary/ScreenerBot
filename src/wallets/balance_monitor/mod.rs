// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
mod worth;

pub(crate) use service::wallet_database_chain;

// Re-export public API
pub use service::{
    clear_dashboard_api_cache,
    force_wallet_snapshot,
    get_comparable_worth_history,
    get_current_wallet_status,
    get_daily_end_balances,
    get_dashboard_cache_metrics,
    get_flow_cache_stats,
    // Snapshot queries
    get_recent_wallet_snapshots,
    get_snapshot_nft_balances,
    get_snapshot_token_balances,
    // Monitoring stats
    get_wallet_monitor_stats,
    get_worth_at_time,
    // Initialization & background service
    initialize_wallet_database,
    // Dashboard cache management
    refresh_dashboard_cache,
    // Rebind the monitor subject after a main-wallet change
    refresh_wallet_monitor_subject,
    start_wallet_monitoring_service,
};

pub use cache::get_cached_wallet_snapshot_status;
pub use dashboard::get_wallet_dashboard_data;
/// Rename declarations, reached by the schema owner's cross-store guard.
#[cfg(test)]
pub(crate) use database::column_names;
pub use database::{get_wallet_service_metrics, is_wallet_database_ready};
pub(crate) use database::{recorded_wallet_sql, SCHEMA};

// The single source of truth for what the wallet is worth, and the hook that keeps
// it live. Every user-facing wallet figure goes through `get_wallet_worth()`.
pub use worth::{
    get_held_mints, get_wallet_worth, held_token_price_native, live_wallet_snapshot,
    request_balance_refresh, worth_change, WorthChange,
};

// Re-export public types
pub use types::{
    CachePerformanceMetrics, DailyFlowPoint, DashboardCacheFreshness, DashboardCacheMetadata,
    DashboardDataSource, NftBalance, SnapshotTokenBalance, WalletBalancePoint, WalletDashboardData,
    WalletFlowCacheStats, WalletFlowMetrics, WalletMonitorStats, WalletNftOverview, WalletSnapshot,
    WalletSnapshotStatus, WalletSummarySnapshot, WalletTokenOverview, WalletWorth, WorthSample,
};
