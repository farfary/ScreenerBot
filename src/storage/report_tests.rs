// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the storage report: category totals in display order, the limit resolved for the disk, and a label in the catalog for every category.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::plan::{budget, BUDGET_MAX_BYTES};
use super::report::{summarize, DiskSpace, LimitMode};
use super::types::{Category, Disk, FamilyId, FileEntry, Limit, StoreUsage, Tier};
use crate::database::catalog::{self, StoreSpec};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

fn measured_at() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(2_000_000_000)
}

fn entry(family: FamilyId, bytes: u64) -> FileEntry {
    FileEntry {
        path: PathBuf::from(format!("/data/{family:?}")),
        family,
        bytes,
        modified: measured_at(),
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

/// Catalog key of each category's label. The match is exhaustive, so a new
/// category fails to compile until it is mapped here and in
/// `STORAGE_CATEGORY_LABELS` (ui/settings/storage_view.js).
fn label_key(category: Category) -> &'static str {
    match category {
        Category::WalletKeys => "settings-data-category-wallet-keys",
        Category::TradingRecords => "settings-data-category-trading-records",
        Category::WalletHistory => "settings-data-category-wallet-history",
        Category::MarketData => "settings-data-category-market-data",
        Category::Settings => "settings-data-category-settings",
        Category::RecoveryCopies => "settings-data-category-recovery-copies",
        Category::Updates => "settings-data-category-updates",
        Category::Logs => "settings-data-category-logs",
        Category::Leftovers => "settings-data-category-leftovers",
    }
}

#[test]
fn every_category_has_a_label_keyed_by_its_id() {
    for category in Category::ALL {
        let key = label_key(category);
        let id = serde_json::to_value(category).expect("serialize category");
        let id = id.as_str().expect("category id is a string");
        assert_eq!(
            key,
            format!("settings-data-category-{}", id.replace('_', "-")),
            "key does not follow the id {id}"
        );
        assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
    }
}

#[test]
fn usage_is_summed_by_category_in_display_order_with_empty_categories_kept() {
    let report = summarize(
        &[
            entry(FamilyId::Logs, 10 * MIB),
            entry(FamilyId::Downloads, 30 * MIB),
        ],
        &[
            store(&catalog::WALLETS, MIB),
            store(&catalog::TOKENS, 100 * MIB),
            store(&catalog::OHLCVS, 50 * MIB),
        ],
        roomy_disk(),
        0,
        measured_at(),
    );

    let categories: Vec<Category> = report.categories.iter().map(|c| c.category).collect();
    assert_eq!(categories, Category::ALL);
    let bytes_of = |category: Category| {
        report
            .categories
            .iter()
            .find(|usage| usage.category == category)
            .map(|usage| usage.bytes)
    };
    assert_eq!(bytes_of(Category::WalletKeys), Some(MIB));
    assert_eq!(bytes_of(Category::MarketData), Some(150 * MIB));
    assert_eq!(bytes_of(Category::Logs), Some(10 * MIB));
    assert_eq!(bytes_of(Category::Updates), Some(30 * MIB));
    assert_eq!(bytes_of(Category::TradingRecords), Some(0));
    assert_eq!(report.total_bytes, 191 * MIB);
    assert_eq!(
        report.disk,
        Some(DiskSpace {
            free_bytes: 400 * GIB,
            total_bytes: 1000 * GIB,
        })
    );
    assert_eq!(report.measured_at, 2_000_000_000);
    assert_eq!(report.last_cleanup, None);
    assert!(!report.running);
}

#[test]
fn an_automatic_limit_resolves_to_what_the_planner_holds_usage_to() {
    let report = summarize(
        &[],
        &[store(&catalog::TOKENS, GIB)],
        roomy_disk(),
        0,
        measured_at(),
    );

    assert_eq!(report.limit_mode, LimitMode::Automatic);
    assert_eq!(report.limit_megabytes, 0);
    assert_eq!(report.limit_bytes, BUDGET_MAX_BYTES);
    assert_eq!(report.automatic_limit_bytes, report.limit_bytes);
    assert_eq!(report.tier, Tier::Normal);
}

#[test]
fn a_changed_limit_applies_without_measuring_again() {
    let mut report = summarize(
        &[],
        &[store(&catalog::TOKENS, 3 * GIB)],
        roomy_disk(),
        0,
        measured_at(),
    );
    assert_eq!(report.tier, Tier::Normal);

    report.apply_limit(1024);
    assert_eq!(report.limit_mode, LimitMode::Fixed);
    assert_eq!(report.limit_megabytes, 1024);
    assert_eq!(
        report.limit_bytes,
        budget(3 * GIB, roomy_disk(), Limit::Fixed(GIB))
    );
    assert_eq!(report.limit_bytes, GIB);
    assert_eq!(report.automatic_limit_bytes, BUDGET_MAX_BYTES);
    assert_eq!(report.tier, Tier::Critical, "3 GiB is past 150% of 1 GiB");

    report.apply_limit(2560);
    assert_eq!(
        report.tier,
        Tier::Low,
        "3 GiB is over 2.5 GiB but within 150%"
    );

    report.apply_limit(0);
    assert_eq!(report.limit_mode, LimitMode::Automatic);
    assert_eq!(report.tier, Tier::Normal);
}

#[test]
fn a_low_disk_raises_the_tier_whatever_the_usage() {
    let report = summarize(
        &[],
        &[store(&catalog::TOKENS, MIB)],
        Some(Disk {
            free: 512 * MIB,
            total: 100 * GIB,
        }),
        0,
        measured_at(),
    );
    assert_eq!(report.tier, Tier::Critical);
}

#[test]
fn an_unknown_disk_still_resolves_a_limit() {
    let report = summarize(&[], &[], None, 2048, measured_at());
    assert_eq!(report.disk, None);
    assert_eq!(report.limit_bytes, 2 * GIB);
    assert_eq!(report.automatic_limit_bytes, BUDGET_MAX_BYTES);
    assert_eq!(report.tier, Tier::Normal);
}
