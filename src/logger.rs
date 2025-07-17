use chrono::Utc;
use colored::*;
use std::io::{ self, Write };

pub struct Logger;

impl Logger {
    pub fn new() -> Self {
        Self
    }

    // Basic log levels with expressive emojis
    pub fn info(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "INFO".blue().bold(),
            "|".dimmed(),
            message.bright_white()
        );
        io::stdout().flush().unwrap();
    }

    pub fn warn(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "WARN".yellow().bold(),
            "|".dimmed(),
            message.bright_white()
        );
        io::stdout().flush().unwrap();
    }

    pub fn error(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "ERROR".red().bold(),
            "|".dimmed(),
            message.bright_white()
        );
        io::stdout().flush().unwrap();
    }

    pub fn debug(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "DEBUG".purple().bold(),
            "|".dimmed(),
            message.bright_white()
        );
        io::stdout().flush().unwrap();
    }

    pub fn success(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "SUCCESS".green().bold(),
            "|".dimmed(),
            message.bright_white()
        );
        io::stdout().flush().unwrap();
    }

    // Specialized category loggers
    pub fn discovery(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "DISCOVERY".magenta().bold(),
            "|".dimmed(),
            Self::format_message(message).bright_white()
        );
        io::stdout().flush().unwrap();
    }

    pub fn database(message: &str) {
        let timestamp = Self::get_timestamp();
        println!(
            "{} {:<10} {} {}",
            format!("[{}]", timestamp).dimmed(),
            "DATABASE".bright_blue().bold(),
            "|".dimmed(),
            Self::format_message(message).bright_white()
        );
        io::stdout().flush().unwrap();
    }

    pub fn header(title: &str) {
        println!();
        println!(
            "{} {}",
            "ScreenerBot".green().bold(),
            format!("- {}", title).bright_white().bold()
        );
        println!("{}", "=".repeat(50).dimmed());
        io::stdout().flush().unwrap();
    }

    pub fn separator() {
        println!("{}", "─".repeat(50).dimmed());
        io::stdout().flush().unwrap();
    }

    // Enhanced formatting for messages with numbers, addresses, and key info
    fn format_message(message: &str) -> String {
        let mut formatted = message.to_string();

        // Highlight numbers (including decimals, percentages, and USD values)
        formatted = regex::Regex
            ::new(r"(\$?[\d,]+\.?\d*%?|\$[\d,]+\.?\d*)")
            .unwrap()
            .replace_all(&formatted, |caps: &regex::Captures| {
                caps[1].bright_white().bold().to_string()
            })
            .to_string();

        // Highlight addresses (Solana public keys - 44 characters base58)
        formatted = regex::Regex
            ::new(r"([1-9A-HJ-NP-Za-km-z]{32,44})")
            .unwrap()
            .replace_all(&formatted, |caps: &regex::Captures| {
                let addr = &caps[1];
                if addr.len() >= 32 {
                    format!(
                        "{}...{}",
                        addr[..8].bright_cyan().bold(),
                        addr[addr.len() - 4..].bright_cyan().bold()
                    )
                } else {
                    caps[1].bright_cyan().bold().to_string()
                }
            })
            .to_string();

        // Highlight transaction signatures (base58, usually 88 chars)
        formatted = regex::Regex
            ::new(r"([1-9A-HJ-NP-Za-km-z]{80,90})")
            .unwrap()
            .replace_all(&formatted, |caps: &regex::Captures| {
                let sig = &caps[1];
                format!(
                    "{}...{}",
                    sig[..12].bright_yellow().bold(),
                    sig[sig.len() - 8..].bright_yellow().bold()
                )
            })
            .to_string();

        // Highlight status words
        formatted = formatted
            .replace("SUCCESS", &"SUCCESS".green().bold().to_string())
            .replace("FAILED", &"FAILED".red().bold().to_string())
            .replace("ERROR", &"ERROR".red().bold().to_string())
            .replace("PENDING", &"PENDING".yellow().bold().to_string())
            .replace("CONFIRMED", &"CONFIRMED".green().bold().to_string())
            .replace("PROCESSING", &"PROCESSING".blue().bold().to_string())
            .replace("COMPLETED", &"COMPLETED".green().bold().to_string());

        formatted
    }

    fn get_timestamp() -> String {
        Utc::now().format("%H:%M:%S").to_string()
    }

    // Utility functions for formatted output
    pub fn print_key_value(key: &str, value: &str) {
        println!("  {} {}", format!("{}:", key).dimmed(), value.bright_white().bold());
    }

    pub fn print_balance(token: &str, amount: f64, sol_value: Option<f64>) {
        if let Some(sol) = sol_value {
            println!(
                "  {} {} {}",
                token.bright_white().bold(),
                format!("{:.4}", amount).bright_white().bold(),
                format!("({:.6} SOL)", sol).green().bold()
            );
        } else {
            println!(
                "  {} {}",
                token.bright_white().bold(),
                format!("{:.4}", amount).bright_white().bold()
            );
        }
    }

    pub fn print_pnl(pnl: f64, percentage: f64) {
        if pnl >= 0.0 {
            println!(
                "  PnL: {} ({}%)",
                format!("{:.6} SOL", pnl).green().bold(),
                format!("{:.2}", percentage).green().bold()
            );
        } else {
            println!(
                "  PnL: {} ({}%)",
                format!("{:.6} SOL", pnl).red().bold(),
                format!("{:.2}", percentage).red().bold()
            );
        }
    }
}
