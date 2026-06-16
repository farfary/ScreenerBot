//! Wallet-data recovery — safe backup-then-clean of the wallet-scoped databases.
//!
//! When the configured wallet no longer matches the wallet recorded in local
//! history (see [`crate::wallets::validation`]), the previous wallet's local
//! trade/position/history data must be cleared before the new wallet can start.
//! This module performs that reset **safely**: every affected database (and its
//! SQLite `-wal`/`-shm` sidecars) is copied into a timestamped backup directory
//! before the originals are removed, so nothing is ever lost irrecoverably.
//!
//! No on-chain funds are touched — only this computer's local databases.

use std::fs;
use std::path::PathBuf;

use crate::logger::{self, LogTag};
use crate::paths;

/// The wallet-scoped databases that must be cleared on a wallet change.
///
/// These are the exact databases [`crate::wallets::validation`] checks for a
/// stored-vs-current wallet mismatch.
fn wallet_scoped_db_paths() -> Vec<PathBuf> {
    vec![
        paths::get_transactions_db_path(),
        paths::get_positions_db_path(),
        paths::get_wallet_db_path(),
    ]
}

/// Back up the wallet-scoped databases, then delete the originals.
///
/// Returns the backup directory that was created (under `data/backups/`) so the
/// caller can report it to the user. Backing up is best-effort per file but the
/// delete only proceeds for files that were successfully backed up, so a partial
/// failure never destroys un-backed-up data.
pub fn backup_and_clean_wallet_data() -> Result<PathBuf, String> {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup_dir = paths::get_data_directory()
        .join("backups")
        .join(format!("wallet-reset-{timestamp}"));

    fs::create_dir_all(&backup_dir).map_err(|e| {
        format!(
            "Failed to create backup directory {}: {e}",
            backup_dir.display()
        )
    })?;

    logger::info(
        LogTag::System,
        &format!(
            "Wallet data reset: backing up wallet-scoped databases to {}",
            backup_dir.display()
        ),
    );

    let mut backed_up = 0usize;
    let mut removed = 0usize;

    for db_path in wallet_scoped_db_paths() {
        for file in paths::get_db_with_wal_files(db_path) {
            if !file.exists() {
                continue;
            }

            let file_name = match file.file_name() {
                Some(name) => name,
                None => continue,
            };
            let dest = backup_dir.join(file_name);

            // Back up first; only delete if the copy succeeded.
            match fs::copy(&file, &dest) {
                Ok(_) => {
                    backed_up += 1;
                    match fs::remove_file(&file) {
                        Ok(_) => removed += 1,
                        Err(e) => {
                            logger::warning(
                                LogTag::System,
                                &format!(
                                    "Backed up but could not remove {} during wallet reset: {e}",
                                    file.display()
                                ),
                            );
                        }
                    }
                }
                Err(e) => {
                    logger::warning(
                        LogTag::System,
                        &format!(
                            "Skipping {} during wallet reset (backup copy failed): {e}",
                            file.display()
                        ),
                    );
                }
            }
        }
    }

    logger::info(
        LogTag::System,
        &format!(
            "Wallet data reset complete: {backed_up} file(s) backed up, {removed} removed. Backup: {}",
            backup_dir.display()
        ),
    );

    Ok(backup_dir)
}
