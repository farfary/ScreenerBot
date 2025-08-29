use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use once_cell::sync::Lazy;
use chrono::{ DateTime, Utc };

// Re-export argument handling from the arguments module for backwards compatibility
pub use crate::arguments::{
    CMD_ARGS,
    set_cmd_args,
    get_cmd_args,
    has_arg,
    get_arg_value,
    is_debug_filtering_enabled,
    is_debug_profit_enabled,
    is_debug_pool_prices_enabled,
    is_debug_trader_enabled,
    is_debug_api_enabled,
    is_debug_monitor_enabled,
    is_debug_discovery_enabled,
    is_debug_price_service_enabled,
    is_debug_rugcheck_enabled,
    is_debug_entry_enabled,
    is_debug_ohlcv_enabled,
    is_debug_wallet_enabled,
    is_debug_swaps_enabled,
    is_debug_decimals_enabled,
    is_debug_summary_enabled,
    is_debug_transactions_enabled,
    is_debug_rpc_enabled,
    is_dry_run_enabled,
    is_any_debug_enabled,
    get_enabled_debug_modes,
    print_debug_info,
};

// Re-export configuration handling from the configs module for backwards compatibility
pub use crate::configs::{
    Configs,
    read_configs,
    read_configs_from_path,
    load_wallet_from_config,
    validate_configs,
    get_wallet_pubkey_string,
    create_default_config,
    save_configs_to_path,
};

// Startup timestamp to track when the bot started for trading logic
pub static STARTUP_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

// Position recalculation completion flag - prevents trading until startup recalculation is done
pub static POSITION_RECALCULATION_COMPLETE: AtomicBool = AtomicBool::new(false);

// ================================================================================================
// 📁 CENTRALIZED DATA PATHS - ALL FILE AND FOLDER PATHS IN ONE PLACE
// ================================================================================================

/// Data directory for all bot-generated files
pub const DATA_DIR: &str = "data";

/// Configuration files
pub const CONFIG_FILE: &str = "data/configs.json";

/// Database files
pub const TOKENS_DATABASE: &str = "data/tokens.db";

/// Cache files
pub const ATA_FAILED_CACHE: &str = "data/ata_failed_cache.json";

pub const TOKEN_BLACKLIST: &str = "data/token_blacklist.json";
pub const RPC_STATS: &str = "data/rpc_stats.json";

/// Position and trading data
pub const ENTRY_ANALYSIS: &str = "data/entry_analysis.json";

/// Cache directories
pub const CACHE_OHLCVS_DIR: &str = "data/cache_ohlcvs";
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
    fs::create_dir_all(CACHE_OHLCVS_DIR)?;
    fs::create_dir_all(CACHE_POOL_DIR)?;

    // Create logs directory
    fs::create_dir_all(LOGS_DIR)?;

    Ok(())
}

/// Get the full path for a data file (convenience function)
pub fn get_data_path(filename: &str) -> String {
    format!("{}/{}", DATA_DIR, filename)
}
