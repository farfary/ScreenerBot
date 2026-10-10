// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `tokens.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{ColumnReconcile, ColumnRename, StoreRenames};

/// No released build re-adds a legacy name here, so both names refuse the open.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "tokens.db",
    tables: &[],
    columns: &[
        ColumnRename {
            table: "token_pools",
            legacy: "is_sol_pair",
            canonical: "is_native_pair",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "token_pools",
            legacy: "liquidity_sol",
            canonical: "liquidity_native",
            reconcile: ColumnReconcile::Refuse,
        },
    ],
};
