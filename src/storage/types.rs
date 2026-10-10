// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The vocabulary of storage housekeeping: what a file is, how much of it a retention rule keeps at each disk-pressure tier, what was measured and what a pass decided.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::database::catalog::StoreSpec;
use crate::version::retention::{DownloadFacts, DownloadRetention};

/// What a group of files is to the person running the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    WalletKeys,
    TradingRecords,
    WalletHistory,
    MarketData,
    Settings,
    RecoveryCopies,
    Updates,
    Logs,
    Leftovers,
}

/// How hard housekeeping presses: from the free space on the data disk and from the
/// app's own usage against its budget, whichever is worse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Normal,
    Low,
    Critical,
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Normal, Tier::Low, Tier::Critical];

    pub const fn as_str(self) -> &'static str {
        match self {
            Tier::Normal => "normal",
            Tier::Low => "low",
            Tier::Critical => "critical",
        }
    }
}

/// How much of a file family one retention step keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Never deleted automatically.
    Forever,
    /// The newest files that fit every set bound; the rest are deleted.
    Bounded {
        max_age: Option<Duration>,
        max_count: Option<usize>,
        max_total_bytes: Option<u64>,
    },
    /// Decided by the update service's download rules.
    Downloads(DownloadRetention),
    /// Deleted on sight.
    Remove,
    /// Deleted only while it is empty: a zero-byte file or a directory with nothing
    /// in it.
    WhileNotEmpty,
}

/// The retention step a family applies at each tier. A higher tier never keeps more
/// than a lower one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ladder {
    pub normal: Keep,
    pub low: Keep,
    pub critical: Keep,
}

impl Ladder {
    pub const fn same(keep: Keep) -> Self {
        Self {
            normal: keep,
            low: keep,
            critical: keep,
        }
    }

    pub const fn at(&self, tier: Tier) -> Keep {
        match tier {
            Tier::Normal => self.normal,
            Tier::Low => self.low,
            Tier::Critical => self.critical,
        }
    }
}

/// A named group of files with one retention ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FamilyId {
    Config,
    AppState,
    Account,
    DeviceId,
    StagedCores,
    Quarantine,
    UpgradeBackups,
    WalletResetBackups,
    LatestLog,
    Downloads,
    PartialDownloads,
    Logs,
    LegacyFiles,
    LegacyEmpty,
}

