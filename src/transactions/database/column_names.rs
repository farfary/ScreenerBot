// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent migration of transaction amount columns to unit-neutral names.

use rusqlite::Connection;

use crate::transactions::error::Error;

const COLUMN_RENAMES: &[(&str, &str, &str)] = &[
    ("raw_transactions", "fee_lamports", "fee_raw"),
    (
        "processed_transactions",
        "sol_balance_change",
        "native_balance_change",
    ),
    ("processed_transactions", "fee_sol", "fee_native"),
    ("processed_transactions", "sol_delta", "native_delta"),
    ("subject_asset_deltas", "fee_lamports", "fee_raw"),
];

fn has_column(conn: &Connection, table: &str, name: &str) -> Result<bool, Error> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| Error::SchemaInspect {
            detail: format!("failed to inspect {table} schema: {e}"),
        })?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| Error::SchemaInspect {
            detail: format!("failed to read {table} schema: {e}"),
        })?;
    for column in columns {
        if column.map_err(|e| Error::SchemaInspect {
            detail: format!("failed to decode {table} schema: {e}"),
        })? == name
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Rename every legacy unit column still present. Runs inside the opener's
/// transaction after the versioned rebuilds, so each of those reads the shape
/// it was written for and a step refused later in that transaction rolls the
/// renames back with it.
pub(super) fn rename_unit_neutral_columns(conn: &Connection) -> Result<(), Error> {
    for (table, old, new) in COLUMN_RENAMES {
        let has_old = has_column(conn, table, old)?;
        let has_new = has_column(conn, table, new)?;
        match (has_old, has_new) {
            (true, true) => {
                return Err(Error::Migration {
                    step: "rename unit columns".to_owned(),
                    detail: format!("both {table}.{old} and {table}.{new} exist"),
                });
            }
            (true, false) => {
                conn.execute(
                    &format!("ALTER TABLE {table} RENAME COLUMN {old} TO {new}"),
                    [],
                )
                .map_err(|e| Error::Migration {
                    step: "rename unit columns".to_owned(),
                    detail: format!("failed to rename {table}.{old} to {new}: {e}"),
                })?;
            }
            (false, _) => {}
        }
    }
    Ok(())
}

/// `legacy_columns` of `table` as the live table stores them. A table created
/// earlier in the same open already carries the canonical names, so each renamed
/// column is read under whichever of its two names is present.
pub(super) fn live_column_list(
    conn: &Connection,
    table: &str,
    legacy_columns: &str,
) -> Result<String, Error> {
    let mut columns: Vec<&str> = legacy_columns.split(", ").collect();
    for (renamed_table, old, new) in COLUMN_RENAMES {
        if *renamed_table != table {
            continue;
        }
        if !has_column(conn, table, old)? && has_column(conn, table, new)? {
            for column in &mut columns {
                if column == old {
                    *column = new;
                }
            }
        }
    }
    Ok(columns.join(", "))
}
