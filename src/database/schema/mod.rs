// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The schema-evolution owner: live-schema inspection and the renamed-object declarations every store reconciles and renames on open.

pub mod inspect;
pub mod renames;
pub mod types;

pub use inspect::{table_columns, table_columns_in, table_exists, table_has_column};
pub use types::{
    ColumnDeclaration, ColumnReconcile, ColumnRename, MergedColumn, StoreRenames, TableReconcile,
    TableRename,
};
