// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Snapshots of ledger and vault stores: a checked backup-API copy at startup, every six hours while the store changes and after every change to the vault's keys, kept by class, and the newest copy that still verifies when a damaged store is restored.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{Connection, OpenFlags};

use crate::database::catalog::{StoreClass, StoreSpec};
use crate::database::handles::{self, StoreHandle};
use crate::database::health;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::paths::{get_db_with_wal_files, BACKUPS_DIRECTORY_NAME};

use super::{copy_store, quick_check};

/// Name of the directory inside the backups directory that holds one directory of
/// snapshots per store.
pub const SNAPSHOTS_DIRECTORY_NAME: &str = "snapshots";

/// How often a changed ledger or vault store is snapshotted while the app runs.
pub const PERIOD: Duration = Duration::from_secs(6 * 60 * 60);

/// Startup snapshots a ledger store keeps.
const LEDGER_BOOT_KEPT: usize = 3;
/// Days of periodic snapshots a ledger store keeps, one per day.
const LEDGER_DAYS_KEPT: usize = 7;
/// Snapshots the vault keeps, of either kind.
const VAULT_KEPT: usize = 10;

/// A snapshot is skipped while the disk has less free space than this multiple of the
/// store's size.
const FREE_SPACE_FACTOR: u64 = 2;

const STAMP_FORMAT: &str = "%Y%m%dT%H%M%S%6fZ";

/// Why a snapshot was taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotKind {
    /// Taken once the app is ready.
    Boot,
    /// Taken by the six-hour schedule because the store changed.
    Periodic,
    /// Taken after the vault's keys or its main wallet changed.
    KeyChange,
}

impl SnapshotKind {
    const fn suffix(self) -> &'static str {
        match self {
            SnapshotKind::Boot => "boot",
            SnapshotKind::Periodic => "periodic",
            SnapshotKind::KeyChange => "keys",
        }
    }
}

/// One snapshot file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub path: PathBuf,
    pub taken_at: DateTime<Utc>,
    pub kind: SnapshotKind,
}

/// Whether stores of `class` are snapshotted. Derived and cache stores are rebuilt
/// instead.
pub const fn is_snapshotted(class: StoreClass) -> bool {
    matches!(class, StoreClass::Ledger | StoreClass::Vault)
}

/// The directory holding the snapshots of the store at `path`:
/// `backups/snapshots/<stem>` beside the store.
pub fn directory(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    parent
        .join(BACKUPS_DIRECTORY_NAME)
        .join(SNAPSHOTS_DIRECTORY_NAME)
        .join(stem)
}

/// The snapshots of the store at `path`, newest first. Files that are not snapshots
/// are ignored.
pub fn list(path: &Path) -> Vec<Snapshot> {
    let Ok(entries) = fs::read_dir(directory(path)) else {
        return Vec::new();
    };
    let mut snapshots: Vec<Snapshot> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| parse(&entry.path()))
        .collect();
    snapshots.sort_by_key(|snapshot| std::cmp::Reverse(snapshot.taken_at));
    snapshots
}

fn parse(path: &Path) -> Option<Snapshot> {
    if path.extension()? != "db" {
        return None;
    }
    let (stamp, kind) = path.file_stem()?.to_str()?.rsplit_once('-')?;
    let kind = match kind {
        "boot" => SnapshotKind::Boot,
        "periodic" => SnapshotKind::Periodic,
        "keys" => SnapshotKind::KeyChange,
        _ => return None,
    };
    let taken_at = NaiveDateTime::parse_from_str(stamp, STAMP_FORMAT)
        .ok()?
        .and_utc();
    Some(Snapshot {
        path: path.to_path_buf(),
        taken_at,
        kind,
    })
}

/// Snapshot the store `spec` opened at `path`. Returns the new file, or `None` when the
/// class is not snapshotted, the store does not exist yet or the disk is too full; a
/// skipped snapshot is logged. Older snapshots beyond the class's retention are removed.
pub fn take(
    spec: &StoreSpec,
    path: &Path,
    kind: SnapshotKind,
) -> Result<Option<PathBuf>, DatabaseError> {
    if !is_snapshotted(spec.class) || !path.exists() {
        return Ok(None);
    }
    let directory = directory(path);
    fs::create_dir_all(&directory).map_err(|error| DatabaseError::Backup {
        store: spec.id.to_owned(),
        message: format!("create {}: {error}", directory.display()),
    })?;
    let size = store_size(path);
    if let Some(disk) = crate::storage::usage::measure_disk(&directory) {
        if disk.free < size.saturating_mul(FREE_SPACE_FACTOR) {
            logger::warning(
                LogTag::System,
                &format!(
                    "{} snapshot skipped: {} bytes free, {} needed",
                    spec.id,
                    disk.free,
                    size.saturating_mul(FREE_SPACE_FACTOR)
                ),
            );
            return Ok(None);
        }
    }
    let file = directory.join(format!(
        "{}-{}.db",
        Utc::now().format(STAMP_FORMAT),
        kind.suffix()
    ));
    copy_store(path, &file, spec.id)?;
    remember(path);
    prune(spec, path);
    Ok(Some(file))
}

