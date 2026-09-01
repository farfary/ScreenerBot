//! Update checking against the strict production release contract, and the
//! decision of which components a release actually has to replace.

use super::manifest;
use super::types::*;
use super::{
    core_platform_key, current_platform_key, local_shell_revision, mutate_state, Error, Result,
    UPDATE_AVAILABLE, UPDATE_SERVER_URL, VERSION,
};
use crate::logger::{self, LogTag};
use chrono::Utc;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub async fn check_for_update() -> Result<Option<UpdateInfo>> {
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
) -> Result<Option<UpdateInfo>> {
    mutate_state(|state| {
        state.phase = UpdatePhase::Checking;
        state.last_check_attempt = Some(Utc::now());
        state.check_error = None;
    })
    .await;

    let result = request_update(&client, server_url, current_version, platform).await;
    match result {
        Ok(Some(update)) => {
            let update = plan_components(update).await;
            record_check_success(Some(update.clone())).await;
            Ok(Some(update))
        }
        Ok(None) => {
            record_check_success(None).await;
            Ok(None)
        }
        Err(error) => {
            record_check_failure(&error).await;
            Err(error)
        }
    }
}

/// Decide whether the offered release can be applied as a core-only update.
///
/// This is deliberately fail-soft: any doubt about the shell — no manifest, an
/// unreadable one, no core artifact for this machine, or a shell revision this
/// process cannot even know (headless, development run) — resolves to
/// [`UpdateKind::Full`], which is always correct, just larger.
async fn plan_components(mut update: UpdateInfo) -> UpdateInfo {
    update.kind = UpdateKind::Full;

    let Some(local_revision) = local_shell_revision() else {
        return update;
    };
    let client = match super::download::build_update_client() {
        Ok(client) => client,
        Err(error) => {
            logger::debug(
                LogTag::System,
                &format!("Update planning could not build a client: {error}"),
            );
            return update;
        }
    };

    let manifest = match manifest::fetch_release_for(&client, &update.version).await {
        Ok(release) => manifest::fetch_manifest(&client, &release, &update.version).await,
        Err(error) => Err(error),
    };
    let manifest = match manifest {
        Ok(manifest) => manifest,
        Err(error) => {
            logger::info(
                LogTag::System,
                &format!(
                    "Release v{} has no usable update manifest ({error}); the full installer will be used",
                    update.version
                ),
            );
            return update;
        }
    };

    update.shell_revision = Some(manifest.shell_revision.clone());
    if manifest.shell_revision != local_revision {
        logger::info(
            LogTag::System,
            &format!(
                "Release v{} rebuilds the desktop shell ({} -> {}); the installer is required",
                update.version, local_revision, manifest.shell_revision
            ),
        );
        return update;
    }

    let Some(core) = manifest.core.get(core_platform_key()).cloned() else {
        logger::info(
            LogTag::System,
            &format!(
                "Release v{} publishes no core artifact for {}; the full installer will be used",
                update.version,
                core_platform_key()
            ),
        );
        return update;
    };

    logger::info(
        LogTag::System,
        &format!(
            "Release v{} only changes the core ({} MB instead of {} MB); it installs silently",
            update.version,
            core.size / (1024 * 1024),
            update.file_size / (1024 * 1024)
        ),
    );
    update.core = Some(core);
    update.kind = UpdateKind::Core;
    update
}

async fn request_update(
    client: &reqwest::Client,
    server_url: &str,
    current_version: &str,
    platform: &str,
) -> Result<Option<UpdateInfo>> {
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
        .map_err(|error| {
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: url.clone(),
                detail: error.to_string(),
            })
        })?;

    if !response.status().is_success() {
        return Err(Error::UpdateCheckFailed {
            status: response.status().as_u16(),
        });
    }

    let body = response.bytes().await.map_err(|error| {
        Error::Network(crate::errors::NetworkError::RequestFailed {
            endpoint: url,
            detail: error.to_string(),
        })
    })?;
    parse_update_response(&body, current_version)
}

