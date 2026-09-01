//! Secure, version-bound artifact download for both update components.

use super::manifest;
use super::types::*;
use super::{
    core_install, mutate_state, mutate_state_with_result, state_lock, Error, Result,
    DOWNLOAD_TIMEOUT_SECS, MAX_UPDATE_BYTES,
};
use crate::logger::{self, LogTag};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Start fetching the advertised update in the background.
///
/// The claim on the state is taken synchronously so two callers (the settings
/// dialog and the automatic service, say) cannot both start a download.
pub async fn start_download(update: UpdateInfo) -> Result<()> {
    if update.kind == UpdateKind::Full && !crate::arguments::is_gui_enabled() {
        return Err(Error::UnsupportedInstall {
            detail: "headless updates must be installed with screenerbot-manager update".to_owned(),
        });
    }
    mutate_state_with_result(|state| claim_download(state, &update)).await?;
    tokio::spawn(async move {
        if let Err(error) = run_download(&update).await {
            logger::warning(LogTag::System, &format!("Update download failed: {error}"));
            record_download_failure(&update, &error).await;
        }
    });
    Ok(())
}

fn claim_download(state: &mut UpdateState, update: &UpdateInfo) -> Result<()> {
    let available = state
        .available_update
        .as_ref()
        .ok_or(Error::NoUpdateAvailable)?;
    if available.version != update.version || available.checksum != update.checksum {
        return Err(Error::UpdateChanged);
    }
    if state.download_progress.downloading {
        return Err(Error::DownloadInProgress);
    }

    state.phase = UpdatePhase::Downloading;
    state.deferred = None;
    state.download_progress = DownloadProgress {
        version: Some(update.version.clone()),
        checksum: Some(update.checksum.clone()),
        downloading: true,
        total_bytes: update.transfer_size(),
        ..DownloadProgress::default()
    };
    Ok(())
}

async fn run_download(update: &UpdateInfo) -> Result<()> {
    match (update.kind, update.core.as_ref()) {
        (UpdateKind::Core, Some(core)) => run_core_download(update, core).await,
        (UpdateKind::Core, None) => Err(Error::DigestMismatch {
            detail: "a core update was planned without a core artifact".to_owned(),
        }),
        (UpdateKind::Full, _) => run_installer_download(update).await,
    }
}

// ============================================================================
// Core component — the silent path
// ============================================================================

/// Fetch the compressed core binary, prove it three ways, and stage it so the
/// desktop shell adopts it on the next backend start.
async fn run_core_download(update: &UpdateInfo, core: &CoreArtifact) -> Result<()> {
    let download_dir = get_download_dir()?;
    let archive_path = download_dir.join(&core.filename);
    let partial_path = download_dir.join(format!("{}.part", core.filename));
    let _ = tokio::fs::remove_file(&partial_path).await;

    let client = build_update_client()?;
    let release = manifest::fetch_release_for(&client, &update.version).await?;
    let asset = release
        .asset(&core.filename)
        .ok_or_else(|| Error::DigestMismatch {
            detail: format!("GitHub release does not contain {}", core.filename),
        })?;
    manifest::verify_release_asset(&release, &core.filename, core.size, &core.sha256)?;
    let source_url = resolve_download_url(&asset.browser_download_url)?;

    let already_staged = file_matches(&archive_path, core.size, &core.sha256).await?;
    if !already_staged {
        let result = stream_to_file(
            &client,
            source_url,
            &partial_path,
            core.size,
            |downloaded| record_progress(downloaded, core.size),
        )
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&partial_path).await;
        }
        result?;

        mutate_state(|state| state.phase = UpdatePhase::Verifying).await;
        let actual = calculate_sha256_async(partial_path.clone()).await?;
        if actual != core.sha256 {
            let _ = tokio::fs::remove_file(&partial_path).await;
            return Err(Error::DigestMismatch {
                detail: format!(
                    "core archive checksum expected {}, got {actual}",
                    core.sha256
                ),
            });
        }
        if archive_path.exists() {
            tokio::fs::remove_file(&archive_path)
                .await
                .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
        }
        tokio::fs::rename(&partial_path, &archive_path)
            .await
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    }

    mutate_state(|state| state.phase = UpdatePhase::Verifying).await;
    let staged = core_install::stage_core(&update.version, core, &archive_path).await?;
    // The archive has served its purpose; the verified binary is what matters.
    let _ = tokio::fs::remove_file(&archive_path).await;

    logger::info(
        LogTag::System,
        &format!(
            "Core update v{} staged and verified ({} MB); it activates on the next backend start",
            staged.version,
            staged.size / (1024 * 1024)
        ),
    );

    mutate_state(|state| {
        state.phase = UpdatePhase::ReadyToApply;
        state.download_progress.downloading = false;
        state.download_progress.completed = true;
        state.download_progress.error = None;
        state.download_progress.bytes_downloaded = core.size;
        state.download_progress.total_bytes = core.size;
        state.download_progress.progress_percent = 100.0;
        state.download_progress.downloaded_path = Some(staged.path.clone());
    })
    .await;
    Ok(())
}

