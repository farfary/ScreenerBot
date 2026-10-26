// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The only code that moves, restores or recreates a store file after a failed open: the header probe, the decision table, the quarantine move, the snapshot restore and the salvage copy.

pub mod policy;

pub use policy::{decide, FailureKind, RecoveryAction};

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::paths::{get_db_with_wal_files, QUARANTINE_DIRECTORY_NAME};

use super::catalog::{SalvageRule, StoreClass, StoreSpec};
use super::configure::configure_connection;
use super::schema::{table_columns, table_columns_in};
use super::WriteTransaction;

/// Schema name the quarantined file is attached under while its rows are salvaged.
const QUARANTINED_SCHEMA: &str = "quarantined";

/// Bytes every SQLite database file starts with.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// Where a quarantined store was moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quarantined {
    /// The directory holding the moved files and `manifest.json`.
    pub directory: PathBuf,
    /// The moved main store file.
    pub store_file: PathBuf,
}

/// What the quarantine directory records about why a store was moved.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuarantineManifest<'a> {
    store: &'a str,
    class: &'static str,
    reason: &'a str,
    core_version: &'static str,
    original_files: Vec<String>,
    quarantined_at_ms: i64,
}

/// Check an existing store file before any connection pool opens it: a file that does
/// not start with the SQLite header, or whose length is not a whole number of its pages,
/// is not a database, and so is an empty file, which SQLite never leaves behind once it
/// has created a store; a schema that cannot be read is classified by its SQLite failure.
/// A missing file passes, since SQLite creates the store in its place.
pub fn probe(path: &Path) -> Result<(), DatabaseError> {
    let operation = format!("probe {}", path.display());
    let length = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(DatabaseError::Io {
                operation,
                message: error.to_string(),
            })
        }
    };
    let header = read_header(path).map_err(|error| DatabaseError::Io {
        operation: operation.clone(),
        message: error.to_string(),
    })?;
    let not_a_database = |message: String| DatabaseError::NotADatabase {
        operation: operation.clone(),
        message,
    };
    let Some(header) = header else {
        return Err(not_a_database(format!(
            "{length} bytes is shorter than a database header"
        )));
    };
    if &header[..16] != SQLITE_HEADER {
        return Err(not_a_database("the file has no SQLite header".to_owned()));
    }
    let page_size = match u16::from_be_bytes([header[16], header[17]]) {
        1 => 65_536,
        size => u64::from(size),
    };
    if page_size < 512 || !page_size.is_power_of_two() || length % page_size != 0 {
        return Err(not_a_database(format!(
            "{length} bytes is not a whole number of {page_size}-byte pages"
        )));
    }

    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| DatabaseError::classify_sqlite_failure(&operation, error))?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation, error))?;
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation, error))?;
    Ok(())
}

/// The first 100 bytes of the file, or `None` when it is shorter.
fn read_header(path: &Path) -> std::io::Result<Option<[u8; 100]>> {
    use std::io::Read;
    let mut header = [0_u8; 100];
    match fs::File::open(path)?.read_exact(&mut header) {
        Ok(()) => Ok(Some(header)),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(error) => Err(error),
    }
}

/// Move the store at `path` and its `-wal` and `-shm` into a new directory of the
/// quarantine directory beside it, with a manifest recording why. Nothing is deleted.
pub fn quarantine(
    spec: &StoreSpec,
    path: &Path,
    reason: &str,
) -> Result<Quarantined, DatabaseError> {
    let root = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(QUARANTINE_DIRECTORY_NAME);
    quarantine_into(&root, spec, path, reason)
}

/// Move the damaged store at `path` to quarantine and put a copy of `snapshot`, a
/// verified snapshot of it, in its place. The damaged file is kept; the snapshot is
/// copied through the backup API and never moved.
pub fn restore(
    spec: &StoreSpec,
    path: &Path,
    snapshot: &Path,
    reason: &str,
) -> Result<Quarantined, DatabaseError> {
    let quarantined = quarantine(spec, path, reason)?;
    super::backup::copy_store(snapshot, path, spec.id)?;
    Ok(quarantined)
}

