// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Which downloaded release files storage housekeeping may delete: only the installer of a pending newer release is kept, and the partial file of a download in progress is never touched.

use std::time::Duration;

use super::types::UpdateKind;
use super::{is_newer_version, VERSION};

/// The release whose artifact is waiting in the updates directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDownload {
    pub version: String,
    /// File name of the artifact this machine downloads for it: the core archive of
    /// a core update, the installer otherwise.
    pub filename: String,
}

/// What the update service knows about downloads at the moment housekeeping runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadFacts {
    /// Version of the running core.
    pub running_version: String,
    pub pending: Option<PendingDownload>,
    /// Whether a download is writing its partial file right now.
    pub downloading: bool,
}

/// How much of the updates directory a retention step keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadRetention {
    /// The pending newer artifact and its partial download.
    KeepPending,
    /// Only the partial file a download is writing right now.
    InFlightOnly,
}

/// Name prefix of every release asset: `ScreenerBot-v<version>-<platform>…`.
pub const RELEASE_ASSET_PREFIX: &str = "ScreenerBot-v";

/// Suffix of the file a download writes before it is verified and renamed.
pub const PARTIAL_SUFFIX: &str = ".part";

/// Whether the updates-directory entry `name`, last written `idle` ago, stays.
///
/// A completed artifact stays only while it is the pending release's own file and
/// that release is newer than the running core; once applied, superseded or older it
/// goes. A partial file stays while its download runs, and otherwise only while it
/// belongs to the pending newer release and was written within `partial_idle_limit`,
/// so a resumable download is not thrown away but an abandoned one does not linger.
pub fn keeps_download(
    name: &str,
    idle: Duration,
    facts: &DownloadFacts,
    retention: DownloadRetention,
    partial_idle_limit: Duration,
) -> bool {
    let Some(pending) = facts
        .pending
        .as_ref()
        .filter(|pending| is_newer_version(&facts.running_version, &pending.version))
    else {
        return false;
    };
    match name.strip_suffix(PARTIAL_SUFFIX) {
        Some(artifact) if artifact == pending.filename => {
            facts.downloading
                || (retention == DownloadRetention::KeepPending && idle < partial_idle_limit)
        }
        Some(_) => false,
        None => retention == DownloadRetention::KeepPending && name == pending.filename,
    }
}

/// The download facts from the update service's current state.
pub async fn download_facts() -> DownloadFacts {
    let state = super::get_update_state().await;
    let pending = state.available_update.map(|update| PendingDownload {
        filename: match (update.kind, update.core.as_ref()) {
            (UpdateKind::Core, Some(core)) => core.filename.clone(),
            _ => update.filename.clone(),
        },
        version: update.version,
    });
    DownloadFacts {
        running_version: VERSION.to_owned(),
        pending,
        downloading: state.download_progress.downloading,
    }
}
