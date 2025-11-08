/// Centralized argument handling system for ScreenerBot
///
/// This module provides a unified interface for command-line argument parsing.
///
/// ## Argument Types
///
/// **Execution Modes** (mutually exclusive - choose one):
/// - `--run`: Start the trading bot
/// - `--reset`: Reset database state
/// - `--help`: Show help information
///
/// **Modifiers** (combine with modes):
/// - `--force`: Skip confirmation prompts (works with: --reset)
/// - `--dry-run`: Simulate without executing trades (works with: --run)
/// - `--cache-only`: Use cached data only (works with debug tools)
/// - `--force-refresh`: Force refresh from RPC (works with debug tools)
///
/// **Profiling Flags** (performance analysis):
/// - `--profile-cpu`: Enable CPU profiling with flamegraph
/// - `--profile-tokio-console`: Enable tokio-console for async profiling
/// - `--profile-tracing`: Enable detailed tracing
/// - `--profile-duration <seconds>`: Set profiling duration (default: 60)
///
/// **Debug Flags** (controlled by logger system):
/// - `--debug-<module>`: Enable debug logging for specific module
/// - `--verbose-<module>`: Enable verbose logging for specific module
///
use crate::logger::{self, LogTag};
use once_cell::sync::Lazy;
use std::env;
use std::sync::Mutex;

/// Global command-line arguments storage
/// Thread-safe singleton that stores arguments for access throughout the application
pub static CMD_ARGS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(env::args().collect()));

/// Sets the global command-line arguments
/// Used by binaries and tests to override the default env::args() collection
pub fn set_cmd_args(args: Vec<String>) {
    if let Ok(mut cmd_args) = CMD_ARGS.lock() {
        *cmd_args = args;
    }
}

/// Gets a copy of the current command-line arguments
/// Returns a vector clone to avoid holding the mutex lock
pub fn get_cmd_args() -> Vec<String> {
    match CMD_ARGS.lock() {
        Ok(args) => args.clone(),
        Err(_) => {
            // Fallback to env::args if mutex is poisoned
            env::args().collect()
        }
    }
}

/// Checks if a specific argument is present in the command line
pub fn has_arg(arg: &str) -> bool {
    get_cmd_args().iter().any(|a| a == arg)
}

