// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the file-family declarations and path classification.

use super::families::{classify, family, FAMILIES};
use super::types::{Classification, FamilyId, Keep, Tier};
use crate::paths;

/// Path functions that name a directory of the layout rather than a family or a store.
const ROOT_ONLY: &[&str] = &[
    "get_base_directory",
    "get_data_directory",
    "get_backups_directory",
    "get_logs_directory",
    "get_updates_directory",
];

fn path_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub fn get_"))
        .filter(|rest| rest.contains("() -> PathBuf"))
        .filter_map(|rest| rest.split('(').next())
        .map(|name| format!("get_{name}"))
        .collect()
}

#[test]
fn every_path_function_is_a_family_a_store_or_a_layout_directory() {
    let families = include_str!("families.rs");
    let catalog = include_str!("../database/catalog.rs");
    let mut functions = path_functions(include_str!("../paths/mod.rs"));
    functions.extend(path_functions(include_str!("../paths/files.rs")));
    functions.extend(path_functions(include_str!("../paths/database.rs")));
    assert!(functions.len() > 20, "path functions were not found");

    let undeclared: Vec<&String> = functions
        .iter()
        .filter(|name| !ROOT_ONLY.contains(&name.as_str()))
        .filter(|name| !families.contains(&format!("paths::{name}")))
        .filter(|name| !catalog.contains(&format!("paths::{name},")))
        .collect();
    assert!(
        undeclared.is_empty(),
        "declare a retention family or a catalog store for {undeclared:?}"
    );
}

#[test]
fn every_family_id_is_declared_once() {
    let ids = [
        FamilyId::Config,
        FamilyId::AppState,
        FamilyId::Account,
        FamilyId::DeviceId,
        FamilyId::StagedCores,
        FamilyId::Quarantine,
        FamilyId::UpgradeBackups,
        FamilyId::StoreSnapshots,
        FamilyId::WalletResetBackups,
        FamilyId::LatestLog,
        FamilyId::Downloads,
        FamilyId::PartialDownloads,
        FamilyId::Logs,
        FamilyId::LegacyFiles,
        FamilyId::LegacyEmpty,
    ];
    for id in ids {
        match id {
            FamilyId::Config
            | FamilyId::AppState
            | FamilyId::Account
            | FamilyId::DeviceId
            | FamilyId::StagedCores
            | FamilyId::Quarantine
            | FamilyId::UpgradeBackups
            | FamilyId::StoreSnapshots
            | FamilyId::WalletResetBackups
            | FamilyId::LatestLog
            | FamilyId::Downloads
            | FamilyId::PartialDownloads
            | FamilyId::Logs
            | FamilyId::LegacyFiles
            | FamilyId::LegacyEmpty => {}
        }
        assert_eq!(
            FAMILIES.iter().filter(|spec| spec.id == id).count(),
            1,
            "{id:?}"
        );
        assert_eq!(family(id).id, id);
    }
    assert_eq!(FAMILIES.len(), ids.len());
}

fn strictness(keep: Keep) -> u8 {
    match keep {
        Keep::Forever => 0,
        Keep::Bounded { .. } | Keep::Downloads(_) | Keep::WhileNotEmpty => 1,
        Keep::Remove => 2,
    }
}

#[test]
fn no_ladder_keeps_more_under_more_pressure() {
    for spec in FAMILIES {
        let steps = Tier::ALL.map(|tier| spec.ladder.at(tier));
        for pair in steps.windows(2) {
            assert!(
                strictness(pair[0]) <= strictness(pair[1]),
                "{:?} loosens from {:?} to {:?}",
                spec.id,
                pair[0],
                pair[1]
            );
            if let (
                Keep::Bounded {
                    max_age: age_a,
                    max_count: count_a,
                    max_total_bytes: bytes_a,
                },
                Keep::Bounded {
                    max_age: age_b,
                    max_count: count_b,
                    max_total_bytes: bytes_b,
                },
            ) = (pair[0], pair[1])
            {
                let tighter = |a: Option<u128>, b: Option<u128>| match (a, b) {
                    (_, None) => a.is_none() || b.is_some(),
                    (None, Some(_)) => true,
                    (Some(a), Some(b)) => b <= a,
                };
                assert!(
                    tighter(
                        age_a.map(|age| age.as_millis()),
                        age_b.map(|age| age.as_millis())
                    ) || count_b == Some(0),
                    "{:?} age",
                    spec.id
                );
                assert!(
                    tighter(count_a.map(|c| c as u128), count_b.map(|c| c as u128)),
                    "{:?} count",
                    spec.id
                );
                assert!(
                    tighter(bytes_a.map(u128::from), bytes_b.map(u128::from)) || count_b == Some(0),
                    "{:?} bytes",
                    spec.id
                );
            }
        }
    }
}

