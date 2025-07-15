use crate::types::{ LogLevel, RESET_COLOR };
use chrono::Utc;
use std::io::{ self, Write };

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
        let timestamp = Utc::now().format("%H:%M:%S");
        println!("{}🔎 [{}] DISCOVERY: {}{}", "\x1b[95m", timestamp, message, RESET_COLOR);
        io::stdout().flush().unwrap();
    }

    pub fn wallet(message: &str) {
        let timestamp = Utc::now().format("%H:%M:%S");
        println!("{}💼 [{}] WALLET: {}{}", "\x1b[94m", timestamp, message, RESET_COLOR);
        io::stdout().flush().unwrap();
    }

    pub fn trader(message: &str) {
        let timestamp = Utc::now().format("%H:%M:%S");
        println!("{}📈 [{}] TRADER: {}{}", "\x1b[93m", timestamp, message, RESET_COLOR);
        io::stdout().flush().unwrap();
    }

    pub fn header(title: &str) {
        println!("\n{}", "═".repeat(60));
        println!("{}🤖 SCREENER BOT - {}{}", "\x1b[96m", title, RESET_COLOR);
        println!("{}", "═".repeat(60));
    }

    pub fn separator() {
        println!("{}", "─".repeat(60));
    }

    fn log(log_level: LogLevel, message: &str) {
        let timestamp = Utc::now().format("%H:%M:%S");
        println!(
            "{}{} [{}] {}{}",
            log_level.color,
            log_level.prefix,
            timestamp,
            message,
            RESET_COLOR
        );
        io::stdout().flush().unwrap();
    }
}
