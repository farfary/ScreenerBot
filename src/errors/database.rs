// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Database-related error classifications.
//!
//! Note: Keep errors `Clone` by storing messages as strings (do not store raw rusqlite errors).

#[derive(Debug, Clone, thiserror::Error)]
pub enum DatabaseError {
    #[error("database connection error: {message}")]
    Connection { message: String },
    #[error("sqlite error: {message}")]
    Sqlite { message: String },
    #[error("database query error (op={operation}): {message}")]
    Query { operation: String, message: String },
    /// Another connection held the SQLite lock past `busy_timeout` (`SQLITE_BUSY` or
    /// `SQLITE_LOCKED`). Nothing was written, and the same statement can succeed later.
    #[error("database busy (op={operation}): {message}")]
    Busy { operation: String, message: String },
    /// The copy of a store taken before an upgrade rebuilds it could not be written, so
    /// the upgrade did not start.
    #[error("could not back up {store} before its upgrade: {message}")]
    Backup { store: String, message: String },
    /// The store file is damaged (`SQLITE_CORRUPT`).
    #[error("database file is damaged (op={operation}): {message}")]
    Corrupt { operation: String, message: String },
    /// The file is not an SQLite database (`SQLITE_NOTADB`).
    #[error("file is not a database (op={operation}): {message}")]
    NotADatabase { operation: String, message: String },
    /// The disk holding the store is full (`SQLITE_FULL`).
    #[error("disk is full (op={operation}): {message}")]
    DiskFull { operation: String, message: String },
    /// The store or its directory cannot be written (`SQLITE_READONLY`, `SQLITE_PERM`).
    #[error("database is read-only (op={operation}): {message}")]
    ReadOnly { operation: String, message: String },
    /// The store file cannot be opened (`SQLITE_CANTOPEN`).
    #[error("database cannot be opened (op={operation}): {message}")]
    CannotOpen { operation: String, message: String },
    /// The operating system reported an I/O failure (`SQLITE_IOERR`).
    #[error("database I/O failure (op={operation}): {message}")]
    Io { operation: String, message: String },
    /// A renamed table or column exists under both its legacy and its canonical name,
    /// and no merge rule covers it, so the open was refused before any write.
    #[error("{store} holds both {object}")]
    SchemaConflict { store: String, object: String },
    /// The store was last written by a newer core whose schema this core cannot read:
    /// its recorded oldest compatible reader is newer than the version this core
    /// supports. The open was refused before any write.
    #[error("{store} is at schema version {version} written by {written_by}; it needs a reader of version {min_reader_version} or newer and this core supports {supported_version}")]
    StoreTooNew {
        store: String,
        version: i64,
        min_reader_version: i64,
        supported_version: i64,
        written_by: String,
    },
    /// A schema step of the store's open failed for a reason of the store's own content.
    /// The open's transaction rolled back, so the store is unchanged.
    #[error("{store} schema step failed: {detail}")]
    Migration { store: String, detail: String },
    /// A store file was to be moved onto a path that already holds a file. Nothing was
    /// moved, so neither file is lost.
    #[error("cannot move {source_path} to {target_path}: the target already exists")]
    MoveTargetExists {
        source_path: String,
        target_path: String,
    },
}

impl DatabaseError {
    /// Maps a captured rusqlite failure onto the database vocabulary by its SQLite result
    /// code: lock contention, damage, a file that is not a database, a full disk, a
    /// read-only or unopenable file and an I/O failure each keep their class; every other
    /// failure is a query error.
    pub fn classify_sqlite_failure(operation: &str, error: rusqlite::Error) -> Self {
        let operation = operation.to_owned();
        let message = error.to_string();
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
                DatabaseError::Busy { operation, message }
            }
            Some(rusqlite::ErrorCode::DatabaseCorrupt) => {
                DatabaseError::Corrupt { operation, message }
            }
            Some(rusqlite::ErrorCode::NotADatabase) => {
                DatabaseError::NotADatabase { operation, message }
            }
            Some(rusqlite::ErrorCode::DiskFull) => DatabaseError::DiskFull { operation, message },
            Some(rusqlite::ErrorCode::ReadOnly | rusqlite::ErrorCode::PermissionDenied) => {
                DatabaseError::ReadOnly { operation, message }
            }
            Some(rusqlite::ErrorCode::CannotOpen) => {
                DatabaseError::CannotOpen { operation, message }
            }
            Some(rusqlite::ErrorCode::SystemIoFailure) => DatabaseError::Io { operation, message },
            _ => DatabaseError::Query { operation, message },
        }
    }

    /// Whether the failure lies in the store's surroundings (a lock, permissions, the
    /// disk, the operating system or the connection pool) rather than in its content.
    /// Such a failure never justifies moving or rebuilding the store file.
    pub fn is_environment_failure(&self) -> bool {
        matches!(
            self,
            DatabaseError::Connection { .. }
                | DatabaseError::Busy { .. }
                | DatabaseError::DiskFull { .. }
                | DatabaseError::ReadOnly { .. }
                | DatabaseError::CannotOpen { .. }
                | DatabaseError::Io { .. }
        )
    }
}

impl From<rusqlite::Error> for DatabaseError {
    /// A failure converted without naming its operation keeps its storage class; only the
    /// unclassified remainder stays a generic SQLite error.
    fn from(err: rusqlite::Error) -> Self {
        match DatabaseError::classify_sqlite_failure("sqlite statement", err) {
            DatabaseError::Query { message, .. } => DatabaseError::Sqlite { message },
            classified => classified,
        }
    }
}

