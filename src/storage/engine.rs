// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The housekeeping task: when a pass runs, how it measures and plans, and how each planned deletion is re-checked before it is carried out and each store operation reaches its store only through the owner's pool.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use crate::database::catalog::Compaction;
use crate::database::handles::{self, StoreHandle};
use crate::database::maintenance;
use crate::errors::StorageError;
use crate::logger::{self, LogTag};
use crate::paths;

use super::families::{classify, family};
use super::plan::{category_of, disk_tier, plan};
use super::types::{Action, Category, Classification, FamilyId, Plan, PlanInput, Tier};
use super::usage::{measure_disk, measure_entries, measure_stores};

/// Delay before the first pass, so startup finishes opening its stores first.
const FIRST_PASS_DELAY: Duration = Duration::from_secs(60);
/// How often free disk space is probed; a change of disk tier starts a pass at once.
const DISK_PROBE_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// Shortest pass interval the configuration may ask for.
const MIN_PASS_INTERVAL_SECS: u64 = 300;
/// Shortest compaction interval the configuration may ask for.
const MIN_COMPACTION_INTERVAL_SECS: u64 = 3600;

static STARTED: AtomicBool = AtomicBool::new(false);

/// Start the housekeeping task once per process. Passes run every
/// `maintenance.wal_checkpoint_interval_secs`, store compaction at most every
/// `maintenance.vacuum_interval_secs`, and a pass starts early when free disk space
/// crosses into another tier.
pub fn start_housekeeping() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tokio::spawn(run_housekeeping());
}

async fn run_housekeeping() {
    let (pass_secs, compaction_secs) = crate::config::with_config(|cfg| {
        (
            cfg.maintenance
                .wal_checkpoint_interval_secs
                .max(MIN_PASS_INTERVAL_SECS),
            cfg.maintenance
                .vacuum_interval_secs
                .max(MIN_COMPACTION_INTERVAL_SECS),
        )
    });
    logger::info(
        LogTag::System,
        &format!(
            "Storage housekeeping scheduled: first pass in {}s, then every {}s, compaction every {}s",
            FIRST_PASS_DELAY.as_secs(),
            pass_secs,
            compaction_secs
        ),
    );
    tokio::time::sleep(FIRST_PASS_DELAY).await;

    let compaction_interval = Duration::from_secs(compaction_secs);
    let mut last_compaction: Option<Instant> = None;
    let mut last_tier = Tier::Normal;
    let mut passes = tokio::time::interval(Duration::from_secs(pass_secs));
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
        let compaction_due =
            last_compaction.is_none_or(|last| last.elapsed() >= compaction_interval);
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

/// Measure, plan and carry out one pass. Returns the plan it carried out.
pub async fn run_pass(compaction_due: bool) -> Result<Plan, StorageError> {
    let started = Instant::now();
    let downloads = crate::version::retention::download_facts().await;
    let active_log = logger::active_log_file();
    let input = blocking("measure storage", move || {
        Ok(PlanInput {
            entries: measure_entries()?,
            stores: measure_stores(),
            evictables: Vec::new(),
            disk: measure_disk(&paths::get_data_directory()),
            now: SystemTime::now(),
            active_log,
            downloads,
            compaction_due,
        })
    })
    .await?;
    let plan = plan(&input);
    let actions = plan.actions.clone();
    let summary = blocking("carry out the storage plan", move || {
        Ok(carry_out(&actions, &handles::open_stores()))
    })
    .await?;

    let freed: Vec<String> = summary
        .freed
        .iter()
        .map(|(category, bytes)| format!("{category:?} {bytes} B"))
        .collect();
    logger::info(
        LogTag::System,
        &format!(
            "Storage housekeeping: tier {}, usage {} B of {} B budget, deleted {} entries ({}), {} store operations, {} failures, shortfall {} B, {} ms",
            plan.tier.as_str(),
            plan.usage,
            plan.limit,
            summary.deleted,
            if freed.is_empty() { "nothing freed".to_owned() } else { freed.join(", ") },
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
            Action::Evict { id, bytes } => {
                logger::warning(
                    LogTag::System,
                    &format!("Storage housekeeping has no evictor for {id} ({bytes} B)"),
                );
                Ok(())
            }
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
