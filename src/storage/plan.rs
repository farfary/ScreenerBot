// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The pure housekeeping planner: from what was measured, the budget the app's usage is held to, the disk-pressure tier, and every deletion, eviction and store operation one pass carries out.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::database::catalog::Compaction;
use crate::database::maintenance::REBUILD_MAX_LIVE_BYTES;
use crate::version::retention::keeps_download;

use super::families::{family, FAMILIES, PARTIAL_DOWNLOAD_IDLE_LIMIT};
use super::types::{
    Action, Category, Disk, FamilyId, FileEntry, Keep, Limit, Plan, PlanInput, StoreUsage, Tier,
};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// The most the app's usage is ever held to.
pub const BUDGET_MAX_BYTES: u64 = 5 * GIB;
/// The least the app's usage is ever held to, however small or full the disk.
pub const BUDGET_MIN_BYTES: u64 = 256 * MIB;
/// Share of the disk's capacity the budget may take, in percent.
pub const BUDGET_DISK_PERCENT: u64 = 10;
/// Share of the room the app could use (its usage plus the free space) the budget
/// may take, in percent, so a nearly full disk lowers the budget.
pub const BUDGET_ROOM_PERCENT: u64 = 25;
/// Usage above this share of the budget, in percent, is critical.
pub const CRITICAL_BUDGET_PERCENT: u64 = 150;
/// Free space below which the disk is low.
pub const LOW_FREE_BYTES: u64 = 5 * GIB;
/// Free space below which the disk is critical.
pub const CRITICAL_FREE_BYTES: u64 = GIB;
/// Free pages that make a store worth compacting, in bytes.
pub const COMPACT_MIN_FREE_BYTES: u64 = 8 * MIB;
/// Free pages that make a store worth compacting, in percent of its file.
pub const COMPACT_MIN_FREE_PERCENT: u64 = 10;

/// The categories whose families step down their ladders under pressure, in order.
const ESCALATION_ORDER: [Category; 2] = [Category::Updates, Category::Logs];

/// The budget for `usage` on `disk`. An automatic limit is the smallest of the fixed
/// maximum, a share of the disk and a share of the room left to the app; a fixed
/// limit replaces the maximum and the disk share. Both are capped by the room share
/// and never fall below the fixed minimum. Deleting moves bytes from usage to free
/// space, so a pass never moves its own budget.
pub fn budget(usage: u64, disk: Option<Disk>, limit: Limit) -> u64 {
    let ceiling = match limit {
        Limit::Automatic => BUDGET_MAX_BYTES,
        Limit::Fixed(bytes) => bytes,
    };
    let Some(disk) = disk else {
        return ceiling.max(BUDGET_MIN_BYTES);
    };
    let disk_share = match limit {
        Limit::Automatic => disk.total.saturating_mul(BUDGET_DISK_PERCENT) / 100,
        Limit::Fixed(_) => u64::MAX,
    };
    let room_share = usage
        .saturating_add(disk.free)
        .saturating_mul(BUDGET_ROOM_PERCENT)
        / 100;
    ceiling
        .min(disk_share)
        .min(room_share)
        .max(BUDGET_MIN_BYTES)
}

/// The tier free space alone puts the disk in; an unmeasured disk is normal.
pub fn disk_tier(free: Option<u64>) -> Tier {
    match free {
        Some(free) if free < CRITICAL_FREE_BYTES => Tier::Critical,
        Some(free) if free < LOW_FREE_BYTES => Tier::Low,
        _ => Tier::Normal,
    }
}

/// The tier the app's usage against its budget alone puts it in.
pub fn budget_tier(usage: u64, limit: u64) -> Tier {
    if u128::from(usage) * 100 > u128::from(limit) * u128::from(CRITICAL_BUDGET_PERCENT) {
        Tier::Critical
    } else if usage > limit {
        Tier::Low
    } else {
        Tier::Normal
    }
}

