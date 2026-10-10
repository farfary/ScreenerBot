// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Recovery of a cache store that will not open: the file and its sidecars move to the quarantine directory, the store's own opener creates it afresh and the salvage tables are copied back.

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::paths::{get_db_with_wal_files, get_quarantine_directory};

use super::catalog::{SalvageRule, StoreClass, StoreSpec};
use super::configure::configure_connection;
use super::schema::{table_columns, table_columns_in};
use super::WriteTransaction;

/// Schema name the quarantined file is attached under while its rows are salvaged.
const QUARANTINED_SCHEMA: &str = "quarantined";

/// Bytes every SQLite database file starts with.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// The error of a store opener, as recovery needs to see it.
pub trait StoreOpenError: From<DatabaseError> + std::fmt::Display {
    /// The classified storage failure behind this error, when it has one. An error
    /// without one is a failure of the store's content or schema.
    fn database_error(&self) -> Option<&DatabaseError>;
}

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

/// Open a cache store through `open`, its own opener, recovering a store whose file or
/// schema is unusable. Before the opener runs, an existing file is probed for a damaged
/// or foreign header and an unreadable schema, so a broken file is never handed to a
/// connection pool. A content or schema failure, from the probe or the opener, moves the
/// file and its sidecars to the quarantine directory, logs the store and the reason, runs
/// the opener again to create the store afresh and copies the salvage tables back. A
/// failure of the surroundings (a lock, permissions, a full disk, I/O) moves nothing and
/// is returned. Ledger, derived and vault stores are opened without recovery.
pub async fn open_cache_store<T, E, F, Fut>(
    spec: &StoreSpec,
    path: &Path,
    mut open: F,
) -> Result<T, E>
where
    E: StoreOpenError,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    if spec.class != StoreClass::Cache {
        return open().await;
    }
    let reason = match probe(path) {
        Err(error) if error.is_environment_failure() => return Err(E::from(error)),
        Err(error) => error.to_string(),
        Ok(()) => match open().await {
            Ok(store) => return Ok(store),
            Err(error)
                if error
                    .database_error()
                    .is_some_and(DatabaseError::is_environment_failure) =>
            {
                return Err(error)
            }
            Err(error) if !path.exists() => return Err(error),
            Err(error) => error.to_string(),
        },
    };

    let quarantined = quarantine(spec, path, &reason).map_err(|error| {
        logger::error(
            LogTag::System,
            &format!(
                "{} at {} could not be opened ({reason}) and could not be moved aside: {error}",
                spec.id,
                path.display()
            ),
        );
        E::from(error)
    })?;
    logger::warning(
        LogTag::System,
        &format!(
            "{} at {} could not be opened ({reason}); moved it to {} and rebuilding it",
            spec.id,
            path.display(),
            quarantined.directory.display()
        ),
    );

    let store = open().await.map_err(|error| {
        logger::error(
            LogTag::System,
            &format!(
                "{} could not be rebuilt at {}: {error}",
                spec.id,
                path.display()
            ),
        );
        error
    })?;
    if !spec.salvage.is_empty() {
        match salvage(spec, &quarantined.store_file, path) {
            Ok(rows) => logger::info(
                LogTag::System,
                &format!(
                    "{} rebuilt; salvaged {rows} rows from {}",
                    spec.id,
                    quarantined.store_file.display()
                ),
            ),
            Err(error) => logger::warning(
                LogTag::System,
                &format!(
                    "{} rebuilt without salvage from {}: {error}",
                    spec.id,
                    quarantined.store_file.display()
                ),
            ),
        }
    } else {
        logger::info(LogTag::System, &format!("{} rebuilt", spec.id));
    }
    Ok(store)
}

