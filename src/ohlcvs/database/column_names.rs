// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent migration of OHLCV pool columns to unit-neutral names.

use rusqlite::Connection;

use crate::ohlcvs::types::{OhlcvError, OhlcvResult};

use super::table_has_column;

const COLUMN_RENAMES: &[(&str, &str, &str)] = &[("ohlcv_pools", "is_sol_pair", "is_native_pair")];

/// Rename every legacy unit column still present. Runs inside the opener's
/// transaction after the chain-scope rebuild, so that rebuild reads the shape
/// it was written for and a step refused later in that transaction rolls the
/// renames back with it.
pub(super) fn rename_unit_neutral_columns(conn: &Connection) -> OhlcvResult<()> {
    for (table, old, new) in COLUMN_RENAMES {
        let has_old = table_has_column(conn, table, old)?;
        let has_new = table_has_column(conn, table, new)?;
        match (has_old, has_new) {
            (true, true) => {
                return Err(OhlcvError::DatabaseError(format!(
                    "both {table}.{old} and {table}.{new} exist"
                )));
            }
            (true, false) => {
                conn.execute(
                    &format!("ALTER TABLE {table} RENAME COLUMN {old} TO {new}"),
                    [],
                )
                .map_err(|e| {
                    OhlcvError::DatabaseError(format!(
                        "Failed to rename {table}.{old} to {new}: {e}"
                    ))
                })?;
            }
            (false, _) => {}
        }
    }
    Ok(())
}
