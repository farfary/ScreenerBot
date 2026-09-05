use crate::{
    config,
    logger::{self, LogTag},
    version,
    webserver::utils::{error_response, success_response},
};
use axum::{http::StatusCode, response::Response, Json};

use super::types::*;

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/version
/// Returns current version information
pub(super) async fn get_version() -> Response {
    logger::debug(LogTag::Webserver, "Version endpoint called");

    let info = version::get_version_info();

    // Extract build number from version (last part after the last dot)
    // Version format: MAJOR.MINOR.BUILD_NUMBER (e.g., 0.1.57)
    let build_number = info.version.rsplit('.').next().unwrap_or("0").to_string();

    let response = VersionResponse {
        version: info.version,
        platform: info.platform,
        build_number,
        shell_revision: info.shell_revision,
        core_staged: info.core_staged,
    };

    success_response(response)
}

/// GET /api/updates/check
/// Checks for available updates
pub(super) async fn check_updates() -> Response {
    logger::debug(LogTag::Webserver, "Checking for updates...");

    let current_version = version::get_version().to_string();

    match version::check_for_update().await {
        Ok(update) => {
            let state = version::get_update_state().await;
            let response = UpdateCheckResponse {
                update_available: update.is_some(),
                current_version,
                update,
                last_check: state.last_check.map(|t| t.to_rfc3339()),
            };
            success_response(response)
        }
        Err(e) => {
            logger::warning(LogTag::Webserver, &format!("Update check failed: {e}"));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPDATE_CHECK_FAILED",
                &e.to_string(),
                None,
            )
        }
    }
}

/// POST /api/updates/download
/// Starts downloading an available update
pub(super) async fn download_update(Json(body): Json<DownloadRequest>) -> Response {
    logger::info(LogTag::Webserver, "Download update requested");

    let state = version::get_update_state().await;

    // Check if update is available
    let update = match state.available_update {
        Some(u) => u,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "NO_UPDATE_AVAILABLE",
                "No update available to download",
                None,
            );
        }
    };

    if update.version != body.version {
        return error_response(
            StatusCode::BAD_REQUEST,
            "UPDATE_VERSION_CHANGED",
            "The available update changed; check for updates again",
            None,
        );
    }

    // Clone version for response before moving into spawn
    let version_str = update.version.clone();

    if let Err(e) = version::start_download(update).await {
        return error_response(
            StatusCode::CONFLICT,
            "DOWNLOAD_NOT_STARTED",
            &e.to_string(),
            None,
        );
    }

    success_response(DownloadResponse {
        started: true,
        message: format!("Downloading update v{version_str}..."),
    })
}

/// GET /api/updates/status
/// Returns current update/download status plus what the dashboard needs to
/// describe the next step without re-deriving the rules.
pub(super) async fn get_status() -> Response {
    let state = version::get_update_state().await;

    let kind = state
        .available_update
        .as_ref()
        .map(|update| update.kind)
        .unwrap_or_default();
    let readiness = if state.phase == version::UpdatePhase::ReadyToApply {
        version::apply_readiness(kind).await.reason()
    } else {
        None
    };

    let blocked_reason = readiness
        .or(state.deferred)
        .map(|reason| reason.message().to_owned());
    let auto_download = config::with_config(|cfg| cfg.updates.auto_download);
    let requires_user_action = needs_user_action(&state, blocked_reason.is_some(), auto_download);

    success_response(UpdateStatusResponse {
        staged_core: version::read_staged_core(),
        blocked_reason,
        requires_user_action,
        state,
    })
}

fn needs_user_action(state: &version::UpdateState, blocked: bool, auto_download: bool) -> bool {
    if state.available_update.is_none() {
        return false;
    }

    match state.phase {
        version::UpdatePhase::Available => !auto_download,
        version::UpdatePhase::ReadyToApply => blocked,
        version::UpdatePhase::ReadyToInstall | version::UpdatePhase::Failed => true,
        _ => false,
    }
}

/// POST /api/updates/apply
/// Restarts the backend onto a staged core update. No operating-system
/// interaction: the running process stops and the new binary takes over.
pub(super) async fn apply_update() -> Response {
    logger::info(LogTag::Webserver, "Apply staged update requested");

    match version::apply_now().await {
        Ok(()) => success_response(ApplyResponse {
            applying: true,
            message: "Installing the update. ScreenerBot restarts and reconnects automatically."
                .to_owned(),
        }),
        Err(e) => error_response(
            StatusCode::CONFLICT,
            "APPLY_FAILED",
            &format!("Could not apply the update: {e}"),
            None,
        ),
    }
}

/// POST /api/updates/install
/// Opens the downloaded installer for a release that also replaces the shell
pub(super) async fn install_update() -> Response {
    logger::info(LogTag::Webserver, "Install update requested");

    match version::prepare_install().await {
        Ok(_) => success_response(InstallResponse {
            opened: true,
            message: "Verified update installer opened. Complete the operating-system installer."
                .to_owned(),
        }),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INSTALL_FAILED",
            &format!("Failed to open update: {e}"),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(phase: version::UpdatePhase) -> version::UpdateState {
        version::UpdateState {
            phase,
            available_update: Some(version::UpdateInfo {
                version: "99.0.0".to_owned(),
                filename: "update".to_owned(),
                download_url: "https://example.com/update".to_owned(),
                file_size: 1,
                checksum: "a".repeat(64),
                release_notes: None,
                release_date: String::new(),
                kind: version::UpdateKind::Core,
                core: None,
                shell_revision: None,
            }),
            ..version::UpdateState::default()
        }
    }

    #[test]
    fn attention_tracks_operator_decisions_only() {
        assert!(needs_user_action(
            &state(version::UpdatePhase::Available),
            false,
            false
        ));
        assert!(!needs_user_action(
            &state(version::UpdatePhase::Available),
            false,
            true
        ));
        assert!(!needs_user_action(
            &state(version::UpdatePhase::Downloading),
            false,
            false
        ));
        assert!(needs_user_action(
            &state(version::UpdatePhase::ReadyToApply),
            true,
            true
        ));
        assert!(needs_user_action(
            &state(version::UpdatePhase::ReadyToInstall),
            false,
            true
        ));
        assert!(needs_user_action(
            &state(version::UpdatePhase::Failed),
            false,
            true
        ));
    }
}
