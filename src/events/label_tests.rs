// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard label maps for event categories and severities must resolve in the `en` catalog.

use super::types::{EventCategory, Severity};

/// Catalog key of each category label, mirrored by `EVENT_CATEGORY_LABELS`
/// (ui/event_labels.js). The match is exhaustive, so a new variant fails to
/// compile until it is mapped here.
fn category_key(category: &EventCategory) -> &'static str {
    match category {
        EventCategory::Swap => "events-category-swap",
        EventCategory::Transaction => "events-category-transaction",
        EventCategory::Pool => "events-category-pool",
        EventCategory::Token => "events-category-token",
        EventCategory::System => "events-category-system",
        EventCategory::Position => "events-category-position",
        EventCategory::Wallet => "events-category-wallet",
        EventCategory::Trader => "events-category-trader",
        EventCategory::Ohlcv => "events-category-ohlcv",
        EventCategory::Rpc => "events-category-rpc",
        EventCategory::Api => "events-category-api",
        EventCategory::Security => "events-category-security",
        EventCategory::Connectivity => "events-category-connectivity",
        EventCategory::Filtering => "events-category-filtering",
        EventCategory::ScheduledTask => "events-category-scheduled-task",
        EventCategory::Other(_) => "events-category-other",
    }
}

/// Catalog key of each severity label, mirrored by `EVENT_SEVERITY_LABELS`.
fn severity_key(severity: &Severity) -> &'static str {
    match severity {
        Severity::Info => "common-severity-info",
        Severity::Warn => "common-severity-warning",
        Severity::Error => "common-severity-error",
        Severity::Debug => "common-severity-debug",
    }
}

fn assert_in_catalog(key: &str) {
    assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
}

#[test]
fn category_labels_exist_in_the_catalog() {
    for category in [
        EventCategory::Swap,
        EventCategory::Transaction,
        EventCategory::Pool,
        EventCategory::Token,
        EventCategory::System,
        EventCategory::Position,
        EventCategory::Wallet,
        EventCategory::Trader,
        EventCategory::Ohlcv,
        EventCategory::Rpc,
        EventCategory::Api,
        EventCategory::Security,
        EventCategory::Connectivity,
        EventCategory::Filtering,
        EventCategory::ScheduledTask,
        EventCategory::Other(String::new()),
    ] {
        assert_in_catalog(category_key(&category));
    }
}

#[test]
fn severity_labels_exist_in_the_catalog() {
    for severity in [
        Severity::Info,
        Severity::Warn,
        Severity::Error,
        Severity::Debug,
    ] {
        assert_in_catalog(severity_key(&severity));
    }
}