fn parse_update_response(body: &[u8], current_version: &str) -> Result<Option<UpdateInfo>> {
    let api_response: ApiResponse<UpdateCheckData> =
        serde_json::from_slice(body).map_err(|error| {
            Error::Data(crate::errors::DataError::ParseError {
                data_type: "update response".to_owned(),
                error: error.to_string(),
            })
        })?;
    if !api_response.success {
        return Err(Error::Data(crate::errors::DataError::InvalidFormat {
            expected: "successful update response".to_owned(),
            received: api_response
                .error
                .unwrap_or_else(|| "an unspecified server error".to_owned()),
        }));
    }

    let data = api_response.data.ok_or_else(|| {
        Error::Data(crate::errors::DataError::InvalidFormat {
            expected: "update response data".to_owned(),
            received: "no data".to_owned(),
        })
    })?;
    if data.current_version != current_version {
        return Err(Error::Data(crate::errors::DataError::InvalidFormat {
            expected: current_version.to_owned(),
            received: data.current_version,
        }));
    }

    if !data.update_available {
        return Ok(None);
    }

    let update = data.update.ok_or_else(|| {
        Error::Data(crate::errors::DataError::InvalidFormat {
            expected: "update build when updateAvailable is true".to_owned(),
            received: "no update build".to_owned(),
        })
    })?;
    if data.latest_version.as_deref() != Some(update.version.as_str()) {
        return Err(Error::Data(crate::errors::DataError::InvalidFormat {
            expected: update.version.clone(),
            received: data
                .latest_version
                .unwrap_or_else(|| "no latest version".to_owned()),
        }));
    }
    if !super::is_newer_version(current_version, &update.version) {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update.version".to_owned(),
            value: update.version.clone(),
            reason: format!("must be newer than {current_version}"),
        }));
    }
    validate_checksum(&update.checksum)?;
    validate_release_filename(&update.filename, &update.version)?;
    if update.file_size == 0 || update.file_size > super::MAX_UPDATE_BYTES {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update.file_size".to_owned(),
            value: update.file_size.to_string(),
            reason: format!("must be between 1 and {} bytes", super::MAX_UPDATE_BYTES),
        }));
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
        kind: UpdateKind::Full,
        core: None,
        shell_revision: None,
    }))
}

pub(super) fn validate_checksum(checksum: &str) -> Result<()> {
    if checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update.checksum".to_owned(),
            value: checksum.to_owned(),
            reason: "must be a SHA-256 digest".to_owned(),
        }))
    }
}

/// A release asset name is used as a filename on disk, so it must contain no
/// path syntax and must belong to the version actually being offered.
pub(super) fn validate_release_filename(filename: &str, version: &str) -> Result<()> {
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
        Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update.filename".to_owned(),
            value: filename.to_owned(),
            reason: "must be a safe release filename for the offered version".to_owned(),
        }))
    }
}

fn validate_download_url(download_url: &str) -> Result<()> {
    if download_url.starts_with("/api/releases/download?") {
        return Ok(());
    }
    let url = reqwest::Url::parse(download_url).map_err(|error| Error::InvalidUpdateUrl {
        url: download_url.to_owned(),
        reason: error.to_string(),
    })?;
    if url.scheme() == "https" && url.host_str() == Some("screenerbot.io") {
        Ok(())
    } else {
        Err(Error::InvalidUpdateUrl {
            url: download_url.to_owned(),
            reason: "must use the ScreenerBot HTTPS origin".to_owned(),
        })
    }
}

async fn record_check_success(update: Option<UpdateInfo>) {
    let available = update.is_some();
    UPDATE_AVAILABLE.store(available, Ordering::SeqCst);
    mutate_state(|state| {
        state.last_check = Some(Utc::now());
        state.check_error = None;
        state.available_update = update.clone();

        let Some(candidate) = update else {
            state.phase = UpdatePhase::UpToDate;
            state.deferred = None;
            state.download_progress = DownloadProgress::default();
            return;
        };

        // A staged core survives a re-check: the artifact is already verified and
        // waiting, so re-downloading it would be pure waste.
        if candidate.kind == UpdateKind::Core
            && super::core_install::staged_core_version().as_deref()
                == Some(candidate.version.as_str())
        {
            state.phase = UpdatePhase::ReadyToApply;
            return;
        }

        let same_ready_download = state.download_progress.completed
            && state.download_progress.version.as_deref() == Some(candidate.version.as_str())
            && state.download_progress.checksum.as_deref() == Some(candidate.checksum.as_str());
        if same_ready_download && candidate.kind == UpdateKind::Full {
            state.phase = UpdatePhase::ReadyToInstall;
        } else {
            state.phase = UpdatePhase::Available;
            state.deferred = None;
            state.download_progress = DownloadProgress::default();
        }
    })
    .await;
}

async fn record_check_failure(error: &Error) {
    UPDATE_AVAILABLE.store(false, Ordering::SeqCst);
    mutate_state(|state| {
        state.phase = UpdatePhase::CheckFailed;
        state.check_error = Some(error.to_string());
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
        // The website contract alone never authorises the silent path.
        assert_eq!(update.kind, UpdateKind::Full);
        assert!(update.core.is_none());
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

    #[test]
    fn release_filenames_cannot_carry_path_syntax_or_a_foreign_version() {
        assert!(
            validate_release_filename("ScreenerBot-v0.2.2-macOS-arm64-core.gz", "0.2.2").is_ok()
        );
        assert!(validate_release_filename("ScreenerBot-v0.2.3-macOS-arm64.dmg", "0.2.2").is_err());
        assert!(validate_release_filename("../ScreenerBot-v0.2.2-x.dmg", "0.2.2").is_err());
        assert!(validate_release_filename("ScreenerBot-v0.2.2-a/b.dmg", "0.2.2").is_err());
    }
}
