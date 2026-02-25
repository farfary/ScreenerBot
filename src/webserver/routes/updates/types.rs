use crate::version::{UpdateInfo, UpdateState};
use serde::{Deserialize, Serialize};

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub platform: String,
    pub build_number: String,
}

#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    pub update_available: bool,
    pub current_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update: Option<UpdateInfo>,
    pub last_check: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateStatusResponse {
    pub state: UpdateState,
}

#[derive(Debug, Deserialize)]
pub struct DownloadRequest {
    // Empty for now, could add options later
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    pub started: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub opened: bool,
    pub message: String,
}
