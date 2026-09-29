//! Types for version management and the two-component update system.
//!
//! A release ships two independently replaceable components that share one
//! version number:
//!
//! * **core** — the `screenerbot` binary (it also embeds the whole dashboard).
//!   It can be replaced silently: the file is staged under the data directory
//!   and the desktop shell picks it up the next time it launches the backend.
//! * **shell** — the Electron bundle (Chromium + the main process). Replacing it
//!   needs the operating-system installer, so it is only downloaded when the
//!   shell actually changed, identified by `shell_revision`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::i18n::{ids, UiArg, UiText};

/// Current version information
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub platform: String,
    /// Revision of the Electron shell this process was launched by, when it is
    /// known. Absent in headless mode and in unpackaged development runs.
    pub shell_revision: Option<String>,
    /// Whether this core binary was activated from a staged silent update
    /// rather than from the installed application bundle.
    pub core_staged: bool,
}

/// Which components a pending update has to replace.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpdateKind {
    /// Only the core binary changed — applied silently with a backend restart.
    #[default]
    Core,
    /// The Electron shell changed too — needs the operating-system installer.
    Full,
}

impl UpdateKind {
    /// Whether the update can be applied without any operating-system dialog.
    pub fn is_silent(self) -> bool {
        matches!(self, UpdateKind::Core)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            UpdateKind::Core => "core",
            UpdateKind::Full => "full",
        }
    }
}

/// The core artifact for one operating system and architecture, as published in
/// the release update manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoreArtifact {
    /// Release asset name, e.g. `ScreenerBot-v0.2.2-macOS-arm64-core.gz`.
    pub filename: String,
    /// Size of the compressed asset in bytes.
    pub size: u64,
    /// SHA-256 of the compressed asset.
    pub sha256: String,
    /// Size of the decompressed binary in bytes.
    pub binary_size: u64,
    /// SHA-256 of the decompressed binary — what the shell re-verifies at launch.
    pub binary_sha256: String,
}

/// The update manifest published as a release asset next to the installers.
///
/// It is the only place that records which Electron shell a release was built
/// with, which is what makes a core-only update decidable by the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub schema: u32,
    pub version: String,
    pub shell_revision: String,
    #[serde(default)]
    pub core: std::collections::BTreeMap<String, CoreArtifact>,
}

/// Information about an available update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub filename: String,
    pub download_url: String,
    pub file_size: u64,
    pub checksum: String,
    /// Website-published digest of the release's update manifest. This is the
    /// independent authority for silent-update metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_checksum: Option<String>,
    pub release_notes: Option<String>,
    pub release_date: String,
    /// Whether this update can be applied silently or needs the OS installer.
    #[serde(default)]
    pub kind: UpdateKind,
    /// Core artifact for this machine when the release published one.
    #[serde(default)]
    pub core: Option<CoreArtifact>,
    /// Shell revision the release was built with, when the manifest was readable.
    #[serde(default)]
    pub shell_revision: Option<String>,
}

impl UpdateInfo {
    /// Bytes that actually have to be transferred to apply this update.
    pub fn transfer_size(&self) -> u64 {
        match (self.kind, self.core.as_ref()) {
            (UpdateKind::Core, Some(core)) => core.size,
            _ => self.file_size,
        }
    }
}

/// Release information retained for the in-app notes after an update restarts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseSummary {
    pub version: String,
    pub release_notes: Option<String>,
    pub release_date: String,
}

impl From<&UpdateInfo> for ReleaseSummary {
    fn from(update: &UpdateInfo) -> Self {
        Self {
            version: update.version.clone(),
            release_notes: update.release_notes.clone(),
            release_date: update.release_date.clone(),
        }
    }
}

/// API response wrapper
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Update check response from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateCheckData {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update: Option<UpdateResponseData>,
    /// Release the server considers current. Present when no update is offered,
    /// so the app can still show the notes for the version it is running.
    #[serde(default)]
    pub latest_release: Option<LatestReleaseData>,
}

/// Published release described without any downloadable artifact.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct LatestReleaseData {
    pub version: String,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

/// Update data from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateResponseData {
    pub version: String,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub download_url: String,
    pub filename: String,
    pub file_size: u64,
    pub checksum: String,
    pub manifest_checksum: Option<String>,
}

/// Where an update currently is in its lifecycle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    CheckFailed,
    Downloading,
    Verifying,
    /// A core update is staged and activates on the next backend start. No
    /// operating-system interaction is left.
    ReadyToApply,
    /// A full installer is downloaded and verified; the operating-system
    /// installer still has to run.
    ReadyToInstall,
    /// A restart to activate a staged core update has been requested.
    Applying,
    /// The running process is the updated version.
    Applied,
    Failed,
}

impl UpdatePhase {
    /// Whether an artifact for the advertised update is downloaded and verified.
    pub fn is_ready(self) -> bool {
        matches!(
            self,
            UpdatePhase::ReadyToApply | UpdatePhase::ReadyToInstall
        )
    }

