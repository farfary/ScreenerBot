// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The housekeeping task: when a pass runs, how it measures and plans, and how each planned deletion is re-checked before it is carried out and each store operation reaches its store only through the owner's pool.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use crate::database::catalog::Compaction;
use crate::database::handles::{self, StoreHandle};
use crate::database::maintenance;
use crate::errors::StorageError;
use crate::logger::{self, LogTag};
use crate::paths;
use crate::process::shutdown::is_shutdown_requested;

use super::eviction;
use super::families::{classify, family};
use super::plan::{category_of, disk_tier, plan};
use super::report::{self, LastCleanup, StorageReport};
use super::types::{Action, Category, Classification, FamilyId, Limit, Plan, PlanInput, Tier};
use super::usage::{measure_disk, measure_entries, measure_stores};

/// Delay before the first pass, so startup finishes opening its stores first.
const FIRST_PASS_DELAY: Duration = Duration::from_secs(60);
/// How often a scheduled pass runs.
const PASS_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// How often a scheduled pass may convert or compact a store.
const COMPACTION_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// How often free disk space is probed; a change of disk tier starts a pass at once.
const DISK_PROBE_INTERVAL: Duration = Duration::from_secs(5 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

static STARTED: AtomicBool = AtomicBool::new(false);
/// Held for the whole of a pass, so scheduled and requested passes never overlap.
static PASS_LOCK: LazyLock<Arc<tokio::sync::Mutex<()>>> =
    LazyLock::new(|| Arc::new(tokio::sync::Mutex::new(())));
static RUNNING: AtomicBool = AtomicBool::new(false);
static LAST_REPORT: Mutex<Option<StorageReport>> = Mutex::new(None);
static LAST_CLEANUP: Mutex<Option<LastCleanup>> = Mutex::new(None);

/// What asking for a cleanup did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupRequest {
    Started,
    AlreadyRunning,
}

/// Start the housekeeping task once per process. A pass runs every hour, store
/// compaction at most once a day, and a pass starts early when free disk space
/// crosses into another tier.
pub fn start_housekeeping() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(run_housekeeping());
}

async fn run_housekeeping() {
    logger::info(
        LogTag::System,
        &format!(
            "Storage housekeeping scheduled: first pass in {}s, then every {}s, compaction every {}s",
            FIRST_PASS_DELAY.as_secs(),
            PASS_INTERVAL.as_secs(),
            COMPACTION_INTERVAL.as_secs()
        ),
    );
    if let Err(error) = report().await {
        logger::warning(
            LogTag::System,
            &format!("Storage could not be measured at startup: {error}"),
        );
    }
    tokio::time::sleep(FIRST_PASS_DELAY).await;

    let mut last_compaction: Option<Instant> = None;
    let mut last_tier = Tier::Normal;
    let mut passes = tokio::time::interval(PASS_INTERVAL);
    passes.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut probes = tokio::time::interval(DISK_PROBE_INTERVAL);
    probes.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    probes.tick().await;

    loop {
        tokio::select! {
            _ = passes.tick() => {}
            _ = probes.tick() => {
                let free = measure_disk(&paths::get_data_directory()).map(|disk| disk.free);
                if disk_tier(free) == last_tier {
                    continue;
                }
            }
        }
        if is_shutdown_requested() {
            return;
        }
        let compaction_due =
            last_compaction.is_none_or(|last| last.elapsed() >= COMPACTION_INTERVAL);
        let _pass = PASS_LOCK.lock().await;
        match run_pass(compaction_due).await {
            Ok(plan) => {
                last_tier = plan.tier;
                if compaction_due {
                    last_compaction = Some(Instant::now());
                }
            }
            Err(error) => logger::warning(
                LogTag::System,
                &format!("Storage housekeeping pass failed: {error}"),
            ),
        }
    }
}

/// Start a pass now, with compaction, unless one is running. The pass runs in the
/// background; its result is the next report's last cleanup.
pub fn request_cleanup() -> CleanupRequest {
    let Ok(guard) = Arc::clone(&PASS_LOCK).try_lock_owned() else {
        return CleanupRequest::AlreadyRunning;
    };
    RUNNING.store(true, Ordering::SeqCst);
    tokio::spawn(async move {
        let _pass = guard;
        logger::info(LogTag::System, "Storage cleanup requested");
        if let Err(error) = run_pass(true).await {
            logger::warning(
                LogTag::System,
                &format!("Requested storage cleanup failed: {error}"),
            );
        }
    });
    CleanupRequest::Started
}

/// The storage report: the last measurement, taken now when there is none yet.
pub async fn report() -> Result<StorageReport, StorageError> {
    let cached = LAST_REPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let mut report = match cached {
        Some(report) => report,
        None => measure_report().await?,
    };
    report.apply_limit(configured_megabytes());
    report.running = RUNNING.load(Ordering::SeqCst);
    Ok(report)
}

