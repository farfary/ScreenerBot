/// Set to false to hide date in logs
const LOG_SHOW_DATE: bool = false;
/// Set to false to hide time in logs
const LOG_SHOW_TIME: bool = false;

/// Log format character widths (hardcoded for precise alignment)
const TAG_WIDTH: usize = 8; // "[SYSTEM  ]" = 10 chars (8 + 2 brackets)
const LOG_TYPE_WIDTH: usize = 13; // "[UPDATE  ]" = 10 chars (8 + 2 brackets)
const LOG_LABEL_WIDTH: usize = 8; // "[LOG     ]" = 10 chars (8 + 2 brackets)
const BRACKET_SPACE_WIDTH: usize = 3; // " [" + "] " = 3 chars between each component
const TOTAL_PREFIX_WIDTH: usize =
    TAG_WIDTH + LOG_TYPE_WIDTH + LOG_LABEL_WIDTH + BRACKET_SPACE_WIDTH * 3; // +1 for final space

/// Maximum line length before wrapping
const MAX_LINE_LENGTH: usize = 155;

use chrono::Local;
use colored::*;

/// Log tags for categorizing log messages.
#[derive(Debug)]
pub enum LogTag {
    Monitor,
    Trader,
    Wallet,
    System,
    Other(String),
}

impl std::fmt::Display for LogTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag_str = match self {
            LogTag::Monitor => format!("{:<8}", "MONITOR").bright_cyan().bold(), // 👁️ Watchful blue
            LogTag::Trader => format!("{:<8}", "TRADER").bright_green().bold(), // 💰 Money green
            LogTag::Wallet => format!("{:<8}", "WALLET").bright_magenta().bold(), // 💜 Rich purple for wealth
            LogTag::System => format!("{:<8}", "SYSTEM").bright_yellow().bold(), // ⚙️ Mechanical yellow
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

    // Fixed-width log label
    let log_label = format!("{:<width$}", "LOG", width = LOG_LABEL_WIDTH)
        .bright_white()
        .bold();
    let msg = message.bright_white();

    // Build the base log line with strict discipline
    let base_line = format!("{}[{}] [{}] [{}] ", prefix, tag_str, log_type_str, log_label);

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

    // Print first line with full prefix
    println!("{}{}", base_line, message_chunks[0].bright_white());

    // Print continuation lines with proper indentation
    if message_chunks.len() > 1 {
        let continuation_prefix = format!(
            "{}{}",
            " ".repeat(prefix.len()),
            " ".repeat(TOTAL_PREFIX_WIDTH)
        );
        for chunk in &message_chunks[1..] {
            println!("{}{}", continuation_prefix, chunk.bright_white());
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
