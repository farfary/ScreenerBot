/// Database module for persistent price history storage
///
/// This module provides SQLite-based storage for price history data,
/// enabling price history to survive service restarts and providing
/// full historical data access beyond the in-memory cache limits.

// Sub-modules
mod blacklist;
mod global;
mod operations;
mod types;
mod writer;

// Re-export public types
pub use types::{BlacklistedAccountRecord, BlacklistedPoolRecord, DbPriceResult};

// Re-export public functions from global module
pub use global::{
    add_account_to_blacklist, add_pool_to_blacklist, cleanup_all_gapped_data,
    cleanup_gapped_data_for_token, cleanup_old_entries, get_blacklist_stats,
    get_extended_price_history, initialize_database, is_account_blacklisted, is_pool_blacklisted,
    list_blacklisted_accounts, list_blacklisted_pools, load_historical_data_for_token,
    queue_price_for_storage,
};

// Re-export PoolsDatabase for advanced usage
pub use operations::PoolsDatabase;
