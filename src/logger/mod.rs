//! Modern structured logging system for ScreenerBot
//!
//! This module provides a clean, ergonomic logging API with:
//! - Automatic debug mode filtering from command-line arguments
//! - Standard log levels (Error/Warning/Info/Debug/Verbose)
//! - Per-module debug control via --debug-<module> flags
//! - Dual output: colored console + file persistence
//!
//! ## Usage
//!
//! ```rust
//! use screenerbot::logger::{self, LogTag};
//!
//! // Level-specific functions
//! logger::error(LogTag::Api, "Connection failed");
//! logger::warning(LogTag::Tokens, "Rate limit approaching");
//! logger::info(LogTag::Trader, "Position opened");
//! logger::debug(LogTag::Api, "Request details: ..."); // Only if --debug-api
//! logger::verbose(LogTag::Pool, "Raw pool data: ..."); // Only if --verbose
//! ```
//!
//! ## Initialization
//!
//! Call once at startup (in main.rs or run.rs):
//! ```rust
//! logger::init();
//! ```
//!
//! This automatically:
//! - Scans command-line arguments for --debug-<module> flags
//! - Configures per-module debug modes
//! - Initializes file logging
//! - Sets up filtering rules

mod config;
mod core;
mod file;
mod format;
mod levels;
mod special;
mod tags;

// Re-export public types
pub use config::{
    get_logger_config, init_from_args, set_logger_config, update_logger_config, LoggerConfig,
};
pub use levels::LogLevel;
pub use special::log_price_change;
pub use tags::LogTag;

/// Initialize the logger system
///
/// This must be called once at application startup, before any logging occurs.
/// It will:
/// 1. Parse command-line arguments for debug flags
/// 2. Configure per-module debug modes
/// 3. Initialize file logging system
/// 4. Set up filtering rules
///
/// Call this in main.rs or run.rs before starting services.
pub fn init() {
    // Initialize configuration from command-line arguments
    config::init_from_args();

    // Initialize file logging
    file::init_file_logging();
}

/// Log at ERROR level (always shown, critical issues)
///
/// Errors are always displayed regardless of debug flags or verbosity settings.
/// Use for critical failures that need immediate attention.
///
/// # Example
/// ```rust
/// logger::error(LogTag::Wallet, "Failed to load wallet keypair");
/// ```
pub fn error(tag: LogTag, message: &str) {
    core::log_internal(tag, LogLevel::Error, message);
}

/// Log at WARNING level (important issues)
///
/// Warnings are shown by default (unless --quiet is used).
/// Use for issues that need attention but aren't critical.
///
/// # Example
/// ```rust
/// logger::warning(LogTag::Api, "Rate limit approaching (80% used)");
/// ```
pub fn warning(tag: LogTag, message: &str) {
    core::log_internal(tag, LogLevel::Warning, message);
}

/// Log at INFO level (standard operations)
///
/// Info logs are shown by default and represent normal operation.
/// Use for important operational events.
///
/// # Example
/// ```rust
/// logger::info(LogTag::Trader, "Position opened: 1.5 SOL");
/// ```
pub fn info(tag: LogTag, message: &str) {
    core::log_internal(tag, LogLevel::Info, message);
}

/// Log at DEBUG level (detailed diagnostics)
///
/// Debug logs are ONLY shown when --debug-<module> flag is provided.
/// Automatically filtered based on the tag.
///
/// # Example
/// ```rust
/// // Only shown with --debug-api flag
/// logger::debug(LogTag::Api, "Request headers: {...}");
///
/// // Only shown with --debug-tokens flag
/// logger::debug(LogTag::Tokens, "Token metadata: {...}");
/// ```
pub fn debug(tag: LogTag, message: &str) {
    core::log_internal(tag, LogLevel::Debug, message);
}

/// Log at VERBOSE level (very detailed tracing)
///
/// Verbose logs are ONLY shown when --verbose flag is provided.
/// Use for extremely detailed diagnostic information.
///
/// # Example
/// ```rust
/// // Only shown with --verbose flag
/// logger::verbose(LogTag::Pool, "Raw pool account data: [...]");
/// ```
pub fn verbose(tag: LogTag, message: &str) {
    core::log_internal(tag, LogLevel::Verbose, message);
}

/// Force flush all pending log writes
///
/// Call this during shutdown to ensure all logs are written to disk.
pub fn flush() {
    file::flush_file_logging();
}
