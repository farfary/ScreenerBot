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
}

impl DatabaseError {
    /// Maps a captured rusqlite failure onto the database vocabulary by its SQLite result
    /// code. Lock contention is the only retryable class; every other failure is a query
    /// error.
    pub fn classify_sqlite_failure(operation: &str, error: rusqlite::Error) -> Self {
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
                DatabaseError::Busy {
                    operation: operation.to_owned(),
                    message: error.to_string(),
                }
            }
            _ => DatabaseError::Query {
                operation: operation.to_owned(),
                message: error.to_string(),
            },
        }
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        DatabaseError::Sqlite {
            message: err.to_string(),
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
}
