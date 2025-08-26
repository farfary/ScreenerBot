/// ScreenerBot Logger with 24-hour File Persistence
///
/// This logger provides dual output: console with colors and file persistence without colors.
///
/// ## Features:
/// - **Console Logging**: Colored output with emoji tags and professional formatting
/// - **File Logging**: Clean text logs stored in `logs/screenerbot_YYYY-MM-DD.log`
/// - **24-hour Retention**: Log files are automatically rotated daily and kept for 24 hours
/// - **Automatic Cleanup**: Old log files are removed after retention period (configurable)
/// - **Thread-Safe**: Concurrent logging from multiple async tasks
/// - **Graceful Fallback**: If file logging fails, console logging continues
///
/// ## Usage:
/// ```rust
/// use screenerbot::logger::{log, LogTag, init_file_logging};
///
/// // Initialize file logging (call once at startup)
/// init_file_logging();
///
/// // Log messages with various tags and types
/// log(LogTag::System, "INFO", "Application started");
/// log(LogTag::Trader, "BUY", "Bought 1000 tokens");
/// log(LogTag::Wallet, "BALANCE", "Current balance: 10.5 SOL");
/// ```
///
/// ## Configuration:
/// - `ENABLE_FILE_LOGGING`: Enable/disable file logging (default: true)
/// - `LOG_RETENTION_HOURS`: How long to keep log files (default: 24 hours)
/// - `MAX_LOG_FILES`: Maximum number of log files to keep (default: 7)
/// - `FLUSH_INTERVAL_WRITES`: Flush buffer every N writes (default: 100 for performance)
/// - `CLEANUP_INTERVAL_WRITES`: Run cleanup every N writes (default: 1000 for performance)
/// - `FILE_BUFFER_SIZE`: Buffer size for file I/O (default: 64KB for better performance)
///
/// ## High-Volume Logging Safety:
/// - **Non-blocking writes**: Uses `try_lock()` to avoid blocking when logger is busy
/// - **Buffered I/O**: 64KB buffer with periodic flushing (every 100 writes) instead of per-message
/// - **Async cleanup**: Background cleanup to avoid blocking write operations
/// - **Drop protection**: Messages are dropped rather than blocking during high-volume periods
/// - **Error throttling**: File write errors are reported every 1000 occurrences to prevent spam

/// Set to false to hide date in logs
const LOG_SHOW_DATE: bool = false;
/// Set to false to hide time in logs
const LOG_SHOW_TIME: bool = false;

/// File logging configuration
const ENABLE_FILE_LOGGING: bool = true;
const LOG_RETENTION_HOURS: u64 = 24; // Keep logs for 24 hours
const MAX_LOG_FILES: usize = 7; // Keep maximum 7 days of logs as backup

/// Buffer configuration for high-performance logging
const FLUSH_INTERVAL_WRITES: u64 = 100; // Flush every 100 writes instead of every write
const CLEANUP_INTERVAL_WRITES: u64 = 1000; // Cleanup every 1000 writes instead of 500
const FILE_BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer for better I/O performance

/// Log format character widths (hardcoded for precise alignment)
const TAG_WIDTH: usize = 10; // "[SYSTEM  ]" = 10 chars (8 + 2 brackets)
const LOG_TYPE_WIDTH: usize = 30; // "[UPDATE  ]" = 10 chars (8 + 2 brackets)
const BRACKET_SPACE_WIDTH: usize = 3; // " [" + "] " = 3 chars between each component
const TOTAL_PREFIX_WIDTH: usize = TAG_WIDTH + LOG_TYPE_WIDTH + BRACKET_SPACE_WIDTH * 2; // +1 for final space

/// Maximum line length before wrapping
const MAX_LINE_LENGTH: usize = 175;

