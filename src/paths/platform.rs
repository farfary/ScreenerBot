//! Platform-specific operations — file manager and browser launching.

use crate::errors::{DataError, IoError};
use crate::logger::{self, LogTag};
use crate::{Error, Result};
use std::path::Path;

/// Opens a directory in the platform's file manager, creating it if needed.
pub fn open_directory_in_file_manager(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| Error::Io(IoError::from(e)))?;
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
            .map_err(|e| Error::Io(IoError::from(e)))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| Error::Io(IoError::from(e)))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| Error::Io(IoError::from(e)))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        return Err(Error::Internal(
            crate::errors::InternalError::UnsupportedCapability {
                capability: "open directory in file manager".to_owned(),
                owner: std::env::consts::OS.to_owned(),
            },
        ));
    }
}

/// Opens a URL in the platform's default browser.
pub fn open_url_in_browser(url: &str) -> Result<()> {
    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(Error::Data(DataError::ValidationError {
            field: "url scheme".to_owned(),
            value: url.to_owned(),
            reason: "only http:// and https:// are allowed".to_owned(),
        }));
    }

    logger::info(LogTag::System, &format!("Opening URL in browser: {url}"));

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| Error::Io(IoError::from(e)))?;
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| Error::Io(IoError::from(e)))?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| Error::Io(IoError::from(e)))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        return Err(Error::Internal(
            crate::errors::InternalError::UnsupportedCapability {
                capability: "open URL in browser".to_owned(),
                owner: std::env::consts::OS.to_owned(),
            },
        ));
    }
}
