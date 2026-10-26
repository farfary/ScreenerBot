// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Every backup of a store: the backup-API copy taken before an upgrade rebuilds it, with a manifest naming the core version that took it so the desktop shell can restore it when it rolls that core back, the periodic snapshots of ledger and vault stores, and the wallet-reset backups that receive the wallet-scoped stores when they are reset or the trading wallet is replaced.

pub mod snapshot;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::paths::{BACKUPS_DIRECTORY_NAME, WALLET_RESET_DIRECTORY_PREFIX};

use super::catalog;

/// What the shell needs to decide whether a backup belongs to the core it rolls back.
/// The field names are read by `electron/src/upgrade_backups.js`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeBackupManifest {
    /// File name of the store beside the backups directory, such as `positions.db`.
    pub store: String,
    /// Version of the core whose upgrade took the backup.
    pub core_version: String,
    /// Schema version the store held before the upgrade, when it recorded one.
    pub from_schema_version: Option<String>,
    /// When the backup was completed, in Unix milliseconds.
    pub created_at_ms: i64,
}

/// Name prefix of every upgrade backup file and manifest.
pub const UPGRADE_BACKUP_PREFIX: &str = "upgrade-";

/// The backup file and manifest of `store`: `upgrade-<stem>.db` and
/// `upgrade-<stem>.json` in the `backups` directory beside the store.
pub fn upgrade_backup_paths(store_path: &Path) -> Option<(PathBuf, PathBuf)> {
    let directory = store_path.parent()?.join(BACKUPS_DIRECTORY_NAME);
    let stem = store_path.file_stem()?.to_str()?;
    Some((
        directory.join(format!("{UPGRADE_BACKUP_PREFIX}{stem}.db")),
        directory.join(format!("{UPGRADE_BACKUP_PREFIX}{stem}.json")),
    ))
}

/// Copy the store at `store_path` into its upgrade backup, replacing the previous one.
///
/// Call it inside the upgrade's IMMEDIATE transaction before its first write: the
/// write lock keeps every other connection from changing the file, so the committed
/// state the copy reads is exactly the state the upgrade starts from. The copy is checked before it replaces the
/// previous backup, and the manifest is written last, so a manifest always describes
/// the file beside it.
pub fn back_up_before_upgrade(
    store_path: &Path,
    from_schema_version: Option<&str>,
) -> Result<PathBuf, DatabaseError> {
    let store = store_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let failed = |message: String| DatabaseError::Backup {
        store: store.clone(),
        message,
    };
    let (backup_path, manifest_path) = upgrade_backup_paths(store_path)
        .ok_or_else(|| failed(format!("no backup location for {}", store_path.display())))?;
    let directory = backup_path
        .parent()
        .ok_or_else(|| failed(format!("no backup directory for {}", backup_path.display())))?;
    fs::create_dir_all(directory)
        .map_err(|e| failed(format!("create {}: {e}", directory.display())))?;

    let partial = copy_to_partial(store_path, &backup_path, &store)?;
    remove_if_present(&manifest_path)
        .map_err(|e| failed(format!("remove {}: {e}", manifest_path.display())))?;
    fs::rename(&partial, &backup_path)
        .map_err(|e| failed(format!("move into {}: {e}", backup_path.display())))?;

    let manifest = UpgradeBackupManifest {
        store: store.clone(),
        core_version: env!("CARGO_PKG_VERSION").to_owned(),
        from_schema_version: from_schema_version.map(str::to_owned),
        created_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    let encoded = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| failed(format!("encode manifest: {e}")))?;
    crate::utils::atomic_file::replace(&manifest_path, &encoded, 0o600)
        .map_err(|e| failed(format!("write {}: {e}", manifest_path.display())))?;
    Ok(backup_path)
}

/// Why the wallet-scoped stores were moved into a wallet-reset backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WalletResetReason {
    /// The wallet data reset requested at launch.
    WalletReset,
    /// The trading wallet was replaced by another key.
    WalletReplaced,
}