use chrono::Local;
use colored::*;
use std::fs::{ self, File, OpenOptions };
use std::io::{ Write, BufWriter };
use std::path::PathBuf;
use std::sync::{ Arc, Mutex };
use once_cell::sync::Lazy;
use crate::arguments::is_dashboard_enabled;

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

        let file = OpenOptions::new().create(true).append(true).open(&log_file_path)?;

        // Use larger buffer for better performance with high-volume logging
        let file_writer = Some(BufWriter::with_capacity(FILE_BUFFER_SIZE, file));

        Ok(FileLogger {
            file_writer,
            current_date: now.format("%Y-%m-%d").to_string(),
            log_dir,
            write_counter: 0,
        })
    }

    fn write_to_file(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        // PERFORMANCE: Optimized for high-volume logging

        if let Some(ref mut writer) = self.file_writer {
            writeln!(writer, "{}", message)?;

            self.write_counter += 1;

            // OPTIMIZATION: Only flush periodically, not on every write
            if self.write_counter % FLUSH_INTERVAL_WRITES == 0 {
                writer.flush()?;
            }

            // OPTIMIZATION: Cleanup less frequently to avoid I/O blocking
            if self.write_counter % CLEANUP_INTERVAL_WRITES == 0 {
                // Spawn cleanup in background to avoid blocking current write
                let log_dir = self.log_dir.clone();
                tokio::spawn(async move {
                    let _ = Self::cleanup_old_logs_static(&log_dir).await;
                });
            }
        }

        Ok(())
    }

    fn cleanup_old_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Use blocking version for sync cleanup
        Self::cleanup_old_logs_blocking(&self.log_dir).map_err(|e| e.into())
    }

    // Static cleanup method that can be called from async context
    async fn cleanup_old_logs_static(
        log_dir: &std::path::Path
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Call blocking version in async context
        match
            tokio::task::spawn_blocking({
                let log_dir = log_dir.to_path_buf();
                move || Self::cleanup_old_logs_blocking(&log_dir)
            }).await
        {
            Ok(result) => result.map_err(|e| format!("Cleanup error: {}", e).into()),
            Err(e) => Err(format!("Cleanup task failed: {}", e).into()),
        }
    }

    // Blocking cleanup implementation
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

            // Sort by modification time (oldest first)
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
                            if let Err(_) = fs::remove_file(entry.path()) {
                                // Silently ignore cleanup errors to avoid recursion
                                // (logging from cleanup could cause infinite loop)
                            }
                        }
                    }
                }
            }

            // Also enforce max file count limit
            let remaining_files: Vec<_> = log_files
                .iter()
                .filter(|entry| entry.path().exists())
                .collect();

            if remaining_files.len() > MAX_LOG_FILES {
                let files_to_remove = remaining_files.len() - MAX_LOG_FILES;
                for entry in remaining_files.iter().take(files_to_remove) {
                    if let Err(_) = fs::remove_file(entry.path()) {
                        // Silently ignore excess file removal errors
                    }
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
                // Can't use file logger yet; last resort stderr print
                if !crate::arguments::is_dashboard_enabled() {
                    eprintln!("Failed to initialize file logger: {}", e);
                }
                Arc::new(Mutex::new(None))
            }
        }
    } else {
        Arc::new(Mutex::new(None))
    }
});

/// Get the log directory path
fn get_log_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Try to create logs directory in the project root
    let current_dir = std::env::current_dir()?;
    let log_dir = current_dir.join("logs");

    // If that fails, try user data directory
    if log_dir.exists() || fs::create_dir_all(&log_dir).is_ok() {
        return Ok(log_dir);
    }

    // Fallback to system temp directory
    if let Some(data_dir) = dirs::data_dir() {
        let app_log_dir = data_dir.join("screenerbot").join("logs");
        fs::create_dir_all(&app_log_dir)?;
        return Ok(app_log_dir);
    }

    // Final fallback to temp directory
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
                let _ = writer.flush(); // Ensure all writes are flushed to disk
            }
        }
    }
}

/// Write message to log file (stripped of color codes) - PERFORMANCE OPTIMIZED
fn write_to_file(message: &str) {
    if !ENABLE_FILE_LOGGING {
        return;
    }

    // OPTIMIZATION: Use try_lock to avoid blocking if logger is busy
    match FILE_LOGGER.try_lock() {
        Ok(mut logger_guard) => {
            if let Some(ref mut logger) = logger_guard.as_mut() {
                let clean_message = strip_ansi_codes(message);
                if let Err(_) = logger.write_to_file(&clean_message) {
                    // SAFETY: Don't spam stderr with file write errors during high-volume logging
                    // Only print error once per 1000 failures to avoid log spam
                    static ERROR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(
                        0
                    );
                    let count = ERROR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count % 1000 == 0 {
                        eprintln!("File logging errors (shown every 1000): count = {}", count + 1);
                    }
                }
            }
        }
        Err(_) => {
            // PERFORMANCE: If lock is busy, drop the message rather than blocking
            // This prevents logging from becoming a bottleneck during high-volume periods
            static DROP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(
                0
            );
            let count = DROP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count % 1000 == 0 && count > 0 {
                eprintln!("Dropped {} log messages due to busy file logger", count + 1);
            }
        }
    }
}

