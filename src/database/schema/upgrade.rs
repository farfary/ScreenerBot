// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The upgrade every store open runs inside its one IMMEDIATE transaction: the ledger is read before any write, renamed objects are reconciled, the store is copied before a change older cores cannot read, the store's step runs, and the legacy stamp and the ledger are written.

use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, Transaction};

use crate::database::backup::back_up_before_upgrade;
use crate::database::catalog::{self, StoreClass, StoreSpec};
use crate::errors::DatabaseError;

use super::ledger::{self, Assessment, POSITIONS_FENCE};
use super::renames;
use super::types::{LegacyStamp, SchemaContext};

/// What one open's upgrade did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrade {
    pub assessment: Assessment,
    /// The copy taken before the upgrade, when one was needed.
    pub backup: Option<PathBuf>,
}

/// Whether any ledger or vault store of `stores` declares a breaking change above the
/// version it held when the ledger was adopted, which cores without the ledger cannot
/// read. While it holds, the positions stamp is raised to [`POSITIONS_FENCE`].
pub fn fence_raised(stores: &[&StoreSpec]) -> bool {
    stores.iter().any(|store| {
        matches!(store.class, StoreClass::Ledger | StoreClass::Vault)
            && store.schema.min_reader_version > store.schema.baseline
    })
}

/// Bring the store `tx` opens at `path` to `spec`'s schema. Nothing is written before
/// the ledger was read, so a store too new for this core is refused unchanged; every
/// later write commits with `tx` or not at all.
pub fn apply(
    tx: &Transaction<'_>,
    spec: &StoreSpec,
    path: &Path,
    context: &SchemaContext,
) -> Result<Upgrade, DatabaseError> {
    apply_with_fence(tx, spec, path, context, fence_raised(&catalog::STORES))
}

pub(crate) fn apply_with_fence(
    tx: &Transaction<'_>,
    spec: &StoreSpec,
    path: &Path,
    context: &SchemaContext,
    fence: bool,
) -> Result<Upgrade, DatabaseError> {
    let schema = &spec.schema;
    let store = store_name(spec, path);
    let row = ledger::read(tx)?;
    let assessment = ledger::assess(row.as_ref(), schema);
    match (assessment, &row) {
        (Assessment::TooNew, Some(row)) => {
            return Err(DatabaseError::StoreTooNew {
                store,
                version: row.version,
                min_reader_version: row.min_reader_version,
                supported_version: schema.version,
                written_by: row.written_by.clone(),
            })
        }
        (Assessment::CompatibleNewer, _) => {
            if let Some(declared) = schema.renames {
                renames::reconcile(tx, declared)?;
            }
            return Ok(Upgrade {
                assessment,
                backup: None,
            });
        }
        _ => {}
    }

    let stamp = schema
        .legacy_stamp
        .map(|legacy| read_stamp(tx, &store, legacy).map(|value| (legacy, value)))
        .transpose()?;
    if let Some(declared) = schema.renames {
        renames::reconcile(tx, declared)?;
    }

    let backup_from = if !holds_tables(tx)? {
        None
    } else if let Assessment::Older {
        from,
        breaking: true,
    } = assessment
    {
        Some(Some(from.to_string()))
    } else {
        stamp.and_then(|(legacy, value)| {
            let target = stamp_target(legacy, fence);
            value
                .is_none_or(|value| value < target)
                .then(|| value.map(|value| value.to_string()))
        })
    };
    let backup = backup_from
        .map(|from| back_up_before_upgrade(path, from.as_deref()))
        .transpose()?;

    (schema.step)(tx, context)?;

    if let Some((legacy, value)) = stamp {
        let target = stamp_target(legacy, fence);
        if value.is_none_or(|value| value < target) {
            write_stamp(tx, &store, legacy, target)?;
        }
    }
    if assessment != Assessment::Current {
        ledger::write(tx, schema)?;
    }
    Ok(Upgrade { assessment, backup })
}

/// The file name errors and logs use for the store at `path`.
pub fn store_name(spec: &StoreSpec, path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{}.db", spec.id))
}

fn stamp_target(legacy: LegacyStamp, fence: bool) -> i64 {
    if fence {
        legacy.version.max(POSITIONS_FENCE)
    } else {
        legacy.version
    }
}

fn holds_tables(tx: &Transaction<'_>) -> Result<bool, DatabaseError> {
    tx.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%')",
        [],
        |row| row.get(0),
    )
    .map_err(|error| DatabaseError::classify_sqlite_failure("inspect store tables", error))
}

fn read_stamp(
    tx: &Transaction<'_>,
    store: &str,
    legacy: LegacyStamp,
) -> Result<Option<i64>, DatabaseError> {
    let classify =
        |error| DatabaseError::classify_sqlite_failure("read legacy schema stamp", error);
    if !super::inspect::table_exists(tx, legacy.table).map_err(classify)? {
        return Ok(None);
    }
    let value: Option<String> = tx
        .query_row(
            &format!("SELECT value FROM {} WHERE key = ?1", legacy.table),
            [legacy.key],
            |row| row.get(0),
        )
        .optional()
        .map_err(classify)?;
    value
        .map(|value| {
            value.parse().map_err(|_| DatabaseError::Migration {
                store: store.to_owned(),
                detail: format!("unreadable {}.{} value {value:?}", legacy.table, legacy.key),
            })
        })
        .transpose()
}

fn write_stamp(
    tx: &Transaction<'_>,
    store: &str,
    legacy: LegacyStamp,
    value: i64,
) -> Result<(), DatabaseError> {
    tx.execute(
        &format!(
            "INSERT OR REPLACE INTO {} (key, value) VALUES (?1, ?2)",
            legacy.table
        ),
        rusqlite::params![legacy.key, value.to_string()],
    )
    .map_err(|error| DatabaseError::Migration {
        store: store.to_owned(),
        detail: format!("write {}.{}: {error}", legacy.table, legacy.key),
    })?;
    Ok(())
}

#[cfg(test)]
#[path = "upgrade_tests.rs"]
mod tests;
