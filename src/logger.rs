use chrono::Local;
use colored::*;
use std::time::Duration;
use std::collections::VecDeque;
use std::sync::{ Arc, Mutex };
use crate::global::{ is_shutdown, get_rpc_stats, get_wallet_balance };

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub task: String,
    pub level: LogLevel,
    pub message: String,
}

// Global log storage
use once_cell::sync::Lazy;
pub static LOG_BUFFER: Lazy<Arc<Mutex<VecDeque<LogEntry>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(VecDeque::with_capacity(1000)))
});

fn color_tag(level: &LogLevel) -> ColoredString {
    match level {
        LogLevel::Info => "INFO ".green().bold(),
        LogLevel::Warn => "WARN ".yellow().bold(),
        LogLevel::Error => "ERROR".red().bold(),
        LogLevel::Debug => "DEBUG".blue().bold(),
    }
}

pub fn log(task: &str, level: LogLevel, msg: &str) {
    let now = Local::now();
    let ts = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let tag = color_tag(&level);
    let padded_task = format!("{:<8}", task).cyan().bold();

    // Print to console
    println!("{} [{}] [{}] {}", ts, padded_task, tag, msg);

    // Store in buffer
    let entry = LogEntry {
        timestamp: ts,
        task: task.to_string(),
        level,
        message: msg.to_string(),
    };

    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        if buffer.len() >= 1000 {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }
}

pub fn get_recent_logs(count: usize) -> Vec<LogEntry> {
    if let Ok(buffer) = LOG_BUFFER.lock() {
        buffer.iter().rev().take(count).cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn get_logs_by_level(level: LogLevel, count: usize) -> Vec<LogEntry> {
    if let Ok(buffer) = LOG_BUFFER.lock() {
        buffer
            .iter()
            .rev()
            .filter(|entry| std::mem::discriminant(&entry.level) == std::mem::discriminant(&level))
            .take(count)
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

pub fn start_logger_manager() {
    tokio::task::spawn(async move {
        log("LOGGER", LogLevel::Info, "Logger Manager initialized successfully");

        let delays = crate::global::get_task_delays();
        let mut stats_counter = 0;

        loop {
            if is_shutdown() {
                log("LOGGER", LogLevel::Info, "Logger Manager shutting down...");

                // Final stats log
                log_system_stats().await;

                // Archive logs or perform cleanup here if needed
                log("LOGGER", LogLevel::Info, "Logger shutdown complete");
                break;
            }

            // Log system stats every 60 cycles (approximately every minute with 1s delay)
            stats_counter += 1;
            if stats_counter >= 60 {
                log_system_stats().await;
                stats_counter = 0;
            }

            // Log buffer management
            if let Ok(mut buffer) = LOG_BUFFER.lock() {
                // Clean old entries if buffer is getting too large
                if buffer.len() > 800 {
                    // Remove older entries, keep last 500
                    let excess = buffer.len() - 500;
                    for _ in 0..excess {
                        buffer.pop_front();
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(delays.logger_delay)).await;
        }
    });
}

async fn log_system_stats() {
    // Log RPC stats
    let rpc_stats = get_rpc_stats();
    if rpc_stats.total_calls > 0 {
        log(
            "STATS",
            LogLevel::Info,
            &format!(
                "RPC Stats - Total: {}, Main: {}, Fallback: {}, Failed: {}, Rate Limited: {}",
                rpc_stats.total_calls,
                rpc_stats.main_rpc_calls,
                rpc_stats.fallback_rpc_calls,
                rpc_stats.failed_calls,
                rpc_stats.rate_limited_calls
            )
        );
    }

    // Log wallet balance
    let balance = get_wallet_balance();
    if balance > 0.0 {
        log("STATS", LogLevel::Info, &format!("Wallet Balance: {:.6} SOL", balance));
    }

    // Log memory usage
    let log_count = if let Ok(buffer) = LOG_BUFFER.lock() { buffer.len() } else { 0 };
    log("STATS", LogLevel::Debug, &format!("Log entries in buffer: {}", log_count));
}
