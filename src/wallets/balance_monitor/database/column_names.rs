// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent migration of wallet-monitor amount columns and tables to
//! unit-neutral names.

use rusqlite::{Connection, OptionalExtension};

use crate::wallets::Error;

const TABLE_RENAMES: &[(&str, &str)] = &[("sol_flow_cache", "native_flow_cache")];

const COLUMN_RENAMES: &[(&str, &str, &str)] = &[
    ("wallet_snapshots", "sol_balance", "native_balance"),
    (
        "wallet_snapshots",
        "sol_balance_lamports",
        "native_balance_raw",
    ),
    (
        "wallet_snapshots",
        "total_equity_sol",
        "total_equity_native",
    ),
    ("native_flow_cache", "sol_delta", "native_delta"),
];

fn has_table(conn: &Connection, table: &'static str) -> Result<bool, Error> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .map(|found| found.is_some())
    .map_err(|e| Error::SchemaInspect {
        table,
        detail: e.to_string(),
    })
}

fn has_column(conn: &Connection, table: &'static str, name: &str) -> Result<bool, Error> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| Error::SchemaInspect {
            table,
            detail: e.to_string(),
        })?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| Error::SchemaInspect {
            table,
            detail: e.to_string(),
        })?;
    for column in columns {
        if column.map_err(|e| Error::SchemaInspect {
            table,
            detail: e.to_string(),
        })? == name
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Rename every legacy unit table and column still present. Runs first inside
/// the opener's transaction, so every later step sees the canonical names and a
/// step refused later in that transaction rolls the renames back with it.
pub(super) fn rename_unit_neutral_columns(conn: &Connection) -> Result<(), Error> {
    for (old, new) in TABLE_RENAMES {
        let has_old = has_table(conn, old)?;
        let has_new = has_table(conn, new)?;
        match (has_old, has_new) {
            (true, true) => {
                return Err(Error::Migration {
                    step: "rename unit tables".to_owned(),
                    detail: format!("both {old} and {new} exist"),
                });
            }
            (true, false) => {
                conn.execute(&format!("ALTER TABLE {old} RENAME TO {new}"), [])
                    .map_err(|e| Error::Migration {
                        step: "rename unit tables".to_owned(),
                        detail: format!("failed to rename {old} to {new}: {e}"),
                    })?;
            }
            (false, _) => {}
        }
    }
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
