//! Transactional update checking and installer staging.

mod checker;
mod download;
mod error;
pub mod types;

pub use checker::{check_for_update, start_update_check_service};
pub use download::{prepare_install, start_download};
pub use error::{Error, Result};
pub use types::*;

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{OnceCell, RwLock};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_SERVER_URL: &str = "https://screenerbot.io/api";
const GITHUB_RELEASES_API_URL: &str = "https://api.github.com/repos/farfary/ScreenerBot";
const UPDATE_CHECK_INTERVAL_SECS: u64 = 6 * 60 * 60;
const DOWNLOAD_TIMEOUT_SECS: u64 = 30 * 60;
const MAX_UPDATE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

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

    normalize_loaded_state(&mut state);
    UPDATE_AVAILABLE.store(state.available_update.is_some(), Ordering::SeqCst);
    state
}

fn normalize_loaded_state(state: &mut UpdateState) {
    if state.phase == UpdatePhase::Installing
        && state
            .available_update
            .as_ref()
            .map(|update| update.version.as_str())
            == Some(VERSION)
    {
        state.phase = UpdatePhase::Applied;
        state.available_update = None;
        state.download_progress.downloading = false;
        state.download_progress.completed = false;
        state.download_progress.error = None;
        return;
    }

    if matches!(
        state.phase,
        UpdatePhase::Checking
            | UpdatePhase::Downloading
            | UpdatePhase::Verifying
            | UpdatePhase::Installing
    ) {
        state.phase = UpdatePhase::Failed;
        state.download_progress.downloading = false;
        state.download_progress.completed = false;
        state.download_progress.error =
            Some("Update was interrupted by an application restart".to_owned());
    }

    if state.phase == UpdatePhase::Ready {
        let ready_file_exists = state
            .download_progress
            .downloaded_path
            .as_deref()
            .map(std::path::Path::new)
            .is_some_and(std::path::Path::exists);
        if !ready_file_exists {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        }
    }

    #[test]
    fn interrupted_update_state_fails_closed() {
        let mut state = UpdateState {
            phase: UpdatePhase::Downloading,
            download_progress: DownloadProgress {
                downloading: true,
                ..DownloadProgress::default()
            },
            ..UpdateState::default()
        };
        normalize_loaded_state(&mut state);
        assert_eq!(state.phase, UpdatePhase::Failed);
        assert!(!state.download_progress.downloading);
        assert!(state.download_progress.error.is_some());
    }

    #[test]
    fn completed_install_is_recorded_after_restart() {
        let mut state = UpdateState {
            phase: UpdatePhase::Installing,
            available_update: Some(UpdateInfo {
                version: VERSION.to_owned(),
                filename: "installer".to_owned(),
                download_url: "/installer".to_owned(),
                file_size: 1,
                checksum: "a".repeat(64),
                release_notes: None,
                release_date: String::new(),
            }),
            ..UpdateState::default()
        };
        normalize_loaded_state(&mut state);
        assert_eq!(state.phase, UpdatePhase::Applied);
        assert!(state.available_update.is_none());
    }
}
