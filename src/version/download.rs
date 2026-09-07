//! Secure, version-bound artifact download for both update components.

use super::manifest;
use super::types::*;
use super::{
    core_install, mutate_state, mutate_state_with_result, state_lock, Error, Result,
    DOWNLOAD_TIMEOUT_SECS, MAX_UPDATE_BYTES,
};
use crate::errors::ErrorClass;
use crate::logger::{self, LogTag};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const MAX_TRANSFER_ATTEMPTS: u32 = 8;
const MAX_METADATA_ATTEMPTS: u32 = 4;
const TRANSFER_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
const TRANSFER_RETRY_MAX_DELAY: Duration = Duration::from_secs(30);
const TRANSFER_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Copy)]
struct TransferPolicy {
    max_attempts: u32,
    base_delay: Duration,
    validate_urls: bool,
}

impl Default for TransferPolicy {
    fn default() -> Self {
        Self {
            max_attempts: MAX_TRANSFER_ATTEMPTS,
            base_delay: TRANSFER_RETRY_BASE_DELAY,
            validate_urls: true,
        }
    }
}

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
            let retained_bytes = retained_partial_bytes(&update).await;
            record_download_failure(&update, &error, retained_bytes).await;
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
    if state.phase.is_busy() {
        return Err(if state.phase == UpdatePhase::Checking {
            Error::CheckInProgress
        } else {
            Error::DownloadInProgress
        });
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
        stream_to_file(
            &client,
            source_url,
            &partial_path,
            core.size,
            |downloaded| record_progress(downloaded, core.size),
        )
        .await?;

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
    run_installer_download_inner(update, &partial_path, &final_path).await
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
        let _ = tokio::fs::remove_file(partial_path).await;
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
    stream_to_file_with_policy(
        client,
        url,
        destination,
        expected_size,
        &mut on_progress,
        TransferPolicy::default(),
    )
    .await
}

async fn stream_to_file_with_policy<F, Fut>(
    client: &reqwest::Client,
    url: reqwest::Url,
    destination: &Path,
    expected_size: u64,
    on_progress: &mut F,
    policy: TransferPolicy,
) -> Result<()>
where
    F: FnMut(u64) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    validate_content_length(Some(expected_size), expected_size)?;
    if policy.max_attempts == 0 {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update transfer attempts".to_owned(),
            value: "0".to_owned(),
            reason: "must allow at least one attempt".to_owned(),
        }));
    }

    for attempt in 1..=policy.max_attempts {
        let offset = prepare_partial(destination, expected_size).await?;
        on_progress(offset).await;
        if offset == expected_size {
            return Ok(());
        }

        match stream_attempt(
            client,
            url.clone(),
            destination,
            expected_size,
            offset,
            on_progress,
            policy.validate_urls,
        )
        .await
        {
            Ok(()) => return Ok(()),
            Err(error) if error.is_retryable() && attempt < policy.max_attempts => {
                let retained = prepare_partial(destination, expected_size).await?;
                let delay = transfer_retry_delay(policy.base_delay, attempt);
                logger::warning(
                    LogTag::System,
                    &format!(
                        "Update transfer interrupted at {retained}/{expected_size} bytes; retry {}/{} in {:.1}s: {error}",
                        attempt + 1,
                        policy.max_attempts,
                        delay.as_secs_f32()
                    ),
                );
                tokio::time::sleep(delay).await;
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("the transfer attempt loop always returns")
}

async fn stream_attempt<F, Fut>(
    client: &reqwest::Client,
    url: reqwest::Url,
    destination: &Path,
    expected_size: u64,
    requested_offset: u64,
    on_progress: &mut F,
    check_url_allowlist: bool,
) -> Result<()>
where
    F: FnMut(u64) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let endpoint = url.to_string();
    let mut request = client
        .get(url)
        .header("User-Agent", format!("ScreenerBot/{}", super::VERSION));
    if requested_offset > 0 {
        request = request.header(reqwest::header::RANGE, format!("bytes={requested_offset}-"));
    }
    let response = request
        .send()
        .await
        .map_err(|error| request_failed(&endpoint, error))?;

    if response.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE && requested_offset > 0 {
        reset_partial(destination).await?;
        return Err(Error::Network(crate::errors::NetworkError::RequestFailed {
            endpoint,
            detail: "server rejected the saved byte range; restarting from byte 0".to_owned(),
        }));
    }
    if !response.status().is_success() {
        return Err(Error::Network(crate::errors::NetworkError::HttpStatus {
            endpoint: response.url().to_string(),
            status: response.status().as_u16(),
            body: None,
        }));
    }
    if check_url_allowlist {
        validate_final_url(response.url())?;
    }

    let (write_offset, response_size) =
        if requested_offset > 0 && response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            let response_size = validate_content_range(&response, requested_offset, expected_size)?;
            (requested_offset, response_size)
        } else if requested_offset > 0 {
            // Range is optional in HTTP. A server or intermediary may ignore it and
            // return the whole object; safely restart instead of appending duplicates.
            validate_content_length(response.content_length(), expected_size)?;
            (0, expected_size)
        } else if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            let response_size = validate_content_range(&response, 0, expected_size)?;
            (0, response_size)
        } else {
            validate_content_length(response.content_length(), expected_size)?;
            (0, expected_size)
        };

    validate_content_length(response.content_length(), response_size)?;
    let mut options = tokio::fs::OpenOptions::new();
    options.create(true).write(true);
    if write_offset == 0 {
        options.truncate(true);
    } else {
        options.append(true);
    }
    let mut file = options
        .open(destination)
        .await
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    let mut stream = response.bytes_stream();
    let mut downloaded = write_offset;
    let mut received = 0_u64;
    let mut last_progress = std::time::Instant::now();

    loop {
        let next = tokio::time::timeout(TRANSFER_IDLE_TIMEOUT, stream.next())
            .await
            .map_err(|_| {
                Error::Network(crate::errors::NetworkError::Timeout {
                    endpoint: endpoint.clone(),
                    timeout_ms: TRANSFER_IDLE_TIMEOUT.as_millis() as u64,
                })
            })?;
        let Some(chunk) = next else { break };
        let chunk = chunk.map_err(|error| request_failed(&endpoint, error))?;
        received = checked_download_size(received, chunk.len() as u64, response_size)?;
        downloaded = checked_download_size(downloaded, chunk.len() as u64, expected_size)?;
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

    if received != response_size || downloaded != expected_size {
        return Err(Error::Network(crate::errors::NetworkError::RequestFailed {
            endpoint,
            detail: format!(
                "response ended early after {received}/{response_size} bytes ({downloaded}/{expected_size} total)"
            ),
        }));
    }
    on_progress(downloaded).await;
    Ok(())
}