/// How a family recognises its entries: by exact path, or by name directly inside
/// one of its roots.
#[derive(Debug, Clone, Copy)]
pub enum Matcher {
    Paths(&'static [fn() -> PathBuf]),
    Names(&'static [&'static str]),
    Prefix(&'static str),
    PrefixSuffix(&'static str, &'static str),
    Suffix(&'static str),
}

impl Matcher {
    /// Whether `name`, an entry directly inside one of the family's roots, belongs to
    /// the family. A `Paths` family matches by full path instead.
    pub fn matches(&self, name: &str) -> bool {
        match *self {
            Matcher::Paths(_) => false,
            Matcher::Names(names) => names.contains(&name),
            Matcher::Prefix(prefix) => name.starts_with(prefix) && name.len() > prefix.len(),
            Matcher::PrefixSuffix(prefix, suffix) => {
                name.len() > prefix.len() + suffix.len()
                    && name.starts_with(prefix)
                    && name.ends_with(suffix)
            }
            Matcher::Suffix(suffix) => name.ends_with(suffix) && name.len() > suffix.len(),
        }
    }
}

/// A file family: where its entries live, how they are recognised and how long
/// they are kept.
#[derive(Debug, Clone, Copy)]
pub struct FamilySpec {
    pub id: FamilyId,
    pub category: Category,
    /// The directories whose direct entries the family covers; empty for a family
    /// matched by exact path.
    pub roots: &'static [fn() -> PathBuf],
    pub matcher: Matcher,
    pub ladder: Ladder,
}

impl FamilySpec {
    /// Whether `path` is one of the family's entries.
    pub fn contains(&self, path: &Path) -> bool {
        if let Matcher::Paths(paths) = self.matcher {
            return paths.iter().any(|entry| entry() == path);
        }
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return false;
        };
        self.roots.iter().any(|root| root() == parent)
            && name.to_str().is_some_and(|name| self.matcher.matches(name))
    }

    /// The directories the family's entries sit directly inside.
    pub fn directories(&self) -> Vec<PathBuf> {
        match self.matcher {
            Matcher::Paths(paths) => paths
                .iter()
                .filter_map(|entry| entry().parent().map(Path::to_path_buf))
                .collect(),
            _ => self.roots.iter().map(|root| root()).collect(),
        }
    }
}

/// What a path in the app's directories is.
#[derive(Debug, Clone, Copy)]
pub enum Classification {
    /// A live store file or one of its SQLite sidecars. Never deleted.
    Store(&'static StoreSpec),
    Family(&'static FamilySpec),
    /// One of the directories the layout itself consists of.
    LayoutDirectory,
    /// Nothing declared; never deleted.
    Unknown,
}

/// One measured entry of a family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub family: FamilyId,
    /// Bytes on disk, everything inside included for a directory.
    pub bytes: u64,
    pub modified: SystemTime,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// A zero-byte file or a directory with no entries.
    pub empty: bool,
}

/// One store file as its header and its sidecars describe it.
#[derive(Debug, Clone)]
pub struct StoreUsage {
    pub spec: &'static StoreSpec,
    pub path: PathBuf,
    /// The main file, its WAL and its shared-memory file.
    pub bytes: u64,
    pub wal_bytes: u64,
    pub page_size: u64,
    pub page_count: u64,
    pub freelist_pages: u64,
    pub incremental: bool,
}

impl StoreUsage {
    pub const fn free_bytes(&self) -> u64 {
        self.freelist_pages * self.page_size
    }

    pub const fn live_bytes(&self) -> u64 {
        self.page_count.saturating_sub(self.freelist_pages) * self.page_size
    }
}

/// Why rebuildable data must not be evicted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    OpenPosition,
    Watched,
    RecentlyViewed,
}

/// A unit of rebuildable data that may be evicted, least recently used first, when
/// deleting files alone cannot bring usage within the budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evictable {
    pub id: String,
    pub bytes: u64,
    pub last_used: SystemTime,
    pub protection: Option<Protection>,
}

/// Size of the disk that holds the data directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disk {
    pub free: u64,
    pub total: u64,
}

/// Everything one housekeeping pass decides from.
#[derive(Debug, Clone)]
pub struct PlanInput {
    pub entries: Vec<FileEntry>,
    pub stores: Vec<StoreUsage>,
    pub evictables: Vec<Evictable>,
    pub disk: Option<Disk>,
    pub now: SystemTime,
    /// The file the logger is writing to now.
    pub active_log: Option<PathBuf>,
    pub downloads: DownloadFacts,
    /// Whether this pass may convert or compact a store.
    pub compaction_due: bool,
}

/// One step a pass takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Delete {
        path: PathBuf,
        family: FamilyId,
        bytes: u64,
        tier: Tier,
    },
    Evict {
        id: String,
        bytes: u64,
    },
    Checkpoint {
        store: &'static str,
        path: PathBuf,
    },
    ConvertToIncremental {
        store: &'static str,
        path: PathBuf,
    },
    Reclaim {
        store: &'static str,
        path: PathBuf,
    },
}

/// What a pass decided, and where usage stands once it is carried out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// The budget the app's usage is held to.
    pub limit: u64,
    /// The app's usage before the pass.
    pub usage: u64,
    /// The app's usage once every action is carried out.
    pub projected_usage: u64,
    pub tier: Tier,
    /// Bytes usage still exceeds the budget by once everything allowed is removed.
    pub shortfall: u64,
    pub actions: Vec<Action>,
}
