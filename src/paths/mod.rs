//! Centralized path resolution for ScreenerBot.
//!
//! All file and directory paths are resolved through this module to ensure consistent
//! behavior across different execution contexts (terminal vs bundle) and platforms.
//!
//! ## Path Strategy
//!
//! Both terminal and bundle execution use the same base directory following
//! platform standards:
//! - **macOS**: `~/Library/Application Support/ScreenerBot/`
//! - **Windows**: `%LOCALAPPDATA%\ScreenerBot\`
//! - **Linux**: `$XDG_DATA_HOME/ScreenerBot/` (fallback `~/.local/share/ScreenerBot/`)

mod database;
mod files;
mod platform;

pub use database::*;
pub use files::*;
pub use platform::*;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

// =============================================================================
// BASE DIRECTORY RESOLUTION
// =============================================================================

/// Tracks whether initialization logging has been done
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Lazy-initialized base directory (thread-safe)
static BASE_DIRECTORY: LazyLock<PathBuf> = LazyLock::new(|| {
    let base_dir = resolve_base_directory();
    INITIALIZED.store(true, Ordering::SeqCst);
    base_dir
});

/// Resolves the base directory for all ScreenerBot data.
///
/// Uses platform-specific application data locations:
/// - macOS: ~/Library/Application Support/ScreenerBot
/// - Windows: %LOCALAPPDATA%\ScreenerBot
/// - Linux: $XDG_DATA_HOME/ScreenerBot (fallback ~/.local/share/ScreenerBot)
fn resolve_base_directory() -> PathBuf {
    const APP_DIR: &str = "ScreenerBot";

    if let Some(dir) = dirs::data_local_dir() {
        return dir.join(APP_DIR);
    }

    if let Some(dir) = dirs::data_dir() {
        return dir.join(APP_DIR);
    }

    if let Some(home) = dirs::home_dir() {
        return home.join(APP_DIR);
    }

    PathBuf::from(APP_DIR)
}

// =============================================================================
// PRIMARY DIRECTORY ACCESSORS
// =============================================================================

/// Returns the base directory for all ScreenerBot data.
///
/// This is the root directory where all data, logs, and exports are stored.
pub fn get_base_directory() -> PathBuf {
    BASE_DIRECTORY.clone()
}

/// Returns the data directory path.
///
/// Contains databases, config files, and cache files.
pub fn get_data_directory() -> PathBuf {
    BASE_DIRECTORY.join("data")
}

/// Returns the logs directory path.
///
/// Contains daily log files with automatic rotation.
pub fn get_logs_directory() -> PathBuf {
    BASE_DIRECTORY.join("logs")
}

/// Returns the cache pool directory path.
///
/// Contains pool-specific cache files.
pub fn get_cache_pool_directory() -> PathBuf {
    get_data_directory().join("cache_pool")
}

/// Returns the analysis exports directory path.
///
/// Contains exported CSV files from analysis tools.
pub fn get_analysis_exports_directory() -> PathBuf {
    BASE_DIRECTORY.join("analysis-exports")
}

// =============================================================================
// DIRECTORY CREATION
// =============================================================================

/// Ensures all required directories exist.
///
/// Creates the base directory and all subdirectories needed for operation.
/// This should be called early in the application startup.
pub fn ensure_all_directories() -> Result<(), String> {
    // Log base directory initialization (safe to log now, outside of lazy init)
    if !is_initialized() {
        eprintln!("Base directory: {}", get_base_directory().display());
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

            eprintln!("Created directory: {}", dir.display());
        }
    }

    Ok(())
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Returns a display string for the base directory (for user-facing messages).
pub fn get_base_directory_display() -> String {
    BASE_DIRECTORY.display().to_string()
}

/// Checks if the base directory has been initialized.
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