fn request_failed(endpoint: &str, error: reqwest::Error) -> Error {
    Error::Network(crate::errors::NetworkError::RequestFailed {
        endpoint: endpoint.to_owned(),
        detail: error.to_string(),
    })
}

fn checked_download_size(current: u64, added: u64, limit: u64) -> Result<u64> {
    let actual = current.checked_add(added).ok_or_else(|| {
        Error::Data(crate::errors::DataError::ValidationError {
            field: "downloaded bytes".to_owned(),
            value: current.to_string(),
            reason: "overflowed the supported byte count".to_owned(),
        })
    })?;
    if actual > limit || actual > MAX_UPDATE_BYTES {
        return Err(Error::DownloadSizeMismatch {
            expected: limit.min(MAX_UPDATE_BYTES),
            actual,
        });
    }
    Ok(actual)
}

async fn prepare_partial(path: &Path, expected_size: u64) -> Result<u64> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) if metadata.is_file() && metadata.len() <= expected_size => Ok(metadata.len()),
        Ok(_) => {
            reset_partial(path).await?;
            Ok(0)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(Error::Io(crate::errors::IoError::from(error))),
    }
}

async fn reset_partial(path: &Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::Io(crate::errors::IoError::from(error))),
    }
}

fn validate_content_range(
    response: &reqwest::Response,
    expected_start: u64,
    expected_total: u64,
) -> Result<u64> {
    let value = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| invalid_content_range("missing Content-Range header"))?;
    let remainder = value
        .strip_prefix("bytes ")
        .ok_or_else(|| invalid_content_range(value))?;
    let (bounds, total) = remainder
        .split_once('/')
        .ok_or_else(|| invalid_content_range(value))?;
    let (start, end) = bounds
        .split_once('-')
        .ok_or_else(|| invalid_content_range(value))?;
    let start = start
        .parse::<u64>()
        .map_err(|_| invalid_content_range(value))?;
    let end = end
        .parse::<u64>()
        .map_err(|_| invalid_content_range(value))?;
    let total = total
        .parse::<u64>()
        .map_err(|_| invalid_content_range(value))?;
    if start != expected_start || end < start || end >= expected_total || total != expected_total {
        return Err(invalid_content_range(value));
    }
    Ok(end - start + 1)
}

fn invalid_content_range(received: &str) -> Error {
    Error::Data(crate::errors::DataError::InvalidFormat {
        expected: "Content-Range matching the requested offset and authenticated asset size"
            .to_owned(),
        received: received.to_owned(),
    })
}

fn transfer_retry_delay(base: Duration, failed_attempt: u32) -> Duration {
    let factor = 1_u32
        .checked_shl(failed_attempt.saturating_sub(1))
        .unwrap_or(u32::MAX);
    base.saturating_mul(factor).min(TRANSFER_RETRY_MAX_DELAY)
}

