//! Version reporting and the transactional two-component update system.
//!
//! # How an update is delivered
//!
//! A published release contains two components under one version number:
//!
//! | Component | What it is | How it is replaced |
//! |---|---|---|
//! | core | the `screenerbot` binary, dashboard included | staged under the data directory and adopted on the next backend start — silent |
//! | shell | the Electron bundle (Chromium + main process) | the operating-system installer |
//!
//! screenerbot.io answers "what is the latest published version"; the GitHub
//! release for that version is the artifact record. Its `…-update-manifest.json`
//! asset names the shell revision the release was built with plus the per-platform
//! core artifacts. When the manifest's shell revision equals the revision of the
//! shell that launched this process, nothing about Electron changed and the
//! update is a [`UpdateKind::Core`] — a few tens of megabytes instead of a whole
//! Chromium bundle, applied with a backend restart and no OS dialog at all.
//!
//! Every artifact is bound three ways before it is used: the size and SHA-256
//! that screenerbot.io published, the digest GitHub reports for the same asset,
//! and — for a core update — the decompressed binary digest recorded in the
//! manifest, which the desktop shell re-verifies before every launch.

mod checker;
mod core_install;
mod download;
mod error;
mod installer;
mod manifest;
mod policy;
mod service;
pub mod types;

pub use checker::check_for_update;
pub use core_install::{read_staged_core, staged_core_version};
pub use download::start_download;
pub use error::{Error, Result};
pub use installer::prepare_install;
pub use policy::{apply_readiness, ApplyReadiness};
pub use service::{apply_now, start_update_check_service};
pub use types::*;

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{OnceCell, RwLock};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_SERVER_URL: &str = "https://screenerbot.io/api";
const GITHUB_RELEASES_API_URL: &str = "https://api.github.com/repos/farfary/ScreenerBot";
const DOWNLOAD_TIMEOUT_SECS: u64 = 30 * 60;
const MAX_UPDATE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Environment variables the Electron shell sets when it spawns the backend.
/// They are the only way this process can know which shell build owns it and
/// whether it was launched from a staged core.
const SHELL_REVISION_ENV: &str = "SCREENERBOT_SHELL_REVISION";
const CORE_STAGED_ENV: &str = "SCREENERBOT_CORE_STAGED";

static UPDATE_AVAILABLE: AtomicBool = AtomicBool::new(false);
static UPDATE_STATE: OnceCell<RwLock<UpdateState>> = OnceCell::const_new();

async fn state_lock() -> &'static RwLock<UpdateState> {
    UPDATE_STATE
        .get_or_init(|| async { RwLock::new(load_persisted_state()) })
        .await
}

fn state_path() -> std::path::PathBuf {
    crate::paths::get_data_directory().join("update-state.json")
}

fn load_persisted_state() -> UpdateState {
    let mut state = std::fs::read(state_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<UpdateState>(&bytes).ok())
        .unwrap_or_default();

    normalize_loaded_state(&mut state, staged_core_version().as_deref());
    UPDATE_AVAILABLE.store(state.available_update.is_some(), Ordering::SeqCst);
    state
}

/// Reconcile persisted state with the process that actually started.
///
/// The updater's whole job spans a restart, so the file on disk always describes
/// the *previous* process. Anything that claims to be mid-flight was killed by
/// that restart and must fail closed rather than look alive.
fn normalize_loaded_state(state: &mut UpdateState, staged_version: Option<&str>) {
    let target = state
        .available_update
        .as_ref()
        .map(|update| update.version.clone());

    // The restart landed: this process IS the version the update advertised.
    if target.as_deref() == Some(VERSION) {
        state.phase = UpdatePhase::Applied;
        state.applied_version = Some(VERSION.to_owned());
        state.available_update = None;
        state.deferred = None;
        state.download_progress = DownloadProgress::default();
        return;
    }

    if matches!(
        state.phase,
        UpdatePhase::Checking
            | UpdatePhase::Downloading
            | UpdatePhase::Verifying
            | UpdatePhase::Applying
    ) {
        state.phase = UpdatePhase::Failed;
        state.download_progress.downloading = false;
        state.download_progress.completed = false;
        state.download_progress.error =
            Some("Update was interrupted by an application restart".to_owned());
    }

    if state.phase == UpdatePhase::ReadyToApply {
        // A staged core is only still "ready" while the pointer names it.
        let still_staged = staged_version.is_some() && staged_version == target.as_deref();
        if !still_staged {
            state.phase = if target.is_some() {
                UpdatePhase::Available
            } else {
                UpdatePhase::Idle
            };
            state.download_progress = DownloadProgress::default();
        }
    }

    if state.phase == UpdatePhase::ReadyToInstall {
        let installer_exists = state
            .download_progress
            .downloaded_path
            .as_deref()
            .map(std::path::Path::new)
            .is_some_and(std::path::Path::exists);
        if !installer_exists {
            state.phase = UpdatePhase::Failed;
            state.download_progress.completed = false;
            state.download_progress.error =
                Some("Downloaded update file is no longer available".to_owned());
            state.download_progress.downloaded_path = None;
        }
    }
}

