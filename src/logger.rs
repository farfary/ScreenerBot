use crate::types::{ LogLevel, RESET_COLOR };
use std::io::{ self, Write };
use chrono::Utc;

pub struct Logger;

impl Logger {
    pub fn new() -> Self {
        Self
    }

    pub fn info(message: &str) {
        Self::log(LogLevel::INFO, message);
    }

    pub fn warn(message: &str) {
        Self::log(LogLevel::WARN, message);
    }

    pub fn error(message: &str) {
        Self::log(LogLevel::ERROR, message);
    }

    pub fn debug(message: &str) {
        Self::log(LogLevel::DEBUG, message);
    }

    pub fn success(message: &str) {
        Self::log(LogLevel::SUCCESS, message);
    }

    pub fn discovery(message: &str) {
        Self::log_with_category("🔎", "\x1b[95m", "DISCOVERY", message);
    }

    pub fn wallet(message: &str) {
        Self::log_with_category("💼", "\x1b[94m", "WALLET", message);
    }

    pub fn trader(message: &str) {
        Self::log_with_category("📈", "\x1b[93m", "TRADER", message);
    }

    pub fn header(title: &str) {
        println!("\n{}{} ScreenerBot - {}{}", "\x1b[92m", "🤖", title, RESET_COLOR);
        io::stdout().flush().unwrap();
    }

    pub fn separator() {
        // Remove heavy separators for cleaner output
    }

    fn get_timestamp() -> String {
        Utc::now().format("%H:%M:%S").to_string()
    }

    fn log(log_level: LogLevel, message: &str) {
        println!(
            "{}{} [{}] {}{}",
            log_level.color,
            log_level.prefix,
            Self::get_timestamp(),
            message,
            RESET_COLOR
        );
        io::stdout().flush().unwrap();
    }

    fn log_with_category(icon: &str, color: &str, category: &str, message: &str) {
        println!(
            "{}{} [{}] {}: {}{}",
            color,
            icon,
            Self::get_timestamp(),
            category,
            message,
            RESET_COLOR
        );
        io::stdout().flush().unwrap();
    }
}
