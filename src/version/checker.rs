//! Update checking against the strict production release contract.

use super::types::*;
use super::{
    current_platform_key, mutate_state, UPDATE_AVAILABLE, UPDATE_CHECK_INTERVAL_SECS,
    UPDATE_SERVER_URL, VERSION,
};
use crate::logger::{self, LogTag};
use chrono::Utc;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub fn start_update_check_service(
    shutdown: std::sync::Arc<tokio::sync::Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(monitor.instrument(async move {
        logger::info(
            LogTag::System,
            &format!(
                "Update check service started (interval: {} hours)",
                UPDATE_CHECK_INTERVAL_SECS / 3600
            ),
        );

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(30)) => {}
            _ = shutdown.notified() => return,
        }

        if !crate::connectivity::is_network_offline() {
            if let Err(error) = check_for_update().await {
                logger::warning(LogTag::System, &format!("Initial update check failed: {error}"));
            }
        }

        let mut interval = tokio::time::interval(Duration::from_secs(UPDATE_CHECK_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !crate::connectivity::is_network_offline() {
                        if let Err(error) = check_for_update().await {
                            logger::warning(LogTag::System, &format!("Periodic update check failed: {error}"));
                        }
                    }
                }
                _ = shutdown.notified() => break,
            }
        }
    }))
}

pub async fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    check_for_update_from(
        crate::net::client(),
        UPDATE_SERVER_URL,
        VERSION,
        current_platform_key(),
    )
    .await
}

async fn check_for_update_from(
    client: reqwest::Client,
    server_url: &str,
    current_version: &str,
    platform: &str,
) -> Result<Option<UpdateInfo>, String> {
    mutate_state(|state| {
        state.phase = UpdatePhase::Checking;
        state.last_check_attempt = Some(Utc::now());
        state.check_error = None;
    })
    .await;

    let result = request_update(&client, server_url, current_version, platform).await;
    match result {
        Ok(update) => {
            record_check_success(update.clone()).await;
            Ok(update)
        }
        Err(error) => {
            record_check_failure(&error).await;
            Err(error)
        }
    }
}

async fn request_update(
    client: &reqwest::Client,
    server_url: &str,
    current_version: &str,
    platform: &str,
) -> Result<Option<UpdateInfo>, String> {
    let url = format!(
        "{}/releases/check?version={}&platform={}",
        server_url, current_version, platform
    );
    let response = client
        .get(&url)
        .header("User-Agent", format!("ScreenerBot/{current_version}"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|error| format!("Failed to check for updates: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("Update check failed: HTTP {}", response.status()));
    }

    let body = response
        .bytes()
        .await
        .map_err(|error| format!("Failed to read update response: {error}"))?;
    parse_update_response(&body, current_version)
}

fn parse_update_response(body: &[u8], current_version: &str) -> Result<Option<UpdateInfo>, String> {
    let api_response: ApiResponse<UpdateCheckData> = serde_json::from_slice(body)
        .map_err(|error| format!("Failed to parse update response: {error}"))?;
    if !api_response.success {
        return Err(api_response
            .error
            .unwrap_or_else(|| "Update server returned an unknown error".to_owned()));
    }

    let data = api_response
        .data
        .ok_or_else(|| "Update response has no data".to_owned())?;
    if data.current_version != current_version {
        return Err("Update server echoed a different current version".to_owned());
    }

    if !data.update_available {
        return Ok(None);
    }

    let update = data
        .update
        .ok_or_else(|| "Update response says an update is available but has no build".to_owned())?;
    if data.latest_version.as_deref() != Some(update.version.as_str()) {
        return Err("Update response version fields do not agree".to_owned());
    }
    if !super::is_newer_version(current_version, &update.version) {
        return Err("Update server offered a version that is not newer".to_owned());
    }
    validate_checksum(&update.checksum)?;
    validate_filename(&update.filename, &update.version)?;
    if update.file_size == 0 || update.file_size > super::MAX_UPDATE_BYTES {
        return Err("Update file size is outside the allowed range".to_owned());
    }
    validate_download_url(&update.download_url)?;

    Ok(Some(UpdateInfo {
        version: update.version,
        filename: update.filename,
        download_url: update.download_url,
        file_size: update.file_size,
        checksum: update.checksum.to_ascii_lowercase(),
        release_notes: update.release_notes,
        release_date: update.published_at.unwrap_or_default(),
    }))
}

pub(super) fn validate_checksum(checksum: &str) -> Result<(), String> {
    if checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("Update checksum is not a SHA-256 digest".to_owned())
    }
}

fn validate_filename(filename: &str, version: &str) -> Result<(), String> {
    let safe = !filename.is_empty()
        && filename.len() <= 255
        && !filename.contains('/')
        && !filename.contains('\\')
        && filename.starts_with(&format!("ScreenerBot-v{version}-"))
        && filename
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte));
    if safe {
        Ok(())
    } else {
        Err("Update filename is invalid".to_owned())
    }
}

