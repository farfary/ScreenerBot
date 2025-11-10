//! Centralized path resolution for ScreenerBot
//!
//! All file and directory paths are resolved through this module to ensure consistent
//! behavior across different execution contexts (terminal vs bundle) and platforms.
//!
//! ## Path Strategy
//!
//! Both terminal and bundle execution use the same base directory:
//! - **macOS**: `~/ScreenerBot/`
//! - **Windows**: `%USERPROFILE%\ScreenerBot\`
//! - **Linux**: `~/.screenerbot/`
//!
//! ## Directory Structure
//!
//! ```text
//! ~/ScreenerBot/
//! ├── data/
//! │   ├── config.toml
//! │   ├── *.db (databases)
//! │   ├── *.json (caches)
//! │   ├── .screenerbot.lock
//! │   └── cache_pool/
//! ├── logs/
//! │   └── screenerbot_*.log
//! └── analysis-exports/
//!     └── *.csv
//! ```

use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::logger::{self, LogTag};

// =============================================================================
// BASE DIRECTORY RESOLUTION
// =============================================================================

/// Tracks whether initialization logging has been done
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Lazy-initialized base directory (thread-safe)
static BASE_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| {
    let base_dir = resolve_base_directory();
    INITIALIZED.store(true, Ordering::SeqCst);
    base_dir
});

/// Resolves the base directory for all ScreenerBot data
///
/// Uses platform-specific user directories:
/// - macOS: ~/ScreenerBot
/// - Windows: %USERPROFILE%\ScreenerBot
/// - Linux: ~/.screenerbot
fn resolve_base_directory() -> PathBuf {
    let home = dirs::home_dir().expect("Failed to determine home directory");

    #[cfg(target_os = "macos")]
    let base = home.join("ScreenerBot");

    #[cfg(target_os = "windows")]
    let base = home.join("ScreenerBot");

    #[cfg(target_os = "linux")]
    let base = home.join(".screenerbot");

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let base = home.join("ScreenerBot");

    base
}

// =============================================================================
// PRIMARY DIRECTORY ACCESSORS
// =============================================================================

/// Returns the base directory for all ScreenerBot data
///
/// This is the root directory where all data, logs, and exports are stored.
pub fn get_base_directory() -> PathBuf {
    BASE_DIRECTORY.clone()
}

/// Returns the data directory path
///
/// Contains databases, config files, and cache files.
pub fn get_data_directory() -> PathBuf {
    BASE_DIRECTORY.join("data")
}

/// Returns the logs directory path
///
/// Contains daily log files with automatic rotation.
pub fn get_logs_directory() -> PathBuf {
    BASE_DIRECTORY.join("logs")
}

/// Returns the cache pool directory path
///
/// Contains pool-specific cache files.
pub fn get_cache_pool_directory() -> PathBuf {
    get_data_directory().join("cache_pool")
}

/// Returns the analysis exports directory path
///
/// Contains exported CSV files from analysis tools.
pub fn get_analysis_exports_directory() -> PathBuf {
    BASE_DIRECTORY.join("analysis-exports")
}

// =============================================================================
// CONFIGURATION FILE PATHS
// =============================================================================

/// Returns the main configuration file path
pub fn get_config_path() -> PathBuf {
    get_data_directory().join("config.toml")
}

// =============================================================================
// DATABASE FILE PATHS
// =============================================================================

/// Returns the tokens database path
pub fn get_tokens_db_path() -> PathBuf {
    get_data_directory().join("tokens.db")
}

/// Returns the transactions database path
pub fn get_transactions_db_path() -> PathBuf {
    get_data_directory().join("transactions.db")
}

/// Returns the positions database path
pub fn get_positions_db_path() -> PathBuf {
    get_data_directory().join("positions.db")
}

/// Returns the wallet database path
pub fn get_wallet_db_path() -> PathBuf {
    get_data_directory().join("wallet.db")
}

/// Returns the events database path
pub fn get_events_db_path() -> PathBuf {
    get_data_directory().join("events.db")
}

/// Returns the pools database path
pub fn get_pools_db_path() -> PathBuf {
    get_data_directory().join("pools.db")
}

/// Returns the strategies database path
pub fn get_strategies_db_path() -> PathBuf {
    get_data_directory().join("strategies.db")
}

/// Returns the OHLCV database path
pub fn get_ohlcvs_db_path() -> PathBuf {
    get_data_directory().join("ohlcvs.db")
}

// =============================================================================
// CACHE AND DATA FILE PATHS
// =============================================================================

/// Returns the ATA failed cache file path
pub fn get_ata_failed_cache_path() -> PathBuf {
    get_data_directory().join("ata_failed_cache.json")
}

