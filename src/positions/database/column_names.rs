// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent migration of position amount columns to unit-neutral names.

use rusqlite::Connection;

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

fn has_column(conn: &Connection, table: &str, name: &str) -> Result<bool> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to inspect {table} schema: {e}"),
        })?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to read {table} schema: {e}"),
        })?;
    for column in columns {
        if column.map_err(|e| Error::SchemaMigration {
            detail: format!("failed to decode {table} schema: {e}"),
        })? == name
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Rename every legacy unit column still present. Runs inside the caller's
/// transaction, so a migration refused later in that transaction rolls the
/// renames back with it.
pub(super) fn rename_unit_neutral_columns(conn: &Connection) -> Result<()> {
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
            }
            (false, _) => {}
        }
    }
    Ok(())
}
