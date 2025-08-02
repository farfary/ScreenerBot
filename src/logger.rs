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
/// - Individual log tags and types can be enabled/disabled via constants

/// Set to false to hide date in logs
const LOG_SHOW_DATE: bool = false;
/// Set to false to hide time in logs
const LOG_SHOW_TIME: bool = false;

/// File logging configuration
const ENABLE_FILE_LOGGING: bool = true;
const LOG_RETENTION_HOURS: u64 = 24; // Keep logs for 24 hours
const MAX_LOG_FILES: usize = 7; // Keep maximum 7 days of logs as backup

/// Log Tag Configuration - Set to false to disable specific tags
const ENABLE_MONITOR_LOGS: bool = true;
const ENABLE_TRADER_LOGS: bool = true;
const ENABLE_WALLET_LOGS: bool = true;
const ENABLE_SYSTEM_LOGS: bool = true;
const ENABLE_POOL_LOGS: bool = true;
const ENABLE_BLACKLIST_LOGS: bool = true;
const ENABLE_DISCOVERY_LOGS: bool = true;
const ENABLE_API_LOGS: bool = true;
const ENABLE_PRICE_SERVICE_LOGS: bool = true;
const ENABLE_FILTERING_LOGS: bool = true;
const ENABLE_RUGCHECK_LOGS: bool = true;
const ENABLE_PROFIT_TAG_LOGS: bool = true;
const ENABLE_RPC_LOGS: bool = true;
const ENABLE_OHLCV_LOGS: bool = true;
const ENABLE_DECIMALS_LOGS: bool = true;
const ENABLE_SWAP_LOGS: bool = true;
const ENABLE_OTHER_LOGS: bool = true;

/// Log Type Configuration - Set to false to disable specific log types
const ENABLE_ERROR_LOGS: bool = true;
const ENABLE_FAILED_LOGS: bool = true;
const ENABLE_WARN_LOGS: bool = true;
const ENABLE_SUCCESS_LOGS: bool = true;
const ENABLE_INFO_LOGS: bool = true;
const ENABLE_DEBUG_LOGS: bool = true; // Enable debug logs to see monitoring activity
const ENABLE_PROFIT_LOGS: bool = true;
const ENABLE_LOSS_LOGS: bool = true;
const ENABLE_BUY_LOGS: bool = true;
const ENABLE_SELL_LOGS: bool = true;
const ENABLE_BALANCE_LOGS: bool = true;
const ENABLE_PRICE_LOGS: bool = true;
const ENABLE_GENERAL_LOGS: bool = true; // For any log type not specifically listed above

