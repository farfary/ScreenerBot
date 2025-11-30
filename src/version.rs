//! Version management and update checking for ScreenerBot
//!
//! Provides version info from Cargo.toml and update checking via screenerbot.io API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

use crate::logger::{self, LogTag};

// =============================================================================
// Constants
// =============================================================================

/// Compile-time version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Update server base URL - configurable via UPDATE_SERVER_URL env var
fn get_update_server_url() -> String {
    std::env::var("UPDATE_SERVER_URL")
        .unwrap_or_else(|_| "https://screenerbot.io/api".to_string())
}

/// Update check interval (6 hours)
const UPDATE_CHECK_INTERVAL_SECS: u64 = 6 * 60 * 60;

// =============================================================================
// Types
// =============================================================================

/// Current version information
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub build_date: String,
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
struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Update check response from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckData {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update: Option<UpdateResponseData>,
}

/// Update data from server
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateResponseData {
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

// =============================================================================
// Global State
// =============================================================================

static UPDATE_AVAILABLE: AtomicBool = AtomicBool::new(false);
static UPDATE_STATE: RwLock<Option<UpdateState>> = RwLock::const_new(None);

// =============================================================================
// Public API
// =============================================================================

/// Get the current version string
pub fn get_version() -> &'static str {
    VERSION
}

/// Get full version info
pub fn get_version_info() -> VersionInfo {
    VersionInfo {
        version: VERSION.to_string(),
        build_date: env!("CARGO_PKG_VERSION").to_string(), // Could add build timestamp
    }
}

/// Check if an update is available (cached)
pub fn is_update_available() -> bool {
    UPDATE_AVAILABLE.load(Ordering::SeqCst)
}

/// Get current update state
pub async fn get_update_state() -> UpdateState {
    UPDATE_STATE.read().await.clone().unwrap_or_default()
}

/// Check for updates from the server
pub async fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let platform = get_platform();
    let server_url = get_update_server_url();
    let url = format!(
        "{}/releases/check?version={}&platform={}",
        server_url, VERSION, platform
    );

    logger::info(
        LogTag::System,
        &format!("Checking for updates at: {}", server_url),
    );
    logger::debug(
        LogTag::System,
        &format!("Update check URL: {}", url),
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Update check failed: HTTP {}", response.status()));
    }

    let api_response: ApiResponse<UpdateCheckData> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse update response: {}", e))?;

    if !api_response.success {
        return Err(api_response.error.unwrap_or_else(|| "Unknown error".to_string()));
    }

    let check_data = api_response.data.ok_or("No data in response")?;

    // Update global state
    let mut state = UPDATE_STATE.write().await;
    let update_state = state.get_or_insert_with(UpdateState::default);
    update_state.last_check = Some(Utc::now());

    if check_data.update_available {
        if let Some(ref update_data) = check_data.update {
            let update_info = UpdateInfo {
                version: update_data.version.clone(),
                download_url: update_data.download_url.clone(),
                file_size: update_data.file_size,
                checksum: update_data.checksum.clone(),
                release_notes: update_data.release_notes.clone(),
                release_date: update_data.published_at.clone().unwrap_or_default(),
            };

            UPDATE_AVAILABLE.store(true, Ordering::SeqCst);
            update_state.available_update = Some(update_info.clone());
            
            logger::info(
                LogTag::System,
                &format!("Update available: v{} → v{}", VERSION, update_info.version),
            );

            return Ok(Some(update_info));
        }
    }

    UPDATE_AVAILABLE.store(false, Ordering::SeqCst);
    update_state.available_update = None;
    
    logger::debug(LogTag::System, "No updates available");

    Ok(None)
}