// ============================================================================
// Full component — the operating-system installer path
// ============================================================================

async fn run_installer_download(update: &UpdateInfo) -> Result<()> {
    let download_dir = get_download_dir()?;
    let final_path = download_dir.join(&update.filename);
    let partial_path = download_dir.join(format!("{}.part", update.filename));
    let _ = tokio::fs::remove_file(&partial_path).await;

    let result = run_installer_download_inner(update, &partial_path, &final_path).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&partial_path).await;
    }
    result
}

async fn run_installer_download_inner(
    update: &UpdateInfo,
    partial_path: &Path,
    final_path: &Path,
) -> Result<()> {
    let client = build_update_client()?;
    let release = manifest::fetch_release_for(&client, &update.version).await?;
    manifest::verify_release_asset(
        &release,
        &update.filename,
        update.file_size,
        &update.checksum,
    )?;

    if file_matches(final_path, update.file_size, &update.checksum).await? {
        record_installer_ready(update, final_path).await;
        return Ok(());
    }

    let download_url = resolve_download_url(&update.download_url)?;
    stream_to_file(
        &client,
        download_url,
        partial_path,
        update.file_size,
        |downloaded| record_progress(downloaded, update.file_size),
    )
    .await?;

    mutate_state(|state| state.phase = UpdatePhase::Verifying).await;
    let actual_checksum = calculate_sha256_async(partial_path.to_owned()).await?;
    if actual_checksum != update.checksum {
        return Err(Error::DigestMismatch {
            detail: format!(
                "staged file checksum expected {}, got {actual_checksum}",
                update.checksum
            ),
        });
    }

    if final_path.exists() {
        tokio::fs::remove_file(final_path)
            .await
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    }
    tokio::fs::rename(partial_path, final_path)
        .await
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    record_installer_ready(update, final_path).await;
    Ok(())
}

// ============================================================================
// Transport
// ============================================================================

/// Stream one allowlisted URL to `destination`, enforcing the authenticated size
/// on every chunk so a hostile server cannot fill the disk.
async fn stream_to_file<F, Fut>(
    client: &reqwest::Client,
    url: reqwest::Url,
    destination: &Path,
    expected_size: u64,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    validate_content_length(Some(expected_size), expected_size)?;
    let endpoint = url.to_string();
    let response = client
        .get(url)
        .header("User-Agent", format!("ScreenerBot/{}", super::VERSION))
        .send()
        .await
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: endpoint.clone(),
                detail: error.to_string(),
            })
        })?;
    if !response.status().is_success() {
        return Err(Error::Network(crate::errors::NetworkError::HttpStatus {
            endpoint: response.url().to_string(),
            status: response.status().as_u16(),
            body: None,
        }));
    }
    validate_final_url(response.url())?;
    validate_content_length(response.content_length(), expected_size)?;

    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0_u64;
    let mut last_progress = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: endpoint.clone(),
                detail: error.to_string(),
            })
        })?;
        downloaded = downloaded.checked_add(chunk.len() as u64).ok_or_else(|| {
            Error::Data(crate::errors::DataError::ValidationError {
                field: "downloaded bytes".to_owned(),
                value: downloaded.to_string(),
                reason: "overflowed the supported byte count".to_owned(),
            })
        })?;
        if downloaded > expected_size || downloaded > MAX_UPDATE_BYTES {
            return Err(Error::DownloadSizeMismatch {
                expected: expected_size.min(MAX_UPDATE_BYTES),
                actual: downloaded,
            });
        }
        file.write_all(&chunk)
            .await
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;

        if last_progress.elapsed() >= Duration::from_millis(500) {
            on_progress(downloaded).await;
            last_progress = std::time::Instant::now();
        }
    }

    file.flush()
        .await
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    file.sync_all()
        .await
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    drop(file);

    if downloaded != expected_size {
        return Err(Error::DownloadSizeMismatch {
            expected: expected_size,
            actual: downloaded,
        });
    }
    on_progress(downloaded).await;
    Ok(())
}