fn validate_download_url(download_url: &str) -> Result<(), String> {
    if download_url.starts_with("/api/releases/download?") {
        return Ok(());
    }
    let url = reqwest::Url::parse(download_url)
        .map_err(|error| format!("Invalid update download URL: {error}"))?;
    if url.scheme() == "https" && url.host_str() == Some("screenerbot.io") {
        Ok(())
    } else {
        Err("Update download URL must use the ScreenerBot HTTPS origin".to_owned())
    }
}

async fn record_check_success(update: Option<UpdateInfo>) {
    let available = update.is_some();
    UPDATE_AVAILABLE.store(available, Ordering::SeqCst);
    mutate_state(|state| {
        state.last_check = Some(Utc::now());
        state.check_error = None;
        state.available_update = update.clone();

        if let Some(ref candidate) = update {
            let same_ready_download = state.download_progress.completed
                && state.download_progress.version.as_deref() == Some(candidate.version.as_str())
                && state.download_progress.checksum.as_deref() == Some(candidate.checksum.as_str());
            if same_ready_download {
                state.phase = UpdatePhase::Ready;
            } else {
                state.phase = UpdatePhase::Available;
                state.download_progress = DownloadProgress::default();
            }
        } else {
            state.phase = UpdatePhase::UpToDate;
            state.download_progress = DownloadProgress::default();
        }
    })
    .await;
}

async fn record_check_failure(error: &str) {
    UPDATE_AVAILABLE.store(false, Ordering::SeqCst);
    mutate_state(|state| {
        state.phase = UpdatePhase::CheckFailed;
        state.check_error = Some(error.to_owned());
        state.available_update = None;
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(update: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&update).unwrap()
    }

    #[test]
    fn parses_a_complete_update_contract() {
        let body = response(serde_json::json!({
            "success": true,
            "data": {
                "updateAvailable": true,
                "currentVersion": "0.1.121",
                "latestVersion": "0.1.122",
                "update": {
                    "version": "0.1.122",
                    "releaseNotes": "notes",
                    "publishedAt": "2026-08-21T00:00:00Z",
                    "downloadUrl": "/api/releases/download?version=0.1.122&platform=macos-x64",
                    "filename": "ScreenerBot-v0.1.122-macOS-x64.dmg",
                    "fileSize": 42,
                    "checksum": "a".repeat(64)
                }
            }
        }));
        let update = parse_update_response(&body, "0.1.121").unwrap().unwrap();
        assert_eq!(update.version, "0.1.122");
        assert_eq!(update.file_size, 42);
    }

    #[test]
    fn rejects_null_or_malformed_checksums() {
        for checksum in [serde_json::Value::Null, serde_json::json!("bad")] {
            let body = response(serde_json::json!({
                "success": true,
                "data": {
                    "updateAvailable": true,
                    "currentVersion": "0.1.121",
                    "latestVersion": "0.1.122",
                    "update": {
                        "version": "0.1.122",
                        "downloadUrl": "/api/releases/download?version=0.1.122&platform=macos-x64",
                        "filename": "ScreenerBot-v0.1.122-macOS-x64.dmg",
                        "fileSize": 42,
                        "checksum": checksum
                    }
                }
            }));
            assert!(parse_update_response(&body, "0.1.121").is_err());
        }
    }

    #[test]
    fn distinguishes_no_update_from_contract_failure() {
        let no_update = response(serde_json::json!({
            "success": true,
            "data": { "updateAvailable": false, "currentVersion": "0.1.121" }
        }));
        assert!(parse_update_response(&no_update, "0.1.121")
            .unwrap()
            .is_none());

        let missing_build = response(serde_json::json!({
            "success": true,
            "data": { "updateAvailable": true, "currentVersion": "0.1.121", "latestVersion": "0.1.122" }
        }));
        assert!(parse_update_response(&missing_build, "0.1.121").is_err());
    }

    #[test]
    fn rejects_insecure_or_unowned_download_urls() {
        assert!(validate_download_url("http://screenerbot.io/file").is_err());
        assert!(validate_download_url("https://evil.example/file").is_err());
        assert!(validate_download_url("https://screenerbot.io/file").is_ok());
    }
}