/// The entries of one family a retention step deletes. A symbolic link, a non-empty
/// directory and the active log file are never selected.
fn select<'a>(keep: Keep, entries: &[&'a FileEntry], input: &PlanInput) -> Vec<&'a FileEntry> {
    let candidates: Vec<&FileEntry> = entries
        .iter()
        .copied()
        .filter(|entry| !entry.is_symlink && (!entry.is_dir || entry.empty))
        .filter(|entry| input.active_log.as_ref() != Some(&entry.path))
        .collect();
    let idle = |entry: &FileEntry| {
        input
            .now
            .duration_since(entry.modified)
            .unwrap_or(Duration::ZERO)
    };
    match keep {
        Keep::Forever => Vec::new(),
        Keep::Remove => candidates,
        Keep::WhileNotEmpty => candidates.into_iter().filter(|entry| entry.empty).collect(),
        Keep::Downloads(retention) => candidates
            .into_iter()
            .filter(|entry| {
                let name = entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                !keeps_download(
                    name,
                    idle(entry),
                    &input.downloads,
                    retention,
                    PARTIAL_DOWNLOAD_IDLE_LIMIT,
                )
            })
            .collect(),
        Keep::Bounded {
            max_age,
            max_count,
            max_total_bytes,
        } => {
            let mut newest_first = candidates;
            newest_first.sort_by(|a, b| b.modified.cmp(&a.modified).then(b.path.cmp(&a.path)));
            let mut kept_count = 0usize;
            let mut kept_bytes = 0u64;
            let mut over = false;
            newest_first
                .into_iter()
                .filter(|entry| {
                    over = over
                        || max_age.is_some_and(|age| idle(entry) > age)
                        || max_count.is_some_and(|count| kept_count >= count)
                        || max_total_bytes
                            .is_some_and(|total| kept_bytes.saturating_add(entry.bytes) > total);
                    if !over {
                        kept_count += 1;
                        kept_bytes += entry.bytes;
                    }
                    over
                })
                .collect()
        }
    }
}

/// Deletions decided so far, keyed by path so a stricter step never counts a file
/// twice.
struct Selection<'a> {
    input: &'a PlanInput,
    by_family: BTreeMap<FamilyId, Vec<&'a FileEntry>>,
    deleted: BTreeMap<PathBuf, (FamilyId, u64, Tier)>,
    freed: u64,
}

impl<'a> Selection<'a> {
    fn new(input: &'a PlanInput) -> Self {
        let mut by_family: BTreeMap<FamilyId, Vec<&FileEntry>> = BTreeMap::new();
        for entry in &input.entries {
            by_family.entry(entry.family).or_default().push(entry);
        }
        Self {
            input,
            by_family,
            deleted: BTreeMap::new(),
            freed: 0,
        }
    }

    fn apply(&mut self, tier: Tier, category: Option<Category>) {
        for spec in FAMILIES {
            if category.is_some_and(|category| spec.category != category) {
                continue;
            }
            let Some(entries) = self.by_family.get(&spec.id) else {
                continue;
            };
            for entry in select(spec.ladder.at(tier), entries, self.input) {
                if !self.deleted.contains_key(&entry.path) {
                    self.freed += entry.bytes;
                    self.deleted
                        .insert(entry.path.clone(), (entry.family, entry.bytes, tier));
                }
            }
        }
    }
}

fn worth_compacting(store: &StoreUsage) -> bool {
    let free = store.free_bytes();
    let file = store.page_count * store.page_size;
    free > 0
        && (free >= COMPACT_MIN_FREE_BYTES
            || u128::from(free) * 100 >= u128::from(file) * u128::from(COMPACT_MIN_FREE_PERCENT))
}

/// The store operations of a pass: a checkpoint of every WAL, and when compaction is
/// due one rewrite at most: converting the largest small store not yet in incremental
/// auto-vacuum, otherwise reclaiming the store with the most free pages. A
/// checkpoint-only store is never rewritten.
fn store_actions(stores: &[StoreUsage], compaction_due: bool) -> Vec<Action> {
    let mut actions: Vec<Action> = stores
        .iter()
        .filter(|store| store.wal_bytes > 0)
        .map(|store| Action::Checkpoint {
            store: store.spec.id,
            path: store.path.clone(),
        })
        .collect();
    if !compaction_due {
        return actions;
    }
    let rewritable = || {
        stores.iter().filter(|store| {
            store.spec.compaction == Compaction::Incremental && store.page_count > 0
        })
    };
    let convert = rewritable()
        .filter(|store| !store.incremental && store.live_bytes() <= REBUILD_MAX_LIVE_BYTES)
        .max_by_key(|store| store.free_bytes());
    if let Some(store) = convert {
        actions.push(Action::ConvertToIncremental {
            store: store.spec.id,
            path: store.path.clone(),
        });
    } else if let Some(store) = rewritable()
        .filter(|store| worth_compacting(store))
        .max_by_key(|store| store.free_bytes())
    {
        actions.push(Action::Reclaim {
            store: store.spec.id,
            path: store.path.clone(),
        });
    }
    actions
}

