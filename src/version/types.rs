//! Types for version management and update checking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Current version information
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub platform: String,
}

/// Information about an available update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub file_size: u64,
    pub checksum: String,
    pub release_notes: Option<String>,
    pub release_date: String,
}

/// API response wrapper
#[derive(Debug, Clone, Deserialize)]
pub(super) struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Update check response from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateCheckData {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update: Option<UpdateResponseData>,
}

/// Update data from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UpdateResponseData {
    pub version: String,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
    pub download_url: String,
    pub file_size: u64,
    pub checksum: String,
}

/// Download progress information
#[derive(Debug, Clone, Serialize, Default)]
pub struct DownloadProgress {
    pub downloading: bool,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub progress_percent: f32,
    pub error: Option<String>,
    pub completed: bool,
    pub downloaded_path: Option<String>,
}

/// Update state
#[derive(Debug, Clone, Serialize, Default)]
pub struct UpdateState {
    pub available_update: Option<UpdateInfo>,
    pub last_check: Option<DateTime<Utc>>,
    pub download_progress: DownloadProgress,
}
