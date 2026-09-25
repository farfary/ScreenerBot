//! Core logging implementation with automatic filtering
//!
//! This module contains the central logging logic that:
//! - Checks if a log should be displayed based on level and tag
//! - Delegates to the old logger.rs formatting/writing code
//! - Implements the filtering rules

use super::config::{get_logger_config, LoggerConfig};
use super::levels::LogLevel;
use super::tags::LogTag;

/// Check if a log message should be displayed
///
/// Filtering rules:
/// 1. Errors are always shown (unless explicitly disabled)
/// 2. Quiet mode suppresses diagnostic output
/// 3. If enabled_tags is non-empty, tag must be in the set
/// 4. Debug level requires --debug-<module> for that tag
/// 5. Verbose level requires --verbose or --verbose-<module> for that tag
/// 6. Other levels follow the minimum log level threshold
pub fn should_log(tag: &LogTag, level: LogLevel) -> bool {
    let config = get_logger_config();
    should_log_with_config(tag, level, &config)
}

fn should_log_with_config(tag: &LogTag, level: LogLevel, config: &LoggerConfig) -> bool {
    // Rule 1: Errors always log (critical)
    if level == LogLevel::Error {
        return true;
    }

    // Quiet mode still suppresses diagnostic output even when a module flag is set.
    if config.min_level == LogLevel::Warning && level > LogLevel::Warning {
        return false;
    }

    let tag_name = tag.to_debug_key();
    if !config.enabled_tags.is_empty() && !config.enabled_tags.contains(&tag_name) {
        return false;
    }

    match level {
        LogLevel::Debug => config.debug_modes.get(&tag_name).copied().unwrap_or(false),
        LogLevel::Verbose => {
            config.min_level == LogLevel::Verbose
                || config
                    .verbose_modes
                    .get(&tag_name)
                    .copied()
                    .unwrap_or(false)
        }
        _ => level <= config.min_level,
    }
}

/// Internal logging function with automatic filtering
///
/// This checks if the log should be displayed, then delegates to
/// the format module for formatting and writing.
pub fn log_internal(tag: LogTag, level: LogLevel, message: &str) {
    // Check if we should log this message
    if !should_log(&tag, level) {
        return;
    }

    // Delegate to format module for formatting and writing
    super::format::format_and_log(tag, level.as_str(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_debug_flag_works_at_default_info_level_without_enabling_other_tags() {
        let mut config = LoggerConfig::default();
        config.debug_modes.insert("wallet_watch".to_owned(), true);

        assert!(should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Debug,
            &config
        ));
        assert!(!should_log_with_config(
            &LogTag::Rpc,
            LogLevel::Debug,
            &config
        ));
        assert!(!should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Verbose,
            &config
        ));

        config.enabled_tags.insert("rpc".to_owned());
        assert!(!should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Debug,
            &config
        ));
    }

    #[test]
    fn quiet_mode_keeps_diagnostic_levels_suppressed() {
        let mut config = LoggerConfig::default();
        config.min_level = LogLevel::Warning;
        config.debug_modes.insert("wallet_watch".to_owned(), true);
        config.verbose_modes.insert("wallet_watch".to_owned(), true);

        assert!(!should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Debug,
            &config
        ));
        assert!(!should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Verbose,
            &config
        ));
        assert!(should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Warning,
            &config
        ));
    }

    #[test]
    fn module_verbose_flag_works_at_default_info_level() {
        let mut config = LoggerConfig::default();
        config.verbose_modes.insert("wallet_watch".to_owned(), true);

        assert!(should_log_with_config(
            &LogTag::WalletWatch,
            LogLevel::Verbose,
            &config
        ));
        assert!(!should_log_with_config(
            &LogTag::Rpc,
            LogLevel::Verbose,
            &config
        ));
    }
}
