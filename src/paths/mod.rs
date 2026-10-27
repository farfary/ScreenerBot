// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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

use crate::errors::IoError;

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

    // Explicit overrides apply to every process. Test processes without an
    // override use a shared temporary root instead of the platform data directory.
    if let Ok(dir) = std::env::var("SCREENERBOT_DATA_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    if cfg!(test) || std::env::var_os("NEXTEST_RUN_ID").is_some() {
        return std::env::temp_dir().join("screenerbot-tests");
    }

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

/// Name of the directory beside the stores that holds their backups.
pub const BACKUPS_DIRECTORY_NAME: &str = "backups";

/// Returns the backups directory path.
///
/// Contains the copy of each store taken before an upgrade rebuilt it and the
/// wallet-scoped stores moved by a wallet data reset or a trading-wallet replacement.
pub fn get_backups_directory() -> PathBuf {
    get_data_directory().join(BACKUPS_DIRECTORY_NAME)
}

/// Returns the path of the entry named `name` inside the backups directory.
pub fn get_backup_path(name: impl AsRef<std::path::Path>) -> PathBuf {
    get_backups_directory().join(name)
}

/// Name of the directory beside the stores that holds stores moved aside after a
/// failed open.
pub const QUARANTINE_DIRECTORY_NAME: &str = "quarantine";

/// Returns the quarantine directory path.
///
/// Holds each store that could not be opened, with its `-wal` and `-shm`, in a
/// directory of its own; nothing in it is deleted automatically.
pub fn get_quarantine_directory() -> PathBuf {
    get_data_directory().join(QUARANTINE_DIRECTORY_NAME)
}

/// Name prefix of the backup directories in the backups directory that receive the
/// wallet-scoped stores when the wallet data reset runs or the trading wallet is
/// replaced, each with a `manifest.json`.
pub const WALLET_RESET_DIRECTORY_PREFIX: &str = "wallet-reset-";

/// Returns the logs directory path.
///
/// Contains the log files, rotated by size and pruned by storage housekeeping.
pub fn get_logs_directory() -> PathBuf {
    BASE_DIRECTORY.join("logs")
}

/// File name of the link that always names the active log file.
pub const LATEST_LOG_NAME: &str = "latest.log";

/// Name prefix of every log file; each is `<prefix><timestamp>[_<n>].log`.
pub const LOG_FILE_PREFIX: &str = "screenerbot_";

/// Returns the path of the link that always names the active log file.
pub fn get_latest_log_path() -> PathBuf {
    get_logs_directory().join(LATEST_LOG_NAME)
}

/// Returns the directory logs are written to when the logs directory cannot be
/// created.
pub fn get_log_fallback_directory() -> PathBuf {
    std::env::temp_dir().join("screenerbot_logs")
}

/// Returns the updates directory path.
///
/// Holds downloaded release installers and core archives, and the partial file
/// of a download in progress.
pub fn get_updates_directory() -> PathBuf {
    get_data_directory().join("updates")
}

/// Returns the staged core directory path.
///
/// Holds every core binary staged by an update and the pointer naming the one
/// the desktop shell launches.
pub fn get_core_directory() -> PathBuf {
    get_data_directory().join("core")
}

// =============================================================================
// DIRECTORY CREATION
// =============================================================================

/// Ensures all required directories exist.
///
/// Creates the base directory and all subdirectories needed for operation.
/// This should be called early in the application startup.
pub fn ensure_all_directories() -> Result<(), IoError> {
    // Log base directory initialization (safe to log now, outside of lazy init)
    if !is_initialized() {
        eprintln!("Base directory: {}", get_base_directory().display());
    }

    let dirs_to_create = vec![
        ("base", get_base_directory()),
        ("data", get_data_directory()),
        ("logs", get_logs_directory()),
    ];

    for (name, dir) in dirs_to_create {
        if !dir.exists() {
            std::fs::create_dir_all(&dir).map_err(|e| IoError::Generic {
                message: format!(
                    "failed to create {name} directory at {}: {e}",
                    dir.display()
                ),
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
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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

        assert!(crate::chains::get_tokens_db_path().starts_with(&data));
        assert!(get_transactions_db_path().starts_with(&data));
        assert!(get_positions_db_path().starts_with(&data));
        assert!(get_wallet_db_path().starts_with(&data));
        assert!(get_events_db_path().starts_with(&data));
        assert!(crate::chains::get_pools_db_path().starts_with(&data));
        assert!(get_strategies_db_path().starts_with(&data));
        assert!(crate::chains::get_ohlcvs_db_path().starts_with(&data));
    }

    #[test]
    fn test_config_path_in_data_dir() {
        let data = get_data_directory();
        let config = get_config_path();
        assert!(config.starts_with(&data));
        assert_eq!(config.file_name().unwrap(), "config.toml");
    }

    #[test]
    fn test_process_lock_in_data_dir() {
        let data = get_data_directory();
        let lock = get_process_lock_path();
        assert!(lock.starts_with(&data));
        assert_eq!(lock.file_name().unwrap(), ".screenerbot.lock");
    }

    #[test]
    fn test_process_without_override_uses_temporary_base_directory() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let previous = std::env::var_os("SCREENERBOT_DATA_DIR");
        unsafe { std::env::remove_var("SCREENERBOT_DATA_DIR") };
        let base = resolve_base_directory();
        if let Some(value) = previous {
            unsafe { std::env::set_var("SCREENERBOT_DATA_DIR", value) };
        }

        assert!(base.starts_with(std::env::temp_dir()));
        for platform_directory in [dirs::data_local_dir(), dirs::data_dir(), dirs::home_dir()]
            .into_iter()
            .flatten()
            .map(|directory| directory.join("ScreenerBot"))
        {
            assert!(!base.starts_with(platform_directory));
        }
    }
}
