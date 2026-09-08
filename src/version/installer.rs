//! Handoff to the operating-system installer for the rare release that also
//! replaces the Electron shell.
//!
//! Everything that can be made quiet is quiet — Windows runs the MSI with its
//! passive UI so no dialog has to be answered — but replacing an installed
//! application bundle is the operating system's job, and this path never
//! pretends otherwise. Core-only releases never reach here.

use super::download::{file_matches, get_download_dir};
use super::types::*;
use super::{manifest, mutate_state, mutate_state_with_result, Error, Result};
use std::path::{Path, PathBuf};

/// Verify the staged installer one last time and hand it to the system.
pub async fn prepare_install() -> Result<String> {
    if !crate::arguments::is_gui_enabled() {
        return Err(Error::UnsupportedInstall {
            detail: "headless updates must be installed with screenerbot-manager update".to_owned(),
        });
    }

    // Claim the installer before any asynchronous verification. This closes the
    // race where a scheduled check could replace the advertised release while
    // an older installer was still being authenticated and opened.
    let (update, path) = mutate_state_with_result(claim_install).await?;

    let result = verify_and_open_installer(&update, &path).await;
    if let Err(error) = result {
        let version = update.version.clone();
        let message = error.to_string();
        mutate_state(|state| {
            if state.phase == UpdatePhase::Applying
                && state
                    .available_update
                    .as_ref()
                    .map(|item| item.version.as_str())
                    == Some(version.as_str())
            {
                state.phase = UpdatePhase::Failed;
                state.download_progress.downloading = false;
                state.download_progress.completed = false;
                state.download_progress.error = Some(message);
            }
        })
        .await;
        return Err(error);
    }

    Ok(path.to_string_lossy().into_owned())
}

fn claim_install(state: &mut UpdateState) -> Result<(UpdateInfo, PathBuf)> {
    let update = state
        .available_update
        .as_ref()
        .ok_or(Error::NoUpdateAvailable)?;
    let progress = &state.download_progress;
    if state.phase != UpdatePhase::ReadyToInstall
        || !progress.completed
        || progress.version.as_deref() != Some(update.version.as_str())
        || progress.checksum.as_deref() != Some(update.checksum.as_str())
    {
        return Err(if state.phase == UpdatePhase::Applying {
            Error::RestartInProgress
        } else if state.phase.is_busy() {
            Error::DownloadInProgress
        } else {
            Error::DigestMismatch {
                detail: "staged installer metadata does not match the available update".to_owned(),
            }
        });
    }
    let path =
        PathBuf::from(
            progress
                .downloaded_path
                .as_ref()
                .ok_or_else(|| Error::DigestMismatch {
                    detail: "staged installer path is missing".to_owned(),
                })?,
        );
    let update = update.clone();
    state.phase = UpdatePhase::Applying;
    state.deferred = None;
    Ok((update, path))
}

async fn verify_and_open_installer(update: &UpdateInfo, path: &Path) -> Result<()> {
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

    open_installer(path)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_state() -> UpdateState {
        let update = UpdateInfo {
            version: "99.0.0".to_owned(),
            filename: "ScreenerBot.pkg".to_owned(),
            download_url: "https://example.com/ScreenerBot.pkg".to_owned(),
            file_size: 42,
            checksum: "a".repeat(64),
            manifest_checksum: None,
            release_notes: None,
            release_date: String::new(),
            kind: UpdateKind::Full,
            core: None,
            shell_revision: None,
        };
        UpdateState {
            phase: UpdatePhase::ReadyToInstall,
            available_update: Some(update.clone()),
            download_progress: DownloadProgress {
                version: Some(update.version),
                checksum: Some(update.checksum),
                completed: true,
                downloaded_path: Some("/tmp/ScreenerBot.pkg".to_owned()),
                ..DownloadProgress::default()
            },
            ..UpdateState::default()
        }
    }

    #[test]
    fn installer_claim_is_atomic_and_exclusive() {
        let mut state = ready_state();
        let (update, path) = claim_install(&mut state).expect("first claim");
        assert_eq!(update.version, "99.0.0");
        assert_eq!(path, PathBuf::from("/tmp/ScreenerBot.pkg"));
        assert_eq!(state.phase, UpdatePhase::Applying);
        assert!(matches!(
            claim_install(&mut state),
            Err(Error::RestartInProgress)
        ));
    }

    #[test]
    fn installer_claim_rejects_mismatched_staging_metadata() {
        let mut state = ready_state();
        state.download_progress.checksum = Some("b".repeat(64));
        assert!(matches!(
            claim_install(&mut state),
            Err(Error::DigestMismatch { .. })
        ));
        assert_eq!(state.phase, UpdatePhase::ReadyToInstall);
    }
}
