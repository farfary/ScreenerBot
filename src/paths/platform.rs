//! Platform-specific operations — file manager and browser launching.

use crate::logger::{self, LogTag};
use std::path::Path;

/// Opens a directory in the platform's file manager, creating it if needed.
pub fn open_directory_in_file_manager(path: &Path) -> Result<(), String> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create directory {}: {}", path.display(), e))?;
    }

    logger::info(
        LogTag::System,
        &format!("Opening directory: {}", path.display()),
    );

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open directory {}: {}", path.display(), e))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open directory {}: {}", path.display(), e))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open directory {}: {}", path.display(), e))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let unsupported = format!(
            "Opening directories is not supported on this platform ({})",
            std::env::consts::OS
        );
        return Err(unsupported);
    }
}

/// Opens a URL in the platform's default browser.
pub fn open_url_in_browser(url: &str) -> Result<(), String> {
    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!(
            "Invalid URL scheme: {}. Only http:// and https:// are allowed",
            url
        ));
    }

    logger::info(LogTag::System, &format!("Opening URL in browser: {url}"));

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL {url}: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("Failed to open URL {url}: {e}"))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| format!("Failed to open URL {url}: {e}"))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let unsupported = format!(
            "Opening URLs is not supported on this platform ({})",
            std::env::consts::OS
        );
        return Err(unsupported);
    }
}
