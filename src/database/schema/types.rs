// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Declarations of the schema-evolution owner: live column declarations and the renamed tables and columns of each store with the rule that reconciles a store holding both names.

/// One column of a live table as `PRAGMA table_info` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDeclaration {
    pub name: String,
    pub declared_type: String,
    pub not_null: bool,
    /// The default expression as written in the DDL; `None` when the column has none.
    pub default: Option<String>,
}

impl ColumnDeclaration {
    /// The default as SQL text, `NULL` for a column declared without one.
    pub fn default_sql(&self) -> &str {
        self.default.as_deref().unwrap_or("NULL")
    }

    /// Whether `other` is declared with the same type, nullability and default.
    pub fn declared_like(&self, other: &ColumnDeclaration) -> bool {
        self.declared_type
            .eq_ignore_ascii_case(&other.declared_type)
            && self.not_null == other.not_null
            && self.default_sql() == other.default_sql()
    }
}

/// Every renamed table and column of one store.
#[derive(Debug, Clone, Copy)]
pub struct StoreRenames {
    /// File name of the store, as errors and logs name it.
    pub store: &'static str,
    /// Renamed tables, reconciled and renamed before the columns.
    pub tables: &'static [TableRename],
    pub columns: &'static [ColumnRename],
}

/// A table renamed from `legacy` to `canonical`.
#[derive(Debug, Clone, Copy)]
pub struct TableRename {
    pub legacy: &'static str,
    pub canonical: &'static str,
    pub reconcile: TableReconcile,
}

impl TableRename {
    /// Both names, as a conflict reports them.
    pub fn object(&self) -> String {
        format!("{} and {}", self.legacy, self.canonical)
    }
}

/// What an open does with a store holding both names of a renamed table.
#[derive(Debug, Clone, Copy)]
pub enum TableReconcile {
    /// An older build re-created the legacy table: its rows are inserted into the
    /// canonical table where the canonical key is absent, then the legacy table is
    /// dropped. The canonical table and its indexes are never altered.
    MergeRows { columns: &'static [MergedColumn] },
    /// No released build re-creates the legacy table, so both names mean a store of
    /// unknown provenance and the open is refused.
    Refuse,
}

/// One canonical column a merged legacy table supplies.
#[derive(Debug, Clone, Copy)]
pub struct MergedColumn {
    pub canonical: &'static str,
    pub legacy: &'static str,
    /// The SQL value for a legacy table shape that predates the column; `None` refuses
    /// such a shape.
    pub absent: Option<&'static str>,
}

/// A column of `table` renamed from `legacy` to `canonical`.
#[derive(Debug, Clone, Copy)]
pub struct ColumnRename {
    pub table: &'static str,
    pub legacy: &'static str,
    pub canonical: &'static str,
    pub reconcile: ColumnReconcile,
}

impl ColumnRename {
    /// Both qualified names, as a conflict reports them.
    pub fn object(&self) -> String {
        format!(
            "{table}.{} and {table}.{}",
            self.legacy,
            self.canonical,
            table = self.table
        )
    }
}

/// What an open does with a table holding both names of a renamed column.
#[derive(Debug, Clone, Copy)]
pub enum ColumnReconcile {
    /// An older build re-added the legacy column with the declaration the rename
    /// carried, so each row holds a value in the column its writer knew and `default`
    /// in the other. The canonical column takes the legacy value exactly where it still
    /// holds `default` and the legacy value does not; the legacy column is then dropped.
    FillDefaulted { default: &'static str },
    /// No released build re-adds the legacy column, so both names mean a store of
    /// unknown provenance and the open is refused.
    Refuse,
}
