//! Log formatting and output with ANSI colors and text wrapping
//!
//! Handles:
//! - Colorized console output with tag and level formatting
//! - Text wrapping at word boundaries
//! - Dual output (console + file)
//! - Broken pipe handling for piped commands

use super::file::write_to_file;
use super::tags::LogTag;
use chrono::Local;
use colored::*;
use std::io::{stdout, ErrorKind, Write};

/// Display configuration
const LOG_SHOW_DATE: bool = false;
const LOG_SHOW_TIME: bool = true;

/// Log format widths for alignment
const TAG_WIDTH: usize = 10;
const LOG_TYPE_WIDTH: usize = 30;
const BRACKET_SPACE_WIDTH: usize = 3;
const TOTAL_PREFIX_WIDTH: usize = TAG_WIDTH + LOG_TYPE_WIDTH + BRACKET_SPACE_WIDTH * 2;

/// Maximum line length before wrapping
const MAX_LINE_LENGTH: usize = 145;

/// Format and output a log message
pub fn format_and_log(tag: LogTag, log_type: &str, message: &str) {
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

    let prefix = if !prefix.is_empty() {
        prefix.dimmed().to_string()
    } else {
        String::new()
    };

    // Format tag with color
    let tag_str = format_tag(&tag);

    // Format log type with color
    let log_type_str = format_log_type(log_type);

    // Build the base log line
    let base_line = format!("{}[{}] [{}] ", prefix, tag_str, log_type_str);

    let base_length = strip_ansi_codes(&base_line)
        .len()
        .max(TOTAL_PREFIX_WIDTH + prefix.len());
    let available_space = if MAX_LINE_LENGTH > base_length {
        MAX_LINE_LENGTH - base_length
    } else {
        50
    };

    // Split message into chunks that fit
    let message_chunks = wrap_text(message, available_space);

    // Print first line
    let console_line = format!("{}{}", base_line, message_chunks[0]);
    print_stdout_safe(&console_line);

    // Write to file
    let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let tag_clean = tag.to_plain_string();
    let file_line = format!(
        "{} [{}] [{}] {}",
        timestamp, tag_clean, log_type, message_chunks[0]
    );
    write_to_file(&file_line);

    // Print continuation lines
    if message_chunks.len() > 1 {
        let continuation_prefix = format!(
            "{}{}",
            " ".repeat(strip_ansi_codes(&prefix).len()),
            " ".repeat(TOTAL_PREFIX_WIDTH)
        );
        for chunk in &message_chunks[1..] {
            let console_continuation = format!("{}{}", continuation_prefix, chunk);
            print_stdout_safe(&console_continuation);

            let file_continuation =
                format!("{} [{}] [{}] {}", timestamp, tag_clean, log_type, chunk);
            write_to_file(&file_continuation);
        }
    }
}

