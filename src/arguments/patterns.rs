//! Common argument parsing patterns used across binaries.

use super::{get_arg_value, has_arg};

/// Checks for help flags.
pub fn is_help_requested() -> bool {
    has_arg("--help") || has_arg("-h")
}

/// Checks for version flags.
pub fn is_version_requested() -> bool {
    has_arg("--version") || has_arg("-V")
}

/// Gets duration argument (commonly used in monitoring tools).
pub fn get_duration_seconds() -> Option<u64> {
    get_arg_value("--duration").and_then(|s| s.parse().ok())
}

/// Gets mint address argument (commonly used in token tools).
pub fn get_mint_address() -> Option<String> {
    get_arg_value("--mint")
}

/// Gets symbol argument (commonly used in token tools).
pub fn get_symbol() -> Option<String> {
    get_arg_value("--symbol")
}

/// Checks for quiet/silent mode.
pub fn is_quiet_mode() -> bool {
    has_arg("--quiet") || has_arg("-q")
}

/// Checks for verbose mode.
pub fn is_verbose_mode() -> bool {
    has_arg("--verbose") || has_arg("-v")
}