/// Log format character widths (hardcoded for precise alignment)
const TAG_WIDTH: usize = 10; // "[SYSTEM  ]" = 10 chars (8 + 2 brackets)
const LOG_TYPE_WIDTH: usize = 26; // "[UPDATE  ]" = 10 chars (8 + 2 brackets)
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

        let file_writer = Some(BufWriter::new(file));

        Ok(FileLogger {
            file_writer,
            current_date: now.format("%Y-%m-%d").to_string(),
            log_dir,
            write_counter: 0,
        })
    }

    fn write_to_file(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        // For per-start log files, we don't need daily rotation
        // Just write to the current file and periodically clean up old files

        if let Some(ref mut writer) = self.file_writer {
            writeln!(writer, "{}", message)?;
            writer.flush()?;

            // Increment write counter and occasionally clean up old log files
            self.write_counter += 1;
            if self.write_counter % 500 == 0 {
                // Every 500 writes
                let _ = self.cleanup_old_logs(); // Ignore cleanup errors
            }
        }

        Ok(())
    }

    fn rotate_log_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Close current file
        if let Some(writer) = self.file_writer.take() {
            drop(writer);
        }

        // Clean up old log files
        self.cleanup_old_logs()?;

        // Create new log file for today
        let today = Local::now().format("%Y-%m-%d").to_string();
        let log_file_path = self.log_dir.join(format!("screenerbot_{}.log", today));

        let file = OpenOptions::new().create(true).append(true).open(&log_file_path)?;

        self.file_writer = Some(BufWriter::new(file));

        Ok(())
    }

    fn cleanup_old_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Local::now();
        let cutoff_time = now - chrono::Duration::hours(LOG_RETENTION_HOURS as i64);

        if let Ok(entries) = fs::read_dir(&self.log_dir) {
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
                            if let Err(e) = fs::remove_file(entry.path()) {
                                eprintln!(
                                    "Failed to remove old log file {:?}: {}",
                                    entry.path(),
                                    e
                                );
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
                    if let Err(e) = fs::remove_file(entry.path()) {
                        eprintln!("Failed to remove excess log file {:?}: {}", entry.path(), e);
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

/// Write message to log file (stripped of color codes)
fn write_to_file(message: &str) {
    if !ENABLE_FILE_LOGGING {
        return;
    }

    if let Ok(mut logger_guard) = FILE_LOGGER.lock() {
        if let Some(ref mut logger) = logger_guard.as_mut() {
            let clean_message = strip_ansi_codes(message);
            if let Err(e) = logger.write_to_file(&clean_message) {
                eprintln!("Failed to write to log file: {}", e);
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
            LogTag::Other(s) => format!("{:<8}", s).white().bold(),
        };
        write!(f, "{}", tag_str)
    }
}

/// Logs a message with date, time, tag, log type, and message.
pub fn log(tag: LogTag, log_type: &str, message: &str) {
    // Check if the tag is enabled
    let tag_enabled = match &tag {
        LogTag::Monitor => ENABLE_MONITOR_LOGS,
        LogTag::Trader => ENABLE_TRADER_LOGS,
        LogTag::Wallet => ENABLE_WALLET_LOGS,
        LogTag::System => ENABLE_SYSTEM_LOGS,
        LogTag::Pool => ENABLE_POOL_LOGS,
        LogTag::Blacklist => ENABLE_BLACKLIST_LOGS,
        LogTag::Discovery => ENABLE_DISCOVERY_LOGS,
        LogTag::Filtering => ENABLE_FILTERING_LOGS,
        LogTag::Api => ENABLE_API_LOGS,
        LogTag::Rugcheck => ENABLE_RUGCHECK_LOGS,
        LogTag::Profit => ENABLE_PROFIT_TAG_LOGS,
        LogTag::PriceService => ENABLE_PRICE_SERVICE_LOGS,
        LogTag::Rpc => ENABLE_RPC_LOGS,
        LogTag::Ohlcv => ENABLE_OHLCV_LOGS,
        LogTag::Decimals => ENABLE_DECIMALS_LOGS,
        LogTag::Swap => ENABLE_SWAP_LOGS,
        LogTag::Other(_) => ENABLE_OTHER_LOGS,
    };

    if !tag_enabled {
        return; // Skip logging if tag is disabled
    }

    // Check if the log type is enabled
    let log_type_enabled = match log_type.to_uppercase().as_str() {
        "ERROR" => ENABLE_ERROR_LOGS,
        "FAILED" => ENABLE_FAILED_LOGS,
        "WARN" | "WARNING" => ENABLE_WARN_LOGS,
        "SUCCESS" => ENABLE_SUCCESS_LOGS,
        "INFO" => ENABLE_INFO_LOGS,
        "DEBUG" => ENABLE_DEBUG_LOGS,
        "PROFIT" => ENABLE_PROFIT_LOGS,
        "LOSS" => ENABLE_LOSS_LOGS,
        "BUY" => ENABLE_BUY_LOGS,
        "SELL" => ENABLE_SELL_LOGS,
        "BALANCE" => ENABLE_BALANCE_LOGS,
        "PRICE" => ENABLE_PRICE_LOGS,
        _ => ENABLE_GENERAL_LOGS,
    };

    if !log_type_enabled {
        return; // Skip logging if log type is disabled
    }

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
    println!("{}", console_line);

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
        LogTag::Other(ref s) => s,
    };
    let file_line = format!("{} [{}] [{}] {}", timestamp, tag_clean, log_type, message_chunks[0]);
    write_to_file(&file_line);

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
            println!("{}", console_continuation);

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

/// Helper function to wrap text at word boundaries
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.len() <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + word.len() + 1 <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Special logging function for price changes with enhanced colors and formatting
/// Shows colored price changes for open positions without requiring debug mode
/// Uses the same format as the main logger system for consistency
pub fn log_price_change(
    _mint: &str,
    symbol: &str,
    old_price: f64,
    new_price: f64,
    price_source: &str,
    pool_type: Option<&str>,
    pool_address: Option<&str>
) {
    let price_change = new_price - old_price;
    let price_change_percent = if old_price != 0.0 {
        (price_change / old_price) * 100.0
    } else {
        0.0
    };

    // Build the main message parts
    let symbol_padded = format!("{:<8}", symbol.chars().take(8).collect::<String>());

    // Price with emoji and color
    let (emoji, price_text) = if price_change > 0.0 {
        ("📈", format!("{:.12}", new_price).bright_green().bold())
    } else if price_change < 0.0 {
        ("📉", format!("{:.12}", new_price).bright_red().bold())
    } else {
        ("➡️", format!("{:.12}", new_price).bright_yellow().bold())
    };

    // Change amount and percentage with colors
    let change_text = if price_change > 0.0 {
        format!("(+{:.12} SOL, +{:.4}%)", price_change, price_change_percent).bright_green()
    } else if price_change < 0.0 {
        format!("({:.12} SOL, {:.4}%)", price_change, price_change_percent).bright_red()
    } else {
        format!("(±{:.12} SOL, ±{:.4}%)", 0.0, 0.0).bright_yellow()
    };

    // Format pool type properly - "raydium-cpmm" becomes "Raydium CPMM"
    let formatted_pool_type = pool_type.map(|pt| {
        pt.split('-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) =>
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_uppercase(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    });

    // Source info with proper pool type formatting
    let source_text = match price_source {
        "pool" => {
            if
                let (Some(formatted_type), Some(pool_addr)) = (
                    formatted_pool_type.as_ref(),
                    pool_address,
                )
            {
                format!("🏊 {} Pool: {}", formatted_type, pool_addr).bright_cyan()
            } else {
                "🏊 Pool".bright_cyan()
            }
        }
        "api" => "🌐 API".bright_magenta(),
        _ => format!("📊 {}", price_source).bright_white(),
    };

    // Build complete message using exact logger format
    let message = format!(
        "{} {} {} {} {}",
        symbol_padded.bright_white().bold(),
        emoji,
        price_text,
        change_text,
        source_text
    );

    // Use standard logger with POSITION tag and PRICE type
    log(LogTag::Other("POSITION".to_string()), "PRICE", &message);
}
