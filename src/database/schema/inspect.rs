// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live-schema inspection shared by every store's open: tables, columns and their declarations.

use rusqlite::{Connection, OptionalExtension};

use super::types::ColumnDeclaration;

/// Whether `table` already carries `column`: the live-schema gate
/// (`PRAGMA table_info`) every structural migration checks instead of a
/// version stamp. Domains map the error onto their own error type.
pub fn table_has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    Ok(table_columns(conn, table)?
        .iter()
        .any(|declaration| declaration.name == column))
}

/// Whether a table named `table` exists in the main schema.
pub fn table_exists(conn: &Connection, table: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .map(|found| found.is_some())
}

/// Every column of `table` with its declaration, in table order. A missing table has no
/// columns.
pub fn table_columns(conn: &Connection, table: &str) -> rusqlite::Result<Vec<ColumnDeclaration>> {
    table_columns_in(conn, "main", table)
}

/// [`table_columns`] of `table` in the attached database `schema`.
pub fn table_columns_in(
    conn: &Connection,
    schema: &str,
    table: &str,
) -> rusqlite::Result<Vec<ColumnDeclaration>> {
    let mut statement = conn.prepare(&format!("PRAGMA {schema}.table_info({table})"))?;
    let columns = statement
        .query_map([], |row| {
            Ok(ColumnDeclaration {
                name: row.get(1)?,
                declared_type: row.get(2)?,
                not_null: row.get(3)?,
                default: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_carry_their_declarations_and_a_missing_table_has_none() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        conn.execute_batch(
            "CREATE TABLE t (a INTEGER NOT NULL DEFAULT 1, b REAL, c TEXT DEFAULT 'x')",
        )
        .expect("create table");

        let columns = table_columns(&conn, "t").expect("read columns");
        assert_eq!(
            columns,
            vec![
                ColumnDeclaration {
                    name: "a".to_owned(),
                    declared_type: "INTEGER".to_owned(),
                    not_null: true,
                    default: Some("1".to_owned()),
                },
                ColumnDeclaration {
                    name: "b".to_owned(),
                    declared_type: "REAL".to_owned(),
                    not_null: false,
                    default: None,
                },
                ColumnDeclaration {
                    name: "c".to_owned(),
                    declared_type: "TEXT".to_owned(),
                    not_null: false,
                    default: Some("'x'".to_owned()),
                },
            ]
        );
        assert!(table_has_column(&conn, "t", "b").expect("inspect"));
        assert!(!table_has_column(&conn, "t", "z").expect("inspect"));
        assert!(table_exists(&conn, "t").expect("inspect"));
        assert!(!table_exists(&conn, "missing").expect("inspect"));
        assert!(table_columns(&conn, "missing")
            .expect("inspect missing table")
            .is_empty());
        conn.execute_batch("ATTACH DATABASE ':memory:' AS other; CREATE TABLE other.t (z TEXT)")
            .expect("attach");
        let attached: Vec<String> = table_columns_in(&conn, "other", "t")
            .expect("inspect attached table")
            .into_iter()
            .map(|column| column.name)
            .collect();
        assert_eq!(attached, vec!["z"]);
    }
}
