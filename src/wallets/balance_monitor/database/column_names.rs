// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed tables and columns of `wallet.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{
    ColumnReconcile, ColumnRename, MergedColumn, StoreRenames, TableReconcile, TableRename,
};

/// Older builds re-create `sol_flow_cache` and re-add `total_equity_sol` and write only
/// those names: the flow rows merge by key into the canonical cache, and the canonical
/// equity takes the legacy value where it is still unset. A flow table older than the
/// chain-scoped cache is attributed as the chain-identity rebuild attributes it. No
/// released build re-adds any other legacy name here.
pub(crate) const RENAMES: StoreRenames = StoreRenames {
    store: "wallet.db",
    tables: &[TableRename {
        legacy: "sol_flow_cache",
        canonical: "native_flow_cache",
        reconcile: TableReconcile::MergeRows {
            columns: &[
                MergedColumn {
                    canonical: "chain_id",
                    legacy: "chain_id",
                    absent: Some("'solana'"),
                },
                MergedColumn {
                    canonical: "wallet_address",
                    legacy: "wallet_address",
                    absent: Some(
                        "COALESCE((SELECT value FROM wallet_metadata WHERE key = 'current_wallet'), '')",
                    ),
                },
                MergedColumn {
                    canonical: "signature",
                    legacy: "signature",
                    absent: None,
                },
                MergedColumn {
                    canonical: "timestamp",
                    legacy: "timestamp",
                    absent: None,
                },
                MergedColumn {
                    canonical: "native_delta",
                    legacy: "sol_delta",
                    absent: None,
                },
                MergedColumn {
                    canonical: "created_at",
                    legacy: "created_at",
                    absent: None,
                },
            ],
        },
    }],
    columns: &[
        ColumnRename {
            table: "wallet_snapshots",
            legacy: "sol_balance",
            canonical: "native_balance",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "wallet_snapshots",
            legacy: "sol_balance_lamports",
            canonical: "native_balance_raw",
            reconcile: ColumnReconcile::Refuse,
        },
        ColumnRename {
            table: "wallet_snapshots",
            legacy: "total_equity_sol",
            canonical: "total_equity_native",
            reconcile: ColumnReconcile::FillDefaulted { default: "NULL" },
        },
        ColumnRename {
            table: "native_flow_cache",
            legacy: "sol_delta",
            canonical: "native_delta",
            reconcile: ColumnReconcile::Refuse,
        },
    ],
};
