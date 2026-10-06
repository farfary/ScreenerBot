// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Idempotent migration of copy-trading amount columns to unit-neutral names.

use rusqlite::Connection;

use crate::trader::error::Error;

use super::schema::table_columns;

const COLUMN_RENAMES: &[(&str, &str, &str)] = &[
    ("copy_tasks", "max_sol_per_trade", "max_native_per_trade"),
    ("copy_tasks", "max_sol_per_token", "max_native_per_token"),
    ("copy_tasks", "total_budget_sol", "total_budget_native"),
    (
        "copy_tasks",
        "min_target_trade_sol",
        "min_target_trade_native",
    ),
    (
        "copy_tasks",
        "max_target_trade_sol",
        "max_target_trade_native",
    ),
    ("copy_spend", "spent_sol", "spent_native"),
    (
        "copy_paper_positions",
        "cost_basis_sol",
        "cost_basis_native",
    ),
    ("copy_paper_positions", "invested_sol", "invested_native"),
    (
        "copy_paper_positions",
        "realized_proceeds_sol",
        "realized_proceeds_native",
    ),
    (
        "copy_paper_positions",
        "realized_cost_sol",
        "realized_cost_native",
    ),
    (
        "copy_paper_positions",
        "last_price_sol",
        "last_price_native",
    ),
    (
        "copy_paper_positions",
        "peak_price_sol",
        "peak_price_native",
    ),
];

/// Rename every legacy unit column still present. Runs inside the caller's
/// transaction after every other schema step, so a step refused later in that
/// transaction rolls the renames back with it.
pub(super) fn rename_unit_neutral_columns(connection: &Connection) -> crate::trader::Result<()> {
    for (table, old, new) in COLUMN_RENAMES {
        let columns = table_columns(connection, table)?;
        let has_old = columns.iter().any(|column| column == old);
        let has_new = columns.iter().any(|column| column == new);
        match (has_old, has_new) {
            (true, true) => {
                return Err(Error::CopyValidation {
                    detail: format!("both {table}.{old} and {table}.{new} exist"),
                });
            }
            (true, false) => {
                connection
                    .execute(
                        &format!("ALTER TABLE {table} RENAME COLUMN {old} TO {new}"),
                        [],
                    )
                    .map_err(crate::errors::DatabaseError::from)?;
            }
            (false, _) => {}
        }
    }
    Ok(())
}
