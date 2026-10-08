// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The copy of a store taken before an upgrade rebuilds it: the latest copy per store, with a manifest naming the core version that took it so the desktop shell can restore it when it rolls that core back.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::errors::DatabaseError;
use crate::paths::BACKUPS_DIRECTORY_NAME;

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

/// The backup file and manifest of `store`: `upgrade-<stem>.db` and
/// `upgrade-<stem>.json` in the `backups` directory beside the store.
pub fn upgrade_backup_paths(store_path: &Path) -> Option<(PathBuf, PathBuf)> {
    let directory = store_path.parent()?.join(BACKUPS_DIRECTORY_NAME);
    let stem = store_path.file_stem()?.to_str()?;
    Some((
        directory.join(format!("upgrade-{stem}.db")),
        directory.join(format!("upgrade-{stem}.json")),
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

    let partial = backup_path.with_extension("db.partial");
    remove_if_present(&partial).map_err(|e| failed(format!("clear {}: {e}", partial.display())))?;
    {
        // SQLite refuses a backup from a connection holding a write transaction, so a
        // read-only connection copies the committed state the upgrade starts from.
        let source = Connection::open_with_flags(store_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| failed(format!("open {} to copy it: {e}", store_path.display())))?;
        let mut destination = Connection::open(&partial)
            .map_err(|e| failed(format!("open {}: {e}", partial.display())))?;
        Backup::new(&source, &mut destination)
            .and_then(|backup| backup.run_to_completion(4096, std::time::Duration::ZERO, None))
            .map_err(|e| failed(format!("copy pages: {e}")))?;
        // A single self-contained file: no WAL or shared-memory sidecars to restore.
        destination
            .pragma_update(None, "journal_mode", "DELETE")
            .map_err(|e| failed(format!("finish {}: {e}", partial.display())))?;
    }
    let check = Connection::open_with_flags(&partial, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|copy| copy.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0)))
        .map_err(|e| failed(format!("check {}: {e}", partial.display())))?;
    if check != "ok" {
        return Err(failed(format!("the copy failed its check: {check}")));
    }

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
    let manifest_partial = manifest_path.with_extension("json.partial");
    fs::write(&manifest_partial, encoded)
        .and_then(|()| fs::rename(&manifest_partial, &manifest_path))
        .map_err(|e| failed(format!("write {}: {e}", manifest_path.display())))?;
    Ok(backup_path)
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
