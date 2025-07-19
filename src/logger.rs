/// Set to false to hide date in logs
const LOG_SHOW_DATE: bool = false;
/// Set to false to hide time in logs
const LOG_SHOW_TIME: bool = false;

use chrono::Local;
use colored::*;

/// Log tags for categorizing log messages.
#[derive(Debug)]
pub enum LogTag {
    Monitor,
    Trader,
    System,
    Other(String),
}

impl std::fmt::Display for LogTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag_str = match self {
            LogTag::Monitor => format!("{:<8}", "MONITOR").blue().bold(),
            LogTag::Trader => format!("{:<8}", "TRADER").green().bold(),
            LogTag::System => format!("{:<8}", "SYSTEM").yellow().bold(),
            LogTag::Other(s) => format!("{:<8}", s).normal(),
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
        prefix = format!("{} {}", date, time);
    } else if LOG_SHOW_DATE {
        prefix = date;
    } else if LOG_SHOW_TIME {
        prefix = time;
    }
    let prefix = if !prefix.is_empty() { prefix.dimmed().to_string() } else { String::new() };
    let log_type_str = format!("{:<8}", log_type).bright_red().bold();
    let log_label = format!("{:<8}", "LOG").bright_white().bold();
    let msg = message.bright_white();
    if !prefix.is_empty() {
        println!("{} [{}] [{}] [{}] {}", prefix, tag, log_type_str, log_label, msg);
    } else {
        println!("[{}] [{}] [{}] {}", tag, log_type_str, log_label, msg);
    }
}