/// Format a tag with appropriate color
fn format_tag(tag: &LogTag) -> ColoredString {
    match tag {
        LogTag::Monitor => format!("{:<width$}", "MONITOR", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::Trader => format!("{:<width$}", "TRADER", width = TAG_WIDTH)
            .bright_green()
            .bold(),
        LogTag::Wallet => format!("{:<width$}", "WALLET", width = TAG_WIDTH)
            .bright_magenta()
            .bold(),
        LogTag::System => format!("{:<width$}", "SYSTEM", width = TAG_WIDTH)
            .bright_yellow()
            .bold(),
        LogTag::Pool => format!("{:<width$}", "POOL", width = TAG_WIDTH)
            .bright_blue()
            .bold(),
        LogTag::PoolService => format!("{:<width$}", "POOLSVC", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::PoolCalculator => format!("{:<width$}", "POOLCALC", width = TAG_WIDTH)
            .bright_green()
            .bold(),
        LogTag::PoolDiscovery => format!("{:<width$}", "POOLDISC", width = TAG_WIDTH)
            .bright_white()
            .bold(),
        LogTag::PoolFetcher => format!("{:<width$}", "POOLFETCH", width = TAG_WIDTH)
            .bright_yellow()
            .bold(),
        LogTag::PoolAnalyzer => format!("{:<width$}", "POOLANLZ", width = TAG_WIDTH)
            .bright_magenta()
            .bold(),
        LogTag::PoolCache => format!("{:<width$}", "POOLCACH", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::PoolDecoder => format!("{:<width$}", "POOLDEC", width = TAG_WIDTH)
            .bright_blue()
            .bold(),
        LogTag::Blacklist => format!("{:<width$}", "BLACKLIST", width = TAG_WIDTH)
            .bright_red()
            .bold(),
        LogTag::Discovery => format!("{:<width$}", "DISCOVER", width = TAG_WIDTH)
            .bright_white()
            .bold(),
        LogTag::Filtering => format!("{:<width$}", "FILTER", width = TAG_WIDTH)
            .bright_yellow()
            .bold(),
        LogTag::Api => format!("{:<width$}", "API", width = TAG_WIDTH)
            .bright_purple()
            .bold(),
        LogTag::Profit => format!("{:<width$}", "PROFIT", width = TAG_WIDTH)
            .bright_green()
            .bold(),
        LogTag::PriceService => format!("{:<width$}", "PRICE", width = TAG_WIDTH)
            .bright_green()
            .bold(),
        LogTag::SolPrice => format!("{:<width$}", "SOLPRICE", width = TAG_WIDTH)
            .bright_yellow()
            .bold(),
        LogTag::Rpc => format!("{:<width$}", "RPC", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::Ohlcv => format!("{:<width$}", "OHLCV", width = TAG_WIDTH)
            .bright_green()
            .bold(),
        LogTag::Decimals => format!("{:<width$}", "DECIMALS", width = TAG_WIDTH)
            .bright_white()
            .bold(),
        LogTag::Cache => format!("{:<width$}", "CACHE", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::Swap => format!("{:<width$}", "SWAP", width = TAG_WIDTH)
            .bright_magenta()
            .bold(),
        LogTag::Entry => format!("{:<width$}", "ENTRY", width = TAG_WIDTH)
            .bright_yellow()
            .bold(),
        LogTag::RlLearn => format!("{:<width$}", "RL_LEARN", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::Summary => format!("{:<width$}", "SUMMARY", width = TAG_WIDTH)
            .bright_white()
            .bold(),
        LogTag::Tokens => format!("{:<width$}", "TOKENS", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::Transactions => format!("{:<width$}", "TX", width = TAG_WIDTH)
            .bright_blue()
            .bold(),
        LogTag::Websocket => format!("{:<width$}", "WS", width = TAG_WIDTH)
            .bright_cyan()
            .bold(),
        LogTag::Positions => format!("{:<width$}", "Positions", width = TAG_WIDTH)
            .bright_yellow()
            .bold(),
        LogTag::Security => format!("{:<width$}", "SECURITY", width = TAG_WIDTH)
            .bright_red()
            .bold(),
        LogTag::Webserver => format!("{:<width$}", "WEBSERVER", width = TAG_WIDTH)
            .bright_green()
            .bold(),
        LogTag::Test => format!("{:<width$}", "TEST", width = TAG_WIDTH)
            .bright_blue()
            .bold(),
        LogTag::Other(ref s) => format!("{:<width$}", s, width = TAG_WIDTH).white().bold(),
    }
}

/// Format log type with appropriate color
fn format_log_type(log_type: &str) -> ColoredString {
    match log_type.to_uppercase().as_str() {
        "ERROR" => format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
            .bright_red()
            .bold(),
        _ => format!("{:<width$}", log_type, width = LOG_TYPE_WIDTH)
            .white()
            .bold(),
    }
}

/// Print to stdout but ignore broken pipe errors
fn print_stdout_safe(message: &str) {
    if let Err(e) = writeln!(stdout(), "{}", message) {
        if e.kind() == ErrorKind::BrokenPipe {
            std::process::exit(0);
        }
        let _ = writeln!(std::io::stderr(), "Logger stdout error: {}", e);
    }
    if let Err(e) = stdout().flush() {
        if e.kind() == ErrorKind::BrokenPipe {
            std::process::exit(0);
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

/// Wrap text at word boundaries, respecting existing newlines
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut result = Vec::new();

    for line in text.split('\n') {
        let line_display_length = strip_ansi_codes(line).len();

        if line_display_length <= max_width {
            result.push(line.to_string());
        } else {
            let mut current_line = String::new();

            for word in line.split_whitespace() {
                let word_display_length = strip_ansi_codes(word).len();
                let current_display_length = strip_ansi_codes(&current_line).len();

                if word_display_length > max_width {
                    if !current_line.is_empty() {
                        result.push(current_line);
                        current_line = String::new();
                    }

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

/// Break a very long word into smaller chunks
fn break_long_word(word: &str, max_width: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut remaining = word;

    while !remaining.is_empty() {
        if remaining.chars().count() <= max_width {
            chunks.push(remaining.to_string());
            break;
        }

        let mut char_boundary = 0;
        let mut char_count = 0;

        for (byte_idx, _) in remaining.char_indices() {
            if char_count >= max_width {
                break;
            }
            char_boundary = byte_idx;
            char_count += 1;
        }

        if char_count == 0 {
            if let Some((next_boundary, _)) = remaining.char_indices().nth(1) {
                char_boundary = next_boundary;
            } else {
                chunks.push(remaining.to_string());
                break;
            }
        }

        let break_point = if char_count < remaining.chars().count() {
            let search_start_chars = char_count;
            let search_end_chars = std::cmp::min(char_count + 15, remaining.chars().count());

            let search_start_bytes = remaining
                .char_indices()
                .nth(search_start_chars)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
            let search_end_bytes = remaining
                .char_indices()
                .nth(search_end_chars)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());

            if search_start_bytes < remaining.len() && search_end_bytes <= remaining.len() {
                let search_slice = &remaining[search_start_bytes..search_end_bytes];

                let break_chars = [
                    '/', '?', '&', '=', ':', '.', '-', '_', '{', '}', '[', ']', ',',
                ];

                if let Some(pos) = search_slice.find(&break_chars[..]) {
                    let actual_pos = search_start_bytes + pos + 1;
                    let actual_pos = std::cmp::min(actual_pos, remaining.len());

                    let mut boundary = actual_pos;
                    for (byte_idx, _) in remaining.char_indices() {
                        if byte_idx > actual_pos {
                            break;
                        }
                        boundary = byte_idx;
                    }
                    boundary
                } else {
                    char_boundary
                }
            } else {
                char_boundary
            }
        } else {
            char_boundary
        };

        let chunk = &remaining[..break_point];
        chunks.push(chunk.to_string());
        remaining = &remaining[break_point..];
    }

    chunks
}