/// Log tags for categorizing log messages.
#[derive(Debug)]
pub enum LogTag {
    Monitor,
    Trader,
    Wallet,
    System,
    Pool,
    Blacklist,
    Discovery,
    Filtering,
    Api,
    Rugcheck,
    Profit,
    PriceService,
    Rpc,
    Ohlcv,
    Decimals,
    Swap,
    Entry,
    RlLearn,
    Summary,
    Transactions,
    Positions,
    Test,
    Other(String),
}

impl std::fmt::Display for LogTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag_str = match self {
            LogTag::Monitor => format!("{:<8}", "MONITOR").bright_cyan().bold(), // 👁️ Watchful blue
            LogTag::Trader => format!("{:<8}", "TRADER").bright_blue().bold(), // 💰 Money green
            LogTag::Wallet => format!("{:<8}", "WALLET").bright_magenta().bold(), // 💜 Rich purple for wealth
            LogTag::System => format!("{:<8}", "SYSTEM").bright_yellow().bold(), // ⚙️ Mechanical yellow
            LogTag::Pool => format!("{:<8}", "POOL").bright_blue().bold(), // 🏊 Pool blue
            LogTag::Blacklist => format!("{:<8}", "BLACKLIST").bright_red().bold(), // 🚫 Warning red
            LogTag::Discovery => format!("{:<8}", "DISCOVER").bright_white().bold(), // 🔍 Search white
            LogTag::Filtering => format!("{:<8}", "FILTER").bright_yellow().bold(), // 🔄 Filter yellow
            LogTag::Api => format!("{:<8}", "API").bright_purple().bold(), // 🌐 API purple
            LogTag::Rugcheck => format!("{:<8}", "RUGCHECK").bright_red().bold(), // 🛡️ Security red
            LogTag::Profit => format!("{:<8}", "PROFIT").bright_purple().bold(), // 💲 Profit green
            LogTag::PriceService => format!("{:<8}", "PRICE").bright_green().bold(), // 💹 Price service green
            LogTag::Rpc => format!("{:<8}", "RPC").bright_cyan().bold(), // 🔗 RPC cyan
            LogTag::Ohlcv => format!("{:<8}", "OHLCV").bright_green().bold(), // 📈 OHLCV chart green
            LogTag::Decimals => format!("{:<8}", "DECIMALS").bright_white().bold(), // 🔢 Decimals white
            LogTag::Swap => format!("{:<8}", "SWAP").bright_magenta().bold(), // 🔄 Swap magenta
            LogTag::Entry => format!("{:<8}", "ENTRY").bright_yellow().bold(), // 🚪 Entry yellow
            LogTag::RlLearn => format!("{:<8}", "RL_LEARN").bright_cyan().bold(), // 🤖 AI cyan
            LogTag::Summary => format!("{:<8}", "SUMMARY").bright_white().bold(), // 📊 Summary white
            LogTag::Transactions => format!("{:<8}", "TX").bright_blue().bold(), // 📝 Transactions blue
            LogTag::Positions => format!("{:<8}", "Positions").bright_yellow().bold(), // 📊 Positions yellow
            LogTag::Test => format!("{:<8}", "TEST").bright_blue().bold(), // 🧪 Test blue
            LogTag::Other(s) => format!("{:<8}", s).white().bold(),
        };
        write!(f, "{}", tag_str)
    }
}

