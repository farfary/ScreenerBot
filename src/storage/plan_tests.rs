// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the pure housekeeping planner: the adaptive budget, the pressure tiers, what each tier deletes, eviction order and which stores are rewritten.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::plan::{budget, budget_tier, disk_tier, plan, BUDGET_MAX_BYTES, BUDGET_MIN_BYTES};
use super::types::{
    Action, Disk, Evictable, FamilyId, FileEntry, PlanInput, Protection, StoreUsage, Tier,
};
use crate::database::catalog::{self, StoreSpec};
use crate::version::retention::{DownloadFacts, PendingDownload};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
const HOUR: Duration = Duration::from_secs(3600);
const INSTALLER: &str = "ScreenerBot-v0.3.0-macOS-arm64.dmg";

fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000)
}

fn entry(path: &str, family: FamilyId, bytes: u64, age: Duration) -> FileEntry {
    FileEntry {
        path: PathBuf::from(path),
        family,
        bytes,
        modified: now() - age,
        is_dir: false,
        is_symlink: false,
        empty: bytes == 0,
    }
}

fn store(spec: &'static StoreSpec, bytes: u64) -> StoreUsage {
    StoreUsage {
        spec,
        path: PathBuf::from(format!("/data/{}.db", spec.id)),
        bytes,
        wal_bytes: 0,
        page_size: 4096,
        page_count: bytes / 4096,
        freelist_pages: 0,
        incremental: true,
    }
}

fn roomy_disk() -> Option<Disk> {
    Some(Disk {
        free: 400 * GIB,
        total: 1000 * GIB,
    })
}

fn input(entries: Vec<FileEntry>, disk: Option<Disk>) -> PlanInput {
    PlanInput {
        entries,
        stores: Vec::new(),
        evictables: Vec::new(),
        disk,
        now: now(),
        active_log: Some(PathBuf::from("/logs/screenerbot_active.log")),
        downloads: DownloadFacts {
            running_version: "0.2.14".to_owned(),
            pending: Some(PendingDownload {
                version: "0.3.0".to_owned(),
                filename: INSTALLER.to_owned(),
            }),
            downloading: false,
        },
        compaction_due: false,
    }
}

