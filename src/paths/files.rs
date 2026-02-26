//! Configuration, cache, and data file path resolution.

use super::get_data_directory;
use std::path::PathBuf;

/// Returns the main configuration file path.
pub fn get_config_path() -> PathBuf {
    get_data_directory().join("config.toml")
}

/// Returns the ATA failed cache file path.
pub fn get_ata_failed_cache_path() -> PathBuf {
    get_data_directory().join("ata_failed_cache.json")
}

/// Returns the token blacklist file path.
pub fn get_token_blacklist_path() -> PathBuf {
    get_data_directory().join("token_blacklist.json")
}

/// Returns the RPC statistics file path.
pub fn get_rpc_stats_path() -> PathBuf {
    get_data_directory().join("rpc_stats.json")
}

/// Returns the entry analysis file path.
pub fn get_entry_analysis_path() -> PathBuf {
    get_data_directory().join("entry_analysis.json")
}

/// Returns the UI state file path (for persisting table state, etc.).
pub fn get_ui_state_path() -> PathBuf {
    get_data_directory().join("ui_state.json")
}

/// Returns the process lock file path.
pub fn get_process_lock_path() -> PathBuf {
    get_data_directory().join(".screenerbot.lock")
}

/// Returns the test output file path.
pub fn get_test_output_path() -> PathBuf {
    get_data_directory().join("test_output.log")
}
