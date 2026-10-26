// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The declaration of every file family the app writes outside its stores, with the retention ladder each one follows, and the classification of any path in the app's directories.

use std::path::Path;
use std::time::Duration;

use crate::database::backup::{snapshot::SNAPSHOTS_DIRECTORY_NAME, UPGRADE_BACKUP_PREFIX};
use crate::database::catalog::{StoreSpec, STORES};
use crate::paths;
use crate::version::retention::{DownloadRetention, PARTIAL_SUFFIX, RELEASE_ASSET_PREFIX};

use super::types::{Category, Classification, FamilyId, FamilySpec, Keep, Ladder, Matcher};

const MIB: u64 = 1024 * 1024;
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// Size at which the logger finishes a log file and continues in a new one.
pub const LOG_FILE_MAX_BYTES: u64 = 64 * MIB;

/// How long the partial file of the pending release is kept for a resumed download
/// after it was last written.
pub const PARTIAL_DOWNLOAD_IDLE_LIMIT: Duration = Duration::from_secs(7 * DAY.as_secs());

/// SQLite sidecars that sit beside a live store file and belong to it.
const STORE_SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];

/// Files earlier releases wrote that nothing reads any more.
const LEGACY_FILE_NAMES: &[&str] = &[
    "ata_failed_cache.json",
    "rpc_stats.json",
    "copilot_token.json",
    "github_token.json",
    "mcp.json",
];

/// A file and directories earlier releases created that nothing writes any more;
/// deleted only while empty, so nothing put into them is lost.
const LEGACY_EMPTY_NAMES: &[&str] = &["screenerbot.db", "cache_pool", "analysis-exports"];

const KEEP_FOREVER: Ladder = Ladder::same(Keep::Forever);

const KEEP_PENDING_DOWNLOADS: Ladder = Ladder {
    normal: Keep::Downloads(DownloadRetention::KeepPending),
    low: Keep::Downloads(DownloadRetention::KeepPending),
    critical: Keep::Downloads(DownloadRetention::InFlightOnly),
};

const LOG_RETENTION: Ladder = Ladder {
    normal: Keep::Bounded {
        max_age: Some(DAY),
        max_count: Some(7),
        max_total_bytes: Some(256 * MIB),
    },
    low: Keep::Bounded {
        max_age: Some(DAY),
        max_count: Some(7),
        max_total_bytes: Some(128 * MIB),
    },
    critical: Keep::Bounded {
        max_age: None,
        max_count: Some(0),
        max_total_bytes: None,
    },
};