/// Returns the token blacklist file path
pub fn get_token_blacklist_path() -> PathBuf {
    get_data_directory().join("token_blacklist.json")
}

/// Returns the RPC statistics file path
pub fn get_rpc_stats_path() -> PathBuf {
    get_data_directory().join("rpc_stats.json")
}

/// Returns the entry analysis file path
pub fn get_entry_analysis_path() -> PathBuf {
    get_data_directory().join("entry_analysis.json")
}

/// Returns the process lock file path
pub fn get_process_lock_path() -> PathBuf {
    get_data_directory().join(".screenerbot.lock")
}

/// Returns the test output file path
pub fn get_test_output_path() -> PathBuf {
    get_data_directory().join("test_output.log")
}

// =============================================================================
// DATABASE WAL/SHM HELPERS
// =============================================================================

/// Returns all related files for a SQLite database (main DB, SHM, WAL)
///
/// SQLite databases create additional files for write-ahead logging and
/// shared memory. This helper returns all three files for cleanup operations.
///
/// ## Arguments
///
/// * `db_path` - Path to the main database file
///
/// ## Returns
///
/// Vector containing paths to: `[db, db-shm, db-wal]`
pub fn get_db_with_wal_files(db_path: PathBuf) -> Vec<PathBuf> {
    vec![
        db_path.clone(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
    ]
}

// =============================================================================
// DIRECTORY CREATION
// =============================================================================

/// Ensures all required directories exist
///
/// Creates the base directory and all subdirectories needed for operation.
/// This should be called early in the application startup.
///
/// ## Created Directories
///
/// - Base directory (~/ScreenerBot or platform equivalent)
/// - data/
/// - logs/
/// - data/cache_pool/
/// - analysis-exports/
///
/// ## Returns
///
/// - `Ok(())` if all directories exist or were created successfully
/// - `Err(String)` if any directory creation failed
pub fn ensure_all_directories() -> Result<(), String> {
    // Log base directory initialization (safe to log now, outside of lazy init)
    if !is_initialized() {
        eprintln!("📁 Base directory: {}", get_base_directory().display());
    }

    let dirs_to_create = vec![
        ("base", get_base_directory()),
        ("data", get_data_directory()),
        ("logs", get_logs_directory()),
        ("cache_pool", get_cache_pool_directory()),
        ("analysis-exports", get_analysis_exports_directory()),
    ];

    for (name, dir) in dirs_to_create {
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| {
                format!(
                    "Failed to create {} directory at {}: {}",
                    name,
                    dir.display(),
                    e
                )
            })?;

            eprintln!("✅ Created directory: {}", dir.display());
        }
    }

    Ok(())
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Returns a display string for the base directory (for user-facing messages)
pub fn get_base_directory_display() -> String {
    BASE_DIRECTORY.display().to_string()
}

/// Checks if the base directory has been initialized
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_directory_not_empty() {
        let base = get_base_directory();
        assert!(!base.as_os_str().is_empty());
    }

    #[test]
    fn test_data_directory_is_subdir() {
        let base = get_base_directory();
        let data = get_data_directory();
        assert!(data.starts_with(&base));
    }

    #[test]
    fn test_logs_directory_is_subdir() {
        let base = get_base_directory();
        let logs = get_logs_directory();
        assert!(logs.starts_with(&base));
    }

    #[test]
    fn test_database_paths_in_data_dir() {
        let data = get_data_directory();

        assert!(get_tokens_db_path().starts_with(&data));
        assert!(get_transactions_db_path().starts_with(&data));
        assert!(get_positions_db_path().starts_with(&data));
        assert!(get_wallet_db_path().starts_with(&data));
        assert!(get_events_db_path().starts_with(&data));
        assert!(get_pools_db_path().starts_with(&data));
        assert!(get_strategies_db_path().starts_with(&data));
        assert!(get_ohlcvs_db_path().starts_with(&data));
    }

    #[test]
    fn test_config_path_in_data_dir() {
        let data = get_data_directory();
        let config = get_config_path();
        assert!(config.starts_with(&data));
        assert_eq!(config.file_name().unwrap(), "config.toml");
    }

    #[test]
    fn test_cache_pool_in_data_dir() {
        let data = get_data_directory();
        let cache = get_cache_pool_directory();
        assert!(cache.starts_with(&data));
    }

    #[test]
    fn test_analysis_exports_in_base_dir() {
        let base = get_base_directory();
        let exports = get_analysis_exports_directory();
        assert!(exports.starts_with(&base));
    }

    #[test]
    fn test_process_lock_in_data_dir() {
        let data = get_data_directory();
        let lock = get_process_lock_path();
        assert!(lock.starts_with(&data));
        assert_eq!(lock.file_name().unwrap(), ".screenerbot.lock");
    }
}
