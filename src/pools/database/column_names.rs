// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent migration of price-history amount columns to unit-neutral names.

use rusqlite::Connection;

use crate::errors::DatabaseError;
use crate::pools::Error;

const COLUMN_RENAMES: &[(&str, &str, &str)] =
    &[("price_history", "sol_reserves", "native_reserves")];

fn has_column(conn: &Connection, table: &str, name: &str) -> Result<bool, Error> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| DatabaseError::Query {
            operation: format!("inspect {table} schema"),
            message: e.to_string(),
        })?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| DatabaseError::Query {
            operation: format!("read {table} schema"),
            message: e.to_string(),
        })?;
    for column in columns {
        if column.map_err(|e| DatabaseError::Query {
            operation: format!("decode {table} schema"),
            message: e.to_string(),
        })? == name
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Rename every legacy unit column still present. Runs inside the opener's
/// transaction after the chain-scope rebuild, so that rebuild reads the shape
/// it was written for and a step refused later in that transaction rolls the
/// renames back with it.
pub(super) fn rename_unit_neutral_columns(conn: &Connection) -> Result<(), Error> {
    for (table, old, new) in COLUMN_RENAMES {
        let has_old = has_column(conn, table, old)?;
        let has_new = has_column(conn, table, new)?;
        match (has_old, has_new) {
            (true, true) => {
                return Err(Error::MigrationIntegrity {
                    table: (*table).to_owned(),
                    detail: format!("both {table}.{old} and {table}.{new} exist"),
                });
            }
            (true, false) => {
                conn.execute(
                    &format!("ALTER TABLE {table} RENAME COLUMN {old} TO {new}"),
                    [],
                )
                .map_err(|e| DatabaseError::Query {
                    operation: format!("rename {table}.{old} to {new}"),
                    message: e.to_string(),
                })?;
            }
            (false, _) => {}
        }
    }
    Ok(())
}