/// What a wallet-reset backup directory records about the stores it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletResetManifest {
    pub reason: WalletResetReason,
    /// The chain whose trading wallet the stores belonged to.
    pub chain: String,
    /// The trading wallet the stores belonged to, when known.
    pub previous_wallet: Option<String>,
    /// The trading wallet that replaced it, for a replacement.
    pub new_wallet: Option<String>,
    /// Version of the core that moved the stores.
    pub core_version: String,
    /// When the stores were moved, in Unix milliseconds.
    pub created_at_ms: i64,
    /// The store files the directory holds.
    pub files: Vec<String>,
    /// Open positions the moved positions store holds, by wallet address. Empty when
    /// it holds none or could not be read.
    #[serde(default)]
    pub moved_open_positions: BTreeMap<String, usize>,
}

/// Name of the manifest inside a wallet-reset backup directory.
const WALLET_RESET_MANIFEST_NAME: &str = "manifest.json";

/// A new wallet-reset backup directory in `backups`:
/// `wallet-reset-<YYYYMMDDTHHMMSSZ>-<label>`.
pub fn wallet_reset_directory(label: &str) -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    crate::paths::get_backups_directory()
        .join(format!("{WALLET_RESET_DIRECTORY_PREFIX}{stamp}-{label}"))
}

/// Move every file of the wallet-scoped stores into `directory` and write `manifest`
/// there atomically, listing every store file the directory holds and the open
/// positions of each wallet the moved positions store holds. Refuses before moving
/// anything while a wallet-scoped store is open in this process: its pool would keep
/// writing to the moved file. Files already moved are skipped, so an interrupted move
/// resumes; a file is never moved onto an existing one, so nothing is deleted.
pub fn move_wallet_stores(
    directory: &Path,
    mut manifest: WalletResetManifest,
) -> Result<WalletResetManifest, DatabaseError> {
    let paths: Vec<PathBuf> = catalog::wallet_scoped_stores()
        .flat_map(|(spec, _)| spec.paths())
        .collect();
    if let Some(open) = paths.iter().find(|path| super::open::is_pooled(path)) {
        return Err(DatabaseError::Busy {
            operation: "move wallet stores".to_owned(),
            message: format!("{} is open in this process", open.display()),
        });
    }
    for path in &paths {
        super::recovery::move_store_files(path, directory)?;
    }
    fs::create_dir_all(directory).map_err(|error| DatabaseError::Io {
        operation: format!("create {}", directory.display()),
        message: error.to_string(),
    })?;
    manifest.files = backup_store_files(directory)?;
    manifest.moved_open_positions = moved_open_positions(directory);

    let manifest_path = directory.join(WALLET_RESET_MANIFEST_NAME);
    let encoded = serde_json::to_vec_pretty(&manifest).map_err(|error| DatabaseError::Io {
        operation: format!("encode {}", manifest_path.display()),
        message: error.to_string(),
    })?;
    crate::utils::atomic_file::replace(&manifest_path, &encoded, 0o600).map_err(|error| {
        DatabaseError::Io {
            operation: format!("write {}", manifest_path.display()),
            message: error.to_string(),
        }
    })?;
    Ok(manifest)
}

/// Move back every wallet-scoped store file a failed [`move_wallet_stores`] left in
/// `directory`: each file whose original path is free and whose name `directory` holds,
/// the store file before its sidecars. A file is never moved onto an existing one, so a
/// file re-created at the original path stays and its moved copy stays in `directory`.
/// Returns the original paths of the files moved back.
pub fn restore_wallet_stores(directory: &Path) -> Result<Vec<String>, DatabaseError> {
    let mut restored = Vec::new();
    for (spec, _) in catalog::wallet_scoped_stores() {
        for path in spec.paths() {
            for original in crate::paths::get_db_with_wal_files(path) {
                let Some(name) = original.file_name() else {
                    continue;
                };
                let moved = directory.join(name);
                if original.symlink_metadata().is_ok() || moved.symlink_metadata().is_err() {
                    continue;
                }
                super::recovery::move_file_without_replacing(&moved, &original)?;
                restored.push(original.display().to_string());
            }
        }
    }
    Ok(restored)
}

