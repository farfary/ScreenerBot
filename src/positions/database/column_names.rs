// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `positions.db` and of the paper store, which shares its schema, and
//! the rule that reconciles each one held under both names.

use crate::database::schema::{ColumnReconcile, ColumnRename, StoreRenames};

/// No released build re-adds a legacy name here, so both names refuse the open.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "positions.db",
    tables: &[],
    columns: &[
        ColumnRename {
            table: "positions",
            legacy: "entry_size_sol",
            canonical: "entry_size_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "positions",
            legacy: "total_size_sol",
            canonical: "total_size_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "positions",
            legacy: "sol_received",
            canonical: "native_received",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "positions",
            legacy: "entry_fee_lamports",
            canonical: "entry_fee_raw",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "positions",
            legacy: "exit_fee_lamports",
            canonical: "exit_fee_raw",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "position_exits",
            legacy: "sol_received",
            canonical: "native_received",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "position_exits",
            legacy: "fees_lamports",
            canonical: "fees_raw",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "position_entries",
            legacy: "sol_spent",
            canonical: "native_spent",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "position_entries",
            legacy: "fees_lamports",
            canonical: "fees_raw",
            reconcile: ColumnReconcile::Refuse,
        },
    ],
};

/// The renames of the paper store: the same tables and columns, named as its own file.
pub(crate) const PAPER_RENAMES: StoreRenames = StoreRenames {
    store: crate::paths::PAPER_POSITIONS_FILE,
    ..RENAMES
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_paper_store_shares_the_renames_under_its_own_file_name() {
        assert_eq!(PAPER_RENAMES.store, "paper_positions.db");
        assert_ne!(PAPER_RENAMES.store, RENAMES.store);
        assert_eq!(PAPER_RENAMES.columns.len(), RENAMES.columns.len());
        assert_eq!(PAPER_RENAMES.tables.len(), RENAMES.tables.len());
    }
}
