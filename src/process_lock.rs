/// Process Lock Module
///
/// Prevents multiple instances of ScreenerBot from running simultaneously using file-based locking.
///
/// **Implementation:**
/// - Uses fslock for advisory file locking (cross-platform)
/// - Lock file: `data/.screenerbot.lock`
/// - RAII pattern: Lock held for entire bot lifetime, automatically released on drop
/// - OS automatically releases lock if process crashes (no stale locks)
///
/// **Usage:**
/// ```rust
/// let _lock = ProcessLock::acquire()?;
/// // Lock held until _lock is dropped (end of scope)
/// ```
use crate::logger::{log, LogTag};
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
    pub fn acquire() -> Result<Self, String> {
        let lock_path = PathBuf::from("data/.screenerbot.lock");

        log(
            LogTag::System,
            "INFO",
            &format!("🔒 Acquiring process lock: {:?}", lock_path),
        );

        // Create parent directory if it doesn't exist
        if let Some(parent) = lock_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("Failed to create lock file directory: {}", e)
                })?;
            }
        }

        // Open lock file
        let mut lock = LockFile::open(&lock_path).map_err(|e| {
            format!(
                "Failed to open lock file {:?}: {}\n\
                 Hint: Check directory permissions for 'data/' folder",
                lock_path, e
            )
        })?;

        // Try to acquire exclusive lock (non-blocking)
        if !lock.try_lock().map_err(|e| {
            format!(
                "Failed to acquire lock on {:?}: {}",
                lock_path, e
            )
        })? {
            return Err(format!(
                "❌ Another instance of ScreenerBot is already running.\n\
                 \n\
                 The process lock file is held by another process:\n\
                   Lock file: {:?}\n\
                 \n\
                 To stop the running instance:\n\
                   1. Find process: ps aux | grep screenerbot | grep -v grep\n\
                   2. Stop process: pkill -f screenerbot\n\
                   3. Verify stopped: ps aux | grep screenerbot | grep -v grep\n\
                 \n\
                 If no process is found but lock persists, it may be stale.\n\
                 In that case, manually remove: rm {:?}",
                lock_path, lock_path
            ));
        }

        log(
            LogTag::System,
            "SUCCESS",
            &format!("✅ Process lock acquired: {:?}", lock_path),
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
        log(
            LogTag::System,
            "INFO",
            &format!("🔓 Releasing process lock: {:?}", self.lock_path),
        );
        // Lock is automatically released when _lock is dropped
        // fslock handles the file unlocking
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_lock_prevents_duplicate() {
        // First lock should succeed
        let lock1 = ProcessLock::acquire();
        assert!(lock1.is_ok(), "First lock should succeed");

        // Second lock should fail
        let lock2 = ProcessLock::acquire();
        assert!(lock2.is_err(), "Second lock should fail");
        assert!(
            lock2.unwrap_err().contains("already running"),
            "Error should mention another instance"
        );

        // Drop first lock
        drop(lock1);

        // Now third lock should succeed
        let lock3 = ProcessLock::acquire();
        assert!(lock3.is_ok(), "Lock should succeed after first is dropped");
    }
}