/// Plan one housekeeping pass.
///
/// Every family first applies its normal step. When usage still exceeds the budget
/// or the disk is short of space, the families of each escalation category step
/// down to the next tier in turn, up to the pressure the pass started at, until
/// usage fits and the disk is out of the low tier. Rebuildable data that is not
/// protected is then evicted, least recently used first. What must be kept is never
/// planned; what still does not fit is reported as the shortfall.
pub fn plan(input: &PlanInput) -> Plan {
    let usage = input.entries.iter().map(|entry| entry.bytes).sum::<u64>()
        + input.stores.iter().map(|store| store.bytes).sum::<u64>();
    let free = input.disk.map(|disk| disk.free);
    let limit = budget(usage, input.disk, input.limit);
    let tier = disk_tier(free).max(budget_tier(usage, limit));
    let fits = |freed: u64| {
        usage.saturating_sub(freed) <= limit
            && disk_tier(free.map(|free| free.saturating_add(freed))) == Tier::Normal
    };

    let mut selection = Selection::new(input);
    selection.apply(Tier::Normal, None);
    'escalation: for step in [Tier::Low, Tier::Critical] {
        if step > tier {
            break;
        }
        for category in ESCALATION_ORDER {
            if fits(selection.freed) {
                break 'escalation;
            }
            selection.apply(step, Some(category));
        }
    }

    let mut actions: Vec<Action> = selection
        .deleted
        .iter()
        .map(|(path, (family, bytes, tier))| Action::Delete {
            path: path.clone(),
            family: *family,
            bytes: *bytes,
            tier: *tier,
        })
        .collect();
    let mut freed = selection.freed;
    // Free pages inside a store are space compaction returns without evicting
    // anything, and the pages an eviction frees stay in its store until compaction
    // runs, so eviction stops once usage fits with them counted.
    let reclaimable: u64 = input.stores.iter().map(StoreUsage::free_bytes).sum();

    if !fits(freed.saturating_add(reclaimable)) {
        let mut evictable: Vec<_> = input
            .evictables
            .iter()
            .filter(|unit| unit.protection.is_none())
            .collect();
        evictable.sort_by(|a, b| {
            a.last_used
                .cmp(&b.last_used)
                .then(a.path.cmp(&b.path))
                .then(a.bytes.cmp(&b.bytes))
        });
        for unit in evictable {
            if fits(freed.saturating_add(reclaimable)) {
                break;
            }
            freed += unit.bytes;
            actions.push(Action::Evict {
                store: unit.store,
                path: unit.path.clone(),
                unit: unit.unit.clone(),
                bytes: unit.bytes,
            });
        }
    }

    let mut operations = store_actions(&input.stores, input.compaction_due);
    for store in &input.stores {
        let evicted = actions
            .iter()
            .any(|action| matches!(action, Action::Evict { path, .. } if *path == store.path));
        let reclaimed = operations
            .iter()
            .any(|action| matches!(action, Action::Reclaim { path, .. } if *path == store.path));
        if evicted
            && !reclaimed
            && store.spec.compaction == Compaction::Incremental
            && store.incremental
        {
            operations.push(Action::Reclaim {
                store: store.spec.id,
                path: store.path.clone(),
            });
        }
    }
    actions.extend(operations);
    let projected_usage = usage.saturating_sub(freed);
    Plan {
        limit,
        usage,
        projected_usage,
        tier,
        shortfall: projected_usage.saturating_sub(limit),
        fits: fits(freed.saturating_add(reclaimable)),
        actions,
    }
}

/// The category a deleted family belongs to, for the pass summary.
pub fn category_of(id: FamilyId) -> Category {
    family(id).category
}
