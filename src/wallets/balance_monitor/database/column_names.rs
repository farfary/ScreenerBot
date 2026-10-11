// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The renamed tables and columns of `wallet.db` and the rule that reconciles each one held under both names.

use crate::database::schema::{
    ColumnReconcile, ColumnRename, MergedColumn, StoreRenames, TableReconcile, TableRename,
};

/// Older builds re-create `sol_flow_cache` and re-add `total_equity_sol` and write only
/// those names: the flow rows merge by key into the canonical cache, and the canonical
/// equity takes the legacy value where it is still unset. A flow row without a wallet
/// column is attributed to the wallet recorded under the chain-scoped
/// `current_wallet:solana` key, or under the unscoped `current_wallet` key older builds
/// wrote when the scoped key is missing. No released build re-adds any other legacy
/// name here.
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
                        "COALESCE((SELECT value FROM wallet_metadata WHERE key = 'current_wallet:solana'), (SELECT value FROM wallet_metadata WHERE key = 'current_wallet'), '')",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;

    /// The merge fallback is a compile-time literal; it must read the keys the
    /// database owner defines.
    #[test]
    fn the_flow_wallet_fallback_is_the_owners_recorded_wallet_expression() {
        let TableReconcile::MergeRows { columns } = RENAMES.tables[0].reconcile else {
            panic!("the flow cache merges by key");
        };
        let wallet = columns
            .iter()
            .find(|column| column.canonical == "wallet_address")
            .expect("merged wallet column");
        assert_eq!(
            wallet.absent,
            Some(super::super::recorded_wallet_sql("wallet_metadata", ChainId::Solana).as_str())
        );
    }
}