/// The open positions of each wallet in the positions store moved into `directory`,
/// read from the moved copy. A copy that cannot be read is logged and counts nothing.
fn moved_open_positions(directory: &Path) -> BTreeMap<String, usize> {
    let Some(name) = crate::paths::get_positions_db_path()
        .file_name()
        .map(ToOwned::to_owned)
    else {
        return BTreeMap::new();
    };
    match crate::positions::database::open_positions_by_wallet_in_file(&directory.join(name)) {
        Ok(counts) => counts,
        Err(error) => {
            logger::warning(
                LogTag::System,
                &format!(
                    "Could not count the open positions moved into {}: {error}",
                    directory.display()
                ),
            );
            BTreeMap::new()
        }
    }
}

/// The names of the files in a wallet-reset backup directory other than its manifest,
/// sorted.
fn backup_store_files(directory: &Path) -> Result<Vec<String>, DatabaseError> {
    let entries = fs::read_dir(directory).map_err(|error| DatabaseError::Io {
        operation: format!("list {}", directory.display()),
        message: error.to_string(),
    })?;
    let mut files: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name != WALLET_RESET_MANIFEST_NAME && !name.starts_with('.'))
        .collect();
    files.sort();
    Ok(files)
}

/// Every wallet-reset backup directory in `backups` with its manifest, newest first. A
/// directory without a readable manifest (a release that wrote none, or a move that
/// stopped before its manifest) is logged and left out.
pub fn wallet_reset_backups() -> Vec<(PathBuf, WalletResetManifest)> {
    let Ok(entries) = fs::read_dir(crate::paths::get_backups_directory()) else {
        return Vec::new();
    };
    let mut backups: Vec<(PathBuf, WalletResetManifest)> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(WALLET_RESET_DIRECTORY_PREFIX)
        })
        .filter_map(|entry| {
            let directory = entry.path();
            let manifest_path = directory.join(WALLET_RESET_MANIFEST_NAME);
            let decoded = fs::read(&manifest_path)
                .map_err(|error| error.to_string())
                .and_then(|bytes| {
                    serde_json::from_slice::<WalletResetManifest>(&bytes)
                        .map_err(|error| error.to_string())
                });
            match decoded {
                Ok(manifest) => Some((directory, manifest)),
                Err(error) => {
                    logger::debug(
                        LogTag::System,
                        &format!("{} has no readable manifest: {error}", directory.display()),
                    );
                    None
                }
            }
        })
        .collect();
    backups.sort_by(|a, b| b.1.created_at_ms.cmp(&a.1.created_at_ms));
    backups
}

/// Copy the committed state of the store at `source` into `destination` through the
/// SQLite backup API, so pages still in the source's write-ahead log are included. The
/// copy is written beside `destination` as a partial file, made self-contained (no WAL
/// or shared-memory sidecars), checked with `quick_check` and only then renamed into
/// place, so `destination` is either the previous file or a complete, checked copy.
pub fn copy_store(source: &Path, destination: &Path, store: &str) -> Result<(), DatabaseError> {
    let partial = copy_to_partial(source, destination, store)?;
    fs::rename(&partial, destination).map_err(|e| DatabaseError::Backup {
        store: store.to_owned(),
        message: format!("move into {}: {e}", destination.display()),
    })
}

