//! Staging and bookkeeping for silently replaceable core binaries.
//!
//! # Why the core lives outside the application bundle
//!
//! Rewriting a file inside an installed `.app`, `Program Files` directory or
//! `/opt` tree needs elevation, breaks the bundle's code-signature seal and, on
//! macOS 14+, trips App Management protection. So a core update is never written
//! back into the installation. It is staged under the data directory:
//!
//! ```text
//! <data>/core/current.json      the pointer the desktop shell reads
//! <data>/core/<version>/screenerbot
//! ```
//!
//! `electron/src/core_resolver.js` reads the pointer before every backend spawn,
//! re-verifies the recorded digest and launches that binary instead of the one
//! shipped in the bundle — unless the bundle is newer, in which case the staged
//! tree is pruned. Nothing is ever activated in-place while it runs, so a failed
//! update can only leave the previous binary in charge.

use super::types::*;
use super::{core_binary_name, Error, Result};
use crate::logger::{self, LogTag};
use std::path::{Path, PathBuf};

/// Directory holding every staged core plus the pointer.
pub(super) fn core_dir() -> PathBuf {
    crate::paths::get_data_directory().join("core")
}

fn pointer_path() -> PathBuf {
    core_dir().join("current.json")
}

/// The staged core the desktop shell would launch, if any.
pub fn read_staged_core() -> Option<StagedCore> {
    let bytes = std::fs::read(pointer_path()).ok()?;
    let staged: StagedCore = serde_json::from_slice(&bytes).ok()?;
    if !is_safe_version(&staged.version) || !is_safe_relative_path(&staged.path) {
        return None;
    }
    core_dir().join(&staged.path).is_file().then_some(staged)
}

/// Version of the staged core, without touching its bytes.
pub fn staged_core_version() -> Option<String> {
    read_staged_core().map(|staged| staged.version)
}

/// Decompress, verify and publish a downloaded core artifact.
///
/// The artifact's digest proves authenticity; the digest recorded in the pointer
/// is taken from the file that actually landed on disk, and is what the shell
/// re-checks before each launch as a local integrity guard.
pub(super) async fn stage_core(
    version: &str,
    core: &CoreArtifact,
    archive: &Path,
) -> Result<StagedCore> {
    let version = version.to_owned();
    let core = core.clone();
    let archive = archive.to_owned();

    tokio::task::spawn_blocking(move || stage_core_blocking(&version, &core, &archive))
        .await
        .map_err(|error| Error::Internal(crate::errors::InternalError::from(error)))?
}

fn stage_core_blocking(version: &str, core: &CoreArtifact, archive: &Path) -> Result<StagedCore> {
    if !is_safe_version(version) {
        return Err(Error::Data(crate::errors::DataError::ValidationError {
            field: "update.version".to_owned(),
            value: version.to_owned(),
            reason: "must be a plain semantic version".to_owned(),
        }));
    }

    let root = core_dir();
    std::fs::create_dir_all(&root).map_err(io)?;
    restrict_permissions(&root)?;

    let staging = root.join(format!(".staging-{version}"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(io)?;
    }
    std::fs::create_dir_all(&staging).map_err(io)?;

    let binary_path = staging.join(core_binary_name());
    let written = decompress_gzip(archive, &binary_path, core.binary_size)?;
    if written != core.binary_size {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::DownloadSizeMismatch {
            expected: core.binary_size,
            actual: written,
        });
    }

    let digest = super::download::calculate_sha256(&binary_path)?;
    if digest != core.binary_sha256 {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::DigestMismatch {
            detail: format!(
                "core binary checksum expected {}, got {digest}",
                core.binary_sha256
            ),
        });
    }
    make_executable(&binary_path)?;

    let destination = root.join(version);
    if destination.exists() {
        std::fs::remove_dir_all(&destination).map_err(io)?;
    }
    std::fs::rename(&staging, &destination).map_err(io)?;

    let staged = StagedCore {
        version: version.to_owned(),
        path: format!("{version}/{}", core_binary_name()),
        sha256: digest,
        size: core.binary_size,
        staged_at: chrono::Utc::now(),
    };
    write_pointer(&staged)?;
    prune_other_versions(&root, version);
    Ok(staged)
}

