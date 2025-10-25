/// Centralized argument handling system for ScreenerBot
///
/// This module consolidates all command-line argument parsing and debug flag checking
/// functionality that was previously scattered across global.rs and binary files.
///
/// Features:
/// - Centralized CMD_ARGS storage with thread-safe access
/// - Debug flag checking functions for all modules
/// - Unified argument parsing utilities
/// - Support for both binary-specific and main application arguments
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
// CPU PROFILING FLAGS
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
// Transactions cache flags
// =============================================================================

/// When enabled, tools should only read raw tx from local DB and never call RPC
pub fn is_cache_only_enabled() -> bool {
    has_arg("--cache-only")
}

/// When enabled, force-refresh raw tx from RPC even if present in DB
pub fn is_force_refresh_enabled() -> bool {
    has_arg("--force-refresh")
}

// =============================================================================
// CORE FLAGS
// =============================================================================

/// Dry-run mode - simulates trading without executing actual transactions
pub fn is_dry_run_enabled() -> bool {
    has_arg("--dry-run")
}

/// Get configured max exit retries (defaults to 3). Clamped 1-10.
pub fn get_max_exit_retries() -> u32 {
    let args = get_cmd_args();
    for i in 0..args.len() {
        if args[i] == "--max-exit-retries" && i + 1 < args.len() {
            if let Ok(v) = args[i + 1].parse::<u32>() {
                return v.clamp(1, 10);
            }
        }
    }
    3
}

/// Run mode - enables actual bot execution (required to start services)
pub fn is_run_enabled() -> bool {
    has_arg("--run")
}

/// Clear all mode - clears all data and resets the system
pub fn is_clear_all_enabled() -> bool {
    has_arg("--clear-all")
}

/// Positions sell all mode - sells all open positions
pub fn is_positions_sell_all_enabled() -> bool {
    has_arg("--positions-sell-all")
}

/// Add to blacklist mode - adds a mint to blacklist
pub fn is_add_to_blacklist_enabled() -> bool {
    has_arg("--add-to-blacklist")
}

/// Get mint address for blacklist operations
pub fn get_blacklist_mint() -> Option<String> {
    get_arg_value("--add-to-blacklist")
}

/// Reset mode - clears pending verifications and optionally deletes database files
pub fn is_reset_enabled() -> bool {
    has_arg("--reset")
}

/// Force mode - skip confirmation prompts
pub fn is_force_enabled() -> bool {
    has_arg("--force")
}

// =============================================================================
// HELP SYSTEM
// =============================================================================

/// Displays the help menu with all available flags and their descriptions
pub fn print_help() {
    println!("ScreenerBot - Advanced Solana DeFi Trading Bot");
    println!();
    println!("USAGE:");
    println!("    screenerbot [FLAGS]");
    println!();
    println!("CORE FLAGS:");
    println!("    --run                     Enable bot execution (required to start trading)");
    println!("    --clear-all               Clear all data and reset the system");
    println!("    --reset                   Reset pending verifications and delete database files");
    println!("    --force                   Skip confirmation prompts (use with --reset)");
    println!("    --positions-sell-all      Sell all open positions");
    println!("    --add-to-blacklist <mint> Add a token mint address to blacklist");
    println!("    --help, -h                Show this help message");
    println!("    --dry-run                 Simulate trading without executing transactions");
    println!();
    println!("DEBUG FLAGS:");
    println!("    --debug-api               API calls debug mode");
    println!("    --debug-blacklist         Blacklist operations debug mode");
    println!("    --debug-decimals          Decimals module debug mode");
    println!("    --debug-discovery         Discovery module debug mode");
    println!("    --debug-entry             Entry module debug mode");
    println!("    --debug-filtering         Filtering module debug mode");
    println!("    --debug-monitor           Monitor module debug mode");
    println!("    --debug-ohlcv             OHLCV analysis debug mode");
    println!("    --debug-pool-calculator   Pool calculator debug mode");
    println!("    --debug-pool-discovery    Pool discovery debug mode");
    println!("    --debug-pool-analyzer     Pool analyzer debug mode");
    println!("    --debug-pool-cache        Pool cache debug mode");
    println!("    --debug-pool-fetcher      Pool fetcher debug mode");
    println!("    --debug-pool-decoders     Pool decoders debug mode");
    println!("    --debug-pool-prices       Pool prices debug mode");
    println!("    --debug-positions         Positions module debug mode");
    println!("    --debug-profit            Profit calculation debug mode");
    println!("    --debug-rpc               RPC operations debug mode");
    println!("    --debug-swaps             Swap operations debug mode");
    println!("    --debug-system            System operations debug mode");
    println!("    --debug-security          Security operations debug mode");
    println!("    --debug-trader            Trader module debug mode");
    println!("    --debug-transactions      Transactions module debug mode");
    println!("    --debug-webserver         Webserver operations debug mode");
    println!("    --debug-websocket         WebSocket connection debug mode");
    println!("    --debug-wallet            Wallet operations debug mode");
    println!();
    println!("EXAMPLES:");
    println!("    screenerbot --run                           # Start bot normally");
    println!("    screenerbot --run --dry-run                 # Start bot in simulation mode");
    println!("    screenerbot --run --debug-trader --dry-run  # Debug trader with simulation");
    println!("    screenerbot --reset                         # Reset with confirmation");
    println!("    screenerbot --reset --force                 # Reset without confirmation");
    println!("    screenerbot --clear-all                     # Clear all data and reset");
    println!("    screenerbot --positions-sell-all            # Sell all open positions");
    println!("    screenerbot --add-to-blacklist <mint>       # Add mint to blacklist");
    println!("    screenerbot --help                          # Show this help");
    println!();
    println!("For more information, visit: https://github.com/farfary/ScreenerBot");
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
            modes.push(module.to_string());
        }
    }

    // Include other modes
    if is_dry_run_enabled() {
        modes.push("dry-run".to_string());
    }
    if is_clear_all_enabled() {
        modes.push("clear-all".to_string());
    }
    if is_positions_sell_all_enabled() {
        modes.push("positions-sell-all".to_string());
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
