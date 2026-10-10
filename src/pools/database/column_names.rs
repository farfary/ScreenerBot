// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `pools.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{ColumnReconcile, ColumnRename, StoreRenames};

/// No released build re-adds a legacy name here, so both names refuse the open.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "pools.db",
    tables: &[],
    columns: &[ColumnRename {
        table: "price_history",
        legacy: "sol_reserves",
        canonical: "native_reserves",
        reconcile: ColumnReconcile::Refuse,
    }],
};