async fn persist_state(snapshot: UpdateState) {
    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        let path = state_path();
        let parent = path.parent().ok_or_else(|| {
            Error::Data(crate::errors::DataError::InvalidFormat {
                expected: "update state path with a parent directory".to_owned(),
                received: path.display().to_string(),
            })
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;

        let temporary = path.with_extension("json.part");
        let bytes = serde_json::to_vec_pretty(&snapshot).map_err(|error| {
            Error::Data(crate::errors::DataError::ParseError {
                data_type: "update state".to_owned(),
                error: error.to_string(),
            })
        })?;

        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
        }
        #[cfg(not(unix))]
        std::fs::write(&temporary, bytes)
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;

        std::fs::rename(&temporary, &path)
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
        Ok(())
    })
    .await;

    let error = match result {
        Ok(Ok(())) => return,
        Ok(Err(error)) => error,
        Err(error) => Error::Internal(crate::errors::InternalError::from(error)),
    };
    crate::logger::warning(
        crate::logger::LogTag::System,
        &format!("Could not persist update state: {error}"),
    );
}

async fn mutate_state<F>(mutator: F) -> UpdateState
where
    F: FnOnce(&mut UpdateState),
{
    // Keep mutation and persistence ordered under the same writer guard. If
    // snapshots were persisted after releasing it, a slower earlier write
    // could overwrite a newer state or race on the shared `.part` file.
    let mut state = state_lock().await.write().await;
    mutator(&mut state);
    let snapshot = state.clone();
    persist_state(snapshot.clone()).await;
    snapshot
}

async fn mutate_state_with_result<F, T>(mutator: F) -> Result<T>
where
    F: FnOnce(&mut UpdateState) -> Result<T>,
{
    let mut state = state_lock().await.write().await;
    let value = mutator(&mut state)?;
    let snapshot = state.clone();
    persist_state(snapshot).await;
    Ok(value)
}

pub fn get_version() -> &'static str {
    VERSION
}

pub fn get_version_info() -> VersionInfo {
    VersionInfo {
        version: VERSION.to_owned(),
        platform: platform_display_name().to_owned(),
        shell_revision: local_shell_revision(),
        core_staged: is_core_staged_launch(),
    }
}

pub fn is_update_available() -> bool {
    UPDATE_AVAILABLE.load(Ordering::SeqCst)
}

pub async fn get_update_state() -> UpdateState {
    state_lock().await.read().await.clone()
}

pub fn is_newer_version(current: &str, remote: &str) -> bool {
    match (
        semver::Version::parse(current),
        semver::Version::parse(remote),
    ) {
        (Ok(current), Ok(remote)) => remote > current,
        _ => false,
    }
}

/// Revision of the Electron shell that launched this process.
///
/// `None` in headless mode and in an unpackaged development run. A core-only
/// update is never offered without it: not knowing which shell owns the process
/// means not being able to prove the shell does not also need replacing.
pub fn local_shell_revision() -> Option<String> {
    let value = std::env::var(SHELL_REVISION_ENV).ok()?;
    let value = value.trim();
    if value.is_empty() || value == "dev" || value == "unknown" {
        return None;
    }
    Some(value.to_owned())
}

/// Whether the shell launched this process from a staged core rather than from
/// the binary inside the installed application bundle.
pub fn is_core_staged_launch() -> bool {
    std::env::var(CORE_STAGED_ENV).is_ok_and(|value| value == "1")
}

fn platform_display_name() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macOS (Apple Silicon)",
        ("macos", "x86_64") => "macOS (Intel)",
        ("windows", "aarch64") => "Windows (ARM64)",
        ("windows", "x86_64") => "Windows (x64)",
        ("linux", "aarch64") => "Linux (ARM64)",
        ("linux", "x86_64") => "Linux (x64)",
        _ => "Unknown",
    }
}