fn family_of(path: &std::path::Path) -> Option<FamilyId> {
    match classify(path) {
        Classification::Family(spec) => Some(spec.id),
        _ => None,
    }
}

#[test]
fn names_are_classified_into_their_family() {
    let data = paths::get_data_directory();
    let updates = paths::get_updates_directory();
    let logs = paths::get_logs_directory();
    let backups = paths::get_backups_directory();
    let cases = [
        (
            updates.join("ScreenerBot-v0.3.0-macOS-arm64.dmg"),
            FamilyId::Downloads,
        ),
        (
            updates.join("ScreenerBot-v0.3.0-macOS-arm64-core.gz"),
            FamilyId::Downloads,
        ),
        (
            updates.join("ScreenerBot-v0.3.0-macOS-arm64.dmg.part"),
            FamilyId::PartialDownloads,
        ),
        (
            logs.join("screenerbot_2026-10-10_12-00-00.log"),
            FamilyId::Logs,
        ),
        (
            logs.join("screenerbot_2026-10-10_12-00-00_2.log"),
            FamilyId::Logs,
        ),
        (
            paths::get_log_fallback_directory().join("screenerbot_2026-10-10_12-00-00.log"),
            FamilyId::Logs,
        ),
        (logs.join("latest.log"), FamilyId::LatestLog),
        (
            backups.join("wallet-reset-20261010-120000"),
            FamilyId::WalletResetBackups,
        ),
        (
            backups.join("upgrade-positions.db"),
            FamilyId::UpgradeBackups,
        ),
        (backups.join("snapshots"), FamilyId::StoreSnapshots),
        (data.join("quarantine"), FamilyId::Quarantine),
        (data.join("core"), FamilyId::StagedCores),
        (data.join("config.toml"), FamilyId::Config),
        (data.join("account.dat"), FamilyId::Account),
        (data.join("mcp.json"), FamilyId::LegacyFiles),
        (data.join("cache_pool"), FamilyId::LegacyEmpty),
        (
            paths::get_base_directory().join("analysis-exports"),
            FamilyId::LegacyEmpty,
        ),
    ];
    for (path, expected) in cases {
        assert_eq!(family_of(&path), Some(expected), "{}", path.display());
    }
}

#[test]
fn stores_and_unknown_names_belong_to_no_family() {
    let data = paths::get_data_directory();
    for name in ["wallets.db", "positions.db-wal", "tokens.db-shm", "ai.db"] {
        assert!(
            matches!(classify(&data.join(name)), Classification::Store(_)),
            "{name}"
        );
    }
    for path in [
        data.join("config.toml.bak"),
        data.join("notes.txt"),
        paths::get_updates_directory().join("installer.dmg"),
        paths::get_logs_directory().join("crash.txt"),
        data.join("quarantine").join("tokens.db"),
    ] {
        assert!(
            matches!(classify(&path), Classification::Unknown),
            "{}",
            path.display()
        );
    }
    assert!(matches!(
        classify(&paths::get_backups_directory()),
        Classification::LayoutDirectory
    ));
}

#[test]
fn the_vault_and_identity_files_are_kept_forever() {
    for id in [
        FamilyId::DeviceId,
        FamilyId::Account,
        FamilyId::Config,
        FamilyId::Quarantine,
        FamilyId::UpgradeBackups,
        FamilyId::StoreSnapshots,
        FamilyId::WalletResetBackups,
    ] {
        for tier in Tier::ALL {
            assert_eq!(family(id).ladder.at(tier), Keep::Forever, "{id:?} {tier:?}");
        }
    }
}
