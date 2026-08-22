//! System route types — response structs for system endpoints.

use crate::services::startup::StartupServiceStatus;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RebootResponse {
    pub success: bool,
    pub message: String,
    /// Identity of the process accepting the restart request. Clients wait for
    /// `/api/health` to report a different value before reloading.
    pub instance_id: String,
}

#[derive(Debug, Serialize)]
pub struct BootStatusResponse {
    pub timestamp: String,
    pub initialization_required: bool,
    pub initialization_complete: bool,
    pub explore_mode: bool,
    pub onboarding_complete: bool,
    pub core_services_ready: bool,
    pub ui_ready: bool,
    pub ready_for_requests: bool,
    pub pending_services: Vec<&'static str>,
    pub services_total: usize,
    pub services_running: usize,
    pub connectivity_ready: bool,
    pub tokens_ready: bool,
    pub positions_ready: bool,
    pub pools_ready: bool,
    pub transactions_ready: bool,
    pub boot_progress: Vec<StartupServiceStatus>,
    pub wallet_snapshot_ready: bool,
    pub wallet_last_updated: Option<String>,
    pub uptime_seconds: u64,
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct PathsResponse {
    pub base_directory: String,
    pub data_directory: String,
    pub logs_directory: String,
    pub cache_pool_directory: String,
    pub analysis_exports_directory: String,
    pub config_path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenPathResponse {
    pub opened: bool,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_mb: f64,
    pub exists: bool,
}

#[derive(Debug, Serialize)]
pub struct DataStatsResponse {
    pub databases: Vec<DatabaseStats>,
    pub total_size_mb: f64,
    pub config_path: String,
    pub config_size_bytes: u64,
    pub data_directory: String,
    pub timestamp: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct OpenUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct OpenUrlResponse {
    pub opened: bool,
    pub message: String,
    pub url: String,
}

/// Payload the dashboard frontend sends once it has fully loaded and started.
/// All fields optional so the signal works even from a minimal caller.
#[derive(Debug, Default, serde::Deserialize)]
pub struct ClientReadyRequest {
    /// The page that was active when the UI became ready (default "home").
    #[serde(default)]
    pub page: Option<String>,
    /// Milliseconds from page navigation start to fully-ready, if measured.
    #[serde(default)]
    pub load_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ClientReadyResponse {
    pub acknowledged: bool,
    /// True only for the first report after boot (the one that was logged).
    pub first_report: bool,
    pub uptime_seconds: u64,
}
