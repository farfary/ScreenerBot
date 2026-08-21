//! Secure, version-bound update download and installer handoff.

use super::types::*;
use super::{
    mutate_state, mutate_state_with_result, state_lock, DOWNLOAD_TIMEOUT_SECS,
    GITHUB_RELEASES_API_URL, MAX_UPDATE_BYTES,
};
use crate::logger::{self, LogTag};
use futures_util::StreamExt;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    digest: Option<String>,
    size: u64,
}

pub async fn start_download(update: UpdateInfo) -> Result<(), String> {
    if !crate::arguments::is_gui_enabled() {
        return Err(
            "Headless updates must be installed with screenerbot-manager update".to_owned(),
        );
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

fn claim_download(state: &mut UpdateState, update: &UpdateInfo) -> Result<(), String> {
    let available = state
        .available_update
        .as_ref()
        .ok_or_else(|| "No update is available to download".to_owned())?;
    if available.version != update.version || available.checksum != update.checksum {
        return Err("The requested update no longer matches the available release".to_owned());
    }
    if state.download_progress.downloading {
        return Err("An update download is already in progress".to_owned());
    }

    state.phase = UpdatePhase::Downloading;
    state.download_progress = DownloadProgress {
        version: Some(update.version.clone()),
        checksum: Some(update.checksum.clone()),
        downloading: true,
        total_bytes: update.file_size,
        ..DownloadProgress::default()
    };
    Ok(())
}

async fn run_download(update: &UpdateInfo) -> Result<(), String> {
    let download_dir = get_download_dir()?;
    let final_path = download_dir.join(&update.filename);
    let partial_path = download_dir.join(format!("{}.part", update.filename));
    let _ = tokio::fs::remove_file(&partial_path).await;

    let result = run_download_inner(update, &partial_path, &final_path).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&partial_path).await;
    }
    result
}

async fn run_download_inner(
    update: &UpdateInfo,
    partial_path: &Path,
    final_path: &Path,
) -> Result<(), String> {
    let client = build_update_client()?;
    verify_github_digest(&client, GITHUB_RELEASES_API_URL, update).await?;

    if file_matches(final_path, update.file_size, &update.checksum).await? {
        record_download_ready(update, final_path).await;
        return Ok(());
    }

    let download_url = resolve_download_url(&update.download_url)?;
    let response = client
        .get(download_url)
        .header("User-Agent", format!("ScreenerBot/{}", super::VERSION))
        .send()
        .await
        .map_err(|error| format!("Download request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Download failed: HTTP {}", response.status()));
    }
    validate_final_url(response.url())?;
    validate_content_length(response.content_length(), update.file_size)?;

    let mut file = tokio::fs::File::create(partial_path)
        .await
        .map_err(|error| format!("Failed to create staged update: {error}"))?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0_u64;
    let mut last_progress = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("Download stream failed: {error}"))?;
        downloaded = downloaded
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| "Downloaded byte count overflowed".to_owned())?;
        if downloaded > update.file_size || downloaded > MAX_UPDATE_BYTES {
            return Err("Downloaded update exceeded its declared size".to_owned());
        }
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("Failed to write staged update: {error}"))?;

        if last_progress.elapsed() >= Duration::from_millis(500) {
            record_progress(downloaded, update.file_size).await;
            last_progress = std::time::Instant::now();
        }
    }

    file.flush()
        .await
        .map_err(|error| format!("Failed to flush staged update: {error}"))?;
    file.sync_all()
        .await
        .map_err(|error| format!("Failed to sync staged update: {error}"))?;
    drop(file);

    if downloaded != update.file_size {
        return Err(format!(
            "Downloaded size mismatch: expected {}, got {}",
            update.file_size, downloaded
        ));
    }

    mutate_state(|state| state.phase = UpdatePhase::Verifying).await;
    let actual_checksum = calculate_sha256_async(partial_path.to_owned()).await?;
    if actual_checksum != update.checksum {
        return Err(format!(
            "Checksum mismatch: expected {}, got {}",
            update.checksum, actual_checksum
        ));
    }

    if final_path.exists() {
        tokio::fs::remove_file(final_path)
            .await
            .map_err(|error| format!("Failed to replace previous staged update: {error}"))?;
    }
    tokio::fs::rename(partial_path, final_path)
        .await
        .map_err(|error| format!("Failed to commit staged update: {error}"))?;
    record_download_ready(update, final_path).await;
    Ok(())
}

