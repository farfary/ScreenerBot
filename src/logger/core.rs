/// Core logging implementation with automatic filtering
///
/// This module contains the central logging logic that:
/// - Checks if a log should be displayed based on level and tag
/// - Delegates to the old logger.rs formatting/writing code
/// - Implements the filtering rules

use super::config::{get_logger_config, is_debug_enabled_for_tag, is_verbose_enabled_for_tag};
use super::levels::LogLevel;
use super::tags::LogTag;

/// Check if a log message should be displayed
///
/// Filtering rules:
/// 1. Errors are always shown (unless explicitly disabled)
/// 2. Check against minimum log level threshold
/// 3. Debug level requires --debug-<module> flag for that tag
/// 4. Verbose level requires --verbose flag OR --verbose-<module> flag for that tag
/// 5. If enabled_tags is non-empty, tag must be in the set
pub fn should_log(tag: &LogTag, level: LogLevel) -> bool {
    let config = get_logger_config();

    // Rule 1: Errors always log (critical)
    if level == LogLevel::Error {
        return true;
    }

    // Rule 2: Check minimum level threshold
    if level > config.min_level {
        return false;
    }

    // Rule 3: Debug level requires debug mode for that specific tag
    if level == LogLevel::Debug {
        return is_debug_enabled_for_tag(tag);
    }

    // Rule 4: Verbose requires explicit --verbose flag OR --verbose-<module> flag
    if level == LogLevel::Verbose {
        return config.min_level == LogLevel::Verbose || is_verbose_enabled_for_tag(tag);
    }

    // Rule 5: Check if tag is enabled (empty set = all enabled)
    if !config.enabled_tags.is_empty() {
        let tag_name = tag.to_debug_key();
        if !config.enabled_tags.contains(&tag_name) {
            return false;
        }
    }

    true
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
