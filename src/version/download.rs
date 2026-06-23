//! Update download — streaming download with progress tracking and checksum verification.

use super::types::*;
use super::{UPDATE_STATE, DOWNLOAD_TIMEOUT_SECS};
use crate::logger::{self, LogTag};
use futures_util::StreamExt;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Download an update with streaming (memory efficient).
pub async fn download_update(update: &UpdateInfo) -> Result<String, String> {
    logger::info(
        LogTag::System,
        &format!("Downloading update v{}...", update.version),
    );

    // Set download in progress
    {
        let mut state = UPDATE_STATE.write().await;
        let update_state = state.get_or_insert_with(UpdateState::default);
        update_state.download_progress = DownloadProgress {
            downloading: true,
            total_bytes: update.file_size,
            ..Default::default()
        };
    }

    // Determine download directory
    let download_dir = get_download_dir()?;
    let mut actual_filename = "screenerbot-update".to_owned();

    // Construct full download URL (handle relative paths)
    let download_url = if update.download_url.starts_with("http://")
        || update.download_url.starts_with("https://")
    {
        update.download_url.clone()
    } else {
        let base_url = super::get_update_server_url()
            .trim_end_matches("/api")
            .to_string();
        format!("{}{}", base_url, update.download_url)
    };

    logger::debug(LogTag::System, &format!("Downloading from: {download_url}"));

    // Download file with timeout
    let client = crate::net::client_builder()
        .timeout(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build()
        .map_err(|e| {
            let err = format!("Failed to create HTTP client: {e}");
            set_download_error_sync(&err);
            err
        })?;

    let response = client.get(&download_url).send().await.map_err(|e| {
        let err = format!("Download request failed: {e}");
        set_download_error_sync(&err);
        err
    })?;

    if !response.status().is_success() {
        let err = format!("Download failed: HTTP {}", response.status());
        set_download_error_sync(&err);
        return Err(err);
    }

    // Extract actual filename from the final URL (after redirects)
    let final_url = response.url().to_string();
    actual_filename = final_url
        .split('/')
        .last()
        .map(|s| s.split('?').next().unwrap_or(s))
        .unwrap_or("screenerbot-update")
        .to_string();

    if !actual_filename.contains('.') {
        actual_filename = format!("screenerbot-{}.dmg", update.version);
    }

    let download_path = download_dir.join(&actual_filename);

    logger::debug(
        LogTag::System,
        &format!(
            "Download target: {} -> {}",
            final_url,
            download_path.display()
        ),
    );

    let total_size = response.content_length().unwrap_or(update.file_size);

    // Stream download to file (memory efficient)
    let mut file = tokio::fs::File::create(&download_path).await.map_err(|e| {
        let err = format!("Failed to create file: {e}");
        set_download_error_sync(&err);
        err
    })?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_progress_update = std::time::Instant::now();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| {
            let err = format!("Download stream error: {e}");
            set_download_error_sync(&err);
            err
        })?;

        file.write_all(&chunk).await.map_err(|e| {
            let err = format!("Write error: {e}");
            set_download_error_sync(&err);
            err
        })?;

        downloaded += chunk.len() as u64;

        // Update progress every 500ms to avoid lock contention
        if last_progress_update.elapsed() > Duration::from_millis(500) {
            let progress_percent = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                0.0
            };

            let mut state = UPDATE_STATE.write().await;
            if let Some(ref mut update_state) = *state {
                update_state.download_progress.bytes_downloaded = downloaded;
                update_state.download_progress.progress_percent = progress_percent;
            }
            last_progress_update = std::time::Instant::now();
        }
    }

    // Ensure file is flushed
    file.flush().await.map_err(|e| {
        let err = format!("Failed to flush file: {e}");
        set_download_error_sync(&err);
        err
    })?;
    drop(file);

    // Final progress update
    {
        let mut state = UPDATE_STATE.write().await;
        if let Some(ref mut update_state) = *state {
            update_state.download_progress.bytes_downloaded = downloaded;
            update_state.download_progress.progress_percent = 100.0;
        }
    }

    // Verify checksum using spawn_blocking (avoid blocking async runtime)
    let checksum_path = download_path.clone();
    let expected_checksum = update.checksum.clone();
    let file_checksum = tokio::task::spawn_blocking(move || calculate_sha256(&checksum_path))
        .await
        .map_err(|e| {
            let err = format!("Checksum task failed: {e}");
            set_download_error_sync(&err);
            err
        })?
        .map_err(|e| {
            set_download_error_sync(&e);
            e
        })?;

    if file_checksum != expected_checksum {
        let err = format!(
            "Checksum mismatch: expected {}, got {}",
            expected_checksum, file_checksum
        );
        set_download_error_sync(&err);
        let _ = tokio::fs::remove_file(&download_path).await;
        return Err(err);
    }

    // Mark download complete
    {
        let mut state = UPDATE_STATE.write().await;
        if let Some(ref mut update_state) = *state {
            update_state.download_progress.downloading = false;
            update_state.download_progress.completed = true;
            update_state.download_progress.downloaded_path =
                Some(download_path.to_string_lossy().to_string());
        }
    }

    logger::info(
        LogTag::System,
        &format!("Update downloaded: {}", download_path.display()),
    );

    Ok(download_path.to_string_lossy().to_string())
}

/// Open the downloaded update for installation.
pub fn open_update(path: &str) -> Result<(), String> {
    logger::info(LogTag::System, &format!("Opening update: {path}"));

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open update: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open update: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open update: {e}"))?;
    }

    Ok(())
}

/// Get download directory.
fn get_download_dir() -> Result<std::path::PathBuf, String> {
    let dir = dirs::cache_dir()
        .ok_or_else(|| "Could not determine cache directory".to_owned())?
        .join("ScreenerBot")
        .join("updates");

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create download directory: {e}"))?;

    Ok(dir)
}

/// Calculate SHA256 checksum of a file.
fn calculate_sha256(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open file for checksum: {e}"))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {e}"))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Set download error in state (synchronous version using blocking task).
fn set_download_error_sync(error: &str) {
    let error = error.to_string();
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let _ = handle.block_on(async {
            let mut state = UPDATE_STATE.write().await;
            if let Some(ref mut update_state) = *state {
                update_state.download_progress.downloading = false;
                update_state.download_progress.error = Some(error);
            }
        });
    } else {
        crate::logger::warning(
            LogTag::System,
            &format!("Download error (state not updated): {error}"),
        );
    }
}