fn build_update_client() -> Result<reqwest::Client, String> {
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
        .map_err(|error| format!("Failed to create update HTTP client: {error}"))
}

fn resolve_download_url(download_url: &str) -> Result<reqwest::Url, String> {
    if download_url.starts_with('/') {
        return reqwest::Url::parse("https://screenerbot.io")
            .and_then(|base| base.join(download_url))
            .map_err(|error| format!("Invalid relative update URL: {error}"));
    }
    let url = reqwest::Url::parse(download_url)
        .map_err(|error| format!("Invalid update URL: {error}"))?;
    if is_allowed_update_url(&url) {
        Ok(url)
    } else {
        Err("Update URL is outside the HTTPS allowlist".to_owned())
    }
}

fn is_allowed_update_url(url: &reqwest::Url) -> bool {
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

fn validate_final_url(url: &reqwest::Url) -> Result<(), String> {
    if is_allowed_update_url(url) {
        Ok(())
    } else {
        Err("Final update URL is outside the HTTPS allowlist".to_owned())
    }
}

fn validate_content_length(actual: Option<u64>, expected: u64) -> Result<(), String> {
    if expected == 0 || expected > MAX_UPDATE_BYTES {
        return Err("Declared update size is outside the allowed range".to_owned());
    }
    if let Some(actual) = actual {
        if actual != expected {
            return Err(format!(
                "Download Content-Length mismatch: expected {expected}, got {actual}"
            ));
        }
    }
    Ok(())
}

async fn verify_github_digest(
    client: &reqwest::Client,
    github_api: &str,
    update: &UpdateInfo,
) -> Result<(), String> {
    let url = format!("{github_api}/releases/tags/v{}", update.version);
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", format!("ScreenerBot/{}", super::VERSION))
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|error| format!("Failed to verify GitHub release digest: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "GitHub release verification failed: HTTP {}",
            response.status()
        ));
    }
    let release: GithubRelease = response
        .json()
        .await
        .map_err(|error| format!("Failed to parse GitHub release metadata: {error}"))?;
    verify_github_release(&release, update)
}

fn verify_github_release(release: &GithubRelease, update: &UpdateInfo) -> Result<(), String> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == update.filename)
        .ok_or_else(|| "GitHub release does not contain the expected asset".to_owned())?;
    if asset.size != update.file_size {
        return Err("Website and GitHub release sizes do not match".to_owned());
    }
    let expected = format!("sha256:{}", update.checksum);
    if asset.digest.as_deref() != Some(expected.as_str()) {
        return Err("Website and GitHub release checksums do not match".to_owned());
    }
    Ok(())
}

async fn record_progress(downloaded: u64, total: u64) {
    let mut state = state_lock().await.write().await;
    state.download_progress.bytes_downloaded = downloaded;
    state.download_progress.progress_percent = (downloaded as f32 / total as f32) * 100.0;
}