/// Logs a message with date, time, tag, log type, and message.
pub fn log(tag: LogTag, log_type: &str, message: &str) {
    let now = Local::now();
    let date = now.format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M:%S").to_string();
    let mut prefix = String::new();
    if LOG_SHOW_DATE && LOG_SHOW_TIME {
        prefix = format!("{} {} ", date, time);
    } else if LOG_SHOW_DATE {
        prefix = format!("{} ", date);
    } else if LOG_SHOW_TIME {
        prefix = format!("{} ", time);
    }
    let prefix = if !prefix.is_empty() { prefix.dimmed().to_string() } else { String::new() };

    // Fixed-width log tag
    let tag_str = match tag {
        LogTag::Monitor =>
            format!("{:<width$}", "MONITOR", width = TAG_WIDTH)
                .bright_cyan()
                .bold(),
        LogTag::Trader =>
            format!("{:<width$}", "TRADER", width = TAG_WIDTH)
                .bright_green()
                .bold(),
        LogTag::Wallet =>
            format!("{:<width$}", "WALLET", width = TAG_WIDTH)
                .bright_magenta()
                .bold(),
        LogTag::System =>
            format!("{:<width$}", "SYSTEM", width = TAG_WIDTH)
                .bright_yellow()
                .bold(),
        LogTag::Pool =>
            format!("{:<width$}", "POOL", width = TAG_WIDTH)
                .bright_blue()
                .bold(),
        LogTag::Blacklist =>
            format!("{:<width$}", "BLACKLIST", width = TAG_WIDTH)
                .bright_red()
                .bold(),
        LogTag::Discovery =>
            format!("{:<width$}", "DISCOVER", width = TAG_WIDTH)
                .bright_white()
                .bold(),
        LogTag::Filtering =>
            format!("{:<width$}", "FILTER", width = TAG_WIDTH)
                .bright_yellow()
                .bold(),
        LogTag::Api =>
            format!("{:<width$}", "API", width = TAG_WIDTH)
                .bright_purple()
                .bold(),
        LogTag::Rugcheck =>
            format!("{:<width$}", "RUGCHECK", width = TAG_WIDTH)
                .bright_red()
                .bold(),
        LogTag::Profit =>
            format!("{:<width$}", "PROFIT", width = TAG_WIDTH)
                .bright_green()
                .bold(),
        LogTag::PriceService =>
            format!("{:<width$}", "PRICE", width = TAG_WIDTH)
                .bright_green()
                .bold(),
        LogTag::Rpc =>
            format!("{:<width$}", "RPC", width = TAG_WIDTH)
                .bright_cyan()
                .bold(),
        LogTag::Ohlcv =>
            format!("{:<width$}", "OHLCV", width = TAG_WIDTH)
                .bright_green()
                .bold(),
        LogTag::Decimals =>
            format!("{:<width$}", "DECIMALS", width = TAG_WIDTH)
                .bright_white()
                .bold(),
        LogTag::Swap =>
            format!("{:<width$}", "SWAP", width = TAG_WIDTH)
                .bright_magenta()
                .bold(),
        LogTag::Entry =>
            format!("{:<width$}", "ENTRY", width = TAG_WIDTH)
                .bright_yellow()
                .bold(),
        LogTag::RlLearn =>
            format!("{:<width$}", "RL_LEARN", width = TAG_WIDTH)
                .bright_cyan()
                .bold(),
        LogTag::Summary =>
            format!("{:<width$}", "SUMMARY", width = TAG_WIDTH)
                .bright_white()
                .bold(),
        LogTag::Transactions =>
            format!("{:<width$}", "TX", width = TAG_WIDTH)
                .bright_blue()
                .bold(),
        LogTag::Positions =>
            format!("{:<width$}", "Positions", width = TAG_WIDTH)
                .bright_yellow()
                .bold(),
        LogTag::Test =>
            format!("{:<width$}", "TEST", width = TAG_WIDTH)
                .bright_blue()
                .bold(),
        LogTag::Other(ref s) =>
            format!("{:<width$}", s, width = TAG_WIDTH)
                .white()
                .bold(),
    };

    // Fixed-width log type
    let log_type_str = match log_type.to_uppercase().as_str() {
        "ERROR" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_red()
                .bold(),
        "FAILED" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_red()
                .bold(),
        "WARN" | "WARNING" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_yellow()
                .bold(),
        "SUCCESS" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_green()
                .bold(),
        "INFO" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_blue()
                .bold(),
        "DEBUG" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_black()
                .bold(),
        "PROFIT" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_green()
                .bold(),
        "LOSS" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_red()
                .bold(),
        "BUY" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_cyan()
                .bold(),
        "SELL" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_magenta()
                .bold(),
        "BALANCE" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_yellow()
                .bold(),
        "PRICE" =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .bright_blue()
                .bold(),
        _ =>
            format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
                .white()
                .bold(),
    };

    // Build the base log line with strict discipline
    let base_line = format!("{}[{}] [{}] ", prefix, tag_str, log_type_str);

    // Use hardcoded TOTAL_PREFIX_WIDTH for alignment
    let base_length = strip_ansi_codes(&base_line)
        .len()
        .max(TOTAL_PREFIX_WIDTH + prefix.len());
    let available_space = if MAX_LINE_LENGTH > base_length {
        MAX_LINE_LENGTH - base_length
    } else {
        50 // Minimum space for message
    };

    // Split message into chunks that fit
    let message_chunks = wrap_text(message, available_space);

    // Check if the original message already contains color codes
    let has_existing_colors = message.contains('\x1b');

    // Determine message color based on log type and message content
    let message_color = if has_existing_colors {
        // If message already has colors, use the first chunk as-is
        message_chunks[0].to_string()
    } else {
        // Apply coloring based on log type only if no existing colors
        match log_type.to_uppercase().as_str() {
            "ERROR" => message_chunks[0].bright_red().to_string(),
            "FAILED" => message_chunks[0].bright_red().to_string(),
            _ => {
                // Check if message contains error/failed keywords
                if
                    message.to_lowercase().contains("error") ||
                    message.to_lowercase().contains("failed") ||
                    message.to_lowercase().contains("fail")
                {
                    message_chunks[0].bright_red().to_string()
                } else {
                    message_chunks[0].bright_white().to_string()
                }
            }
        }
    };

    // Print first line with full prefix (console output)
    let console_line = format!("{}{}", base_line, message_color);
    if !is_dashboard_enabled() {
        println!("{}", console_line);
    }

    // Write to file (clean version without color codes)
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let tag_clean = match tag {
        LogTag::Monitor => "MONITOR",
        LogTag::Trader => "TRADER",
        LogTag::Wallet => "WALLET",
        LogTag::System => "SYSTEM",
        LogTag::Pool => "POOL",
        LogTag::Blacklist => "BLACKLIST",
        LogTag::Discovery => "DISCOVER",
        LogTag::Filtering => "FILTER",
        LogTag::Api => "API",
        LogTag::Rugcheck => "RUGCHECK",
        LogTag::Profit => "PROFIT",
        LogTag::PriceService => "PRICE",
        LogTag::Rpc => "RPC",
        LogTag::Ohlcv => "OHLCV",
        LogTag::Decimals => "DECIMALS",
        LogTag::Swap => "SWAP",
        LogTag::Entry => "ENTRY",
        LogTag::RlLearn => "RL_LEARN",
        LogTag::Summary => "SUMMARY",
        LogTag::Transactions => "TX",
        LogTag::Positions => "Positions",
        LogTag::Test => "TEST",
        LogTag::Other(ref s) => s,
    };
    let file_line = format!("{} [{}] [{}] {}", timestamp, tag_clean, log_type, message_chunks[0]);
    write_to_file(&file_line);

    // Send to dashboard if dashboard mode is active
    if is_dashboard_enabled() {
        crate::dashboard::dashboard_log(tag_clean, log_type, &message_chunks[0]);
    }

    // Print continuation lines with proper indentation (console)
    if message_chunks.len() > 1 {
        let continuation_prefix = format!(
            "{}{}",
            " ".repeat(strip_ansi_codes(&prefix).len()),
            " ".repeat(TOTAL_PREFIX_WIDTH)
        );
        for chunk in &message_chunks[1..] {
            // Apply same color logic to continuation lines
            let chunk_color = if has_existing_colors {
                // If original message had colors, use chunks as-is
                chunk.to_string()
            } else {
                // Apply coloring based on log type only if no existing colors
                match log_type.to_uppercase().as_str() {
                    "ERROR" => chunk.bright_red().to_string(),
                    "FAILED" => chunk.bright_red().to_string(),
                    _ => {
                        // Check if message contains error/failed keywords
                        if
                            message.to_lowercase().contains("error") ||
                            message.to_lowercase().contains("failed") ||
                            message.to_lowercase().contains("fail")
                        {
                            chunk.bright_red().to_string()
                        } else {
                            chunk.bright_white().to_string()
                        }
                    }
                }
            };

            let console_continuation = format!("{}{}", continuation_prefix, chunk_color);
            if !is_dashboard_enabled() {
                println!("{}", console_continuation);
            }

            // Write continuation lines to file as well
            let file_continuation = format!(
                "{} [{}] [{}] {}",
                timestamp,
                tag_clean,
                log_type,
                chunk
            );
            write_to_file(&file_continuation);
        }
    }
}