/// Measure the app's storage and keep the result as the current report.
async fn measure_report() -> Result<StorageReport, StorageError> {
    let limit = configured_megabytes();
    let mut report = blocking("measure storage", move || {
        Ok(report::summarize(
            &measure_entries()?,
            &measure_stores(),
            measure_disk(&paths::get_data_directory()),
            limit,
            SystemTime::now(),
        ))
    })
    .await?;
    report.last_cleanup = *LAST_CLEANUP.lock().unwrap_or_else(PoisonError::into_inner);
    *LAST_REPORT.lock().unwrap_or_else(PoisonError::into_inner) = Some(report.clone());
    Ok(report)
}

fn configured_megabytes() -> u64 {
    crate::config::with_config(|cfg| cfg.maintenance.storage_limit_mb)
}

/// Data used within this span is never evicted.
fn eviction_floor() -> Duration {
    let days = crate::config::with_config(|cfg| cfg.maintenance.stale_token_days).max(1);
    DAY * days
}

/// Measure, plan and carry out one pass, then measure again for the report. When
/// deleting files and reclaiming free pages cannot bring usage within the budget,
/// the tokens in use are listed, the rebuildable units the stores offer are
/// collected and the pass is planned again with them. Returns the plan it carried
/// out.
pub async fn run_pass(compaction_due: bool) -> Result<Plan, StorageError> {
    RUNNING.store(true, Ordering::SeqCst);
    let result = pass(compaction_due).await;
    RUNNING.store(false, Ordering::SeqCst);
    result
}

async fn pass(compaction_due: bool) -> Result<Plan, StorageError> {
    let started = Instant::now();
    let downloads = crate::version::retention::download_facts().await;
    let active_log = logger::active_log_file();
    let limit = Limit::from_megabytes(configured_megabytes());
    let mut input = blocking("measure storage", move || {
        Ok(PlanInput {
            entries: measure_entries()?,
            stores: measure_stores(),
            evictables: Vec::new(),
            disk: measure_disk(&paths::get_data_directory()),
            limit,
            now: SystemTime::now(),
            active_log,
            downloads,
            compaction_due,
        })
    })
    .await?;
    let mut plan = plan(&input);
    if !plan.fits {
        match crate::tokens::retention::protected_mints().await {
            Ok(protected) => {
                let floor = eviction_floor();
                let (stores, now) = (input.stores.clone(), input.now);
                let (evictables, failures) = blocking("collect evictable data", move || {
                    Ok(eviction::collect(
                        &handles::open_stores(),
                        &stores,
                        &protected,
                        floor,
                        now,
                    ))
                })
                .await?;
                for error in failures {
                    logger::warning(
                        LogTag::System,
                        &format!("Storage housekeeping could not list evictable data: {error}"),
                    );
                }
                input.evictables = evictables;
                plan = super::plan::plan(&input);
            }
            Err(reason) => logger::info(
                LogTag::System,
                &format!("Storage housekeeping evicts nothing this pass: {reason:?}"),
            ),
        }
    }
    let actions = plan.actions.clone();
    let summary = blocking("carry out the storage plan", move || {
        Ok(carry_out(&actions, &handles::open_stores()))
    })
    .await?;
    let after = measure_report().await?;
    let last = LastCleanup {
        finished_at: report::unix_seconds(SystemTime::now()),
        freed_bytes: plan.usage.saturating_sub(after.total_bytes),
        deleted_files: summary.deleted,
        evicted_rows: summary.evicted_rows,
        failures: summary.failures,
    };
    *LAST_CLEANUP.lock().unwrap_or_else(PoisonError::into_inner) = Some(last);
    if let Some(cached) = LAST_REPORT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        cached.last_cleanup = Some(last);
    }

    let freed: Vec<String> = summary
        .freed
        .iter()
        .map(|(category, bytes)| format!("{category:?} {bytes} B"))
        .collect();
    logger::info(
        LogTag::System,
        &format!(
            "Storage housekeeping: tier {}, usage {} B of {} B budget, now {} B, deleted {} entries ({}), evicted {} rows in {} units, {} store operations, {} failures, shortfall {} B, {} ms",
            plan.tier.as_str(),
            plan.usage,
            plan.limit,
            after.total_bytes,
            summary.deleted,
            if freed.is_empty() { "nothing freed".to_owned() } else { freed.join(", ") },
            summary.evicted_rows,
            summary.evictions,
            summary.store_operations,
            summary.failures,
            plan.shortfall,
            started.elapsed().as_millis()
        ),
    );
    Ok(plan)
}

async fn blocking<T, F>(operation: &str, work: F) -> Result<T, StorageError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, StorageError> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| StorageError::Interrupted {
            operation: operation.to_owned(),
            message: error.to_string(),
        })?
}

/// What carrying out a plan did.
#[derive(Debug, Default)]
pub struct Outcome {
    pub deleted: usize,
    pub freed: BTreeMap<Category, u64>,
    pub evictions: usize,
    pub evicted_rows: u64,
    pub store_operations: usize,
    pub failures: usize,
}