/// Publish the pointer atomically so the shell never reads a half-written record.
fn write_pointer(staged: &StagedCore) -> Result<()> {
    let path = pointer_path();
    let temporary = path.with_extension("json.part");
    let bytes = serde_json::to_vec_pretty(staged).map_err(|error| {
        Error::Data(crate::errors::DataError::ParseError {
            data_type: "staged core pointer".to_owned(),
            error: error.to_string(),
        })
    })?;
    std::fs::write(&temporary, bytes).map_err(io)?;
    std::fs::rename(&temporary, &path).map_err(io)?;
    Ok(())
}

/// Remove staged cores other than the one the pointer names. Only entries whose
/// name parses as a version are touched, so nothing outside the updater's own
/// layout can be deleted even if the directory is shared.
pub(super) fn prune_other_versions(root: &Path, keep: &str) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let stale_staging = name.starts_with(".staging-");
        if name == keep || (!is_safe_version(&name) && !stale_staging) {
            continue;
        }
        if entry.path().is_dir() {
            if let Err(error) = std::fs::remove_dir_all(entry.path()) {
                logger::debug(
                    LogTag::System,
                    &format!("Could not remove stale staged core {name}: {error}"),
                );
            }
        }
    }
}

/// Stream a single-member gzip stream to `destination`, refusing to write more
/// than the authenticated size so a decompression bomb cannot fill the disk.
fn decompress_gzip(archive: &Path, destination: &Path, expected_size: u64) -> Result<u64> {
    use std::io::{Read, Write};

    let source = std::fs::File::open(archive).map_err(io)?;
    let mut decoder = flate2::read::GzDecoder::new(std::io::BufReader::new(source));
    let mut file = std::fs::File::create(destination).map_err(io)?;
    let mut buffer = vec![0_u8; 256 * 1024];
    let mut written = 0_u64;

    loop {
        let read = decoder.read(&mut buffer).map_err(io)?;
        if read == 0 {
            break;
        }
        written += read as u64;
        if written > expected_size {
            return Err(Error::DownloadSizeMismatch {
                expected: expected_size,
                actual: written,
            });
        }
        file.write_all(&buffer[..read]).map_err(io)?;
    }
    file.sync_all().map_err(io)?;
    Ok(written)
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn restrict_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn io(error: std::io::Error) -> Error {
    Error::Io(crate::errors::IoError::from(error))
}

/// A version is only ever used as a single path component, so it must contain
/// nothing that could climb out of the core directory.
fn is_safe_version(value: &str) -> bool {
    semver::Version::parse(value).is_ok()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte))
}

fn is_safe_relative_path(value: &str) -> bool {
    let mut parts = value.split('/');
    let (Some(version), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    is_safe_version(version) && name == core_binary_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_versions_become_directory_names() {
        assert!(is_safe_version("0.2.2"));
        assert!(is_safe_version("1.0.0-rc.1"));
        assert!(!is_safe_version("../etc"));
        assert!(!is_safe_version("0.2"));
        assert!(!is_safe_version(""));
    }

    #[test]
    fn pointer_paths_are_bound_to_the_core_layout() {
        assert!(is_safe_relative_path(&format!(
            "0.2.2/{}",
            core_binary_name()
        )));
        assert!(!is_safe_relative_path(&format!(
            "../../{}",
            core_binary_name()
        )));
        assert!(!is_safe_relative_path("0.2.2/other"));
        assert!(!is_safe_relative_path(&format!(
            "0.2.2/nested/{}",
            core_binary_name()
        )));
    }

    #[test]
    fn decompression_refuses_to_exceed_the_authenticated_size() {
        use std::io::Write;
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("core.gz");
        let payload = vec![7_u8; 4096];

        let file = std::fs::File::create(&archive).unwrap();
        let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
        encoder.write_all(&payload).unwrap();
        encoder.finish().unwrap();

        let good = directory.path().join("good");
        assert_eq!(decompress_gzip(&archive, &good, 4096).unwrap(), 4096);
        assert_eq!(std::fs::read(&good).unwrap(), payload);

        let capped = directory.path().join("capped");
        assert!(decompress_gzip(&archive, &capped, 1024).is_err());
    }

    #[test]
    fn pruning_keeps_the_pointer_version_and_ignores_foreign_entries() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        for name in ["0.2.1", "0.2.2", ".staging-0.2.3", "notes"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        std::fs::write(root.join("current.json"), b"{}").unwrap();

        prune_other_versions(root, "0.2.2");

        assert!(root.join("0.2.2").exists());
        assert!(!root.join("0.2.1").exists());
        assert!(!root.join(".staging-0.2.3").exists());
        assert!(root.join("notes").exists());
        assert!(root.join("current.json").exists());
    }
}
