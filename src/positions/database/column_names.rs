// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent column-level migrations of positions storage: additive historical
//! columns and the rename of amount columns to unit-neutral names.

use rusqlite::Connection;

use crate::database::schema::table_has_column;

use crate::positions::{Error, Result};

const COLUMN_RENAMES: &[(&str, &str, &str)] = &[
    ("positions", "entry_size_sol", "entry_size_native"),
    ("positions", "total_size_sol", "total_size_native"),
    ("positions", "sol_received", "native_received"),
    ("positions", "entry_fee_lamports", "entry_fee_raw"),
    ("positions", "exit_fee_lamports", "exit_fee_raw"),
    ("position_exits", "sol_received", "native_received"),
    ("position_exits", "fees_lamports", "fees_raw"),
    ("position_entries", "sol_spent", "native_spent"),
    ("position_entries", "fees_lamports", "fees_raw"),
];

/// Whether `table` has `column`, through the shared live-schema gate.
pub(super) fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    table_has_column(conn, table, column).map_err(|e| Error::SchemaMigration {
        detail: format!("failed to inspect {table} schema: {e}"),
    })
}

/// Add each `(column, ALTER TABLE ...)` the table does not have yet and return the
/// columns added. A column already present is left as it is; an ALTER that fails
/// refuses the open.
pub(super) fn add_missing_columns(
    conn: &Connection,
    table: &str,
    columns: &[(&'static str, &str)],
) -> Result<Vec<&'static str>> {
    let mut added = Vec::new();
    for (column, sql) in columns {
        if !has_column(conn, table, column)? {
            conn.execute(sql, []).map_err(|e| Error::SchemaMigration {
                detail: format!("failed to add {table}.{column}: {e}"),
            })?;
            added.push(*column);
        }
    }
    Ok(added)
}

/// Rename every legacy unit column still present and return how many were renamed.
/// Runs inside the caller's transaction, so a migration refused later in that
/// transaction rolls the renames back with it.
pub(super) fn rename_unit_neutral_columns(conn: &Connection) -> Result<usize> {
    let mut renamed = 0;
    for (table, old, new) in COLUMN_RENAMES {
        let has_old = has_column(conn, table, old)?;
        let has_new = has_column(conn, table, new)?;
        match (has_old, has_new) {
            (true, true) => {
                return Err(Error::SchemaMigration {
                    detail: format!("both {table}.{old} and {table}.{new} exist"),
                });
            }
            (true, false) => {
                conn.execute(
                    &format!("ALTER TABLE {table} RENAME COLUMN {old} TO {new}"),
                    [],
                )
                .map_err(|e| Error::SchemaMigration {
                    detail: format!("failed to rename {table}.{old} to {new}: {e}"),
                })?;
                renamed += 1;
            }
            (false, _) => {}
        }
    }
    Ok(renamed)
}
