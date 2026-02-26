//! Types for the reset module.

use crate::paths;
use std::path::PathBuf;

/// Configuration for reset operation
#[derive(Debug, Clone)]
pub struct ResetConfig {
    pub force: bool,
    pub targets: Vec<PathBuf>,
}

impl Default for ResetConfig {
    fn default() -> Self {
        Self {
            force: false,
            targets: get_reset_targets(),
        }
    }
}

/// Get list of files and directories to be removed during reset
pub(super) fn get_reset_targets() -> Vec<PathBuf> {
    let mut targets = Vec::new();

    // Database files with WAL and SHM
    targets.extend(paths::get_db_with_wal_files(paths::get_positions_db_path()));
    targets.extend(paths::get_db_with_wal_files(paths::get_events_db_path()));

    // Cache files
    targets.push(paths::get_rpc_stats_path());
    targets.push(paths::get_ata_failed_cache_path());

    targets
}
