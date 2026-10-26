// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Crash-safe whole-file writes: replace a file atomically, create one that must not exist, or remove one durably.
//!
//! Both writers stage the bytes in a hidden sibling temp file (same directory, so the final
//! step stays within one filesystem), created exclusively with the requested mode before its
//! first byte is written and fsynced before it becomes visible. A reader therefore sees the
//! old file, no file, or the complete new file, never a truncated one, and a secret-bearing
//! file never exists at the process umask.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Upper bound on the temp-name tokens tried before giving up. With a process-monotonic
/// counter one attempt is enough in practice; the slack absorbs stale leftover temps.
const MAX_TEMP_ATTEMPTS: u32 = 16;

/// Process-monotonic token that makes every in-process temp path distinct, so two
/// concurrent writers never select the same sibling temp.
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Replace `path` with `bytes`, created with `mode` (Unix permission bits; ignored
/// elsewhere). The final rename replaces the destination atomically: `rename(2)` on Unix,
/// `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` on Windows. On a failure the temp is
/// removed and `path` is untouched.
pub fn replace(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let temporary = write_temporary(path, bytes, mode)?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    sync_parent(path);
    Ok(())
}

/// Create `path` with `bytes` and `mode`, failing with `AlreadyExists` when `path` exists.
/// The temp is hard-linked onto `path`, which never replaces an existing entry on any
/// platform (a rename would), and then unlinked. On a failure the temp is removed and an
/// existing `path` is untouched.
pub fn write_new(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    let temporary = write_temporary(path, bytes, mode)?;
    let linked = std::fs::hard_link(&temporary, path);
    let _ = std::fs::remove_file(&temporary);
    linked?;
    sync_parent(path);
    Ok(())
}

/// Remove `path` and persist the directory entry, so a removal that returned is not
/// undone by a crash. A file that is already gone is not an error.
pub fn remove(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    sync_parent(path);
    Ok(())
}

/// Write `bytes` to a fresh exclusive sibling temp of `destination` with `mode`, fsynced.
fn write_temporary(destination: &Path, bytes: &[u8], mode: u32) -> io::Result<PathBuf> {
    let (temporary, mut file) = create_temporary(destination, mode)?;
    let written = (|| -> io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        set_mode(&temporary, mode)
    })();
    drop(file);
    match written {
        Ok(()) => Ok(temporary),
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            Err(error)
        }
    }
}

/// Open a new sibling temp with `O_EXCL` and the restrictive mode already applied, so an
/// existing candidate (a concurrent writer's temp or a stale leftover) is never opened or
/// truncated; the next token is tried instead.
fn create_temporary(destination: &Path, mode: u32) -> io::Result<(PathBuf, File)> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;

    let mut last_error = None;
    for _ in 0..MAX_TEMP_ATTEMPTS {
        let candidate = sibling_temp_path(destination, TEMP_SEQ.fetch_add(1, Ordering::Relaxed));
        match options.open(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| io::Error::from(io::ErrorKind::AlreadyExists)))
}

/// `OpenOptionsExt::mode` is subject to the umask; re-assert the exact mode so the file
/// carries what the caller asked for, no narrower and no wider.
#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

/// A hidden temp path beside `destination`, unique per process and per attempt.
fn sibling_temp_path(destination: &Path, seq: u64) -> PathBuf {
    let name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temporary = format!(".{name}.{}.{seq}.tmp", std::process::id());
    match destination.parent() {
        Some(parent) => parent.join(temporary),
        None => PathBuf::from(temporary),
    }
}

/// Persist the directory entry of a rename or link where the platform supports it.
fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
#[path = "atomic_file_tests.rs"]
mod tests;
