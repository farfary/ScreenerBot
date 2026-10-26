// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The one way a store is opened: probe, check, upgrade inside one IMMEDIATE transaction, recover a failed open by the decision table, and hand every owner the same cached connection pool.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::chains::{ChainId, ChainScope};
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};

use super::backup::snapshot::{self, Snapshot};
use super::backup::{WalletResetManifest, WalletResetReason};
use super::catalog::{self, PoolShape, StoreClass, StoreLocation, StoreSpec};
use super::configure::{configure_connection, DbConfig};
use super::health::{self, StoreHealth};
use super::recovery::{self, policy::BUSY_RETRY_BUDGET, FailureKind, RecoveryAction};
use super::schema::{upgrade, SchemaContext};
use super::{handles, WriteTransaction};

/// A connection pool on one store file.
pub type SqlitePool = Pool<SqliteConnectionManager>;

/// The longest pause between two opens of a store another program holds locked.
const BUSY_RETRY_MAX_PAUSE: Duration = Duration::from_secs(1);

type Slot = Arc<Mutex<Option<SqlitePool>>>;

/// The pool of every store file opened in this process, one slot per file so a slow
/// open of one store never holds up another.
static POOLS: LazyLock<Mutex<HashMap<PathBuf, Slot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static READERS: LazyLock<Mutex<HashMap<PathBuf, Slot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// The error each store file's failed open ended with, kept until the process restarts.
/// A failed open has already spent its retry budget and published the store as
/// unavailable; opening it again would repeat the probe and `quick_check` on every
/// caller without a different outcome.
static FAILED: LazyLock<Mutex<HashMap<PathBuf, DatabaseError>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The pool of the shared store `spec`, opening it on first use.
pub fn store(spec: &'static StoreSpec) -> Result<SqlitePool, DatabaseError> {
    let path = spec.path(crate::chains::legacy_row_chain());
    pool_at(spec, &path, shared_chains())
}

/// The pool of the per-chain store `spec` for `chain`, opening it on first use.
pub fn chain_store(spec: &'static StoreSpec, chain: ChainId) -> Result<SqlitePool, DatabaseError> {
    pool_at(spec, &spec.path(chain), vec![chain])
}

/// The pool of the store `spec` at `path`. One of the store's own files is opened once
/// per process and every later caller shares its pool; a failed open is not tried again
/// until the process restarts, and every later caller receives its error. Any other file
/// (an isolated test store, an offline copy) is opened afresh on every call.
pub fn pool_at(
    spec: &'static StoreSpec,
    path: &Path,
    chains: Vec<ChainId>,
) -> Result<SqlitePool, DatabaseError> {
    let context = SchemaContext { chains };
    if is_store_file(spec, path) {
        cached(&POOLS, path, || {
            open_once(path, || open(spec, path, &context))
        })
    } else {
        open(spec, path, &context)
    }
}

fn is_store_file(spec: &StoreSpec, path: &Path) -> bool {
    spec.paths().iter().any(|file| file == path)
}

/// The read-only pool of the store `spec` at `path`, for a store whose readers must not
/// wait behind its writer. The store is opened first, so its schema is current.
pub fn reader(spec: &'static StoreSpec, path: &Path) -> Result<SqlitePool, DatabaseError> {
    let Some(reader) = spec.reader else {
        return pool_at(spec, path, shared_chains());
    };
    pool_at(spec, path, shared_chains())?;
    if is_store_file(spec, path) {
        cached(&READERS, path, || {
            build_pool(path, reader.config, reader.shape)
        })
    } else {
        build_pool(path, reader.config, reader.shape)
    }
}

fn shared_chains() -> Vec<ChainId> {
    ChainScope::All.chains()
}

fn cached(
    pools: &Mutex<HashMap<PathBuf, Slot>>,
    path: &Path,
    open: impl FnOnce() -> Result<SqlitePool, DatabaseError>,
) -> Result<SqlitePool, DatabaseError> {
    let slot = pools
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(path.to_path_buf())
        .or_default()
        .clone();
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(pool) = slot.as_ref() {
        return Ok(pool.clone());
    }
    let pool = open()?;
    *slot = Some(pool.clone());
    Ok(pool)
}

/// Run `open` unless an earlier open of `path` failed, in which case its error is
/// returned without touching the file. A failure is recorded for good.
fn open_once(
    path: &Path,
    open: impl FnOnce() -> Result<SqlitePool, DatabaseError>,
) -> Result<SqlitePool, DatabaseError> {
    if let Some(error) = FAILED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(path)
    {
        return Err(error.clone());
    }
    open().inspect_err(|error| {
        FAILED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_path_buf(), error.clone());
    })
}