/// Check an existing store file before any connection pool opens it: a file that does
/// not start with the SQLite header, or whose length is not a whole number of its pages,
/// is not a database; a schema that cannot be read is classified by its SQLite failure.
/// A missing or empty file passes, since SQLite creates the store in it.
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
    if length == 0 {
        return Ok(());
    }
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
/// quarantine directory, with a manifest recording why. Nothing is deleted.
pub fn quarantine(
    spec: &StoreSpec,
    path: &Path,
    reason: &str,
) -> Result<Quarantined, DatabaseError> {
    quarantine_into(&get_quarantine_directory(), spec, path, reason)
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

    // Sidecars move before the store file. A crash between moves must never
    // leave an old write-ahead log beside the path the store is recreated at,
    // where SQLite would replay its frames into the new file.
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
mod tests {
    use super::*;
    use crate::database::catalog::{OHLCVS, POOLS};

    fn store_with(path: &Path, sql: &str) {
        let connection = Connection::open(path).expect("create store");
        configure_connection(&connection, POOLS.config).expect("configure store");
        connection.execute_batch(sql).expect("seed store");
    }

    #[test]
    fn probe_classifies_foreign_truncated_and_healthy_files() {
        let directory = tempfile::tempdir().expect("temp dir");
        let missing = directory.path().join("missing.db");
        assert!(probe(&missing).is_ok());

        let empty = directory.path().join("empty.db");
        fs::write(&empty, b"").expect("write empty file");
        assert!(probe(&empty).is_ok());

        let garbage = directory.path().join("garbage.db");
        fs::write(&garbage, vec![0x5a_u8; 8192]).expect("write garbage");
        assert!(matches!(
            probe(&garbage),
            Err(DatabaseError::NotADatabase { .. })
        ));

        let short = directory.path().join("short.db");
        fs::write(&short, b"SQLite format 3\0").expect("write short file");
        assert!(matches!(
            probe(&short),
            Err(DatabaseError::NotADatabase { .. })
        ));

        let healthy = directory.path().join("healthy.db");
        store_with(
            &healthy,
            "CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);",
        );
        assert!(probe(&healthy).is_ok());

        let truncated = directory.path().join("truncated.db");
        let mut bytes = fs::read(&healthy).expect("read healthy store");
        bytes.truncate(bytes.len() - 1);
        fs::write(&truncated, bytes).expect("write truncated copy");
        assert!(matches!(
            probe(&truncated),
            Err(DatabaseError::NotADatabase { .. })
        ));
    }

    #[test]
    fn quarantine_moves_the_store_and_sidecars_and_writes_a_manifest() {
        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("pools.db");
        // The open connection keeps the WAL and shared-memory files on disk.
        let open = Connection::open(&path).expect("create store");
        configure_connection(&open, POOLS.config).expect("configure store");
        open.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1);")
            .expect("seed store");
        let sidecars: Vec<PathBuf> = get_db_with_wal_files(path.clone())
            .into_iter()
            .filter(|file| file.exists())
            .collect();
        assert!(
            sidecars.len() > 1,
            "WAL sidecars exist while the store is open"
        );
        drop(open);
        let sidecars: Vec<PathBuf> = get_db_with_wal_files(path.clone())
            .into_iter()
            .filter(|file| file.exists())
            .collect();

        let root = directory.path().join("quarantine");
        let moved = quarantine_into(&root, &POOLS, &path, "file is not a database")
            .expect("quarantine store");

        assert!(moved.directory.starts_with(&root));
        assert_eq!(moved.store_file, moved.directory.join("pools.db"));
        for file in &sidecars {
            assert!(!file.exists(), "{} left behind", file.display());
            assert!(moved.directory.join(file.file_name().unwrap()).exists());
        }
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(moved.directory.join("manifest.json")).expect("read manifest"),
        )
        .expect("decode manifest");
        assert_eq!(manifest["store"], "pools");
        assert_eq!(manifest["class"], "cache");
        assert_eq!(manifest["reason"], "file is not a database");
        assert_eq!(
            manifest["originalFiles"].as_array().map(Vec::len),
            Some(sidecars.len())
        );
        let rows: i64 = Connection::open(&moved.store_file)
            .and_then(|copy| copy.query_row("SELECT count(*) FROM t", [], |row| row.get(0)))
            .expect("read quarantined store");
        assert_eq!(rows, 1, "the quarantined store keeps its rows");
    }

    #[test]
    fn salvage_copies_only_the_listed_rows_and_columns_and_never_writes_the_source() {
        let directory = tempfile::tempdir().expect("temp dir with a space");
        let quarantined_dir = directory.path().join("quarantine dir");
        fs::create_dir(&quarantined_dir).expect("create quarantine dir");
        let quarantined = quarantined_dir.join("ohlcvs.db");
        store_with(
            &quarantined,
            "CREATE TABLE ohlcv_monitor_config (chain_id TEXT, mint TEXT, priority TEXT, source TEXT, backfill_1m_complete INTEGER DEFAULT 0, PRIMARY KEY (chain_id, mint));
             INSERT INTO ohlcv_monitor_config VALUES ('solana', 'manual-a', 'high', 'manual', 1), ('solana', 'auto-b', 'low', 'auto', 1), ('solana', 'kept', 'low', 'manual', 1);",
        );
        let source_before = fs::read(&quarantined).expect("read source");
        let live = directory.path().join("live-ohlcvs.db");
        store_with(
            &live,
            "CREATE TABLE ohlcv_monitor_config (chain_id TEXT, mint TEXT, priority TEXT, source TEXT, backfill_1m_complete INTEGER DEFAULT 0, PRIMARY KEY (chain_id, mint));
             INSERT INTO ohlcv_monitor_config VALUES ('solana', 'kept', 'critical', 'manual', 0);",
        );

        let rows = salvage(&OHLCVS, &quarantined, &live).expect("salvage");

        assert_eq!(rows, 1);
        let connection = Connection::open(&live).expect("open live store");
        let monitors: Vec<(String, String, i64)> = connection
            .prepare("SELECT mint, priority, backfill_1m_complete FROM ohlcv_monitor_config ORDER BY mint")
            .expect("prepare")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("query")
            .collect::<rusqlite::Result<_>>()
            .expect("decode");
        assert_eq!(
            monitors,
            vec![
                ("kept".to_owned(), "critical".to_owned(), 0),
                ("manual-a".to_owned(), "high".to_owned(), 0),
            ],
            "manual monitors salvaged without backfill progress; live rows kept"
        );
        assert_eq!(
            fs::read(&quarantined).expect("reread source"),
            source_before
        );
    }

    #[test]
    fn salvage_skips_an_unreadable_source() {
        let directory = tempfile::tempdir().expect("temp dir");
        let quarantined = directory.path().join("garbage.db");
        fs::write(&quarantined, vec![0x5a_u8; 8192]).expect("write garbage");
        let live = directory.path().join("ohlcvs.db");
        store_with(
            &live,
            "CREATE TABLE ohlcv_monitor_config (chain_id TEXT, mint TEXT, source TEXT, PRIMARY KEY (chain_id, mint));",
        );

        let outcome = salvage(&OHLCVS, &quarantined, &live);

        assert!(matches!(
            outcome,
            Ok(0) | Err(DatabaseError::NotADatabase { .. })
        ));
    }

    #[test]
    fn read_only_uri_escapes_every_reserved_byte() {
        assert_eq!(
            read_only_uri(Path::new("/a b/c?d#e%f.db")),
            "file:/a%20b/c%3Fd%23e%25f.db?mode=ro"
        );
        #[cfg(windows)]
        assert_eq!(
            read_only_uri(Path::new("C:\\Users\\a b\\ohlcvs.db")),
            "file:/C:/Users/a%20b/ohlcvs.db?mode=ro"
        );
    }
}