/// The checked copy of [`copy_store`], left as a partial file beside `destination`.
fn copy_to_partial(
    source: &Path,
    destination: &Path,
    store: &str,
) -> Result<PathBuf, DatabaseError> {
    let failed = |message: String| DatabaseError::Backup {
        store: store.to_owned(),
        message,
    };
    let partial = destination.with_extension("db.partial");
    remove_if_present(&partial).map_err(|e| failed(format!("clear {}: {e}", partial.display())))?;
    {
        // SQLite refuses a backup from a connection holding a write transaction, so a
        // read-only connection copies the committed state.
        let source = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| failed(format!("open {} to copy it: {e}", source.display())))?;
        let mut copy = Connection::open(&partial)
            .map_err(|e| failed(format!("open {}: {e}", partial.display())))?;
        // One step reads every page under one read snapshot, so a concurrent writer
        // never restarts the copy.
        let step = Backup::new(&source, &mut copy)
            .and_then(|backup| backup.step(-1))
            .map_err(|e| failed(format!("copy pages: {e}")))?;
        if step != StepResult::Done {
            return Err(failed(format!(
                "copy pages: the source was locked ({step:?})"
            )));
        }
        copy.pragma_update(None, "journal_mode", "DELETE")
            .map_err(|e| failed(format!("finish {}: {e}", partial.display())))?;
    }
    let check =
        quick_check(&partial).map_err(|e| failed(format!("check {}: {e}", partial.display())))?;
    if check != "ok" {
        return Err(failed(format!("the copy failed its check: {check}")));
    }
    Ok(partial)
}

/// The first line of `PRAGMA quick_check` on the file at `path`, opened read-only:
/// `ok` for a sound file.
pub fn quick_check(path: &Path) -> rusqlite::Result<String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|copy| copy.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0)))
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{configure_connection, WriteTransaction, POSITIONS_DB};

    fn store(directory: &Path, rows: i64) -> (PathBuf, Connection) {
        let path = directory.join("positions.db");
        let conn = Connection::open(&path).unwrap();
        configure_connection(&conn, POSITIONS_DB).unwrap();
        conn.execute_batch("CREATE TABLE IF NOT EXISTS t (v INTEGER NOT NULL)")
            .unwrap();
        for v in 0..rows {
            conn.execute("INSERT INTO t (v) VALUES (?1)", [v]).unwrap();
        }
        (path, conn)
    }

    fn backed_up_rows(path: &Path) -> i64 {
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap()
            .query_row("SELECT COUNT(*) FROM t", [], |row| row.get(0))
            .unwrap()
    }

    /// The backup holds the store as the upgrade found it, including pages still in
    /// the WAL, even though it is taken from inside the upgrade's own transaction, and
    /// the store itself is left unchanged.
    #[test]
    fn a_backup_inside_the_upgrade_transaction_copies_the_store_as_found() {
        let directory = tempfile::tempdir().unwrap();
        let (path, mut conn) = store(directory.path(), 3);

        let tx = conn.write_tx().unwrap();
        let backup = back_up_before_upgrade(&path, Some("5")).unwrap();
        tx.execute("INSERT INTO t (v) VALUES (99)", []).unwrap();
        tx.commit().unwrap();

        assert_eq!(
            backup,
            directory.path().join("backups/upgrade-positions.db")
        );
        assert_eq!(backed_up_rows(&backup), 3);
        let manifest: UpgradeBackupManifest = serde_json::from_slice(
            &fs::read(directory.path().join("backups/upgrade-positions.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest.store, "positions.db");
        assert_eq!(manifest.core_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.from_schema_version.as_deref(), Some("5"));
        assert!(!directory
            .path()
            .join("backups/upgrade-positions.db.partial")
            .exists());
    }

    /// Each store keeps only its latest backup.
    #[test]
    fn a_later_backup_replaces_the_previous_one() {
        let directory = tempfile::tempdir().unwrap();
        let (path, conn) = store(directory.path(), 1);
        back_up_before_upgrade(&path, None).unwrap();
        conn.execute("INSERT INTO t (v) VALUES (2)", []).unwrap();

        let backup = back_up_before_upgrade(&path, Some("6")).unwrap();

        assert_eq!(backed_up_rows(&backup), 2);
        let entries = fs::read_dir(directory.path().join("backups"))
            .unwrap()
            .count();
        assert_eq!(entries, 2, "one backup and one manifest per store");
    }
}