    pub fn is_busy(self) -> bool {
        matches!(
            self,
            UpdatePhase::Checking
                | UpdatePhase::Downloading
                | UpdatePhase::Verifying
                | UpdatePhase::Applying
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DownloadProgress {
    pub version: Option<String>,
    pub checksum: Option<String>,
    pub downloading: bool,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
    pub error: Option<String>,
    pub completed: bool,
    pub downloaded_path: Option<String>,
}

/// Why an otherwise ready update has not been applied automatically.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeferReason {
    /// Automatic installation is switched off in settings.
    AutomaticInstallDisabled,
    /// Positions are open or a trade is in flight and deferring is enabled.
    TradingActive,
    /// The update replaces the Electron shell, which needs the OS installer.
    NeedsInstaller,
}

impl DeferReason {
    pub fn ui_text(self) -> UiText {
        UiText::new(match self {
            DeferReason::AutomaticInstallDisabled => ids::UPDATES_DEFER_AUTOMATIC_INSTALL_DISABLED,
            DeferReason::TradingActive => ids::UPDATES_DEFER_TRADING_ACTIVE,
            DeferReason::NeedsInstaller => ids::UPDATES_DEFER_NEEDS_INSTALLER,
        })
    }
}

/// Text for a failed update check; `cause` is the technical error.
pub fn check_failed_text(cause: String) -> UiText {
    UiText::new(ids::UPDATES_CHECK_FAILED).arg("cause", UiArg::Text(cause))
}

/// Reads `check_error` from either shape. Builds before this field carried
/// catalog text persisted the error as a plain string; that string becomes the
/// `cause` of the legacy message so stored state still loads.
fn deserialize_check_error<'de, D>(deserializer: D) -> Result<Option<UiText>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Stored {
        Text(UiText),
        Legacy(String),
    }

    Ok(
        Option::<Stored>::deserialize(deserializer)?.map(|stored| match stored {
            Stored::Text(text) => text,
            Stored::Legacy(cause) => {
                UiText::new(ids::UPDATES_CHECK_FAILED_LEGACY).arg("cause", UiArg::Text(cause))
            }
        }),
    )
}

/// Update state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct UpdateState {
    pub phase: UpdatePhase,
    pub available_update: Option<UpdateInfo>,
    pub last_check: Option<DateTime<Utc>>,
    pub last_check_attempt: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "deserialize_check_error")]
    pub check_error: Option<UiText>,
    pub download_progress: DownloadProgress,
    /// Set when an update is staged but its activation was postponed.
    pub deferred: Option<DeferReason>,
    /// Version this process is running after activating a staged core update.
    pub applied_version: Option<String>,
    /// The most recently applied release, retained for Settings > Release Notes.
    pub last_release: Option<ReleaseSummary>,
}

/// The record that tells the desktop shell which core binary to launch.
///
/// Written atomically the moment a core update finishes verification, and read
/// by `electron/src/core_resolver.js` before every backend spawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedCore {
    pub version: String,
    /// Path of the staged binary relative to the `core` directory.
    pub path: String,
    /// SHA-256 of the staged binary.
    pub sha256: String,
    pub size: u64,
    pub staged_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{format_en, format_message, source_locale, LanguageIdentifier};

    fn source() -> LanguageIdentifier {
        source_locale().parse().unwrap()
    }

    /// Exhaustive on purpose: a new variant fails to compile until its catalog
    /// key is named here.
    fn catalog_key(reason: DeferReason) -> &'static str {
        match reason {
            DeferReason::AutomaticInstallDisabled => "updates-defer-automatic-install-disabled",
            DeferReason::TradingActive => "updates-defer-trading-active",
            DeferReason::NeedsInstaller => "updates-defer-needs-installer",
        }
    }

    #[test]
    fn every_defer_reason_has_a_catalog_message() {
        for reason in [
            DeferReason::AutomaticInstallDisabled,
            DeferReason::TradingActive,
            DeferReason::NeedsInstaller,
        ] {
            let key = catalog_key(reason);
            assert_eq!(reason.ui_text().id, key);
            assert_ne!(format_en(key, None), key, "missing {key}");
            let code = serde_json::to_value(reason).unwrap();
            assert_eq!(
                key.strip_prefix("updates-defer-")
                    .map(|c| c.replace('-', "_")),
                code.as_str().map(str::to_owned),
                "key does not follow the serialized code {code}"
            );
        }
    }

    #[test]
    fn a_check_error_round_trips_as_catalog_text() {
        let state = UpdateState {
            check_error: Some(check_failed_text("HTTP 503".to_owned())),
            ..UpdateState::default()
        };
        let json = serde_json::to_string(&state).unwrap();
        let loaded: UpdateState = serde_json::from_str(&json).unwrap();
        let text = loaded.check_error.expect("check error persists");
        assert_eq!(text.id, "updates-check-failed");
        assert_eq!(
            text.args.get("cause"),
            Some(&UiArg::Text("HTTP 503".into()))
        );
    }

    #[test]
    fn a_plain_string_check_error_from_an_older_build_still_loads() {
        let loaded: UpdateState =
            serde_json::from_str(r#"{"phase":"check_failed","check_error":"connection refused"}"#)
                .unwrap();
        let text = loaded.check_error.expect("legacy error is kept");
        assert_eq!(text.id, "updates-check-failed-legacy");
        assert_eq!(
            text.args.get("cause"),
            Some(&UiArg::Text("connection refused".into()))
        );
        assert_eq!(text.render_plain(&source()), "connection refused");
        assert!(format_message(&source(), &text.id, None).is_some());
    }

    #[test]
    fn a_missing_or_null_check_error_loads_as_none() {
        let missing: UpdateState = serde_json::from_str("{}").unwrap();
        assert!(missing.check_error.is_none());
        let null: UpdateState = serde_json::from_str(r#"{"check_error":null}"#).unwrap();
        assert!(null.check_error.is_none());
    }
}