/// Carry out every action of a plan. A failed action is logged and the rest still
/// run; the next pass re-measures and plans again.
pub fn carry_out(actions: &[Action], stores: &[StoreHandle]) -> Outcome {
    let mut outcome = Outcome::default();
    for action in actions {
        let result = match action {
            Action::Delete {
                path,
                family,
                bytes,
                tier,
            } => delete_entry(path, *family).map(|()| {
                logger::info(
                    LogTag::System,
                    &format!(
                        "Storage housekeeping deleted {} ({:?}, {bytes} B, {} rule)",
                        path.display(),
                        family,
                        tier.as_str()
                    ),
                );
                outcome.deleted += 1;
                *outcome.freed.entry(category_of(*family)).or_default() += bytes;
            }),
            Action::Evict {
                store,
                path,
                unit,
                bytes,
            } => evict_unit(store, path, unit, stores).map(|rows| {
                if let Some(rows) = rows {
                    logger::info(
                        LogTag::System,
                        &format!(
                            "Storage housekeeping evicted {unit} from {store}: {rows} rows (about {bytes} B)"
                        ),
                    );
                    outcome.evictions += 1;
                    outcome.evicted_rows += rows;
                }
            }),
            Action::Checkpoint { store, path }
            | Action::ConvertToIncremental { store, path }
            | Action::Reclaim { store, path } => {
                run_store_action(action, store, path, stores).map(|ran| {
                    if ran {
                        outcome.store_operations += 1;
                    }
                })
            }
        };
        if let Err(error) = result {
            outcome.failures += 1;
            logger::warning(
                LogTag::System,
                &format!("Storage housekeeping step failed: {error}"),
            );
        }
    }
    outcome
}

/// Run one store operation on the pool the store's owner opened. A store not open in
/// this process is skipped: opening it by path could recreate a file recovery moved
/// aside. Returns whether the operation ran.
fn run_store_action(
    action: &Action,
    store: &str,
    path: &Path,
    stores: &[StoreHandle],
) -> Result<bool, StorageError> {
    let Some(handle) = stores.iter().find(|handle| handle.path == path) else {
        logger::debug(
            LogTag::System,
            &format!("Storage housekeeping skipped {store}: not open in this process"),
        );
        return Ok(false);
    };
    if handle.spec.compaction == Compaction::CheckpointOnly
        && !matches!(action, Action::Checkpoint { .. })
    {
        logger::warning(
            LogTag::System,
            &format!("Storage housekeeping refused to rewrite {store}: the store is only ever checkpointed"),
        );
        return Ok(false);
    }
    let failed = |source| StorageError::Store {
        store: store.to_owned(),
        source,
    };
    let connection = handle
        .pool
        .get()
        .map_err(|error| failed(crate::errors::DatabaseError::from(error)))?;
    match action {
        Action::Checkpoint { .. } => maintenance::checkpoint(store, &connection).map_err(failed)?,
        Action::ConvertToIncremental { .. } => {
            maintenance::convert_to_incremental(store, &connection).map_err(failed)?;
        }
        Action::Reclaim { .. } => {
            maintenance::reclaim(store, &connection).map_err(failed)?;
        }
        Action::Delete { .. } | Action::Evict { .. } => return Ok(false),
    }
    Ok(true)
}

/// Evict one unit through the pool the store's owner opened. A store not open in this
/// process is skipped. Returns the rows deleted, or `None` when the eviction did not
/// run.
fn evict_unit(
    store: &str,
    path: &Path,
    unit: &super::types::EvictionUnit,
    stores: &[StoreHandle],
) -> Result<Option<u64>, StorageError> {
    let Some(handle) = stores.iter().find(|handle| handle.path == path) else {
        logger::debug(
            LogTag::System,
            &format!(
                "Storage housekeeping skipped evicting from {store}: not open in this process"
            ),
        );
        return Ok(None);
    };
    eviction::evict(handle, unit, &is_shutdown_requested).map(Some)
}

/// Delete one planned entry after checking again that it still is what the plan
/// selected: a plain entry of the same family, sitting directly in one of the
/// family's own directories. A directory is removed only when empty.
fn delete_entry(path: &Path, id: FamilyId) -> Result<(), StorageError> {
    let spec = family(id);
    let parent = path.parent().unwrap_or(path);
    match classify(path) {
        Classification::Family(found) if found.id == id => {}
        _ => {
            return Err(StorageError::OutsideRoot {
                path: path.display().to_string(),
                root: parent.display().to_string(),
            })
        }
    }
    let canonical_parent = fs::canonicalize(parent).map_err(|error| StorageError::Delete {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    let in_root = spec
        .directories()
        .iter()
        .filter_map(|root| fs::canonicalize(root).ok())
        .any(|root| root == canonical_parent);
    if !in_root {
        return Err(StorageError::OutsideRoot {
            path: path.display().to_string(),
            root: parent.display().to_string(),
        });
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(StorageError::Delete {
                path: path.display().to_string(),
                message: error.to_string(),
            })
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(StorageError::OutsideRoot {
            path: path.display().to_string(),
            root: parent.display().to_string(),
        });
    }
    let removed = if metadata.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    };
    removed.map_err(|error| StorageError::Delete {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}
