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
        prefix = format!("{} {}", date, time);
    } else if LOG_SHOW_DATE {
        prefix = date;
    } else if LOG_SHOW_TIME {
        prefix = time;
    }
    let prefix = if !prefix.is_empty() { prefix.dimmed().to_string() } else { String::new() };

    // Emotional color mapping for log types
    let log_type_str = match log_type.to_uppercase().as_str() {
        "ERROR" => format!("{:<8}", log_type).bright_red().bold(), // 🔥 Urgent red for errors
        "WARN" | "WARNING" => format!("{:<8}", log_type).bright_yellow().bold(), // ⚠️ Caution yellow
        "SUCCESS" => format!("{:<8}", log_type).bright_green().bold(), // ✅ Success green
        "INFO" => format!("{:<8}", log_type).bright_blue().bold(), // ℹ️ Calm blue for info
        "DEBUG" => format!("{:<8}", log_type).bright_black().bold(), // 🔍 Subtle for debug
        "PROFIT" => format!("{:<8}", log_type).bright_green().bold(), // 💰 Money green for profits
        "LOSS" => format!("{:<8}", log_type).bright_red().bold(), // 💸 Red for losses
        "BUY" => format!("{:<8}", log_type).bright_cyan().bold(), // 🚀 Exciting cyan for buys
        "SELL" => format!("{:<8}", log_type).bright_magenta().bold(), // 📈 Purple for sells
        "BALANCE" => format!("{:<8}", log_type).bright_yellow().bold(), // 💳 Golden for balance
        "PRICE" => format!("{:<8}", log_type).bright_blue().bold(), // 📊 Blue for price data
        _ => format!("{:<8}", log_type).white().bold(), // Default white
    };

    let log_label = format!("{:<8}", "LOG").bright_white().bold();
    let msg = message.bright_white();
    if !prefix.is_empty() {
        println!("{} [{}] [{}] [{}] {}", prefix, tag, log_type_str, log_label, msg);
    } else {
        println!("[{}] [{}] [{}] {}", tag, log_type_str, log_label, msg);
    }
}