/// Helper function to remove ANSI color codes for length calculation
fn strip_ansi_codes(text: &str) -> String {
    // Simple regex-free approach to estimate length without ANSI codes
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

/// Helper function to wrap text at word boundaries, respecting existing newlines
/// and breaking very long words (like URLs) that exceed the available space
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut result = Vec::new();

    // First, split by existing newlines to respect intentional line breaks
    for line in text.split('\n') {
        // Use stripped length for accurate width calculation
        let line_display_length = strip_ansi_codes(line).len();

        if line_display_length <= max_width {
            result.push(line.to_string());
        } else {
            // Only wrap lines that exceed max_width (based on display length, not raw length)
            let mut current_line = String::new();

            for word in line.split_whitespace() {
                let word_display_length = strip_ansi_codes(word).len();
                let current_display_length = strip_ansi_codes(&current_line).len();

                // Check if this single word is longer than max_width
                if word_display_length > max_width {
                    // If current line has content, flush it first
                    if !current_line.is_empty() {
                        result.push(current_line);
                        current_line = String::new();
                    }

                    // Break the long word into chunks
                    let word_chunks = break_long_word(word, max_width);
                    for chunk in word_chunks {
                        result.push(chunk);
                    }
                } else if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_display_length + word_display_length + 1 <= max_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    result.push(current_line);
                    current_line = word.to_string();
                }
            }

            if !current_line.is_empty() {
                result.push(current_line);
            }
        }
    }

    if result.is_empty() {
        result.push(String::new());
    }

    result
}