/// Every file family, in classification order: the first family that matches a
/// path owns it.
pub const FAMILIES: &[FamilySpec] = &[
    FamilySpec {
        id: FamilyId::Config,
        category: Category::Settings,
        roots: &[],
        matcher: Matcher::Paths(&[paths::get_config_path]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::AppState,
        category: Category::Settings,
        roots: &[paths::get_data_directory],
        matcher: Matcher::Any(&[
            Matcher::Paths(&[
                paths::get_ui_state_path,
                paths::get_update_state_path,
                paths::get_agent_runtime_path,
                paths::get_process_lock_path,
                paths::get_wallet_replacement_marker_path,
                paths::get_wallet_replacement_outcome_path,
            ]),
            Matcher::PrefixSuffix(
                paths::WALLET_REPLACEMENT_SET_ASIDE_PREFIX,
                paths::WALLET_REPLACEMENT_SET_ASIDE_SUFFIX,
            ),
        ]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::Account,
        category: Category::Settings,
        roots: &[],
        matcher: Matcher::Paths(&[paths::get_account_store_path]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::DeviceId,
        category: Category::WalletKeys,
        roots: &[],
        matcher: Matcher::Paths(&[paths::get_device_id_path]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::StagedCores,
        category: Category::Updates,
        roots: &[],
        matcher: Matcher::Paths(&[paths::get_core_directory]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::Quarantine,
        category: Category::RecoveryCopies,
        roots: &[],
        matcher: Matcher::Paths(&[paths::get_quarantine_directory]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::UpgradeBackups,
        category: Category::RecoveryCopies,
        roots: &[paths::get_backups_directory],
        matcher: Matcher::Prefix(UPGRADE_BACKUP_PREFIX),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::StoreSnapshots,
        category: Category::RecoveryCopies,
        roots: &[paths::get_backups_directory],
        matcher: Matcher::Names(&[SNAPSHOTS_DIRECTORY_NAME]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::WalletResetBackups,
        category: Category::RecoveryCopies,
        roots: &[paths::get_backups_directory],
        matcher: Matcher::Prefix(paths::WALLET_RESET_DIRECTORY_PREFIX),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::LatestLog,
        category: Category::Logs,
        roots: &[],
        matcher: Matcher::Paths(&[paths::get_latest_log_path]),
        ladder: KEEP_FOREVER,
    },
    FamilySpec {
        id: FamilyId::PartialDownloads,
        category: Category::Updates,
        roots: &[paths::get_updates_directory],
        matcher: Matcher::PrefixSuffix(RELEASE_ASSET_PREFIX, PARTIAL_SUFFIX),
        ladder: KEEP_PENDING_DOWNLOADS,
    },
    FamilySpec {
        id: FamilyId::Downloads,
        category: Category::Updates,
        roots: &[paths::get_updates_directory],
        matcher: Matcher::Prefix(RELEASE_ASSET_PREFIX),
        ladder: KEEP_PENDING_DOWNLOADS,
    },
    FamilySpec {
        id: FamilyId::Logs,
        category: Category::Logs,
        roots: &[paths::get_logs_directory, paths::get_log_fallback_directory],
        matcher: Matcher::PrefixSuffix(paths::LOG_FILE_PREFIX, ".log"),
        ladder: LOG_RETENTION,
    },
    FamilySpec {
        id: FamilyId::LegacyFiles,
        category: Category::Leftovers,
        roots: &[paths::get_data_directory],
        matcher: Matcher::Names(LEGACY_FILE_NAMES),
        ladder: Ladder::same(Keep::Remove),
    },
    FamilySpec {
        id: FamilyId::LegacyEmpty,
        category: Category::Leftovers,
        roots: &[paths::get_data_directory, paths::get_base_directory],
        matcher: Matcher::Names(LEGACY_EMPTY_NAMES),
        ladder: Ladder::same(Keep::WhileNotEmpty),
    },
];

/// The directories the layout consists of; housekeeping lists them but never
/// deletes them.
pub const LAYOUT_DIRECTORIES: &[fn() -> std::path::PathBuf] = &[
    paths::get_base_directory,
    paths::get_data_directory,
    paths::get_logs_directory,
    paths::get_backups_directory,
    paths::get_updates_directory,
];

/// The declaration of `id`.
pub fn family(id: FamilyId) -> &'static FamilySpec {
    FAMILIES
        .iter()
        .find(|family| family.id == id)
        .expect("every family id is declared")
}

/// The live store `path` belongs to, as its main file or one of its sidecars.
fn store_of(path: &Path) -> Option<&'static StoreSpec> {
    let name = path.file_name()?.to_str()?;
    STORES.into_iter().find(|store| {
        store.paths().iter().any(|file| {
            if file.parent() != path.parent() {
                return false;
            }
            let Some(file_name) = file.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            name == file_name
                || name
                    .strip_prefix(file_name)
                    .is_some_and(|suffix| STORE_SIDECAR_SUFFIXES.contains(&suffix))
        })
    })
}

/// What `path` is. A live store or its sidecar wins over every family, so no rule can
/// ever select one.
pub fn classify(path: &Path) -> Classification {
    if LAYOUT_DIRECTORIES
        .iter()
        .any(|directory| directory() == path)
    {
        return Classification::LayoutDirectory;
    }
    if let Some(store) = store_of(path) {
        return Classification::Store(store);
    }
    FAMILIES
        .iter()
        .find(|family| family.contains(path))
        .map_or(Classification::Unknown, Classification::Family)
}
