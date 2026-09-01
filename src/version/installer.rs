//! Handoff to the operating-system installer for the rare release that also
//! replaces the Electron shell.
//!
//! Everything that can be made quiet is quiet — Windows runs the MSI with its
//! passive UI so no dialog has to be answered — but replacing an installed
//! application bundle is the operating system's job, and this path never
//! pretends otherwise. Core-only releases never reach here.

use super::download::{file_matches, get_download_dir};
use super::types::*;
use super::{manifest, mutate_state, Error, Result};
use std::path::{Path, PathBuf};

/// Verify the staged installer one last time and hand it to the system.
pub async fn prepare_install() -> Result<String> {
    if !crate::arguments::is_gui_enabled() {
        return Err(Error::UnsupportedInstall {
            detail: "headless updates must be installed with screenerbot-manager update".to_owned(),
        });
    }

    let (update, path) = {
        let state = super::get_update_state().await;
        let update = state.available_update.ok_or(Error::NoUpdateAvailable)?;
        let progress = state.download_progress;
        if state.phase != UpdatePhase::ReadyToInstall
            || !progress.completed
            || progress.version.as_deref() != Some(update.version.as_str())
            || progress.checksum.as_deref() != Some(update.checksum.as_str())
        {
            return Err(Error::DigestMismatch {
                detail: "staged installer metadata does not match the available update".to_owned(),
            });
        }
        let path =
            PathBuf::from(
                progress
                    .downloaded_path
                    .ok_or_else(|| Error::DigestMismatch {
                        detail: "staged installer path is missing".to_owned(),
                    })?,
            );
        (update, path)
    };

    let expected_path = get_download_dir()?.join(&update.filename);
    if path != expected_path || !file_matches(&path, update.file_size, &update.checksum).await? {
        return Err(Error::DigestMismatch {
            detail: "staged installer failed final integrity verification".to_owned(),
        });
    }

    let client = super::download::build_update_client()?;
    let release = manifest::fetch_release_for(&client, &update.version).await?;
    manifest::verify_release_asset(
        &release,
        &update.filename,
        update.file_size,
        &update.checksum,
    )?;

    open_installer(&path)?;
    mutate_state(|state| state.phase = UpdatePhase::Applying).await;
    Ok(path.to_string_lossy().into_owned())
}

fn open_installer(path: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        if path.extension().and_then(|value| value.to_str()) != Some("msi") {
            return Err(Error::UnsupportedInstall {
                detail: "Windows updates require an .msi installer".to_owned(),
            });
        }
        // `/passive` shows a progress bar and answers nothing; `/norestart`
        // keeps the installer from rebooting the machine behind the owner.
        let mut command = std::process::Command::new("msiexec.exe");
        command.arg("/i");
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        if path.extension().and_then(|value| value.to_str()) != Some("deb") {
            return Err(Error::UnsupportedInstall {
                detail: "Linux desktop updates require a .deb installer".to_owned(),
            });
        }
        std::process::Command::new("xdg-open")
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return Err(Error::UnsupportedInstall {
        detail: "this operating system has no update installer adapter".to_owned(),
    });

    command.arg(path);
    #[cfg(target_os = "windows")]
    command.args(["/passive", "/norestart"]);

    command
        .spawn()
        .map_err(|error| Error::Io(crate::errors::IoError::from(error)))?;
    Ok(())
}