fn deleted(plan: &super::types::Plan) -> BTreeSet<PathBuf> {
    plan.actions
        .iter()
        .filter_map(|action| match action {
            Action::Delete { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect()
}

/// A mixed install: logs, downloads, legacy leftovers and must-keep files.
fn install() -> Vec<FileEntry> {
    let mut entries = vec![
        entry("/data/config.toml", FamilyId::Config, 10_000, HOUR * 9000),
        entry("/data/account.dat", FamilyId::Account, 2_000, HOUR * 9000),
        entry("/data/.device_id", FamilyId::DeviceId, 36, HOUR * 9000),
        entry(
            "/data/quarantine",
            FamilyId::Quarantine,
            300 * MIB,
            HOUR * 900,
        ),
        entry(
            "/data/backups/upgrade-positions.db",
            FamilyId::UpgradeBackups,
            40 * MIB,
            HOUR * 900,
        ),
        entry("/logs/latest.log", FamilyId::LatestLog, 0, HOUR),
        entry(
            "/logs/screenerbot_active.log",
            FamilyId::Logs,
            60 * MIB,
            Duration::ZERO,
        ),
        entry(
            &format!("/updates/{INSTALLER}"),
            FamilyId::Downloads,
            200 * MIB,
            HOUR,
        ),
        entry(
            "/updates/ScreenerBot-v0.2.14-macOS-arm64.dmg",
            FamilyId::Downloads,
            200 * MIB,
            HOUR * 300,
        ),
        entry(
            "/updates/ScreenerBot-v0.2.13-macOS-arm64.dmg.part",
            FamilyId::PartialDownloads,
            50 * MIB,
            HOUR * 300,
        ),
        entry("/data/mcp.json", FamilyId::LegacyFiles, 300, HOUR * 9000),
        entry("/data/cache_pool", FamilyId::LegacyEmpty, 0, HOUR * 9000),
    ];
    entries[11].is_dir = true;
    for index in 0..10u32 {
        entries.push(entry(
            &format!("/logs/screenerbot_{index:02}.log"),
            FamilyId::Logs,
            20 * MIB,
            HOUR * (index + 1) * 4,
        ));
    }
    entries
}

#[test]
fn the_budget_adapts_to_the_disk() {
    assert_eq!(budget(0, None), BUDGET_MAX_BYTES);
    assert_eq!(budget(GIB, roomy_disk()), BUDGET_MAX_BYTES);
    let small = Disk {
        free: 100 * GIB,
        total: 20 * GIB,
    };
    assert_eq!(budget(GIB, Some(small)), 2 * GIB);
    let nearly_full = Disk {
        free: 2 * GIB,
        total: 500 * GIB,
    };
    assert_eq!(budget(2 * GIB, Some(nearly_full)), GIB);
    let full = Disk {
        free: 0,
        total: 500 * GIB,
    };
    assert_eq!(budget(100 * MIB, Some(full)), BUDGET_MIN_BYTES);
}

#[test]
fn deleting_never_moves_the_budget() {
    let disk = Disk {
        free: 3 * GIB,
        total: 500 * GIB,
    };
    let after = Disk {
        free: 4 * GIB,
        total: 500 * GIB,
    };
    assert_eq!(budget(2 * GIB, Some(disk)), budget(GIB, Some(after)));
}

#[test]
fn tiers_follow_free_space_and_usage() {
    assert_eq!(disk_tier(None), Tier::Normal);
    assert_eq!(disk_tier(Some(6 * GIB)), Tier::Normal);
    assert_eq!(disk_tier(Some(4 * GIB)), Tier::Low);
    assert_eq!(disk_tier(Some(GIB / 2)), Tier::Critical);
    assert_eq!(budget_tier(GIB, GIB), Tier::Normal);
    assert_eq!(budget_tier(GIB + 1, GIB), Tier::Low);
    assert_eq!(budget_tier(2 * GIB, GIB), Tier::Critical);
}

#[test]
fn a_normal_pass_removes_stale_downloads_old_logs_and_leftovers() {
    let plan = plan(&input(install(), roomy_disk()));
    assert_eq!(plan.tier, Tier::Normal);
    let deleted = deleted(&plan);
    let expected: BTreeSet<PathBuf> = [
        "/updates/ScreenerBot-v0.2.14-macOS-arm64.dmg",
        "/updates/ScreenerBot-v0.2.13-macOS-arm64.dmg.part",
        "/data/mcp.json",
        "/data/cache_pool",
        "/logs/screenerbot_06.log",
        "/logs/screenerbot_07.log",
        "/logs/screenerbot_08.log",
        "/logs/screenerbot_09.log",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect();
    assert_eq!(deleted, expected);
}

#[test]
fn must_keep_files_are_never_planned_at_any_pressure() {
    let must_keep = [
        "/data/config.toml",
        "/data/account.dat",
        "/data/.device_id",
        "/data/quarantine",
        "/data/backups/upgrade-positions.db",
        "/logs/latest.log",
        "/logs/screenerbot_active.log",
    ];
    for free in [400 * GIB, 3 * GIB, 0] {
        let disk = Some(Disk {
            free,
            total: 1000 * GIB,
        });
        let mut input = input(install(), disk);
        input.stores = catalog::STORES
            .iter()
            .map(|spec| store(spec, 50 * MIB))
            .collect();
        let deleted = deleted(&plan(&input));
        for path in must_keep {
            assert!(
                !deleted.contains(&PathBuf::from(path)),
                "{path} at {free} free"
            );
        }
    }
}

#[test]
fn deletions_only_grow_with_pressure() {
    let mut previous = BTreeSet::new();
    for free in [400 * GIB, 4 * GIB, GIB / 2] {
        let disk = Some(Disk {
            free,
            total: 1000 * GIB,
        });
        let deleted = deleted(&plan(&input(install(), disk)));
        assert!(deleted.is_superset(&previous), "{free} free");
        previous = deleted;
    }
}

#[test]
fn critical_pressure_keeps_only_the_active_log_and_no_pending_installer() {
    let disk = Some(Disk {
        free: GIB / 2,
        total: 1000 * GIB,
    });
    let plan = plan(&input(install(), disk));
    assert_eq!(plan.tier, Tier::Critical);
    let deleted = deleted(&plan);
    assert!(deleted.contains(&PathBuf::from(format!("/updates/{INSTALLER}"))));
    for index in 0..10 {
        assert!(deleted.contains(&PathBuf::from(format!("/logs/screenerbot_{index:02}.log"))));
    }
    assert!(!deleted.contains(&PathBuf::from("/logs/screenerbot_active.log")));
}

#[test]
fn escalation_stops_once_usage_fits_and_the_disk_is_normal() {
    let disk = Some(Disk {
        free: 400 * GIB,
        total: 1000 * GIB,
    });
    let mut input = input(install(), disk);
    input.stores = vec![store(&catalog::TOKENS, 5 * GIB - 800 * MIB)];
    let plan = plan(&input);
    assert_eq!(plan.tier, Tier::Low);
    let deleted = deleted(&plan);
    assert!(plan.projected_usage <= plan.limit);
    assert!(
        !deleted.contains(&PathBuf::from(format!("/updates/{INSTALLER}"))),
        "the Low step of the update family keeps the pending installer"
    );
}

#[test]
fn must_keep_data_over_the_budget_is_reported_not_deleted() {
    let entries = vec![
        entry("/data/quarantine", FamilyId::Quarantine, 8 * GIB, HOUR),
        entry("/data/config.toml", FamilyId::Config, 1000, HOUR),
    ];
    let plan = plan(&input(entries, roomy_disk()));
    assert!(deleted(&plan).is_empty());
    assert_eq!(plan.shortfall, 8 * GIB + 1000 - plan.limit);
}

#[test]
fn applying_a_plan_and_planning_again_deletes_nothing() {
    let disk = Some(Disk {
        free: 3 * GIB,
        total: 1000 * GIB,
    });
    let first = plan(&input(install(), disk));
    let removed = deleted(&first);
    let freed: u64 = install()
        .iter()
        .filter(|entry| removed.contains(&entry.path))
        .map(|entry| entry.bytes)
        .sum();
    let remaining: Vec<FileEntry> = install()
        .into_iter()
        .filter(|entry| !removed.contains(&entry.path))
        .collect();
    let after = Some(Disk {
        free: 3 * GIB + freed,
        total: 1000 * GIB,
    });
    assert!(deleted(&plan(&input(remaining, after))).is_empty());
}

#[test]
fn symbolic_links_and_non_empty_directories_are_never_selected() {
    let mut link = entry("/logs/screenerbot_old.log", FamilyId::Logs, 0, HOUR * 1000);
    link.is_symlink = true;
    let mut full_dir = entry("/data/cache_pool", FamilyId::LegacyEmpty, 4096, HOUR * 1000);
    full_dir.is_dir = true;
    let mut legacy_dir = entry("/data/mcp.json", FamilyId::LegacyFiles, 4096, HOUR * 1000);
    legacy_dir.is_dir = true;
    let disk = Some(Disk {
        free: GIB / 2,
        total: 1000 * GIB,
    });
    assert!(deleted(&plan(&input(vec![link, full_dir, legacy_dir], disk))).is_empty());
}

fn evictable(id: &str, age: Duration, protection: Option<Protection>) -> Evictable {
    Evictable {
        id: id.to_owned(),
        bytes: GIB,
        last_used: now() - age,
        protection,
    }
}

#[test]
fn eviction_takes_the_least_recently_used_unprotected_data_first() {
    let mut input = input(Vec::new(), roomy_disk());
    input.stores = vec![store(&catalog::OHLCVS, 7 * GIB)];
    input.evictables = vec![
        evictable("held", HOUR * 900, Some(Protection::OpenPosition)),
        evictable("watched", HOUR * 800, Some(Protection::Watched)),
        evictable("viewed", HOUR * 700, Some(Protection::RecentlyViewed)),
        evictable("oldest", HOUR * 600, None),
        evictable("older", HOUR * 500, None),
        evictable("recent", HOUR, None),
    ];
    let plan = plan(&input);
    let evicted: Vec<&str> = plan
        .actions
        .iter()
        .filter_map(|action| match action {
            Action::Evict { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(evicted, ["oldest", "older"]);
    assert_eq!(plan.shortfall, 0);
}

#[test]
fn protected_data_is_never_evicted_even_when_usage_stays_over_budget() {
    let mut input = input(Vec::new(), roomy_disk());
    input.stores = vec![store(&catalog::OHLCVS, 9 * GIB)];
    input.evictables = vec![
        evictable("held", HOUR, Some(Protection::OpenPosition)),
        evictable("watched", HOUR, Some(Protection::Watched)),
        evictable("viewed", HOUR, Some(Protection::RecentlyViewed)),
    ];
    let plan = plan(&input);
    assert!(!plan
        .actions
        .iter()
        .any(|action| matches!(action, Action::Evict { .. })));
    assert!(plan.shortfall > 0);
}

fn rewrites(plan: &super::types::Plan) -> Vec<(&'static str, &'static str)> {
    plan.actions
        .iter()
        .filter_map(|action| match action {
            Action::ConvertToIncremental { store, .. } => Some(("convert", *store)),
            Action::Reclaim { store, .. } => Some(("reclaim", *store)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_vault_is_only_ever_checkpointed() {
    let mut vault = store(&catalog::WALLETS, 32 * MIB);
    vault.wal_bytes = MIB;
    vault.freelist_pages = vault.page_count / 2;
    vault.incremental = false;
    let mut input = input(Vec::new(), roomy_disk());
    input.stores = vec![vault];
    input.compaction_due = true;
    let plan = plan(&input);
    assert_eq!(
        plan.actions,
        [Action::Checkpoint {
            store: "wallets",
            path: PathBuf::from("/data/wallets.db"),
        }]
    );
}

#[test]
fn compaction_converts_a_small_store_before_reclaiming_free_pages() {
    let mut legacy = store(&catalog::COPY_TRADING, 4 * MIB);
    legacy.incremental = false;
    let mut large_legacy = store(&catalog::EVENTS, 2 * GIB);
    large_legacy.incremental = false;
    let mut fragmented = store(&catalog::POOLS, 900 * MIB);
    fragmented.freelist_pages = fragmented.page_count / 3;
    let mut input = input(Vec::new(), roomy_disk());
    input.stores = vec![legacy.clone(), large_legacy, fragmented.clone()];
    input.compaction_due = true;
    assert_eq!(rewrites(&plan(&input)), [("convert", "copy_trading")]);

    input.stores = vec![fragmented];
    assert_eq!(rewrites(&plan(&input)), [("reclaim", "pools")]);

    input.compaction_due = false;
    assert!(rewrites(&plan(&input)).is_empty());
}

#[test]
fn a_store_with_few_free_pages_is_not_compacted() {
    let mut store = store(&catalog::POOLS, 900 * MIB);
    store.freelist_pages = 10;
    let mut input = input(Vec::new(), roomy_disk());
    input.stores = vec![store];
    input.compaction_due = true;
    assert!(rewrites(&plan(&input)).is_empty());
}
