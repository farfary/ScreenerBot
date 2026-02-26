//! Centralized argument handling system for ScreenerBot.
//!
//! This module provides a unified interface for command-line argument parsing.
//!
//! ## Argument Types
//!
//! **Execution Modes** (mutually exclusive — choose one):
//! - `--reset`: Reset database state
//! - `--clean-wallet-data`: Clean all wallet-specific databases
//! - `--help`: Show help information
//!
//! **Display Modes**:
//! - `--gui`: Launch with desktop GUI window (requires 'gui' feature)
//!
//! **Modifiers**:
//! - `--force`: Skip confirmation prompts (works with: --reset)
//! - `--cache-only`: Use cached data only (works with debug tools)
//! - `--force-refresh`: Force refresh from RPC (works with debug tools)

mod help;
mod modes;
pub mod patterns;
mod validation;

pub use help::*;
pub use modes::*;
pub use validation::*;

use std::env;
use std::sync::LazyLock;
use std::sync::Mutex;

/// Global command-line arguments storage.
/// Thread-safe singleton that stores arguments for access throughout the application.
pub static CMD_ARGS: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(env::args().collect()));

/// Sets the global command-line arguments.
/// Used by binaries and tests to override the default env::args() collection.
pub fn set_cmd_args(args: Vec<String>) {
    if let Ok(mut cmd_args) = CMD_ARGS.lock() {
        *cmd_args = args;
    }
}

/// Gets a copy of the current command-line arguments.
/// Returns a vector clone to avoid holding the mutex lock.
pub fn get_cmd_args() -> Vec<String> {
    match CMD_ARGS.lock() {
        Ok(args) => args.clone(),
        Err(_) => env::args().collect(),
    }
}

/// Checks if a specific argument is present in the command line.
pub fn has_arg(arg: &str) -> bool {
    get_cmd_args().iter().any(|a| a == arg)
}

/// Gets the value of a command-line argument that follows a flag.
/// Returns None if the flag is not found or has no value.
pub fn get_arg_value(flag: &str) -> Option<String> {
    let args = get_cmd_args();
    for (i, arg) in args.iter().enumerate() {
        if arg == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

/// Get profiling duration in seconds (default: 60).
pub fn get_profile_duration() -> u64 {
    get_arg_value("--profile-duration")
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}
