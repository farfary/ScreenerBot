//! Version management and update checking for ScreenerBot.
//!
//! Version format: MAJOR.MINOR.BUILD_NUMBER (e.g., 0.1.34)
//! The version patch number IS the build number, auto-incremented each publish.
//! Provides version info from Cargo.toml and update checking via screenerbot.io API.
//! Includes background periodic update checking service.

mod checker;
mod download;
pub mod types;

pub use checker::{check_for_update, start_update_check_service};
pub use download::{download_update, open_update};
pub use types::*;

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

/// Compile-time version from Cargo.toml (format: MAJOR.MINOR.BUILD_NUMBER).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Update check interval (6 hours)
const UPDATE_CHECK_INTERVAL_SECS: u64 = 6 * 60 * 60;

/// Download timeout (30 minutes for large files)
const DOWNLOAD_TIMEOUT_SECS: u64 = 30 * 60;

static UPDATE_AVAILABLE: AtomicBool = AtomicBool::new(false);
static UPDATE_STATE: RwLock<Option<UpdateState>> = RwLock::const_new(None);

/// Update server base URL — configurable via UPDATE_SERVER_URL env var.
fn get_update_server_url() -> String {
    std::env::var("UPDATE_SERVER_URL").unwrap_or_else(|_| "https://screenerbot.io/api".to_owned())
}

/// Get the current version string.
pub fn get_version() -> &'static str {
    VERSION
}

/// Get full version info including platform detection.
pub fn get_version_info() -> VersionInfo {
    let platform = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macOS (Apple Silicon)"
        } else if cfg!(target_arch = "x86_64") {
            "macOS (Intel)"
        } else {
            "macOS (Universal)"
        }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "Windows (ARM64)"
        } else {
            "Windows (x64)"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "Linux (ARM64)"
        } else {
            "Linux (x64)"
        }
    } else {
        "Unknown"
    };

    VersionInfo {
        version: VERSION.to_string(),
        platform: platform.to_string(),
    }
}

/// Check if an update is available (cached).
pub fn is_update_available() -> bool {
    UPDATE_AVAILABLE.load(Ordering::SeqCst)
}

/// Get current update state.
pub async fn get_update_state() -> UpdateState {
    UPDATE_STATE.read().await.clone().unwrap_or_default()
}

/// Compare versions (returns true if remote is newer).
pub fn is_newer_version(current: &str, remote: &str) -> bool {
    let parse_version =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse::<u32>().ok()).collect() };

    let current_parts = parse_version(current);
    let remote_parts = parse_version(remote);

    for i in 0..current_parts.len().max(remote_parts.len()) {
        let c = current_parts.get(i).copied().unwrap_or_default();
        let r = remote_parts.get(i).copied().unwrap_or_default();
        if r > c {
            return true;
        }
        if r < c {
            return false;
        }
    }

    false
}

/// Get platform identifier matching build.sh naming.
fn get_platform() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x64";

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-arm64";

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