impl From<r2d2::Error> for DatabaseError {
    fn from(err: r2d2::Error) -> Self {
        DatabaseError::Connection {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::WriteTransaction;
    use crate::errors::ErrorClass;
    use rusqlite::Connection;

    /// A writer that cannot take the lock within `busy_timeout` is contention and must be
    /// retryable; a malformed statement is a query error and must not be.
    #[test]
    fn lock_contention_is_busy_and_other_failures_are_queries() {
        let dir = std::env::temp_dir().join(format!("sb_classify_sqlite_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("classify.db");
        let _ = std::fs::remove_file(&path);

        let mut holder = Connection::open(&path).expect("open holder");
        holder
            .pragma_update(None, "journal_mode", "WAL")
            .expect("WAL");
        holder
            .execute("CREATE TABLE t (k INTEGER PRIMARY KEY)", [])
            .expect("create table");
        let mut contender = Connection::open(&path).expect("open contender");
        contender
            .pragma_update(None, "busy_timeout", 0)
            .expect("busy_timeout");

        let held = holder.write_tx().expect("holder begins");
        let failure = contender
            .write_tx()
            .err()
            .expect("a second IMMEDIATE transaction must not begin");
        let busy = DatabaseError::classify_sqlite_failure("contend", failure);
        assert!(
            matches!(busy, DatabaseError::Busy { .. }),
            "lock contention classified as {busy:?}"
        );
        assert!(busy.is_retryable());
        drop(held);

        let syntax = contender
            .execute("NOT A STATEMENT", [])
            .expect_err("malformed SQL must fail");
        let query = DatabaseError::classify_sqlite_failure("malformed", syntax);
        assert!(
            matches!(query, DatabaseError::Query { .. }),
            "malformed SQL classified as {query:?}"
        );
        assert!(!query.is_retryable());

        let _ = std::fs::remove_file(&path);
    }

    fn sqlite_failure(code: std::os::raw::c_int) -> rusqlite::Error {
        rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None)
    }

    /// Every storage-level result code keeps its class through both the named
    /// classification and the plain conversion, and only environment classes are
    /// reported as environment failures.
    #[test]
    fn storage_result_codes_keep_their_class() {
        type Expect = fn(&DatabaseError) -> bool;
        let cases: [(std::os::raw::c_int, Expect, bool); 9] = [
            (
                rusqlite::ffi::SQLITE_BUSY,
                |e| matches!(e, DatabaseError::Busy { .. }),
                true,
            ),
            (
                rusqlite::ffi::SQLITE_LOCKED,
                |e| matches!(e, DatabaseError::Busy { .. }),
                true,
            ),
            (
                rusqlite::ffi::SQLITE_CORRUPT,
                |e| matches!(e, DatabaseError::Corrupt { .. }),
                false,
            ),
            (
                rusqlite::ffi::SQLITE_NOTADB,
                |e| matches!(e, DatabaseError::NotADatabase { .. }),
                false,
            ),
            (
                rusqlite::ffi::SQLITE_FULL,
                |e| matches!(e, DatabaseError::DiskFull { .. }),
                true,
            ),
            (
                rusqlite::ffi::SQLITE_READONLY,
                |e| matches!(e, DatabaseError::ReadOnly { .. }),
                true,
            ),
            (
                rusqlite::ffi::SQLITE_PERM,
                |e| matches!(e, DatabaseError::ReadOnly { .. }),
                true,
            ),
            (
                rusqlite::ffi::SQLITE_CANTOPEN,
                |e| matches!(e, DatabaseError::CannotOpen { .. }),
                true,
            ),
            (
                rusqlite::ffi::SQLITE_IOERR,
                |e| matches!(e, DatabaseError::Io { .. }),
                true,
            ),
        ];
        for (code, expect, environment) in cases {
            let named = DatabaseError::classify_sqlite_failure("probe", sqlite_failure(code));
            assert!(expect(&named), "code {code} classified as {named:?}");
            assert_eq!(named.is_environment_failure(), environment, "{named:?}");
            let converted = DatabaseError::from(sqlite_failure(code));
            assert!(expect(&converted), "code {code} converted to {converted:?}");
        }

        let constraint = DatabaseError::from(sqlite_failure(rusqlite::ffi::SQLITE_CONSTRAINT));
        assert!(
            matches!(constraint, DatabaseError::Sqlite { .. }),
            "{constraint:?}"
        );
        assert!(!constraint.is_environment_failure());
        let conflict = DatabaseError::SchemaConflict {
            store: "ohlcvs.db".to_owned(),
            object: "t.a and t.b".to_owned(),
        };
        assert!(!conflict.is_environment_failure());
    }

    /// A file that is not SQLite fails its first read as `NotADatabase`, and a store
    /// opened read-only fails its first write as `ReadOnly`.
    #[test]
    fn real_files_classify_as_not_a_database_and_read_only() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let garbage = dir.path().join("garbage.db");
        std::fs::write(&garbage, vec![0x5a_u8; 8192]).expect("write garbage file");
        let connection = Connection::open(&garbage).expect("open garbage file");
        let failure = connection
            .query_row("SELECT COUNT(*) FROM sqlite_master", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect_err("garbage must not read as a database");
        let classified = DatabaseError::classify_sqlite_failure("read schema", failure);
        assert!(
            matches!(classified, DatabaseError::NotADatabase { .. }),
            "{classified:?}"
        );

        let store = dir.path().join("store.db");
        Connection::open(&store)
            .and_then(|c| c.execute_batch("CREATE TABLE t (k INTEGER)"))
            .expect("create store");
        let read_only =
            Connection::open_with_flags(&store, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("open read-only");
        let failure = read_only
            .execute("INSERT INTO t (k) VALUES (1)", [])
            .expect_err("a read-only connection must not write");
        let classified = DatabaseError::classify_sqlite_failure("write", failure);
        assert!(
            matches!(classified, DatabaseError::ReadOnly { .. }),
            "{classified:?}"
        );
    }
}
