// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The recovery decision table: what a failed store open does, by the store's class, the kind of failure and whether a verified snapshot exists.

use std::time::Duration;

use crate::database::catalog::StoreClass;
use crate::errors::DatabaseError;

/// How long a store locked by another program is retried before it is reported
/// unavailable.
pub const BUSY_RETRY_BUDGET: Duration = Duration::from_secs(10);

/// Why a store open failed, as the decision table distinguishes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// The file is not an SQLite database.
    NotADatabase,
    /// The file is a damaged database.
    Corrupt,
    /// A newer core wrote a schema this core cannot read.
    StoreTooNew,
    /// The store holds both names of a renamed object with no merge rule.
    SchemaConflict,
    /// A schema step failed on the store's content.
    Migration,
    /// Another connection or program holds the store's lock.
    Busy,
    /// The store or its directory cannot be written.
    ReadOnly,
    /// The store file cannot be opened.
    CannotOpen,
    /// The disk holding the store is full.
    DiskFull,
    /// The operating system reported an I/O failure.
    Io,
}

impl FailureKind {
    pub const ALL: [FailureKind; 10] = [
        FailureKind::NotADatabase,
        FailureKind::Corrupt,
        FailureKind::StoreTooNew,
        FailureKind::SchemaConflict,
        FailureKind::Migration,
        FailureKind::Busy,
        FailureKind::ReadOnly,
        FailureKind::CannotOpen,
        FailureKind::DiskFull,
        FailureKind::Io,
    ];

    /// The kind of a classified open failure. A failed pre-upgrade copy and a failed
    /// connection are failures of the surroundings, never of the store's content.
    pub fn of(error: &DatabaseError) -> Self {
        match error {
            DatabaseError::NotADatabase { .. } => FailureKind::NotADatabase,
            DatabaseError::Corrupt { .. } => FailureKind::Corrupt,
            DatabaseError::StoreTooNew { .. } => FailureKind::StoreTooNew,
            DatabaseError::SchemaConflict { .. } => FailureKind::SchemaConflict,
            DatabaseError::Migration { .. }
            | DatabaseError::Query { .. }
            | DatabaseError::Sqlite { .. } => FailureKind::Migration,
            DatabaseError::Busy { .. } => FailureKind::Busy,
            DatabaseError::ReadOnly { .. } => FailureKind::ReadOnly,
            DatabaseError::CannotOpen { .. } | DatabaseError::Connection { .. } => {
                FailureKind::CannotOpen
            }
            DatabaseError::DiskFull { .. } => FailureKind::DiskFull,
            DatabaseError::Io { .. } | DatabaseError::Backup { .. } => FailureKind::Io,
        }
    }

    /// Whether the failure lies in the store's content, the only kind that may lead to
    /// a file action.
    pub const fn is_content(self) -> bool {
        matches!(
            self,
            FailureKind::NotADatabase
                | FailureKind::Corrupt
                | FailureKind::StoreTooNew
                | FailureKind::SchemaConflict
                | FailureKind::Migration
        )
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            FailureKind::NotADatabase => "not_a_database",
            FailureKind::Corrupt => "corrupt",
            FailureKind::StoreTooNew => "store_too_new",
            FailureKind::SchemaConflict => "schema_conflict",
            FailureKind::Migration => "migration",
            FailureKind::Busy => "busy",
            FailureKind::ReadOnly => "read_only",
            FailureKind::CannotOpen => "cannot_open",
            FailureKind::DiskFull => "disk_full",
            FailureKind::Io => "io",
        }
    }
}

/// What the storage phase does with a store whose open failed. No action deletes a
/// file: a moved store goes to quarantine and stays there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Move the store to quarantine and create it afresh; with `salvage`, copy the
    /// store's salvage tables back from the quarantined file.
    Rebuild { salvage: bool },
    /// Move the store to quarantine and put a copy of its latest verified snapshot in
    /// its place.
    Restore,
    /// Leave the store where it is, unopened, and report it unavailable.
    Isolate,
    /// Open again until [`BUSY_RETRY_BUDGET`] has passed, then report it unavailable.
    RetryUntilBudget,
    /// Open once more, then report it unavailable.
    RetryOnce,
    /// Report it unavailable; nothing is touched.
    Unavailable,
}

impl RecoveryAction {
    /// Whether the action moves or replaces the store file.
    pub const fn moves_file(self) -> bool {
        matches!(
            self,
            RecoveryAction::Rebuild { .. } | RecoveryAction::Restore
        )
    }
}

/// The recovery of a store of `class` whose open failed with `failure`, given whether
/// a verified snapshot of it exists.
pub const fn decide(class: StoreClass, failure: FailureKind, snapshot: bool) -> RecoveryAction {
    use FailureKind as F;
    use RecoveryAction as A;
    use StoreClass as C;
    match (failure, class) {
        (F::Busy, _) => A::RetryUntilBudget,
        (F::Io, _) => A::RetryOnce,
        (F::ReadOnly | F::CannotOpen | F::DiskFull, _) => A::Unavailable,

        (F::NotADatabase | F::Corrupt, C::Cache) => A::Rebuild { salvage: true },
        (F::NotADatabase | F::Corrupt, C::Derived) => A::Rebuild { salvage: false },
        (F::NotADatabase | F::Corrupt, C::Ledger) if snapshot => A::Restore,
        (F::NotADatabase | F::Corrupt, C::Ledger) => A::Rebuild { salvage: true },
        (F::NotADatabase | F::Corrupt, C::Vault) if snapshot => A::Restore,
        (F::NotADatabase | F::Corrupt, C::Vault) => A::Isolate,

        (F::StoreTooNew, C::Cache | C::Derived) => A::Rebuild { salvage: false },
        (F::StoreTooNew, C::Ledger | C::Vault) => A::Isolate,

        (F::SchemaConflict | F::Migration, C::Cache) => A::Rebuild { salvage: true },
        (F::SchemaConflict | F::Migration, C::Derived) => A::Rebuild { salvage: false },
        (F::SchemaConflict | F::Migration, C::Ledger | C::Vault) => A::Isolate,
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
