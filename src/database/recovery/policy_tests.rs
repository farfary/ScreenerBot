// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests the recovery decision table against its literal expected rows and the properties no row may break.

use super::*;
use crate::database::catalog::StoreClass;
use crate::errors::DatabaseError;

const CLASSES: [StoreClass; 4] = [
    StoreClass::Cache,
    StoreClass::Derived,
    StoreClass::Ledger,
    StoreClass::Vault,
];

/// The expected action for every class and failure, without and with a verified
/// snapshot.
fn expected(class: StoreClass, failure: FailureKind) -> (RecoveryAction, RecoveryAction) {
    use FailureKind as F;
    use RecoveryAction as A;
    use StoreClass as C;
    let same = |action| (action, action);
    match (class, failure) {
        (_, F::Busy) => same(A::RetryUntilBudget),
        (_, F::Io) => same(A::RetryOnce),
        (_, F::ReadOnly | F::CannotOpen | F::DiskFull) => same(A::Unavailable),
        (C::Cache, F::NotADatabase | F::Corrupt) => same(A::Rebuild { salvage: true }),
        (C::Cache, F::StoreTooNew) => same(A::Rebuild { salvage: false }),
        (C::Cache, F::SchemaConflict | F::Migration) => same(A::Rebuild { salvage: true }),
        (C::Derived, F::NotADatabase | F::Corrupt | F::StoreTooNew) => {
            same(A::Rebuild { salvage: false })
        }
        (C::Derived, F::SchemaConflict | F::Migration) => same(A::Rebuild { salvage: false }),
        (C::Ledger, F::NotADatabase | F::Corrupt) => (A::Rebuild { salvage: true }, A::Restore),
        (C::Ledger, F::StoreTooNew | F::SchemaConflict | F::Migration) => same(A::Isolate),
        (C::Vault, F::NotADatabase | F::Corrupt) => (A::Isolate, A::Restore),
        (C::Vault, F::StoreTooNew | F::SchemaConflict | F::Migration) => same(A::Isolate),
    }
}

#[test]
fn every_class_and_failure_follows_the_table() {
    for class in CLASSES {
        for failure in FailureKind::ALL {
            let (without, with) = expected(class, failure);
            assert_eq!(
                decide(class, failure, false),
                without,
                "{class:?} {failure:?}"
            );
            assert_eq!(
                decide(class, failure, true),
                with,
                "{class:?} {failure:?} snapshot"
            );
        }
    }
}

/// The vault is moved only when a verified snapshot replaces it.
#[test]
fn the_vault_never_moves_without_a_verified_snapshot() {
    for failure in FailureKind::ALL {
        let action = decide(StoreClass::Vault, failure, false);
        assert!(!action.moves_file(), "{failure:?} -> {action:?}");
        assert_ne!(
            decide(StoreClass::Vault, failure, true),
            RecoveryAction::Rebuild { salvage: true }
        );
        assert_ne!(
            decide(StoreClass::Vault, failure, true),
            RecoveryAction::Rebuild { salvage: false }
        );
    }
}

/// Lock contention, permissions, a full disk and I/O failures never touch a file.
#[test]
fn surroundings_failures_never_move_a_file() {
    for class in CLASSES {
        for failure in FailureKind::ALL.into_iter().filter(|f| !f.is_content()) {
            for snapshot in [false, true] {
                let action = decide(class, failure, snapshot);
                assert!(!action.moves_file(), "{class:?} {failure:?} -> {action:?}");
            }
        }
    }
}

/// A store this core cannot read is never salvaged into the rebuilt file: its rows
/// carry a schema this core does not know.
#[test]
fn a_too_new_store_is_never_salvaged() {
    for class in CLASSES {
        for snapshot in [false, true] {
            assert_ne!(
                decide(class, FailureKind::StoreTooNew, snapshot),
                RecoveryAction::Rebuild { salvage: true }
            );
        }
    }
}

#[test]
fn every_open_failure_has_a_kind_and_environment_failures_are_not_content() {
    let op = || "open".to_owned();
    let message = || "m".to_owned();
    let cases = [
        (
            DatabaseError::NotADatabase {
                operation: op(),
                message: message(),
            },
            FailureKind::NotADatabase,
        ),
        (
            DatabaseError::Corrupt {
                operation: op(),
                message: message(),
            },
            FailureKind::Corrupt,
        ),
        (
            DatabaseError::StoreTooNew {
                store: "s.db".to_owned(),
                version: 2,
                min_reader_version: 2,
                supported_version: 1,
                written_by: "9.9.9".to_owned(),
            },
            FailureKind::StoreTooNew,
        ),
        (
            DatabaseError::SchemaConflict {
                store: "s.db".to_owned(),
                object: "a and b".to_owned(),
            },
            FailureKind::SchemaConflict,
        ),
        (
            DatabaseError::Migration {
                store: "s.db".to_owned(),
                detail: message(),
            },
            FailureKind::Migration,
        ),
        (
            DatabaseError::Query {
                operation: op(),
                message: message(),
            },
            FailureKind::Migration,
        ),
        (
            DatabaseError::Sqlite { message: message() },
            FailureKind::Migration,
        ),
        (
            DatabaseError::Busy {
                operation: op(),
                message: message(),
            },
            FailureKind::Busy,
        ),
        (
            DatabaseError::ReadOnly {
                operation: op(),
                message: message(),
            },
            FailureKind::ReadOnly,
        ),
        (
            DatabaseError::CannotOpen {
                operation: op(),
                message: message(),
            },
            FailureKind::CannotOpen,
        ),
        (
            DatabaseError::Connection { message: message() },
            FailureKind::CannotOpen,
        ),
        (
            DatabaseError::DiskFull {
                operation: op(),
                message: message(),
            },
            FailureKind::DiskFull,
        ),
        (
            DatabaseError::Io {
                operation: op(),
                message: message(),
            },
            FailureKind::Io,
        ),
        (
            DatabaseError::Backup {
                store: "s.db".to_owned(),
                message: message(),
            },
            FailureKind::Io,
        ),
    ];
    for (error, kind) in cases {
        assert_eq!(FailureKind::of(&error), kind, "{error:?}");
        if error.is_environment_failure() {
            assert!(!kind.is_content(), "{error:?}");
        }
    }
}
