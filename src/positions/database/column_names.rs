// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `positions.db` and the rule that reconciles each one held under both names.

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