/// Retry a small update-control request as a whole. This covers connection
/// failures while checking the website, reading GitHub release metadata, or
/// fetching the authenticated manifest; parsing and integrity failures remain
/// terminal because repeating untrusted bytes cannot make them valid.
pub(super) async fn retry_update_operation<T, F, Fut>(label: &str, mut operation: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    for attempt in 1..=MAX_METADATA_ATTEMPTS {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) if error.is_retryable() && attempt < MAX_METADATA_ATTEMPTS => {
                let delay = transfer_retry_delay(Duration::from_millis(500), attempt);
                logger::warning(
                    LogTag::System,
                    &format!(
                        "{label} failed; retry {}/{} in {:.1}s: {error}",
                        attempt + 1,
                        MAX_METADATA_ATTEMPTS,
                        delay.as_secs_f32()
                    ),
                );
                tokio::time::sleep(delay).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the metadata attempt loop always returns")
}

/// Read a small control-plane response without ever buffering more than its
/// protocol limit. Content-Length is only an early rejection; the streaming cap
/// is authoritative when the header is absent or dishonest.
pub(super) async fn read_limited_body(
    response: reqwest::Response,
    limit: u64,
    endpoint: &str,
) -> Result<Vec<u8>> {
    if response.content_length().is_some_and(|size| size > limit) {
        return Err(Error::DownloadSizeMismatch {
            expected: limit,
            actual: response.content_length().unwrap_or_default(),
        });
    }
    let mut body =
        Vec::with_capacity(response.content_length().unwrap_or_default().min(limit) as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| request_failed(endpoint, error))?;
        let next_len =
            body.len()
                .checked_add(chunk.len())
                .ok_or_else(|| Error::DownloadSizeMismatch {
                    expected: limit,
                    actual: u64::MAX,
                })?;
        if next_len as u64 > limit {
            return Err(Error::DownloadSizeMismatch {
                expected: limit,
                actual: next_len as u64,
            });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub(super) fn build_update_client() -> Result<reqwest::Client> {
    crate::net::client_builder()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(20))
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

async fn record_download_failure(update: &UpdateInfo, error: &Error, retained_bytes: Option<u64>) {
    mutate_state(|state| {
        if state.download_progress.version.as_deref() == Some(update.version.as_str()) {
            state.phase = UpdatePhase::Failed;
            state.download_progress.downloading = false;
            state.download_progress.completed = false;
            state.download_progress.error = Some(error.to_string());
            state.download_progress.downloaded_path = None;
            if let Some(retained) = retained_bytes {
                state.download_progress.bytes_downloaded = retained;
                state.download_progress.progress_percent =
                    if state.download_progress.total_bytes == 0 {
                        0.0
                    } else {
                        retained as f32 / state.download_progress.total_bytes as f32 * 100.0
                    };
            }
        }
    })
    .await;
}

async fn retained_partial_bytes(update: &UpdateInfo) -> Option<u64> {
    let filename = match (update.kind, update.core.as_ref()) {
        (UpdateKind::Core, Some(core)) => &core.filename,
        _ => &update.filename,
    };
    let path = crate::paths::get_data_directory()
        .join("updates")
        .join(format!("{filename}.part"));
    tokio::fs::metadata(path)
        .await
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len().min(update.transfer_size()))
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
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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

    async fn spawn_transfer_server(
        body: Vec<u8>,
        first_response_bytes: Option<usize>,
        ignore_range: bool,
        invalid_range: bool,
    ) -> (reqwest::Url, Arc<Mutex<Vec<Option<String>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed = Arc::clone(&requests);
        tokio::spawn(async move {
            let expected_requests = if first_response_bytes.is_some() { 2 } else { 1 };
            for request_number in 0..expected_requests {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1024];
                loop {
                    let read = socket.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let text = String::from_utf8(request).unwrap();
                let range = text.lines().find_map(|line| {
                    line.strip_prefix("range: ")
                        .or_else(|| line.strip_prefix("Range: "))
                        .map(str::to_owned)
                });
                observed.lock().unwrap().push(range.clone());

                if request_number == 0 {
                    if let Some(prefix) = first_response_bytes {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        socket.write_all(header.as_bytes()).await.unwrap();
                        socket.write_all(&body[..prefix]).await.unwrap();
                        socket.shutdown().await.unwrap();
                        continue;
                    }
                }

                let requested_start = range
                    .as_deref()
                    .and_then(|value| value.strip_prefix("bytes="))
                    .and_then(|value| value.strip_suffix('-'))
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                if ignore_range || requested_start == 0 {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    socket.write_all(header.as_bytes()).await.unwrap();
                    socket.write_all(&body).await.unwrap();
                } else {
                    let remaining = &body[requested_start..];
                    let content_range = if invalid_range {
                        format!("bytes 0-{}/{}", body.len() - 1, body.len())
                    } else {
                        format!("bytes {requested_start}-{}/{}", body.len() - 1, body.len())
                    };
                    let header = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: {content_range}\r\nConnection: close\r\n\r\n",
                        remaining.len()
                    );
                    socket.write_all(header.as_bytes()).await.unwrap();
                    socket.write_all(remaining).await.unwrap();
                }
                socket.shutdown().await.unwrap();
            }
        });
        (
            reqwest::Url::parse(&format!("http://{address}/artifact")).unwrap(),
            requests,
        )
    }

    fn test_policy(max_attempts: u32) -> TransferPolicy {
        TransferPolicy {
            max_attempts,
            base_delay: Duration::ZERO,
            validate_urls: false,
        }
    }

    #[tokio::test]
    async fn interrupted_transfer_retries_from_the_retained_offset() {
        let body = b"a complete update artifact".to_vec();
        let (url, requests) = spawn_transfer_server(body.clone(), Some(7), false, false).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("artifact.part");
        let client = reqwest::Client::new();

        stream_to_file_with_policy(
            &client,
            url,
            &destination,
            body.len() as u64,
            &mut |_| async {},
            test_policy(2),
        )
        .await
        .unwrap();

        assert_eq!(tokio::fs::read(destination).await.unwrap(), body);
        assert_eq!(
            *requests.lock().unwrap(),
            vec![None, Some("bytes=7-".to_owned())]
        );
    }

    #[tokio::test]
    async fn saved_partial_resumes_on_a_new_download_invocation() {
        let body = b"resume this artifact".to_vec();
        let (url, requests) = spawn_transfer_server(body.clone(), None, false, false).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("artifact.part");
        tokio::fs::write(&destination, &body[..6]).await.unwrap();

        stream_to_file_with_policy(
            &reqwest::Client::new(),
            url,
            &destination,
            body.len() as u64,
            &mut |_| async {},
            test_policy(1),
        )
        .await
        .unwrap();

        assert_eq!(tokio::fs::read(destination).await.unwrap(), body);
        assert_eq!(*requests.lock().unwrap(), vec![Some("bytes=6-".to_owned())]);
    }

    #[tokio::test]
    async fn range_ignorance_restarts_without_duplicating_bytes() {
        let body = b"server sends the complete object".to_vec();
        let (url, requests) = spawn_transfer_server(body.clone(), None, true, false).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("artifact.part");
        tokio::fs::write(&destination, &body[..9]).await.unwrap();

        stream_to_file_with_policy(
            &reqwest::Client::new(),
            url,
            &destination,
            body.len() as u64,
            &mut |_| async {},
            test_policy(1),
        )
        .await
        .unwrap();

        assert_eq!(tokio::fs::read(destination).await.unwrap(), body);
        assert_eq!(*requests.lock().unwrap(), vec![Some("bytes=9-".to_owned())]);
    }

    #[tokio::test]
    async fn mismatched_content_range_fails_closed() {
        let body = b"authenticated artifact".to_vec();
        let (url, _) = spawn_transfer_server(body.clone(), None, false, true).await;
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("artifact.part");
        tokio::fs::write(&destination, &body[..5]).await.unwrap();

        let error = stream_to_file_with_policy(
            &reqwest::Client::new(),
            url,
            &destination,
            body.len() as u64,
            &mut |_| async {},
            test_policy(1),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, Error::Data(_)));
        assert_eq!(tokio::fs::read(destination).await.unwrap(), &body[..5]);
    }

    #[tokio::test]
    async fn control_response_body_is_streamed_under_its_limit() {
        let body = b"metadata larger than its protocol limit".to_vec();
        let (url, _) = spawn_transfer_server(body.clone(), None, false, false).await;
        let response = reqwest::Client::new()
            .get(url.clone())
            .send()
            .await
            .unwrap();

        let error = read_limited_body(response, 8, url.as_str())
            .await
            .unwrap_err();

        assert!(matches!(error, Error::DownloadSizeMismatch { .. }));
    }

    #[test]
    fn retry_backoff_is_exponential_and_capped() {
        assert_eq!(
            transfer_retry_delay(Duration::from_secs(1), 1),
            Duration::from_secs(1)
        );
        assert_eq!(
            transfer_retry_delay(Duration::from_secs(1), 4),
            Duration::from_secs(8)
        );
        assert_eq!(
            transfer_retry_delay(Duration::from_secs(1), 20),
            TRANSFER_RETRY_MAX_DELAY
        );
    }
}
