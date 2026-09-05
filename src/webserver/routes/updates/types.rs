use crate::version::{StagedCore, UpdateInfo, UpdateState};
use serde::{Deserialize, Serialize};

// =============================================================================
// Response Types
// =============================================================================

#[derive(Debug, Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub platform: String,
    pub build_number: String,
    /// Revision of the Electron shell hosting this core, when it reported one.
    pub shell_revision: Option<String>,
    /// True when this core was activated by a silent update rather than shipped
    /// with the installed application.
    pub core_staged: bool,
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
    /// The verified core waiting to be activated, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_core: Option<StagedCore>,
    /// Why a ready update is not installing itself right now.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    /// Whether the current phase needs an explicit choice from the operator.
    pub requires_user_action: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadRequest {
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct DownloadResponse {
    pub started: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyResponse {
    pub applying: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct InstallResponse {
    pub opened: bool,
    pub message: String,
}