/// Gets the value of a command-line argument that follows a flag
/// Returns None if the flag is not found or has no value
pub fn get_arg_value(flag: &str) -> Option<String> {
    let args = get_cmd_args();
    for (i, arg) in args.iter().enumerate() {
        if arg == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

// =============================================================================
// EXECUTION MODES
// =============================================================================

/// Run mode - enables actual bot execution (required to start services)
pub fn is_run_enabled() -> bool {
    has_arg("--run")
}

/// Reset mode - clears pending verifications and database files
pub fn is_reset_enabled() -> bool {
    has_arg("--reset")
}

// =============================================================================
// MODIFIER FLAGS
// =============================================================================

/// Dry-run mode - simulates trading without executing actual transactions
/// Works with: --run
pub fn is_dry_run_enabled() -> bool {
    has_arg("--dry-run")
}

/// Force mode - skip confirmation prompts
/// Works with: --reset
pub fn is_force_enabled() -> bool {
    has_arg("--force")
}

/// Cache-only mode - read from local DB only, never call RPC
/// Works with: debug tools and binaries
pub fn is_cache_only_enabled() -> bool {
    has_arg("--cache-only")
}

/// Force-refresh mode - refresh from RPC even if cached
/// Works with: debug tools and binaries
pub fn is_force_refresh_enabled() -> bool {
    has_arg("--force-refresh")
}

/// Clean wallet data - delete all wallet-specific databases
/// Use when switching to a different wallet
pub fn is_clean_wallet_data_enabled() -> bool {
    has_arg("--clean-wallet-data")
}

// =============================================================================
// PROFILING FLAGS
// =============================================================================

/// Enable tokio-console for async task profiling
/// Requires console-subscriber feature: cargo run --features console --bin screenerbot -- --run --profile-tokio-console
pub fn is_profile_tokio_console_enabled() -> bool {
    has_arg("--profile-tokio-console")
}

/// Enable detailed tracing for performance analysis
pub fn is_profile_tracing_enabled() -> bool {
    has_arg("--profile-tracing")
}

/// Enable CPU profiling with pprof and flamegraph generation
/// Requires flamegraph feature: cargo run --features flamegraph --bin screenerbot -- --run --profile-cpu
pub fn is_profile_cpu_enabled() -> bool {
    has_arg("--profile-cpu")
}

/// Get profiling duration in seconds (default: 60)
pub fn get_profile_duration() -> u64 {
    get_arg_value("--profile-duration")
        .and_then(|v| v.parse().ok())
        .unwrap_or(60)
}

// =============================================================================
// HELP SYSTEM
// =============================================================================

/// Displays the help menu with all available flags and their descriptions
pub fn print_help() {
    println!("ScreenerBot - Advanced Solana DeFi Trading Bot");
    println!();
    println!("USAGE:");
    println!("    screenerbot <MODE> [MODIFIERS] [DEBUG_FLAGS]");
    println!();
    println!("EXECUTION MODES (choose one):");
    println!("    --run                       Start the trading bot");
    println!(
        "    --reset                     Reset pending verifications and delete database files"
    );
    println!("    --clean-wallet-data         Clean all wallet-specific databases (use when switching wallets)");
    println!("    --help, -h                  Show this help message");
    println!();
    println!("MODIFIERS (combine with modes above):");
    println!("    --dry-run                   Simulate trading without executing transactions (with --run)");
    println!("    --force                     Skip confirmation prompts (with --reset)");
    println!("    --cache-only                Use cached data only, no RPC calls (debug tools)");
    println!("    --force-refresh             Force refresh from RPC even if cached (debug tools)");
    println!();
    println!("PROFILING FLAGS (performance analysis):");
    println!("    --profile-cpu               Enable CPU profiling with flamegraph generation");
    println!("    --profile-tokio-console     Enable tokio-console for async task profiling");
    println!("    --profile-tracing           Enable detailed tracing for performance analysis");
    println!("    --profile-duration <n>      Set profiling duration in seconds (default: 60)");
    println!();
    println!("DEBUG FLAGS (enable detailed logging per module):");
    println!("    --debug-<module>            Enable debug logging for specific module");
    println!("    --verbose-<module>          Enable verbose logging for specific module");
    println!();
    println!("    Available modules:");
    println!("      api, blacklist, decimals, discovery, entry, filtering, monitor, ohlcv,");
    println!("      pool-calculator, pool-discovery, pool-analyzer, pool-cache, pool-fetcher,");
    println!("      pool-decoders, pool-prices, positions, profit, rpc, swaps, system,");
    println!("      security, trader, transactions, webserver, websocket, wallet");
    println!();
    println!("EXAMPLES:");
    println!(
        "    screenerbot --run                                # Start bot in live trading mode"
    );
    println!("    screenerbot --run --dry-run                      # Start bot in simulation mode");
    println!(
        "    screenerbot --run --dry-run --debug-trader       # Simulate with trader debug logs"
    );
    println!(
        "    screenerbot --reset                              # Reset with confirmation prompt"
    );
    println!("    screenerbot --reset --force                      # Reset without confirmation");
    println!("    screenerbot --clean-wallet-data                  # Clean databases when switching wallets");
    println!("    screenerbot --help                               # Show this help message");
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Gets a list of all enabled debug modes by checking command-line arguments
/// Note: The logger system handles actual debug filtering, this is for informational purposes only
pub fn get_enabled_debug_modes() -> Vec<String> {
    let mut modes = Vec::new();
    let args = get_cmd_args();

    // Check for debug flags
    for arg in &args {
        if let Some(module) = arg.strip_prefix("--debug-") {
            modes.push(format!("debug-{}", module));
        } else if let Some(module) = arg.strip_prefix("--verbose-") {
            modes.push(format!("verbose-{}", module));
        }
    }

    // Include execution mode
    if is_run_enabled() {
        modes.push("run".to_string());
    }
    if is_reset_enabled() {
        modes.push("reset".to_string());
    }

    // Include active modifiers
    if is_dry_run_enabled() {
        modes.push("dry-run".to_string());
    }
    if is_force_enabled() {
        modes.push("force".to_string());
    }
    if is_cache_only_enabled() {
        modes.push("cache-only".to_string());
    }
    if is_force_refresh_enabled() {
        modes.push("force-refresh".to_string());
    }

    // Include profiling flags
    if is_profile_cpu_enabled() {
        modes.push("profile-cpu".to_string());
    }
    if is_profile_tokio_console_enabled() {
        modes.push("profile-tokio-console".to_string());
    }
    if is_profile_tracing_enabled() {
        modes.push("profile-tracing".to_string());
    }

    modes
}

/// Prints debug information about current arguments and enabled debug modes
pub fn print_debug_info() {
    let args = get_cmd_args();
    logger::debug(
        LogTag::System,
        &format!("Command-line arguments: {:?}", args),
    );

    let enabled_modes = get_enabled_debug_modes();
    if enabled_modes.is_empty() {
        logger::debug(LogTag::System, "No debug modes enabled");
    } else {
        logger::debug(
            LogTag::System,
            &format!("Enabled debug modes: {:?}", enabled_modes),
        );
    }
}

// =============================================================================
// COMMON ARGUMENT PATTERNS
// =============================================================================

/// Common argument parsing patterns used across binaries
pub mod patterns {
    use super::*;

    /// Checks for help flags
    pub fn is_help_requested() -> bool {
        has_arg("--help") || has_arg("-h")
    }

    /// Checks for version flags
    pub fn is_version_requested() -> bool {
        has_arg("--version") || has_arg("-V")
    }

    /// Gets duration argument (commonly used in monitoring tools)
    pub fn get_duration_seconds() -> Option<u64> {
        get_arg_value("--duration").and_then(|s| s.parse().ok())
    }

    /// Gets mint address argument (commonly used in token tools)
    pub fn get_mint_address() -> Option<String> {
        get_arg_value("--mint")
    }

    /// Gets symbol argument (commonly used in token tools)
    pub fn get_symbol() -> Option<String> {
        get_arg_value("--symbol")
    }

    /// Checks for quiet/silent mode
    pub fn is_quiet_mode() -> bool {
        has_arg("--quiet") || has_arg("-q")
    }

    /// Checks for verbose mode
    pub fn is_verbose_mode() -> bool {
        has_arg("--verbose") || has_arg("-v")
    }
}
