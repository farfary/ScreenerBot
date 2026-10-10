// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed columns of `copy_trading.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{ColumnReconcile, ColumnRename, StoreRenames};

/// Older builds re-add `peak_price_sol` and cannot run copy trading on a store with the
/// canonical task columns, so the canonical peak keeps its value and takes the legacy one
/// only where it is still unset. No released build re-adds any other legacy name here.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "copy_trading.db",
    tables: &[],
    columns: &[
        ColumnRename {
            table: "copy_tasks",
            legacy: "max_sol_per_trade",
            canonical: "max_native_per_trade",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_tasks",
            legacy: "max_sol_per_token",
            canonical: "max_native_per_token",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_tasks",
            legacy: "total_budget_sol",
            canonical: "total_budget_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_tasks",
            legacy: "min_target_trade_sol",
            canonical: "min_target_trade_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_tasks",
            legacy: "max_target_trade_sol",
            canonical: "max_target_trade_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_spend",
            legacy: "spent_sol",
            canonical: "spent_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_paper_positions",
            legacy: "cost_basis_sol",
            canonical: "cost_basis_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_paper_positions",
            legacy: "invested_sol",
            canonical: "invested_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_paper_positions",
            legacy: "realized_proceeds_sol",
            canonical: "realized_proceeds_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_paper_positions",
            legacy: "realized_cost_sol",
            canonical: "realized_cost_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_paper_positions",
            legacy: "last_price_sol",
            canonical: "last_price_native",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "copy_paper_positions",
            legacy: "peak_price_sol",
            canonical: "peak_price_native",
            reconcile: ColumnReconcile::FillDefaulted { default: "NULL" },
        },
    ],
};
