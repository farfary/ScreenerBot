// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Schema statements a store's step runs, each failure classified and named by the operation it was part of.

use rusqlite::Connection;

use crate::errors::DatabaseError;

use super::inspect::{table_columns, table_exists, table_has_column};

/// Run one schema statement.
pub fn run(conn: &Connection, operation: &str, sql: &str) -> Result<(), DatabaseError> {
    conn.execute(sql, [])
        .map(|_| ())
        .map_err(|error| DatabaseError::classify_sqlite_failure(operation, error))
}

/// Run a batch of schema statements.
pub fn run_batch(conn: &Connection, operation: &str, sql: &str) -> Result<(), DatabaseError> {
    conn.execute_batch(sql)
        .map_err(|error| DatabaseError::classify_sqlite_failure(operation, error))
}

/// Whether the table `table` exists, with a failed inspection classified.
pub fn has_table(conn: &Connection, table: &str) -> Result<bool, DatabaseError> {
    table_exists(conn, table).map_err(|error| {
        DatabaseError::classify_sqlite_failure(&format!("inspect table {table}"), error)
    })
}

/// The column names of the live `table`, in table order; none for a missing table.
pub fn column_names(conn: &Connection, table: &str) -> Result<Vec<String>, DatabaseError> {
    table_columns(conn, table)
        .map(|columns| columns.into_iter().map(|column| column.name).collect())
        .map_err(|error| {
            DatabaseError::classify_sqlite_failure(&format!("inspect {table} columns"), error)
        })
}

/// Whether the live `table` has `column`, with a failed inspection classified.
pub fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, DatabaseError> {
    table_has_column(conn, table, column).map_err(|error| {
        DatabaseError::classify_sqlite_failure(&format!("inspect {table}.{column}"), error)
    })
}

/// Add `column` to `table` with `declaration` unless the live table already has it.
/// Returns whether the column was added.
pub fn add_column(
    conn: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<bool, DatabaseError> {
    if has_column(conn, table, column)? {
        return Ok(false);
    }
    run(
        conn,
        &format!("add {table}.{column}"),
        &format!("ALTER TABLE {table} ADD COLUMN {column} {declaration}"),
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_column_is_added_once_and_failures_name_their_operation() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        run(&conn, "create t", "CREATE TABLE t (a INTEGER)").expect("create table");
        assert!(add_column(&conn, "t", "b", "TEXT NOT NULL DEFAULT 'x'").expect("add b"));
        assert!(!add_column(&conn, "t", "b", "TEXT NOT NULL DEFAULT 'x'").expect("keep b"));
        let error = run(&conn, "broken statement", "NOT SQL").expect_err("malformed SQL");
        assert!(error.to_string().contains("broken statement"), "{error}");
    }
}
