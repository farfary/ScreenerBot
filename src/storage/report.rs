// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The storage report Settings shows: the app's usage by category against its limit, the free space of the data disk, and what the last housekeeping pass did.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use super::plan::{budget, budget_tier, category_of, disk_tier};
use super::types::{store_category, Category, Disk, FileEntry, Limit, StoreUsage, Tier};

/// Whether the limit adapts to the disk or is set to a fixed size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitMode {
    Automatic,
    Fixed,
}

/// The bytes one category takes on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CategoryUsage {
    pub category: Category,
    pub bytes: u64,
}

/// Free and total space of the disk that holds the data directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DiskSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// What one housekeeping pass did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LastCleanup {
    /// Unix seconds.
    pub finished_at: i64,
    /// Bytes the app's usage dropped by over the pass.
    pub freed_bytes: u64,
    pub deleted_files: usize,
    pub evicted_rows: u64,
    pub failures: usize,
}

/// The app's storage as last measured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageReport {
    /// Unix seconds.
    pub measured_at: i64,
    pub total_bytes: u64,
    /// The budget the app's usage is held to, resolved for this disk.
    pub limit_bytes: u64,
    pub limit_mode: LimitMode,
    /// The configured limit in megabytes; zero is automatic.
    pub limit_megabytes: u64,
    /// What the automatic limit resolves to on this disk.
    pub automatic_limit_bytes: u64,
    pub tier: Tier,
    pub disk: Option<DiskSpace>,
    /// Every category in display order, empty ones included.
    pub categories: Vec<CategoryUsage>,
    pub last_cleanup: Option<LastCleanup>,
    /// Whether a housekeeping pass is running now.
    pub running: bool,
}

pub(crate) fn unix_seconds(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
    })
}

/// The report for what was measured, against the limit `limit_megabytes` sets.
/// `last_cleanup` and `running` are the engine's.
pub fn summarize(
    entries: &[FileEntry],
    stores: &[StoreUsage],
    disk: Option<Disk>,
    limit_megabytes: u64,
    measured_at: SystemTime,
) -> StorageReport {
    let mut by_category: BTreeMap<Category, u64> = BTreeMap::new();
    for entry in entries {
        *by_category.entry(category_of(entry.family)).or_default() += entry.bytes;
    }
    for store in stores {
        *by_category.entry(store_category(store.spec)).or_default() += store.bytes;
    }
    let mut report = StorageReport {
        measured_at: unix_seconds(measured_at),
        total_bytes: by_category.values().sum(),
        limit_bytes: 0,
        limit_mode: LimitMode::Automatic,
        limit_megabytes: 0,
        automatic_limit_bytes: 0,
        tier: Tier::Normal,
        disk: disk.map(|disk| DiskSpace {
            free_bytes: disk.free,
            total_bytes: disk.total,
        }),
        categories: Category::ALL
            .into_iter()
            .map(|category| CategoryUsage {
                category,
                bytes: by_category.get(&category).copied().unwrap_or(0),
            })
            .collect(),
        last_cleanup: None,
        running: false,
    };
    report.apply_limit(limit_megabytes);
    report
}

impl StorageReport {
    /// Resolve the limit `maintenance.storage_limit_mb` sets against what was
    /// measured, so a changed setting shows before the next measurement.
    pub fn apply_limit(&mut self, limit_megabytes: u64) {
        let limit = Limit::from_megabytes(limit_megabytes);
        let disk = self.disk.map(|disk| Disk {
            free: disk.free_bytes,
            total: disk.total_bytes,
        });
        self.limit_megabytes = limit_megabytes;
        self.limit_mode = match limit {
            Limit::Automatic => LimitMode::Automatic,
            Limit::Fixed(_) => LimitMode::Fixed,
        };
        self.limit_bytes = budget(self.total_bytes, disk, limit);
        self.automatic_limit_bytes = budget(self.total_bytes, disk, Limit::Automatic);
        self.tier = disk_tier(disk.map(|disk| disk.free))
            .max(budget_tier(self.total_bytes, self.limit_bytes));
    }
}