pub(super) fn build_update_client() -> Result<reqwest::Client> {
    crate::net::client_builder()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("too many update redirects");
            }
            if is_allowed_update_url(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("update redirect left the HTTPS allowlist")
            }
        }))
        .build()
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: "update HTTP client".to_owned(),
                detail: error.to_string(),
            })
        })
}

pub(super) fn resolve_download_url(download_url: &str) -> Result<reqwest::Url> {
    if download_url.starts_with('/') {
        return reqwest::Url::parse("https://screenerbot.io")
            .and_then(|base| base.join(download_url))
            .map_err(|error| Error::InvalidUpdateUrl {
                url: download_url.to_owned(),
                reason: error.to_string(),
            });
    }
    let url = reqwest::Url::parse(download_url).map_err(|error| Error::InvalidUpdateUrl {
        url: download_url.to_owned(),
        reason: error.to_string(),
    })?;
    if is_allowed_update_url(&url) {
        Ok(url)
    } else {
        Err(Error::InvalidUpdateUrl {
            url: download_url.to_owned(),
            reason: "outside the HTTPS allowlist".to_owned(),
        })
    }
}

pub(super) fn is_allowed_update_url(url: &reqwest::Url) -> bool {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    matches!(
        url.host_str(),
        Some("screenerbot.io" | "api.github.com" | "github.com")
    ) || url
        .host_str()
        .is_some_and(|host| host.ends_with(".githubusercontent.com"))
}

pub(super) fn validate_final_url(url: &reqwest::Url) -> Result<()> {
    if is_allowed_update_url(url) {
        Ok(())
    } else {
        Err(Error::InvalidUpdateUrl {
            url: url.to_string(),
            reason: "outside the HTTPS allowlist after redirects".to_owned(),
        })
    }
}

fn validate_content_length(actual: Option<u64>, expected: u64) -> Result<()> {
    if expected == 0 || expected > MAX_UPDATE_BYTES {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update.file_size".to_owned(),
            value: expected.to_string(),
            reason: format!("must be between 1 and {MAX_UPDATE_BYTES} bytes"),
        }));
    }
    if let Some(actual) = actual {
        if actual != expected {
            return Err(Error::DownloadSizeMismatch { expected, actual });
        }
    }
    Ok(())
}

// ============================================================================
// State recording
// ============================================================================

async fn record_progress(downloaded: u64, total: u64) {
    let mut state = state_lock().await.write().await;
    state.download_progress.bytes_downloaded = downloaded;
    state.download_progress.progress_percent = if total == 0 {
        0.0
    } else {
        (downloaded as f32 / total as f32) * 100.0
    };
}

async fn record_installer_ready(update: &UpdateInfo, path: &Path) {
    mutate_state(|state| {
        state.phase = UpdatePhase::ReadyToInstall;
        state.download_progress.downloading = false;
        state.download_progress.completed = true;
        state.download_progress.error = None;
        state.download_progress.bytes_downloaded = update.file_size;
        state.download_progress.total_bytes = update.file_size;
        state.download_progress.progress_percent = 100.0;
        state.download_progress.version = Some(update.version.clone());
        state.download_progress.checksum = Some(update.checksum.clone());
        state.download_progress.downloaded_path = Some(path.to_string_lossy().into_owned());
    })
    .await;
}