/// Open the store, recovering a failed open as the decision table says.
fn open(
    spec: &'static StoreSpec,
    path: &Path,
    context: &SchemaContext,
) -> Result<SqlitePool, DatabaseError> {
    let started = Instant::now();
    let mut pause = Duration::from_millis(100);
    let mut retried = false;
    loop {
        let error = match attempt(spec, path, context) {
            Ok(pool) => return Ok(opened(spec, path, pool, StoreHealth::Healthy)),
            Err(error) => error,
        };
        let failure = FailureKind::of(&error);
        let verified = if failure.is_content() && snapshot::is_snapshotted(spec.class) {
            snapshot::latest_verified(spec, path)
        } else {
            None
        };
        let action = recovery::decide(spec.class, failure, verified.is_some());
        match (action, verified) {
            (RecoveryAction::RetryUntilBudget, _) if started.elapsed() < BUSY_RETRY_BUDGET => {
                std::thread::sleep(pause);
                pause = (pause * 2).min(BUSY_RETRY_MAX_PAUSE);
            }
            (RecoveryAction::RetryOnce, _) if !retried => retried = true,
            (RecoveryAction::Rebuild { salvage }, _) => {
                return rebuild(spec, path, context, &error, salvage)
            }
            (RecoveryAction::Restore, Some(snapshot)) => {
                return restore(spec, path, context, &error, &snapshot)
            }
            _ => return Err(unavailable(spec, path, error, action)),
        }
    }
}

/// One open: the file is probed, a ledger or vault store is checked, and the upgrade
/// runs inside one IMMEDIATE transaction before any pool connection exists.
fn attempt(
    spec: &StoreSpec,
    path: &Path,
    context: &SchemaContext,
) -> Result<SqlitePool, DatabaseError> {
    let operation = |step: &str| format!("{step} {}", path.display());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| DatabaseError::Io {
            operation: operation("create the directory of"),
            message: error.to_string(),
        })?;
    }
    recovery::probe(path)?;
    {
        let mut connection = Connection::open(path)
            .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("open"), error))?;
        configure_connection(&connection, spec.config).map_err(|error| {
            DatabaseError::classify_sqlite_failure(&operation("configure"), error)
        })?;
        if matches!(spec.class, StoreClass::Vault | StoreClass::Ledger) {
            check(&connection, &operation("check"))?;
        }
        // Foreign keys are off while the schema changes, so a table rebuilt by a step
        // never cascades into the rows that reference it.
        connection
            .pragma_update(None, "foreign_keys", "OFF")
            .map_err(|error| DatabaseError::classify_sqlite_failure(&operation("open"), error))?;
        let transaction = connection.write_tx().map_err(|error| {
            DatabaseError::classify_sqlite_failure(&operation("begin the upgrade of"), error)
        })?;
        let upgrade = upgrade::apply(&transaction, spec, path, context)?;
        transaction.commit().map_err(|error| {
            DatabaseError::classify_sqlite_failure(&operation("commit the upgrade of"), error)
        })?;
        if let Some(backup) = upgrade.backup {
            logger::info(
                LogTag::System,
                &format!(
                    "{}: copied to {} before its upgrade",
                    spec.id,
                    backup.display()
                ),
            );
        }
    }
    build_pool(path, spec.config, spec.pool)
}

/// `quick_check` of the store: a store whose pages do not check out is damaged.
fn check(connection: &Connection, operation: &str) -> Result<(), DatabaseError> {
    let result: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| DatabaseError::classify_sqlite_failure(operation, error))?;
    if result == "ok" {
        Ok(())
    } else {
        Err(DatabaseError::Corrupt {
            operation: operation.to_owned(),
            message: result,
        })
    }
}

fn build_pool(
    path: &Path,
    config: DbConfig,
    shape: PoolShape,
) -> Result<SqlitePool, DatabaseError> {
    let manager = SqliteConnectionManager::file(path)
        .with_init(move |connection| configure_connection(connection, config));
    Pool::builder()
        .max_size(shape.max_size)
        .min_idle(shape.min_idle)
        .connection_timeout(shape.connection_timeout)
        // Connections are never recycled: each holds the WAL read mark and page cache
        // of a long-lived SQLite handle.
        .idle_timeout(None)
        .max_lifetime(None)
        .build(manager)
        .map_err(DatabaseError::from)
}