/// The newest snapshot of the store at `path` that passes `quick_check` and, for the
/// vault, its key check.
pub fn latest_verified(spec: &StoreSpec, path: &Path) -> Option<Snapshot> {
    list(path).into_iter().find(|snapshot| {
        let candidate = &snapshot.path;
        quick_check(candidate).is_ok_and(|check| check == "ok")
            && spec.verify_snapshot.is_none_or(|verify| {
                Connection::open_with_flags(candidate, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(DatabaseError::from)
                    .and_then(|connection| verify(&connection))
                    .unwrap_or(false)
            })
    })
}

/// The snapshots of `path` the class's retention no longer keeps.
pub fn expired(class: StoreClass, snapshots: &[Snapshot]) -> Vec<PathBuf> {
    if class == StoreClass::Vault {
        return snapshots
            .iter()
            .skip(VAULT_KEPT)
            .map(|snapshot| snapshot.path.clone())
            .collect();
    }
    let mut boots = 0;
    let mut days = Vec::new();
    snapshots
        .iter()
        .filter(|snapshot| match snapshot.kind {
            SnapshotKind::Boot | SnapshotKind::KeyChange => {
                boots += 1;
                boots > LEDGER_BOOT_KEPT
            }
            SnapshotKind::Periodic => {
                let day = snapshot.taken_at.date_naive();
                if days.contains(&day) || days.len() >= LEDGER_DAYS_KEPT {
                    true
                } else {
                    days.push(day);
                    false
                }
            }
        })
        .map(|snapshot| snapshot.path.clone())
        .collect()
}

fn prune(spec: &StoreSpec, path: &Path) {
    for file in expired(spec.class, &list(path)) {
        if let Err(error) = fs::remove_file(&file) {
            logger::warning(
                LogTag::System,
                &format!(
                    "{}: could not remove expired snapshot {}: {error}",
                    spec.id,
                    file.display()
                ),
            );
        }
    }
}

fn store_size(path: &Path) -> u64 {
    get_db_with_wal_files(path.to_path_buf())
        .iter()
        .filter_map(|file| fs::metadata(file).ok())
        .map(|metadata| metadata.len())
        .sum()
}

/// Size and modification time of the store file and its write-ahead log; a store whose
/// fingerprint is unchanged since its last snapshot is not snapshotted again.
type Fingerprint = Vec<Option<(u64, SystemTime)>>;

static LAST_SNAPSHOT: LazyLock<Mutex<HashMap<PathBuf, Fingerprint>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn fingerprint(path: &Path) -> Fingerprint {
    [path.to_path_buf(), path.with_extension("db-wal")]
        .iter()
        .map(|file| {
            fs::metadata(file)
                .ok()
                .and_then(|metadata| Some((metadata.len(), metadata.modified().ok()?)))
        })
        .collect()
}

fn remember(path: &Path) {
    LAST_SNAPSHOT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(path.to_path_buf(), fingerprint(path));
}

fn changed_since_last(path: &Path) -> bool {
    LAST_SNAPSHOT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(path)
        .is_none_or(|last| *last != fingerprint(path))
}

/// Snapshot every open ledger and vault store now, and again every [`PERIOD`] while
/// it changed. Called once the app is ready.
pub fn start_schedule() {
    tokio::spawn(async {
        snapshot_open_stores(SnapshotKind::Boot).await;
        let mut ticks = tokio::time::interval(PERIOD);
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        ticks.tick().await;
        loop {
            ticks.tick().await;
            snapshot_open_stores(SnapshotKind::Periodic).await;
        }
    });
}

async fn snapshot_open_stores(kind: SnapshotKind) {
    let outcome =
        tokio::task::spawn_blocking(move || snapshot_stores(&handles::open_stores(), kind)).await;
    if let Err(error) = outcome {
        logger::warning(LogTag::System, &format!("Store snapshots stopped: {error}"));
    }
}

/// Snapshot each ledger and vault store of `stores`; a periodic pass skips a store that
/// has not changed. A store the health registry reports as not open is skipped and never
/// opened here: its opener left it unavailable or is replacing it.
fn snapshot_stores(stores: &[StoreHandle], kind: SnapshotKind) {
    for handle in stores {
        if !is_snapshotted(handle.spec.class)
            || (kind == SnapshotKind::Periodic && !changed_since_last(&handle.path))
        {
            continue;
        }
        if let Some(status) = health::status(&handle.path).filter(|status| !status.health.is_open())
        {
            logger::debug(
                LogTag::System,
                &format!(
                    "{} snapshot skipped: the store is {}",
                    handle.spec.id,
                    status.health.as_str()
                ),
            );
            continue;
        }
        match take(handle.spec, &handle.path, kind) {
            Ok(Some(file)) => logger::debug(
                LogTag::System,
                &format!("{} snapshot written to {}", handle.spec.id, file.display()),
            ),
            Ok(None) => {}
            Err(error) => logger::warning(
                LogTag::System,
                &format!("{} snapshot failed: {error}", handle.spec.id),
            ),
        }
    }
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