async fn record_download_ready(update: &UpdateInfo, path: &Path) {
    mutate_state(|state| {
        state.phase = UpdatePhase::Ready;
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

async fn record_download_failure(update: &UpdateInfo, error: &str) {
    mutate_state(|state| {
        if state.download_progress.version.as_deref() == Some(update.version.as_str()) {
            state.phase = UpdatePhase::Failed;
            state.download_progress.downloading = false;
            state.download_progress.completed = false;
            state.download_progress.error = Some(error.to_owned());
            state.download_progress.downloaded_path = None;
        }
    })
    .await;
}

fn get_download_dir() -> Result<PathBuf, String> {
    let dir = crate::paths::get_data_directory().join("updates");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create update directory: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("Failed to secure update directory: {error}"))?;
    }
    Ok(dir)
}

async fn file_matches(
    path: &Path,
    expected_size: u64,
    expected_checksum: &str,
) -> Result<bool, String> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Failed to inspect staged update: {error}")),
    };
    if !metadata.is_file() || metadata.len() != expected_size {
        return Ok(false);
    }
    Ok(calculate_sha256_async(path.to_owned()).await? == expected_checksum)
}

async fn calculate_sha256_async(path: PathBuf) -> Result<String, String> {
    tokio::task::spawn_blocking(move || calculate_sha256(&path))
        .await
        .map_err(|error| format!("Checksum task failed: {error}"))?
}

fn calculate_sha256(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("Failed to open update for checksum: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read update for checksum: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub async fn prepare_install() -> Result<String, String> {
    if !crate::arguments::is_gui_enabled() {
        return Err(
            "Headless updates must be installed with screenerbot-manager update".to_owned(),
        );
    }

    let (update, path) = {
        let state = super::get_update_state().await;
        let update = state
            .available_update
            .ok_or_else(|| "No update is currently available".to_owned())?;
        let progress = state.download_progress;
        if state.phase != UpdatePhase::Ready
            || !progress.completed
            || progress.version.as_deref() != Some(update.version.as_str())
            || progress.checksum.as_deref() != Some(update.checksum.as_str())
        {
            return Err("The staged installer does not match the available update".to_owned());
        }
        let path = PathBuf::from(
            progress
                .downloaded_path
                .ok_or_else(|| "The staged installer path is missing".to_owned())?,
        );
        (update, path)
    };

    let expected_path = get_download_dir()?.join(&update.filename);
    if path != expected_path || !file_matches(&path, update.file_size, &update.checksum).await? {
        return Err("The staged installer failed final integrity verification".to_owned());
    }

    let client = build_update_client()?;
    verify_github_digest(&client, GITHUB_RELEASES_API_URL, &update).await?;
    open_installer(&path)?;
    mutate_state(|state| state.phase = UpdatePhase::Installing).await;
    Ok(path.to_string_lossy().into_owned())
}

fn open_installer(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        if path.extension().and_then(|value| value.to_str()) != Some("msi") {
            return Err("Windows updates require an .msi installer".to_owned());
        }
        let mut command = std::process::Command::new("msiexec.exe");
        command.arg("/i");
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        if path.extension().and_then(|value| value.to_str()) != Some("deb") {
            return Err("Linux desktop updates require a .deb installer".to_owned());
        }
        std::process::Command::new("xdg-open")
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return Err("This operating system has no update installer adapter".to_owned());

    command
        .arg(path)
        .spawn()
        .map_err(|error| format!("Failed to open update installer: {error}"))?;
    Ok(())
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
    fn github_digest_must_match_name_size_and_checksum() {
        let candidate = update();
        let release = GithubRelease {
            assets: vec![GithubAsset {
                name: candidate.filename.clone(),
                digest: Some(format!("sha256:{}", candidate.checksum)),
                size: candidate.file_size,
            }],
        };
        assert!(verify_github_release(&release, &candidate).is_ok());

        let mut changed = candidate.clone();
        changed.checksum = "b".repeat(64);
        assert!(verify_github_release(&release, &changed).is_err());
    }

    #[test]
    fn download_urls_and_sizes_fail_closed() {
        assert!(resolve_download_url("http://screenerbot.io/update").is_err());
        assert!(resolve_download_url("https://evil.example/update").is_err());
        assert!(resolve_download_url("/api/releases/download?x=1").is_ok());
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
    }
}
