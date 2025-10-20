use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;

// Re-export argument handling from the arguments module for backwards compatibility
pub use crate::arguments::{
    get_arg_value, get_cmd_args, get_enabled_debug_modes, has_arg, is_any_debug_enabled,
    is_debug_api_enabled, is_debug_ata_enabled, is_debug_blacklist_enabled,
    is_debug_decimals_enabled, is_debug_discovery_enabled, is_debug_entry_enabled,
    is_debug_filtering_enabled, is_debug_monitor_enabled, is_debug_ohlcv_enabled,
    is_debug_pool_calculator_enabled, is_debug_pool_cleanup_enabled,
    is_debug_pool_decoders_enabled, is_debug_pool_discovery_enabled, is_debug_pool_monitor_enabled,
    is_debug_pool_prices_enabled, is_debug_pool_service_enabled, is_debug_pool_tokens_enabled,
    is_debug_price_service_enabled, is_debug_profit_enabled, is_debug_rpc_enabled,
    is_debug_sol_price_enabled, is_debug_swaps_enabled, is_debug_trader_enabled,
    is_debug_transactions_enabled, is_debug_wallet_enabled, is_debug_webserver_enabled,
    is_dry_run_enabled, print_debug_info, set_cmd_args, CMD_ARGS,
};

// Startup timestamp to track when the bot started for trading logic
pub static STARTUP_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

// ================================================================================================
// 🚀 STARTUP COORDINATION SYSTEM - ENSURES PROPER SERVICE INITIALIZATION ORDER
// ================================================================================================

/// Core services readiness flags - prevents trading until all critical services are ready
pub static TOKENS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POSITIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POOL_SERVICE_READY: AtomicBool = AtomicBool::new(false);
pub static TRANSACTIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);

/// Check if all critical services are ready for trading operations
pub fn are_core_services_ready() -> bool {
    TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst)
        && TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
}

/// Get list of services that are not yet ready (for debugging)
pub fn get_pending_services() -> Vec<&'static str> {
    let mut pending = Vec::new();

    if !TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Tokens System");
    }
    if !POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Positions System");
    }
    if !POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Pool Service");
    }
    if !TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Transactions System");
    }

    pending
}

// ================================================================================================
// 📁 CENTRALIZED DATA PATHS - ALL FILE AND FOLDER PATHS IN ONE PLACE
// ================================================================================================

/// Data directory for all bot-generated files
pub const DATA_DIR: &str = "data";

/// Configuration files
pub const CONFIG_FILE: &str = "data/config.toml";

/// Database files
pub const TOKENS_DATABASE: &str = "data/tokens.db";

/// Cache files
pub const ATA_FAILED_CACHE: &str = "data/ata_failed_cache.json";

pub const TOKEN_BLACKLIST: &str = "data/token_blacklist.json";
pub const RPC_STATS: &str = "data/rpc_stats.json";

/// Position and trading data
pub const ENTRY_ANALYSIS: &str = "data/entry_analysis.json";

/// Cache directories
pub const CACHE_POOL_DIR: &str = "data/cache_pool";

/// Log directory
pub const LOGS_DIR: &str = "logs";

/// Test output file
pub const TEST_OUTPUT: &str = "data/test_output.log";

/// Function to ensure data directory and subdirectories exist
pub fn ensure_data_directories() -> Result<(), Box<dyn std::error::Error>> {
    // Create main data directory
    fs::create_dir_all(DATA_DIR)?;

    // Create cache subdirectories
    fs::create_dir_all(CACHE_POOL_DIR)?;

    // Create logs directory
    fs::create_dir_all(LOGS_DIR)?;

    Ok(())
}

/// Get the full path for a data file (convenience function)
pub fn get_data_path(filename: &str) -> String {
    format!("{}/{}", DATA_DIR, filename)
}