/// Download an update
pub async fn download_update(update: &UpdateInfo) -> Result<String, String> {
    use std::io::Write;
    use std::path::PathBuf;

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

    // Determine download path
    let download_dir = get_download_dir()?;
    let filename = update
        .download_url
        .split('/')
        .last()
        .unwrap_or("screenerbot-update");
    let download_path = download_dir.join(filename);

    // Construct full download URL (handle relative paths)
    let download_url = if update.download_url.starts_with("http://") || update.download_url.starts_with("https://") {
        update.download_url.clone()
    } else {
        // Relative path - prepend base URL
        let base_url = get_update_server_url()
            .trim_end_matches("/api")
            .to_string();
        format!("{}{}", base_url, update.download_url)
    };

    logger::debug(
        LogTag::System,
        &format!("Downloading from: {}", download_url),
    );

    // Download file
    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| {
            set_download_error(&format!("Download failed: {}", e));
            format!("Download failed: {}", e)
        })?;

    if !response.status().is_success() {
        let err = format!("Download failed: HTTP {}", response.status());
        set_download_error(&err);
        return Err(err);
    }

    let total_size = response.content_length().unwrap_or(update.file_size);

    // Download entire content (simpler approach without streaming)
    let content = response.bytes().await.map_err(|e| {
        let err = format!("Download error: {}", e);
        set_download_error(&err);
        err
    })?;

    // Update progress to 100%
    {
        let mut state = UPDATE_STATE.write().await;
        if let Some(ref mut update_state) = *state {
            update_state.download_progress.bytes_downloaded = content.len() as u64;
            update_state.download_progress.progress_percent = 100.0;
        }
    }

    // Write to file
    let mut file = std::fs::File::create(&download_path).map_err(|e| {
        let err = format!("Failed to create file: {}", e);
        set_download_error(&err);
        err
    })?;

    file.write_all(&content).map_err(|e| {
        let err = format!("Write error: {}", e);
        set_download_error(&err);
        err
    })?;

    // Verify checksum
    let file_checksum = calculate_sha256(&download_path)?;
    if file_checksum != update.checksum {
        let err = format!(
            "Checksum mismatch: expected {}, got {}",
            update.checksum, file_checksum
        );
        set_download_error(&err);
        // Clean up bad file
        let _ = std::fs::remove_file(&download_path);
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

/// Open the downloaded update for installation
pub fn open_update(path: &str) -> Result<(), String> {
    logger::info(LogTag::System, &format!("Opening update: {}", path));

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open update: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open update: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open update: {}", e))?;
    }

    Ok(())
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get platform identifier
fn get_platform() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64";

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x64";

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64";

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x64";

    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return "unknown";
}

/// Get download directory
fn get_download_dir() -> Result<std::path::PathBuf, String> {
    let dir = dirs::cache_dir()
        .ok_or_else(|| "Could not determine cache directory".to_string())?
        .join("ScreenerBot")
        .join("updates");

    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create download directory: {}", e))?;

    Ok(dir)
}

/// Calculate SHA256 checksum of a file
fn calculate_sha256(path: &std::path::Path) -> Result<String, String> {
    use sha2::{Sha256, Digest};
    use std::io::Read;

    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Failed to open file for checksum: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Set download error in state
fn set_download_error(error: &str) {
    tokio::spawn({
        let error = error.to_string();
        async move {
            let mut state = UPDATE_STATE.write().await;
            if let Some(ref mut update_state) = *state {
                update_state.download_progress.downloading = false;
                update_state.download_progress.error = Some(error);
            }
        }
    });
}

/// Compare versions (returns true if remote is newer)
pub fn is_newer_version(current: &str, remote: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let current_parts = parse_version(current);
    let remote_parts = parse_version(remote);

    for i in 0..std::cmp::max(current_parts.len(), remote_parts.len()) {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let r = remote_parts.get(i).copied().unwrap_or(0);
        if r > c {
            return true;
        }
        if r < c {
            return false;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("1.0.0", "1.1.0"));
        assert!(is_newer_version("1.0.0", "2.0.0"));
        assert!(!is_newer_version("1.0.1", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.0"));
    }
}