/// Break a very long word (like URLs or JSON) into smaller chunks
fn break_long_word(word: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut remaining = word;

    while !remaining.is_empty() {
        if remaining.len() <= max_width {
            chunks.push(remaining.to_string());
            break;
        }

        let chunk_length = max_width;

        // For URLs and other structured text, try to break at natural points
        let break_point = if chunk_length < remaining.len() {
            // Look for good break points in the next few characters (up to 15 chars ahead)
            let search_end = std::cmp::min(chunk_length + 15, remaining.len());
            let search_slice = &remaining[chunk_length..search_end];

            // Priority order for URL/JSON break points:
            // 1. URL path separators and query params: /, ?, &
            // 2. Assignment and value separators: =, :
            // 3. General separators: ., -, _
            // 4. JSON/data separators: {, }, [, ], ,
            let break_chars = ['/', '?', '&', '=', ':', '.', '-', '_', '{', '}', '[', ']', ','];

            if let Some(pos) = search_slice.find(&break_chars[..]) {
                let actual_pos = chunk_length + pos + 1;
                // Make sure we don't go beyond the string
                std::cmp::min(actual_pos, remaining.len())
            } else {
                chunk_length
            }
        } else {
            chunk_length
        };

        let chunk = &remaining[..break_point];
        chunks.push(chunk.to_string());
        remaining = &remaining[break_point..];
    }

    chunks
}
/// Enhanced logging function for price changes with comprehensive Positions details
/// Shows full symbol, both pool and API prices, pool information, and current P&L
/// Displays information in two well-formatted lines for better readability
pub fn log_price_change(
    mint: &str,
    symbol: &str,
    old_price: f64,
    new_price: f64,
    price_source: &str,
    pool_type: Option<&str>,
    pool_address: Option<&str>,
    api_price: Option<f64>,
    current_pnl: Option<(f64, f64)> // (pnl_sol, pnl_percent)
) {
    let price_change = new_price - old_price;
    let price_change_percent = if old_price != 0.0 {
        (price_change / old_price) * 100.0
    } else {
        0.0
    };

    // Price direction emoji and color
    let (emoji, price_color) = if price_change > 0.0 {
        ("🟢", "green")
    } else if price_change < 0.0 {
        ("🔴", "red")
    } else {
        ("➡️", "yellow")
    };

    // Format pool type properly
    let formatted_pool_type = pool_type
        .map(|pt| {
            if pt.chars().any(|c| c.is_uppercase()) && pt.contains(' ') {
                pt.to_string()
            } else {
                pt.split('-')
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) =>
                                first.to_uppercase().collect::<String>() +
                                    &chars.as_str().to_uppercase(),
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" ")
            }
        })
        .unwrap_or_else(|| "Unknown".to_string());

    // Build line 1: Symbol, price change, and P&L information
    let mut line1_parts = Vec::new();

    // Symbol and price change
    let price_part = format!(
        "{} {} {:.10} SOL ( {}SOL, {} % )",
        emoji,
        format!("{}", symbol).bold(),
        match price_color {
            "green" => format!("{:.10}", new_price).green().bold(),
            "red" => format!("{:.10}", new_price).red().bold(),
            _ => format!("{:.10}", new_price).white().bold(),
        },
        match price_color {
            "green" => format!("+{:.10} ", price_change).green().bold(),
            "red" => format!("{:.10} ", price_change).red().bold(),
            _ => format!("+{:.10} ", 0.0).white().bold(),
        },
        match price_color {
            "green" => format!("+{:.2}", price_change_percent).green().bold(),
            "red" => format!("{:.2}", price_change_percent).red().bold(),
            _ => format!("+{:.2}", 0.0).white().bold(),
        }
    );
    line1_parts.push(price_part);

    // P&L section in first line
    if let Some((pnl_sol, pnl_percent)) = current_pnl {
        let pnl_text = if pnl_percent > 0.0 {
            format!(
                "💰 P&L: {} SOL ( {} % )",
                format!("+{:.6}", pnl_sol).green().bold(),
                format!("+{:.2}", pnl_percent).green().bold()
            )
        } else if pnl_percent < 0.0 {
            format!(
                "💸 P&L: {} SOL ( {} % )",
                format!("{:.6}", pnl_sol).red().bold(),
                format!("{:.2}", pnl_percent).red().bold()
            )
        } else {
            format!(
                "🟡 P&L: {} SOL ( {} % )",
                format!("±{:.6}", 0.0).white().bold(),
                format!("±{:.2}", 0.0).white().bold()
            )
        };
        line1_parts.push(pnl_text);
    }

    let line1 = line1_parts.join(" ");

    // Build line 2: Pool vs API comparison and pool details only
    let mut line2_parts = Vec::new();

    // Price comparison section with consistent mono styling
    if price_source == "pool" {
        if let Some(api_price_val) = api_price {
            let diff = new_price - api_price_val;
            let diff_percent = if api_price_val != 0.0 {
                (diff / api_price_val) * 100.0
            } else {
                0.0
            };

            line2_parts.push(format!("🏊 Pool: {}", format!("{:.10}", new_price).white().bold()));
            line2_parts.push(
                format!("🌐 API: {}", format!("{:.10}", api_price_val).white().bold())
            );

            let diff_text = if diff > 0.0 {
                format!("( Pool {} % )", format!("+{:.2}", diff_percent).green().bold())
            } else if diff < 0.0 {
                format!("( Pool {} % )", format!("{:.2}", diff_percent).red().bold())
            } else {
                "(Perfect Match)".white().to_string()
            };
            line2_parts.push(diff_text);
        } else {
            line2_parts.push(format!("🏊 {} Pool", formatted_pool_type).dimmed().to_string());
        }
    } else {
        line2_parts.push("🌐 API Price".dimmed().to_string());
    }

    // Pool details with better color
    if pool_address.is_some() {
        line2_parts.push(format!("[ {} ]", formatted_pool_type).bright_yellow().to_string());
    }

    // Join line2 parts with proper spacing
    let line2 = line2_parts
        .into_iter()
        .map(|part| part.to_string())
        .collect::<Vec<String>>()
        .join(" ");

    // Combine both lines into a single message with newline separator
    let combined_message = format!("{}\n{}", line1, line2);

    // Log both lines using a single logger call
    log(LogTag::Positions, "PRICE", &combined_message);
}