fn platform_key(gui: bool) -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH, gui) {
        ("macos", "x86_64", _) => "macos-x64",
        ("macos", "aarch64", _) => "macos-arm64",
        ("windows", "x86_64", _) => "windows-x64",
        ("windows", "aarch64", _) => "windows-arm64",
        ("linux", "x86_64", true) => "linux-x64-deb",
        ("linux", "aarch64", true) => "linux-arm64-deb",
        ("linux", "x86_64", false) => "linux-x64-headless",
        ("linux", "aarch64", false) => "linux-arm64-headless",
        _ => "unknown",
    }
}

fn current_platform_key() -> &'static str {
    platform_key(crate::arguments::is_gui_enabled())
}

/// Key a core artifact is published under. The core binary does not depend on
/// the packaging variant, so desktop and headless on the same machine share one.
pub(super) fn core_platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "x86_64") => "macos-x64",
        ("macos", "aarch64") => "macos-arm64",
        ("windows", "x86_64") => "windows-x64",
        ("windows", "aarch64") => "windows-arm64",
        ("linux", "x86_64") => "linux-x64",
        ("linux", "aarch64") => "linux-arm64",
        _ => "unknown",
    }
}

/// Name of the executable inside a staged core directory.
pub(super) fn core_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "screenerbot.exe"
    } else {
        "screenerbot"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(version: &str) -> UpdateInfo {
        UpdateInfo {
            version: version.to_owned(),
            filename: format!("ScreenerBot-v{version}-macOS-arm64.dmg"),
            download_url: "/api/releases/download".to_owned(),
            file_size: 1,
            checksum: "a".repeat(64),
            release_notes: None,
            release_date: String::new(),
            kind: UpdateKind::Core,
            core: None,
            shell_revision: None,
        }
    }

    #[test]
    fn strict_version_comparison_rejects_malformed_versions() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("1.0.1", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.bad.1"));
        assert!(!is_newer_version("1", "2.0.0"));
    }

    #[test]
    fn linux_platform_distinguishes_desktop_and_headless_channels() {
        if std::env::consts::OS == "linux" {
            assert!(platform_key(true).ends_with("-deb"));
            assert!(platform_key(false).ends_with("-headless"));
            // The core artifact is shared by both packaging variants.
            assert!(!core_platform_key().ends_with("-deb"));
        }
    }

    #[test]
    fn interrupted_update_state_fails_closed() {
        let mut state = UpdateState {
            phase: UpdatePhase::Downloading,
            available_update: Some(update("99.0.0")),
            download_progress: DownloadProgress {
                downloading: true,
                ..DownloadProgress::default()
            },
            ..UpdateState::default()
        };
        normalize_loaded_state(&mut state, None);
        assert_eq!(state.phase, UpdatePhase::Failed);
        assert!(!state.download_progress.downloading);
        assert!(state.download_progress.error.is_some());
    }

    #[test]
    fn completed_install_is_recorded_after_restart() {
        let mut state = UpdateState {
            phase: UpdatePhase::Applying,
            available_update: Some(update(VERSION)),
            ..UpdateState::default()
        };
        normalize_loaded_state(&mut state, Some(VERSION));
        assert_eq!(state.phase, UpdatePhase::Applied);
        assert_eq!(state.applied_version.as_deref(), Some(VERSION));
        assert!(state.available_update.is_none());
    }

    #[test]
    fn staged_core_that_vanished_falls_back_to_available() {
        let mut state = UpdateState {
            phase: UpdatePhase::ReadyToApply,
            available_update: Some(update("99.0.0")),
            ..UpdateState::default()
        };
        normalize_loaded_state(&mut state, None);
        assert_eq!(state.phase, UpdatePhase::Available);

        let mut still_there = UpdateState {
            phase: UpdatePhase::ReadyToApply,
            available_update: Some(update("99.0.0")),
            ..UpdateState::default()
        };
        normalize_loaded_state(&mut still_there, Some("99.0.0"));
        assert_eq!(still_there.phase, UpdatePhase::ReadyToApply);
    }

    #[test]
    fn transfer_size_prefers_the_core_artifact() {
        let mut info = update("99.0.0");
        info.file_size = 200_000_000;
        info.core = Some(CoreArtifact {
            filename: "core.gz".to_owned(),
            size: 20_000_000,
            sha256: "b".repeat(64),
            binary_size: 80_000_000,
            binary_sha256: "c".repeat(64),
        });
        assert_eq!(info.transfer_size(), 20_000_000);

        info.kind = UpdateKind::Full;
        assert_eq!(info.transfer_size(), 200_000_000);
    }
}
