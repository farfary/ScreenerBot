// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The schema-evolution owner: live-schema inspection, the renamed-object declarations every store reconciles and renames on open, the per-store schema ledger and the upgrade every open runs.

pub mod ddl;
pub mod inspect;
pub mod ledger;
pub mod renames;
pub mod types;
pub mod upgrade;

pub use inspect::{
    primary_key_width, table_columns, table_columns_in, table_exists, table_has_column,
};
pub use types::{
    ColumnDeclaration, ColumnReconcile, ColumnRename, LegacyStamp, MergedColumn, SchemaContext,
    SchemaStep, StoreRenames, StoreSchema, TableReconcile, TableRename,
};