fn quarantine_into(
    root: &Path,
    spec: &StoreSpec,
    path: &Path,
    reason: &str,
) -> Result<Quarantined, DatabaseError> {
    let io = |operation: String| {
        move |error: std::io::Error| DatabaseError::Io {
            operation,
            message: error.to_string(),
        }
    };
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(spec.id);
    fs::create_dir_all(root).map_err(io(format!("create {}", root.display())))?;
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let directory = root.join(format!("{stem}-{stamp}"));
    fs::create_dir(&directory).map_err(io(format!("create {}", directory.display())))?;

    let moved = move_store_files(path, &directory)?;

    let manifest = QuarantineManifest {
        store: spec.id,
        class: class_name(spec.class),
        reason,
        core_version: env!("CARGO_PKG_VERSION"),
        original_files: moved,
        quarantined_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    let manifest_path = directory.join("manifest.json");
    let encoded = serde_json::to_vec_pretty(&manifest).map_err(|error| DatabaseError::Io {
        operation: format!("encode {}", manifest_path.display()),
        message: error.to_string(),
    })?;
    fs::write(&manifest_path, encoded).map_err(io(format!("write {}", manifest_path.display())))?;

    let store_file = directory.join(path.file_name().unwrap_or_default());
    Ok(Quarantined {
        directory,
        store_file,
    })
}

/// Move the store at `path` and its `-wal` and `-shm` into `directory`, created when
/// missing, and return the original paths of the files moved. Sidecars move before the
/// store file: a crash between moves must never leave an old write-ahead log beside the
/// path the store is recreated at, where SQLite would replay its frames into the new
/// file. A file that is not there is skipped, so a move interrupted by a crash resumes.
pub fn move_store_files(path: &Path, directory: &Path) -> Result<Vec<String>, DatabaseError> {
    let io = |operation: String| {
        move |error: std::io::Error| DatabaseError::Io {
            operation,
            message: error.to_string(),
        }
    };
    fs::create_dir_all(directory).map_err(io(format!("create {}", directory.display())))?;
    let mut moved = Vec::new();
    for file in get_db_with_wal_files(path.to_path_buf()).into_iter().rev() {
        if !file.exists() {
            continue;
        }
        let Some(name) = file.file_name() else {
            continue;
        };
        let target = directory.join(name);
        fs::rename(&file, &target).map_err(io(format!(
            "move {} to {}",
            file.display(),
            target.display()
        )))?;
        moved.push(file.display().to_string());
    }
    Ok(moved)
}

const fn class_name(class: StoreClass) -> &'static str {
    match class {
        StoreClass::Vault => "vault",
        StoreClass::Ledger => "ledger",
        StoreClass::Derived => "derived",
        StoreClass::Cache => "cache",
    }
}

/// Copy the rows of each salvage table that both files still share from the quarantined
/// file into the rebuilt store, keeping any row the rebuilt store already holds. The
/// quarantined file is attached read-only and never written. A table that cannot be read
/// is logged and skipped; the total number of rows copied is returned.
pub fn salvage(spec: &StoreSpec, quarantined: &Path, live: &Path) -> Result<usize, DatabaseError> {
    let operation = |step: &str| format!("salvage {} {step}", spec.id);
    let mut connection = Connection::open(live)
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("open"), error))?;
    configure_connection(&connection, spec.config)
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("configure"), error))?;
    connection
        .execute(
            &format!("ATTACH DATABASE ?1 AS {QUARANTINED_SCHEMA}"),
            [read_only_uri(quarantined)],
        )
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("attach"), error))?;

    let copied = copy_salvage_tables(&mut connection, spec);
    connection
        .execute(&format!("DETACH DATABASE {QUARANTINED_SCHEMA}"), [])
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("detach"), error))?;
    copied
}

fn copy_salvage_tables(
    connection: &mut Connection,
    spec: &StoreSpec,
) -> Result<usize, DatabaseError> {
    let operation = |step: &str| format!("salvage {} {step}", spec.id);
    let transaction = connection
        .write_tx()
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("begin"), error))?;
    let mut total = 0;
    for rule in spec.salvage {
        match copy_table(&transaction, rule) {
            Ok(rows) => total += rows,
            Err(error) => logger::warning(
                LogTag::System,
                &format!("{}: could not salvage {}: {error}", spec.id, rule.table),
            ),
        }
    }
    transaction
        .commit()
        .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("commit"), error))?;
    Ok(total)
}

fn copy_table(connection: &Connection, rule: &SalvageRule) -> rusqlite::Result<usize> {
    let live = table_columns(connection, rule.table)?;
    let quarantined = table_columns_in(connection, QUARANTINED_SCHEMA, rule.table)?;
    let columns: Vec<&str> = live
        .iter()
        .map(|column| column.name.as_str())
        .filter(|name| quarantined.iter().any(|column| column.name == *name))
        .filter(|name| rule.columns.is_none_or(|wanted| wanted.contains(name)))
        .collect();
    if columns.is_empty() {
        return Ok(0);
    }
    let columns = columns.join(", ");
    let filter = rule
        .filter
        .map(|condition| format!(" WHERE {condition}"))
        .unwrap_or_default();
    connection.execute(
        &format!(
            "INSERT OR IGNORE INTO main.{table} ({columns}) SELECT {columns} FROM {QUARANTINED_SCHEMA}.{table}{filter}",
            table = rule.table
        ),
        [],
    )
}

/// A `file:` URI opening `path` read-only. Windows separators become `/` and a drive
/// path gains the leading `/` SQLite expects; every other byte outside the unreserved set
/// is percent-encoded so spaces and `?` or `#` in the path survive.
fn read_only_uri(path: &Path) -> String {
    #[cfg(windows)]
    let path = {
        let mut path = path.to_string_lossy().replace('\\', "/");
        if !path.starts_with('/') {
            path.insert(0, '/');
        }
        path
    };
    #[cfg(not(windows))]
    let path = path.to_string_lossy();
    let mut uri = String::from("file:");
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/:".contains(&byte) {
            uri.push(char::from(byte));
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    uri.push_str("?mode=ro");
    uri
}

#[cfg(test)]
#[path = "recovery_tests.rs"]
mod tests;
