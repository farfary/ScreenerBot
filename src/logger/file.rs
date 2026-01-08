//! File logging implementation with rotation and cleanup
//!
//! Handles writing logs to disk with:
//! - Daily rotation with timestamp-based filenames
//! - Automatic cleanup of old logs (24h retention)
//! - Buffered I/O for performance
//! - Thread-safe concurrent writes

use chrono::Local;
use once_cell::sync::Lazy;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// File logging configuration
const ENABLE_FILE_LOGGING: bool = true;
const LOG_RETENTION_HOURS: u64 = 24; // Keep logs for 24 hours
const MAX_LOG_FILES: usize = 7; // Keep maximum 7 days of logs as backup

/// Buffer configuration for high-performance logging
const FLUSH_INTERVAL_WRITES: u64 = 1; // Flush every write for debugging
const CLEANUP_INTERVAL_WRITES: u64 = 1000; // Cleanup every 1000 writes
const FILE_BUFFER_SIZE: usize = 4 * 1024; // 4KB buffer

/// File logger state for thread-safe file operations
struct FileLogger {
    file_writer: Option<BufWriter<File>>,
    current_date: String,
    log_dir: PathBuf,
    write_counter: u64,
}

impl FileLogger {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let log_dir = get_log_directory()?;
        fs::create_dir_all(&log_dir)?;

        // Create unique log file for each application start
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
        let log_file_name = format!("screenerbot_{}.log", timestamp);
        let log_file_path = log_dir.join(&log_file_name);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;

        let file_writer = Some(BufWriter::with_capacity(FILE_BUFFER_SIZE, file));

        // Create/update latest.log symlink for easy access to current log
        let latest_link = log_dir.join("latest.log");
        // Remove existing symlink if it exists (ignore errors)
        let _ = fs::remove_file(&latest_link);
        // Create new symlink pointing to current log file
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&log_file_path, &latest_link);
        }
        #[cfg(windows)]
        {
            // On Windows, use hard link or copy as fallback
            let _ = fs::hard_link(&log_file_path, &latest_link)
                .or_else(|_| fs::copy(&log_file_path, &latest_link).map(|_| ()));
        }

        Ok(FileLogger {
            file_writer,
            current_date: now.format("%Y-%m-%d").to_string(),
            log_dir,
            write_counter: 0,
        })
    }

    fn write_to_file(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut writer) = self.file_writer {
            writeln!(writer, "{}", message)?;

            self.write_counter += 1;

            // Only flush periodically for performance
            if self.write_counter % FLUSH_INTERVAL_WRITES == 0 {
                writer.flush()?;
            }

            // Cleanup less frequently to avoid I/O blocking
            if self.write_counter % CLEANUP_INTERVAL_WRITES == 0 {
                let log_dir = self.log_dir.clone();
                tokio::spawn(async move {
                    let _ = Self::cleanup_old_logs_static(&log_dir).await;
                });
            }
        }

        Ok(())
    }

    async fn cleanup_old_logs_static(
        log_dir: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match tokio::task::spawn_blocking({
            let log_dir = log_dir.to_path_buf();
            move || Self::cleanup_old_logs_blocking(&log_dir)
        })
        .await
        {
            Ok(result) => result.map_err(|e| format!("Cleanup error: {}", e).into()),
            Err(e) => Err(format!("Cleanup task failed: {}", e).into()),
        }
    }

    fn cleanup_old_logs_blocking(log_dir: &std::path::Path) -> Result<(), String> {
        let now = Local::now();
        let cutoff_time = now - chrono::Duration::hours(LOG_RETENTION_HOURS as i64);

        if let Ok(entries) = fs::read_dir(log_dir) {
            let mut log_files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    let file_name = entry.file_name();
                    let filename = file_name.to_string_lossy();
                    filename.starts_with("screenerbot_") && filename.ends_with(".log")
                })
                .collect();

            log_files.sort_by_key(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });

            // Remove files older than retention period
            for entry in &log_files {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_chrono = chrono::DateTime::<Local>::from(modified);
                        if modified_chrono < cutoff_time {
                            let _ = fs::remove_file(entry.path());
                        }
                    }
                }
            }

            // Enforce max file count limit
            let remaining_files: Vec<_> = log_files
                .iter()
                .filter(|entry| entry.path().exists())
                .collect();

            if remaining_files.len() > MAX_LOG_FILES {
                let files_to_remove = remaining_files.len() - MAX_LOG_FILES;
                for entry in remaining_files.iter().take(files_to_remove) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }

        Ok(())
    }
}

/// Global file logger instance
static FILE_LOGGER: Lazy<Arc<Mutex<Option<FileLogger>>>> = Lazy::new(|| {
    if ENABLE_FILE_LOGGING {
        match FileLogger::new() {
            Ok(logger) => Arc::new(Mutex::new(Some(logger))),
            Err(e) => {
                eprintln!("Failed to initialize file logger: {}", e);
                Arc::new(Mutex::new(None))
            }
        }
    } else {
        Arc::new(Mutex::new(None))
    }
});

/// Get the log directory path
fn get_log_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Primary: Use centralized paths module (works for both terminal and bundle)
    let log_dir = crate::paths::get_logs_directory();

    if log_dir.exists() || fs::create_dir_all(&log_dir).is_ok() {
        return Ok(log_dir);
    }

    // Final fallback to temp directory only if paths module fails
    let temp_log_dir = std::env::temp_dir().join("screenerbot_logs");
    fs::create_dir_all(&temp_log_dir)?;
    Ok(temp_log_dir)
}

/// Initialize the file logging system
pub fn init_file_logging() {
    if ENABLE_FILE_LOGGING {
        Lazy::force(&FILE_LOGGER);
    }
}

/// Force flush all pending log writes (call during shutdown)
pub fn flush_file_logging() {
    if !ENABLE_FILE_LOGGING {
        return;
    }

    if let Ok(mut logger_guard) = FILE_LOGGER.lock() {
        if let Some(ref mut logger) = logger_guard.as_mut() {
            if let Some(ref mut writer) = logger.file_writer {
                let _ = writer.flush();
            }
        }
    }
}

/// Write message to log file (stripped of color codes)
pub fn write_to_file(message: &str) {
    if !ENABLE_FILE_LOGGING {
        return;
    }

    match FILE_LOGGER.try_lock() {
        Ok(mut logger_guard) => {
            if let Some(ref mut logger) = logger_guard.as_mut() {
                let clean_message = strip_ansi_codes(message);
                if let Err(_) = logger.write_to_file(&clean_message) {
                    static ERROR_COUNTER: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    let count = ERROR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count % 1000 == 0 {
                        eprintln!(
                            "File logging errors (shown every 1000): count = {}",
                            count + 1
                        );
                    }
                }
            }
        }
        Err(_) => {
            static DROP_COUNTER: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(0);
            let count = DROP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count % 1000 == 0 && count > 0 {
                eprintln!("Dropped {} log messages due to busy file logger", count + 1);
            }
        }
    }
}

/// Remove ANSI color codes from text
fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;

    for ch in text.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape && ch == 'm' {
            in_escape = false;
        } else if !in_escape {
            result.push(ch);
        }
    }
    result
}
