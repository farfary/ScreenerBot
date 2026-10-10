// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests for the downloaded-release retention rules.

use std::time::Duration;

use super::retention::{keeps_download, DownloadFacts, DownloadRetention, PendingDownload};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);
const IDLE_LIMIT: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const INSTALLER: &str = "ScreenerBot-v0.3.0-macOS-arm64.dmg";

fn facts(running: &str, pending: Option<&str>, downloading: bool) -> DownloadFacts {
    DownloadFacts {
        running_version: running.to_owned(),
        pending: pending.map(|version| PendingDownload {
            version: version.to_owned(),
            filename: INSTALLER.to_owned(),
        }),
        downloading,
    }
}

fn keeps(name: &str, idle: Duration, facts: &DownloadFacts, retention: DownloadRetention) -> bool {
    keeps_download(name, idle, facts, retention, IDLE_LIMIT)
}

#[test]
fn the_pending_newer_installer_is_kept() {
    let facts = facts("0.2.14", Some("0.3.0"), false);
    assert!(keeps(
        INSTALLER,
        DAY,
        &facts,
        DownloadRetention::KeepPending
    ));
}

#[test]
fn an_installer_the_running_core_already_reached_is_deleted() {
    for running in ["0.3.0", "0.3.1"] {
        let facts = facts(running, Some("0.3.0"), false);
        assert!(!keeps(
            INSTALLER,
            DAY,
            &facts,
            DownloadRetention::KeepPending
        ));
    }
    let nothing_pending = facts("0.2.14", None, false);
    assert!(!keeps(
        INSTALLER,
        DAY,
        &nothing_pending,
        DownloadRetention::KeepPending
    ));
}

#[test]
fn an_installer_superseded_by_a_newer_pending_release_is_deleted() {
    let facts = facts("0.2.14", Some("0.3.0"), false);
    for name in [
        "ScreenerBot-v0.2.13-macOS-arm64.dmg",
        "ScreenerBot-v0.2.15-macOS-arm64.dmg",
        "ScreenerBot-v0.3.0-macOS-arm64-core.gz",
    ] {
        assert!(
            !keeps(name, DAY, &facts, DownloadRetention::KeepPending),
            "{name} kept"
        );
    }
}

#[test]
fn a_recent_partial_of_the_pending_release_is_kept_for_resume() {
    let partial = format!("{INSTALLER}.part");
    let facts = facts("0.2.14", Some("0.3.0"), false);
    assert!(keeps(&partial, DAY, &facts, DownloadRetention::KeepPending));
    assert!(!keeps(
        &partial,
        IDLE_LIMIT + DAY,
        &facts,
        DownloadRetention::KeepPending
    ));
}

#[test]
fn a_partial_of_any_other_artifact_is_deleted() {
    let facts = facts("0.2.14", Some("0.3.0"), true);
    assert!(!keeps(
        "ScreenerBot-v0.2.15-macOS-arm64.dmg.part",
        Duration::ZERO,
        &facts,
        DownloadRetention::KeepPending
    ));
}

#[test]
fn critical_retention_deletes_the_pending_installer() {
    let facts = facts("0.2.14", Some("0.3.0"), false);
    assert!(!keeps(
        INSTALLER,
        Duration::ZERO,
        &facts,
        DownloadRetention::InFlightOnly
    ));
    assert!(!keeps(
        &format!("{INSTALLER}.part"),
        Duration::ZERO,
        &facts,
        DownloadRetention::InFlightOnly
    ));
}

#[test]
fn the_partial_of_a_running_download_survives_every_retention() {
    let partial = format!("{INSTALLER}.part");
    let facts = facts("0.2.14", Some("0.3.0"), true);
    for retention in [
        DownloadRetention::KeepPending,
        DownloadRetention::InFlightOnly,
    ] {
        assert!(keeps(&partial, IDLE_LIMIT * 4, &facts, retention));
    }
}