fn opened(
    spec: &'static StoreSpec,
    path: &Path,
    pool: SqlitePool,
    state: StoreHealth,
) -> SqlitePool {
    handles::register(spec, path, &pool);
    health::publish(spec, path, state);
    pool
}

fn unavailable(
    spec: &StoreSpec,
    path: &Path,
    error: DatabaseError,
    action: RecoveryAction,
) -> DatabaseError {
    health::publish(spec, path, StoreHealth::unavailable(&error, action));
    error
}

/// Move the store to quarantine, create it afresh and, with `salvage`, copy its salvage
/// tables back from the moved file.
fn rebuild(
    spec: &'static StoreSpec,
    path: &Path,
    context: &SchemaContext,
    cause: &DatabaseError,
    salvage: bool,
) -> Result<SqlitePool, DatabaseError> {
    health::publish(
        spec,
        path,
        StoreHealth::Rebuilding {
            since: Utc::now(),
            phase: "quarantine".to_owned(),
        },
    );
    let quarantined = recovery::quarantine(spec, path, &cause.to_string())
        .map_err(|error| unavailable(spec, path, error, RecoveryAction::Unavailable))?;
    let pool = attempt(spec, path, context)
        .map_err(|error| unavailable(spec, path, error, RecoveryAction::Unavailable))?;
    if salvage && !spec.salvage.is_empty() {
        match recovery::salvage(spec, &quarantined.store_file, path) {
            Ok(rows) => logger::info(
                LogTag::System,
                &format!("{}: {rows} rows salvaged from the moved file", spec.id),
            ),
            Err(error) => logger::warning(
                LogTag::System,
                &format!("{}: nothing salvaged from the moved file: {error}", spec.id),
            ),
        }
    }
    Ok(opened(
        spec,
        path,
        pool,
        StoreHealth::Recovered {
            quarantine: quarantined.directory,
        },
    ))
}

/// Move the damaged store to quarantine and open a copy of its latest verified snapshot
/// in its place.
fn restore(
    spec: &'static StoreSpec,
    path: &Path,
    context: &SchemaContext,
    cause: &DatabaseError,
    snapshot: &Snapshot,
) -> Result<SqlitePool, DatabaseError> {
    health::publish(
        spec,
        path,
        StoreHealth::Rebuilding {
            since: Utc::now(),
            phase: "restore".to_owned(),
        },
    );
    let quarantined = recovery::restore(spec, path, &snapshot.path, &cause.to_string())
        .map_err(|error| unavailable(spec, path, error, RecoveryAction::Unavailable))?;
    let pool = attempt(spec, path, context)
        .map_err(|error| unavailable(spec, path, error, RecoveryAction::Unavailable))?;
    Ok(opened(
        spec,
        path,
        pool,
        StoreHealth::Restored {
            snapshot_at: Some(snapshot.taken_at),
            quarantine: quarantined.directory,
        },
    ))
}

/// Every store file the storage phase opens: a shared store once, a per-chain store on
/// every enabled chain.
fn phase_targets(spec: &StoreSpec) -> Vec<(PathBuf, Vec<ChainId>)> {
    match spec.location {
        StoreLocation::Shared(_) => {
            vec![(
                spec.path(crate::chains::legacy_row_chain()),
                shared_chains(),
            )]
        }
        StoreLocation::PerChain(_) => shared_chains()
            .into_iter()
            .map(|chain| (spec.path(chain), vec![chain]))
            .collect(),
    }
}

/// The storage phase: open every catalog store, vault first and caches last, before any
/// service reads one. A store that stays unavailable is recorded in the health registry
/// and its owner receives the error when it asks for the store; the phase itself never
/// fails.
pub fn open_all() -> Duration {
    let started = Instant::now();
    for spec in catalog::STORES {
        for (path, chains) in phase_targets(spec) {
            if let Err(error) = pool_at(spec, &path, chains) {
                logger::error(
                    LogTag::System,
                    &format!("{} could not be opened: {error}", spec.id),
                );
            }
        }
    }
    let elapsed = started.elapsed();
    logger::info(
        LogTag::System,
        &format!(
            "Storage phase opened {} store files in {} ms",
            handles::open_stores().len(),
            elapsed.as_millis()
        ),
    );
    elapsed
}

/// Whether the store file at `path` has a pool in this process.
pub(super) fn is_pooled(path: &Path) -> bool {
    POOLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains_key(path)
}

/// Why the wallet data reset refused to move the wallet-scoped stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WalletResetRefusal {
    /// The trading wallet has open positions in the positions store.
    #[error("the trading wallet has {count} open position(s)")]
    OpenPositions { count: usize },
    /// The positions store could not be read, so open positions cannot be ruled out.
    #[error("the positions store could not be read")]
    PositionsUnreadable,
}

