// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `ohlcvs.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{ColumnReconcile, ColumnRename, StoreRenames};

/// Older builds re-add `is_sol_pair` with `DEFAULT 1` and write only that name, so the
/// canonical flag takes the legacy value where it still holds the default.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "ohlcvs.db",
    tables: &[],
    columns: &[ColumnRename {
        table: "ohlcv_pools",
        legacy: "is_sol_pair",
        canonical: "is_native_pair",
        reconcile: ColumnReconcile::FillDefaulted { default: "1" },
    }],
};
