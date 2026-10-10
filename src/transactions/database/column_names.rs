// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `transactions.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{ColumnReconcile, ColumnRename, StoreRenames};

/// Older builds re-add `fee_sol` (`DEFAULT 0`) and `sol_delta` (no default) and write
/// only those names, so each canonical column takes the legacy value where it still
/// holds the default. No released build re-adds any other legacy name here.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "transactions.db",
    tables: &[],
    columns: &[
        ColumnRename {
            table: "raw_transactions",
            legacy: "fee_lamports",
            canonical: "fee_raw",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "processed_transactions",
            legacy: "sol_balance_change",
            canonical: "native_balance_change",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "processed_transactions",
            legacy: "fee_sol",
            canonical: "fee_native",
            reconcile: ColumnReconcile::FillDefaulted { default: "0" },
        },
        ColumnRename {
            table: "processed_transactions",
            legacy: "sol_delta",
            canonical: "native_delta",
            reconcile: ColumnReconcile::FillDefaulted { default: "NULL" },
        },
        ColumnRename {
            table: "subject_asset_deltas",
            legacy: "fee_lamports",
            canonical: "fee_raw",
            reconcile: ColumnReconcile::Refuse,
        },
    ],
};
