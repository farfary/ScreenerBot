// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! File logging: buffered, thread-safe writes to one file per application start,
//! rotated into a fresh file whenever it reaches the size cap storage housekeeping
//! declares, with `latest.log` always naming the active file. Pruning old files
//! belongs to storage housekeeping.

use chrono::Local;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

use crate::paths::{LATEST_LOG_NAME, LOG_FILE_PREFIX};
use crate::storage::LOG_FILE_MAX_BYTES;

/// File logging configuration
const ENABLE_FILE_LOGGING: bool = true;

/// Buffer configuration for high-performance logging
const FLUSH_INTERVAL_WRITES: u64 = 10; // Flush every 10 writes for performance
const FILE_BUFFER_SIZE: usize = 4 * 1024; // 4KB buffer

/// File logger state for thread-safe file operations
struct FileLogger {
    file_writer: Option<BufWriter<File>>,
    log_dir: PathBuf,
    active_path: PathBuf,
    active_bytes: u64,
    write_counter: u64,
}

impl FileLogger {
    fn new() -> std::io::Result<Self> {
        let log_dir = get_log_directory()?;
        let (file_writer, active_path) = open_log_file(&log_dir)?;
        Ok(FileLogger {
            file_writer: Some(file_writer),
            log_dir,
            active_path,
            active_bytes: 0,
            write_counter: 0,
        })
    }

    fn write_to_file(&mut self, message: &str) -> std::io::Result<()> {
        if let Some(ref mut writer) = self.file_writer {
            writeln!(writer, "{}", message)?;

            self.write_counter += 1;
            self.active_bytes += message.len() as u64 + 1;

            // Only flush periodically for performance
            if self.write_counter.is_multiple_of(FLUSH_INTERVAL_WRITES) {
                writer.flush()?;
            }
        }
        if self.active_bytes >= LOG_FILE_MAX_BYTES {
            self.rotate()?;
        }

        Ok(())
    }

    /// Finish the active file and continue in a new one.
    fn rotate(&mut self) -> std::io::Result<()> {
        if let Some(ref mut writer) = self.file_writer {
            writer.flush()?;
        }
        let (file_writer, active_path) = open_log_file(&self.log_dir)?;
        self.file_writer = Some(file_writer);
        self.active_path = active_path;
        self.active_bytes = 0;
        Ok(())
    }
}

/// Create a new log file named after the current time, suffixed when a file of that
/// name already exists, and point `latest.log` at it.
fn open_log_file(log_dir: &Path) -> std::io::Result<(BufWriter<File>, PathBuf)> {
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let mut log_file_path = log_dir.join(format!("{LOG_FILE_PREFIX}{timestamp}.log"));
    let mut suffix = 1u32;
    while log_file_path.exists() {
        log_file_path = log_dir.join(format!("{LOG_FILE_PREFIX}{timestamp}_{suffix}.log"));
        suffix += 1;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;

    let latest_link = log_dir.join(LATEST_LOG_NAME);
    let _ = fs::remove_file(&latest_link);
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

    Ok((
        BufWriter::with_capacity(FILE_BUFFER_SIZE, file),
        log_file_path,
    ))
}

/// Global file logger instance
static FILE_LOGGER: LazyLock<Arc<Mutex<Option<FileLogger>>>> = LazyLock::new(|| {
    if ENABLE_FILE_LOGGING {
        match FileLogger::new() {
            Ok(logger) => Arc::new(Mutex::new(Some(logger))),
            Err(e) => {
                eprintln!("Failed to initialize file logger: {e}");
                Arc::new(Mutex::new(None))
            }
        }
    } else {
        Arc::new(Mutex::new(None))
    }
});

/// Get the log directory path
fn get_log_directory() -> std::io::Result<PathBuf> {
    // Primary: Use centralized paths module (works for both terminal and bundle)
    let log_dir = crate::paths::get_logs_directory();

    if log_dir.exists() || fs::create_dir_all(&log_dir).is_ok() {
        return Ok(log_dir);
    }

    let fallback = crate::paths::get_log_fallback_directory();
    fs::create_dir_all(&fallback)?;
    Ok(fallback)
}

/// The file the logger is writing to now, which housekeeping never deletes.
pub fn active_log_file() -> Option<PathBuf> {
    FILE_LOGGER
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|logger| logger.active_path.clone()))
}

/// Initialize the file logging system
pub fn init_file_logging() {
    if ENABLE_FILE_LOGGING {
        LazyLock::force(&FILE_LOGGER);
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
                if logger.write_to_file(&clean_message).is_err() {
                    static ERROR_COUNTER: std::sync::atomic::AtomicU64 =
                        std::sync::atomic::AtomicU64::new(0);
                    let count = ERROR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count.is_multiple_of(1000) {
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
            if count.is_multiple_of(1000) && count > 0 {
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