async fn record_download_failure(update: &UpdateInfo, error: &Error) {
    mutate_state(|state| {
        if state.download_progress.version.as_deref() == Some(update.version.as_str()) {
            state.phase = UpdatePhase::Failed;
            state.download_progress.downloading = false;
            state.download_progress.completed = false;
            state.download_progress.error = Some(error.to_string());
            state.download_progress.downloaded_path = None;
        }
    })
    .await;
}

pub(super) fn get_download_dir() -> Result<PathBuf> {
    let dir = crate::paths::get_data_directory().join("updates");
    std::fs::create_dir_all(&dir)
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    }
    Ok(dir)
}

pub(super) async fn file_matches(
    path: &Path,
    expected_size: u64,
    expected_checksum: &str,
) -> Result<bool> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(Error::Io(crate::errors::IoError::from(error))),
    };
    if !metadata.is_file() || metadata.len() != expected_size {
        return Ok(false);
    }
    Ok(calculate_sha256_async(path.to_owned()).await? == expected_checksum)
}

pub(super) async fn calculate_sha256_async(path: PathBuf) -> Result<String> {
    tokio::task::spawn_blocking(move || calculate_sha256(&path))
        .await
        .map_err(|error| Error::Internal(crate::errors::InternalError::from(error)))?
}

pub(super) fn calculate_sha256(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update() -> UpdateInfo {
        UpdateInfo {
            version: "0.1.122".to_owned(),
            filename: "ScreenerBot-v0.1.122-Linux-x64-headless.tar.gz".to_owned(),
            download_url: "/api/releases/download?version=0.1.122&platform=linux-x64-headless"
                .to_owned(),
            file_size: 4,
            checksum: "a".repeat(64),
            release_notes: None,
            release_date: String::new(),
            kind: UpdateKind::Full,
            core: None,
            shell_revision: None,
        }
    }

    #[test]
    fn download_claim_is_atomic_and_version_bound() {
        let candidate = update();
        let mut state = UpdateState {
            available_update: Some(candidate.clone()),
            phase: UpdatePhase::Available,
            ..UpdateState::default()
        };
        claim_download(&mut state, &candidate).unwrap();
        assert_eq!(state.phase, UpdatePhase::Downloading);
        assert!(claim_download(&mut state, &candidate).is_err());

        state.download_progress.downloading = false;
        let mut other = candidate.clone();
        other.version = "0.1.123".to_owned();
        assert!(claim_download(&mut state, &other).is_err());
    }

    #[test]
    fn claim_tracks_the_bytes_the_planned_component_transfers() {
        let mut candidate = update();
        candidate.kind = UpdateKind::Core;
        candidate.file_size = 200_000_000;
        candidate.core = Some(CoreArtifact {
            filename: "ScreenerBot-v0.1.122-Linux-x64-core.gz".to_owned(),
            size: 20_000_000,
            sha256: "b".repeat(64),
            binary_size: 80_000_000,
            binary_sha256: "c".repeat(64),
        });
        let mut state = UpdateState {
            available_update: Some(candidate.clone()),
            phase: UpdatePhase::Available,
            ..UpdateState::default()
        };
        claim_download(&mut state, &candidate).unwrap();
        assert_eq!(state.download_progress.total_bytes, 20_000_000);
    }

    #[test]
    fn download_urls_and_sizes_fail_closed() {
        assert!(resolve_download_url("http://screenerbot.io/update").is_err());
        assert!(resolve_download_url("https://evil.example/update").is_err());
        assert!(resolve_download_url("/api/releases/download?x=1").is_ok());
        assert!(
            resolve_download_url("https://objects.githubusercontent.com/releases/asset").is_ok()
        );
        assert!(validate_content_length(Some(5), 4).is_err());
        assert!(validate_content_length(Some(4), 4).is_ok());
        assert!(validate_content_length(None, 4).is_ok());
    }

    #[test]
    fn checksum_reads_exact_file_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("artifact");
        std::fs::write(&path, b"test").unwrap();
        assert_eq!(
            calculate_sha256(&path).unwrap(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
        assert_eq!(sha256_bytes(b"test"), calculate_sha256(&path).unwrap());
    }
}
