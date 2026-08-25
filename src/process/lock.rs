//! Process Lock Module
//!
//! Prevents multiple instances of ScreenerBot from running simultaneously using file-based locking.
//!
//! **Implementation:**
//! - Uses fslock for advisory file locking (cross-platform)
//! - Lock file: `data/.screenerbot.lock`
//! - RAII pattern: Lock held for entire bot lifetime, automatically released on drop
//! - OS automatically releases lock if process crashes (no stale locks)
//!
//! **Usage:**
//! ```rust
//! let _lock = ProcessLock::acquire()?;
//! // Lock held until _lock is dropped (end of scope)
//! ```

use crate::errors::{InternalError, IoError};
use crate::logger::{self, LogTag};
use crate::{Error, Result};
use fslock::LockFile;
use std::path::PathBuf;

/// Process lock guard - holds file lock for bot lifetime
///
/// The lock is automatically released when this struct is dropped (RAII pattern).
/// If the process crashes, the OS automatically releases the lock.
pub struct ProcessLock {
    _lock: LockFile,
    lock_path: PathBuf,
}

impl ProcessLock {
    /// Acquire the process lock
    ///
    /// Returns error if another instance is already running or if lock file cannot be created.
    ///
    /// **Lock file location:** `data/.screenerbot.lock`
    ///
    /// **Error cases:**
    /// - Another instance is running (lock is held)
    /// - Cannot create lock file (permission/path issues)
    pub fn acquire() -> Result<Self> {
        let lock_path = crate::paths::get_process_lock_path();

        logger::info(
            LogTag::System,
            &format!("Acquiring process lock: {:?}", lock_path),
        );

        // Open lock file (directory creation handled by paths module)
        let mut lock = LockFile::open(&lock_path).map_err(|e| {
            Error::Io(IoError::Generic {
                message: format!(
                    "failed to open process lock file {}: {e}",
                    lock_path.display()
                ),
            })
        })?;

        // Try to acquire exclusive lock (non-blocking)
        if !lock.try_lock().map_err(|e| {
            Error::Io(IoError::Generic {
                message: format!(
                    "failed to acquire process lock {}: {e}",
                    lock_path.display()
                ),
            })
        })? {
            return Err(Error::Internal(InternalError::InvariantViolation {
                message: format!(
                    "another ScreenerBot instance holds process lock {}",
                    lock_path.display()
                ),
            }));
        }

        logger::info(
            LogTag::System,
            &format!("Process lock acquired: {:?}", lock_path),
        );

        Ok(Self {
            _lock: lock,
            lock_path,
        })
    }

    /// Get the path to the lock file
    pub fn lock_path(&self) -> &PathBuf {
        &self.lock_path
    }
}

impl Drop for ProcessLock {
    fn drop(&mut self) {
        logger::info(
            LogTag::System,
            &format!("Releasing process lock: {:?}", self.lock_path),
        );
        // Lock is automatically released when _lock is dropped
        // fslock handles the file unlocking
    }
}
