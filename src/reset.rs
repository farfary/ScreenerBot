// Reset utility for clearing bot state and databases
//
// This module provides functionality to reset various parts of the bot's state,
// including pending verifications, database files, and cache files.
//
// Usage:
//   cargo run --bin screenerbot -- --reset          # Interactive mode (asks for confirmation)
//   cargo run --bin screenerbot -- --reset --force  # Force mode (no confirmation)

use crate::logger::{self, LogTag};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Files and directories to be removed during reset
const RESET_TARGETS: &[&str] = &[
    "data/positions.db",
    "data/positions.db-shm",
    "data/positions.db-wal",
    "data/events.db",
    "data/events.db-shm",
    "data/events.db-wal",
    "data/rpc_stats.json",
    "data/ata_failed_cache.json",
];

/// Configuration for reset operation
#[derive(Debug, Clone)]
pub struct ResetConfig {
    pub force: bool,
    pub targets: Vec<String>,
}

impl Default for ResetConfig {
    fn default() -> Self {
        Self {
            force: false,
            targets: RESET_TARGETS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Execute reset operation with given configuration
pub fn execute_reset(config: ResetConfig) -> Result<(), String> {
    logger::info(LogTag::System, "🔄 Reset operation starting...");

    // Show what will be reset
    print_reset_targets(&config.targets);

    // Ask for confirmation if not forced
    if !config.force {
        if !confirm_reset()? {
            logger::info(LogTag::System, "❌ Reset operation cancelled by user");
            return Ok(());
        }
    } else {
        logger::warning(
            LogTag::System,
            "⚠️  Force mode enabled - skipping confirmation",
        );
    }

    // Execute reset
    let mut removed_count = 0;
    let mut error_count = 0;
    let mut total_size = 0u64;

    for target in &config.targets {
        let path = PathBuf::from(target);

        if !path.exists() {
            logger::info(
                LogTag::System,
                &format!("⏭️  Skipping (does not exist): {}", target),
            );
            continue;
        }

        // Get file size before removal
        if let Ok(metadata) = fs::metadata(&path) {
            total_size += metadata.len();
        }

        // Remove file
        match remove_file_or_dir(&path) {
            Ok(_) => {
                removed_count += 1;
                logger::info(LogTag::System, &format!("✅ Removed: {}", target));
            }
            Err(e) => {
                error_count += 1;
                logger::error(
                    LogTag::System,
                    &format!("❌ Failed to remove {}: {}", target, e),
                );
            }
        }
    }

    // Print summary
    logger::info(LogTag::System, "");
    logger::info(
        LogTag::System,
        "═══════════════════════════════════════════════════════════════",
    );
    logger::info(
        LogTag::System,
        &format!("✅ Reset operation complete!"),
    );
    logger::info(
        LogTag::System,
        &format!("   Files removed: {}", removed_count),
    );
    logger::info(
        LogTag::System,
        &format!("   Errors: {}", error_count),
    );
    logger::info(
        LogTag::System,
        &format!("   Space freed: {:.2} MB", total_size as f64 / 1_048_576.0),
    );
    logger::info(
        LogTag::System,
        "═══════════════════════════════════════════════════════════════",
    );

    if error_count > 0 {
        return Err(format!(
            "Reset completed with {} errors",
            error_count
        ));
    }

    Ok(())
}

/// Print list of targets that will be reset
fn print_reset_targets(targets: &[String]) {
    logger::warning(
        LogTag::System,
        "⚠️  The following files/directories will be DELETED:",
    );
    logger::info(LogTag::System, "");

    for target in targets {
        let path = PathBuf::from(target);
        let exists = path.exists();
        let size = if exists {
            fs::metadata(&path)
                .map(|m| format!(" ({:.2} MB)", m.len() as f64 / 1_048_576.0))
                .unwrap_or_default()
        } else {
            String::from(" (does not exist)")
        };

        logger::info(LogTag::System, &format!("   • {}{}", target, size));
    }

    logger::info(LogTag::System, "");
}

/// Ask user for confirmation to proceed with reset
fn confirm_reset() -> Result<bool, String> {
    print!("⚠️  Are you sure you want to proceed? (y/n): ");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {}", e))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;

    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

/// Remove a file or directory
fn remove_file_or_dir(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("Failed to remove directory: {}", e))?;
    } else {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to remove file: {}", e))?;
    }
    Ok(())
}

/// Clear pending verification metadata from positions database
pub fn clear_pending_verifications() -> Result<(), String> {
    use rusqlite::Connection;

    let db_path = "data/positions.db";
    if !Path::new(db_path).exists() {
        logger::info(
            LogTag::System,
            &format!("⏭️  Positions database does not exist: {}", db_path),
        );
        return Ok(());
    }

    logger::info(
        LogTag::System,
        "🧹 Clearing pending verification metadata...",
    );

    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open positions database: {}", e))?;

    // Clear pending DCA swaps metadata
    match conn.execute(
        "DELETE FROM position_metadata WHERE key = 'pending_dca_swaps'",
        [],
    ) {
        Ok(count) => {
            if count > 0 {
                logger::info(
                    LogTag::System,
                    &format!("✅ Cleared pending DCA swaps metadata ({} rows)", count),
                );
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::System,
                &format!("Failed to clear DCA metadata: {}", e),
            );
        }
    }

    // Clear pending partial exits metadata
    match conn.execute(
        "DELETE FROM position_metadata WHERE key = 'pending_partial_exits'",
        [],
    ) {
        Ok(count) => {
            if count > 0 {
                logger::info(
                    LogTag::System,
                    &format!(
                        "✅ Cleared pending partial exits metadata ({} rows)",
                        count
                    ),
                );
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::System,
                &format!("Failed to clear partial exit metadata: {}", e),
            );
        }
    }

    logger::info(
        LogTag::System,
        "✅ Pending verification metadata cleared",
    );

    Ok(())
}

/// Extended reset operation that also clears pending verification metadata
pub fn execute_extended_reset(config: ResetConfig) -> Result<(), String> {
    // First clear pending verifications from database
    if let Err(e) = clear_pending_verifications() {
        logger::error(
            LogTag::System,
            &format!("Failed to clear pending verifications: {}", e),
        );
    }

    // Then proceed with normal reset
    execute_reset(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_config_default() {
        let config = ResetConfig::default();
        assert!(!config.force);
        assert!(!config.targets.is_empty());
    }

    #[test]
    fn test_reset_targets_defined() {
        assert!(RESET_TARGETS.len() > 0);
        assert!(RESET_TARGETS.contains(&"data/positions.db"));
        assert!(RESET_TARGETS.contains(&"data/events.db"));
    }
}