/// Why the wallet data reset did not complete.
#[derive(Debug, thiserror::Error)]
pub enum WalletResetFailure {
    /// The reset refused before moving anything.
    #[error(transparent)]
    Refused(#[from] WalletResetRefusal),
    /// The move failed, and every file it had moved is back at its original path.
    #[error("the wallet data reset failed and every file is back in place: {0}")]
    Restored(DatabaseError),
    /// The move failed, and the files it had moved could not all be moved back: some
    /// wallet-scoped files are in `directory`, the rest at their original paths.
    #[error("the wallet data reset failed ({error}) and moving its files back from {} failed: {restore_error}", directory.display())]
    Split {
        directory: PathBuf,
        error: DatabaseError,
        restore_error: DatabaseError,
    },
}

/// Move every file of the wallet-scoped stores into the new wallet-reset backup
/// `directory` (see [`super::backup::wallet_reset_directory`]) with a manifest, so the
/// stores open empty for the current wallet. Runs under the process lock before the
/// storage phase. Refuses while the positions store holds an open position of one of
/// `trading_wallets`, the vault's trading wallets, or cannot be read; open positions of
/// other wallets move with the stores and the manifest counts them. A move that fails
/// part-way moves its files back, so the stores are never left split unless that fails
/// too. Returns the backup directory, or `None` when there was nothing to move.
pub fn reset_wallet_stores(
    directory: &Path,
    trading_wallets: &[String],
) -> Result<Option<PathBuf>, WalletResetFailure> {
    let positions = catalog::POSITIONS.path(crate::chains::legacy_row_chain());
    let mut count = 0;
    for wallet in trading_wallets {
        count += crate::positions::database::count_open_positions_in_file(&positions, wallet)
            .map_err(|_| WalletResetRefusal::PositionsUnreadable)?;
    }
    if count > 0 {
        return Err(WalletResetRefusal::OpenPositions { count }.into());
    }
    let present = catalog::wallet_scoped_stores()
        .flat_map(|(spec, _)| spec.paths())
        .any(|path| path.exists());
    if !present {
        return Ok(None);
    }

    let moved = super::backup::move_wallet_stores(
        directory,
        WalletResetManifest {
            reason: WalletResetReason::WalletReset,
            chain: crate::chains::legacy_row_chain().as_str().to_owned(),
            previous_wallet: None,
            new_wallet: None,
            core_version: env!("CARGO_PKG_VERSION").to_owned(),
            created_at_ms: Utc::now().timestamp_millis(),
            files: Vec::new(),
            moved_open_positions: Default::default(),
        },
    );
    let manifest = match moved {
        Ok(manifest) => manifest,
        Err(error) => return Err(restore_after_failed_reset(directory, error)),
    };
    logger::info(
        LogTag::System,
        &format!(
            "Wallet data reset moved {} to {}; open positions moved by wallet: {:?}",
            manifest.files.join(", "),
            directory.display(),
            manifest.moved_open_positions
        ),
    );
    Ok(Some(directory.to_path_buf()))
}

/// Move back what a failed wallet data reset moved into `directory`, and remove the
/// directory when that leaves it empty.
fn restore_after_failed_reset(directory: &Path, error: DatabaseError) -> WalletResetFailure {
    match super::backup::restore_wallet_stores(directory) {
        Ok(restored) => {
            logger::error(
                LogTag::System,
                &format!(
                    "Wallet data reset failed and moved {} back: {error}",
                    if restored.is_empty() {
                        "nothing".to_owned()
                    } else {
                        restored.join(", ")
                    }
                ),
            );
            // Removes only an empty directory; a file left in it keeps it.
            let _ = fs::remove_dir(directory);
            WalletResetFailure::Restored(error)
        }
        Err(restore_error) => WalletResetFailure::Split {
            directory: directory.to_path_buf(),
            error,
            restore_error,
        },
    }
}

/// Run the whole upgrade of `spec` on `connection` in one IMMEDIATE transaction, as an
/// open does, for tests of a store's schema on a scratch connection.
#[cfg(test)]
pub(crate) fn upgrade_connection(
    connection: &mut Connection,
    spec: &StoreSpec,
    chains: Vec<ChainId>,
) -> Result<(), DatabaseError> {
    let path = PathBuf::from(connection.path().unwrap_or_default());
    let transaction = connection.write_tx()?;
    upgrade::apply(&transaction, spec, &path, &SchemaContext { chains })?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
