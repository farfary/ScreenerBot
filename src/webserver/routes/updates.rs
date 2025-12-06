//! Update management API endpoints
//!
//! Provides endpoints for version info, update checking, downloading, and status.

use axum::{
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    logger::{self, LogTag},
    version::{self, DownloadProgress, UpdateInfo, UpdateState, VersionInfo},
    webserver::{
        state::AppState,
        utils::{error_response, success_response},
    },
};

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
struct VersionResponse {
    version: String,
    build_number: String,
    platform: String,
}

#[derive(Debug, Serialize)]
struct UpdateCheckResponse {
    update_available: bool,
    current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    update: Option<UpdateInfo>,
    last_check: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateStatusResponse {
    state: UpdateState,
}

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    // Empty for now, could add options later
}

#[derive(Debug, Serialize)]
struct DownloadResponse {
    started: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct InstallResponse {
    opened: bool,
    message: String,
}

// =============================================================================
// Routes
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/version", get(get_version))
        .route("/updates/check", get(check_updates))
        .route("/updates/download", post(download_update))
        .route("/updates/status", get(get_status))
        .route("/updates/install", post(install_update))
}

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/version
/// Returns current version information
async fn get_version() -> Response {
    logger::debug(LogTag::Webserver, "Version endpoint called");

    let info = version::get_version_info();
    let response = VersionResponse {
        version: info.version,
        build_number: info.build_number,
        platform: get_platform().to_string(),
    };

    success_response(response)
}

/// GET /api/updates/check
/// Checks for available updates
async fn check_updates() -> Response {
    logger::info(LogTag::Webserver, "Checking for updates...");

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
            logger::warning(LogTag::Webserver, &format!("Update check failed: {}", e));
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "UPDATE_CHECK_FAILED",
                &e,
                None,
            )
        }
    }
}

/// POST /api/updates/download
/// Starts downloading an available update
async fn download_update(_body: Json<DownloadRequest>) -> Response {
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

    // Check if already downloading
    if state.download_progress.downloading {
        return error_response(
            StatusCode::BAD_REQUEST,
            "DOWNLOAD_IN_PROGRESS",
            "Download already in progress",
            None,
        );
    }

    // Clone version for response before moving into spawn
    let version_str = update.version.clone();

    // Start download in background
    tokio::spawn(async move {
        if let Err(e) = version::download_update(&update).await {
            logger::warning(LogTag::System, &format!("Download failed: {}", e));
        }
    });

    success_response(DownloadResponse {
        started: true,
        message: format!("Downloading update v{}...", version_str),
    })
}

/// GET /api/updates/status
/// Returns current update/download status
async fn get_status() -> Response {
    let state = version::get_update_state().await;
    success_response(UpdateStatusResponse { state })
}

/// POST /api/updates/install
/// Opens the downloaded update for installation
async fn install_update() -> Response {
    logger::info(LogTag::Webserver, "Install update requested");

    let state = version::get_update_state().await;

    // Check if download is complete
    let path = match state.download_progress.downloaded_path {
        Some(p) if state.download_progress.completed => p,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "NO_DOWNLOADED_UPDATE",
                "No downloaded update available",
                None,
            );
        }
    };

    match version::open_update(&path) {
        Ok(_) => success_response(InstallResponse {
            opened: true,
            message: "Update installer opened. Please complete the installation.".to_string(),
        }),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INSTALL_FAILED",
            &format!("Failed to open update: {}", e),
            None,
        ),
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn get_platform() -> &'static str {
    // macOS always uses universal builds (Intel + Apple Silicon combined)
    #[cfg(target_os = "macos")]
    return "macos-universal";

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64";

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-arm64";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x64";

    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "windows-arm64";

    #[cfg(not(any(
        target_os = "macos",
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
    )))]
    return "unknown";
}
