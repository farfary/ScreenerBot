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
    get_cmd_args()
        .iter()
        .any(|a| a == arg)
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
// DEBUG FLAG CHECKING FUNCTIONS
// These functions check for specific debug flags in the command-line arguments
// =============================================================================

/// Filtering module debug mode
pub fn is_debug_filtering_enabled() -> bool {
    has_arg("--debug-filtering")
}

/// Profit calculation debug mode
pub fn is_debug_profit_enabled() -> bool {
    has_arg("--debug-profit")
}

/// Pool prices debug mode
pub fn is_debug_pool_prices_enabled() -> bool {
    has_arg("--debug-pool-prices")
}

/// Pool calculator debug mode
pub fn is_debug_pool_calculator_enabled() -> bool {
    has_arg("--debug-pool-calculator")
}

/// Trader module debug mode
pub fn is_debug_trader_enabled() -> bool {
    has_arg("--debug-trader")
}

/// API calls debug mode
pub fn is_debug_api_enabled() -> bool {
    has_arg("--debug-api")
}

/// Monitor module debug mode
pub fn is_debug_monitor_enabled() -> bool {
    has_arg("--debug-monitor")
}

/// Discovery module debug mode
pub fn is_debug_discovery_enabled() -> bool {
    has_arg("--debug-discovery")
}

/// Price service debug mode
pub fn is_debug_price_service_enabled() -> bool {
    has_arg("--debug-price-service")
}

/// Rugcheck module debug mode
pub fn is_debug_rugcheck_enabled() -> bool {
    has_arg("--debug-rugcheck")
}

/// Entry module debug mode
pub fn is_debug_entry_enabled() -> bool {
    has_arg("--debug-entry")
}

/// OHLCV analysis debug mode
pub fn is_debug_ohlcv_enabled() -> bool {
    has_arg("--debug-ohlcv")
}

/// Wallet operations debug mode
pub fn is_debug_wallet_enabled() -> bool {
    has_arg("--debug-wallet")
}

/// Swap operations debug mode
pub fn is_debug_swaps_enabled() -> bool {
    has_arg("--debug-swaps")
}

/// Decimals module debug mode
pub fn is_debug_decimals_enabled() -> bool {
    has_arg("--debug-decimals")
}

/// Summary module debug mode
pub fn is_debug_summary_enabled() -> bool {
    has_arg("--debug-summary")
}

/// Summary logging debug mode - writes summary tables to log file
pub fn is_debug_summary_logging_enabled() -> bool {
    has_arg("--debug-summary-logging")
}

/// Transactions module debug mode
pub fn is_debug_transactions_enabled() -> bool {
    has_arg("--debug-transactions")
}

/// RPC operations debug mode
pub fn is_debug_rpc_enabled() -> bool {
    has_arg("--debug-rpc")
}

/// Positions module debug mode
pub fn is_debug_positions_enabled() -> bool {
    has_arg("--debug-positions")
}

/// ATA operations debug mode
pub fn is_debug_ata_enabled() -> bool {
    has_arg("--debug-ata")
}

/// System operations debug mode
pub fn is_debug_system_enabled() -> bool {
    has_arg("--debug-system")
}

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

/// Summary mode - enables console output from summary module
pub fn is_summary_enabled() -> bool {
    has_arg("--summary")
}

/// Dashboard mode - enables terminal UI and disables all console logging
pub fn is_dashboard_enabled() -> bool {
    has_arg("--dashboard")
}

/// Run mode - enables actual bot execution (required to start services)
/// SAFETY: Bot execution disabled - always returns false to prevent accidental trading
pub fn is_run_enabled() -> bool {
    false // Bot execution permanently disabled for safety
}

/// Clear all mode - clears all data and resets the system
pub fn is_clear_all_enabled() -> bool {
    has_arg("--clear-all")
}

/// Positions sell all mode - sells all open positions
pub fn is_positions_sell_all_enabled() -> bool {
    has_arg("--positions-sell-all")
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
    println!("    --positions-sell-all      Sell all open positions");
    println!("    --help, -h                Show this help message");
    println!("    --dry-run                 Simulate trading without executing transactions");
    println!("    --dashboard               Enable terminal UI mode");
    println!("    --summary                 Enable console output from summary module");
    println!();
    println!("DEBUG FLAGS:");
    println!("    --debug-api               API calls debug mode");
    println!("    --debug-decimals          Decimals module debug mode");
    println!("    --debug-discovery         Discovery module debug mode");
    println!("    --debug-entry             Entry module debug mode");
    println!("    --debug-filtering         Filtering module debug mode");
    println!("    --debug-monitor           Monitor module debug mode");
    println!("    --debug-ohlcv             OHLCV analysis debug mode");
    println!("    --debug-pool-calculator   Pool calculator debug mode");
    println!("    --debug-pool-prices       Pool prices debug mode");
    println!("    --debug-positions         Positions module debug mode");
    println!("    --debug-price-service     Price service debug mode");
    println!("    --debug-profit            Profit calculation debug mode");
    println!("    --debug-rpc               RPC operations debug mode");
    println!("    --debug-rugcheck          Rugcheck module debug mode");
    println!("    --debug-summary           Summary module debug mode");
    println!("    --debug-summary-logging   Write summary tables to log file");
    println!("    --debug-swaps             Swap operations debug mode");
    println!("    --debug-system            System operations debug mode");
    println!("    --debug-trader            Trader module debug mode");
    println!("    --debug-transactions      Transactions module debug mode");
    println!("    --debug-wallet            Wallet operations debug mode");
    println!();
    println!("EXAMPLES:");
    println!("    screenerbot --run                           # Start bot normally");
    println!("    screenerbot --run --dry-run                 # Start bot in simulation mode");
    println!("    screenerbot --run --dashboard               # Start bot with terminal UI");
    println!("    screenerbot --run --summary                 # Start bot with console summary");
    println!("    screenerbot --run --debug-trader --dry-run  # Debug trader with simulation");
    println!("    screenerbot --clear-all                     # Clear all data and reset");
    println!("    screenerbot --positions-sell-all            # Sell all open positions");
    println!("    screenerbot --help                          # Show this help");
    println!();
    println!("For more information, visit: https://github.com/farfary/ScreenerBot");
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Checks if any debug mode is enabled
pub fn is_any_debug_enabled() -> bool {
    is_debug_filtering_enabled() ||
        is_debug_profit_enabled() ||
        is_debug_pool_prices_enabled() ||
        is_debug_pool_calculator_enabled() ||
        is_debug_trader_enabled() ||
        is_debug_api_enabled() ||
        is_debug_monitor_enabled() ||
        is_debug_discovery_enabled() ||
        is_debug_price_service_enabled() ||
        is_debug_rugcheck_enabled() ||
        is_debug_entry_enabled() ||
        is_debug_ohlcv_enabled() ||
        is_debug_wallet_enabled() ||
        is_debug_swaps_enabled() ||
        is_debug_decimals_enabled() ||
        is_debug_summary_enabled() ||
        is_debug_summary_logging_enabled() ||
        is_debug_transactions_enabled() ||
        is_debug_rpc_enabled() ||
        is_debug_positions_enabled() ||
        is_debug_ata_enabled()
}

/// Gets a list of all enabled debug modes
pub fn get_enabled_debug_modes() -> Vec<&'static str> {
    let mut modes = Vec::new();

    if is_debug_filtering_enabled() {
        modes.push("filtering");
    }
    if is_debug_profit_enabled() {
        modes.push("profit");
    }
    if is_debug_pool_prices_enabled() {
        modes.push("pool-prices");
    }
    if is_debug_pool_calculator_enabled() {
        modes.push("pool-calculator");
    }
    if is_debug_trader_enabled() {
        modes.push("trader");
    }
    if is_debug_api_enabled() {
        modes.push("api");
    }
    if is_debug_monitor_enabled() {
        modes.push("monitor");
    }
    if is_debug_discovery_enabled() {
        modes.push("discovery");
    }
    if is_debug_price_service_enabled() {
        modes.push("price-service");
    }
    if is_debug_rugcheck_enabled() {
        modes.push("rugcheck");
    }
    if is_debug_entry_enabled() {
        modes.push("entry");
    }
    if is_debug_ohlcv_enabled() {
        modes.push("ohlcv");
    }
    if is_debug_wallet_enabled() {
        modes.push("wallet");
    }
    if is_debug_swaps_enabled() {
        modes.push("swaps");
    }
    if is_debug_decimals_enabled() {
        modes.push("decimals");
    }
    if is_debug_summary_enabled() {
        modes.push("summary");
    }
    if is_debug_summary_logging_enabled() {
        modes.push("summary-logging");
    }
    if is_debug_transactions_enabled() {
        modes.push("transactions");
    }
    if is_debug_rpc_enabled() {
        modes.push("rpc");
    }
    if is_debug_positions_enabled() {
        modes.push("positions");
    }
    if is_dry_run_enabled() {
        modes.push("dry-run");
    }
    if is_summary_enabled() {
        modes.push("summary");
    }
    if is_dashboard_enabled() {
        modes.push("dashboard");
    }
    if is_clear_all_enabled() {
        modes.push("clear-all");
    }
    if is_positions_sell_all_enabled() {
        modes.push("positions-sell-all");
    }

    modes
}

/// Prints debug information about current arguments and enabled debug modes
pub fn print_debug_info() {
    let args = get_cmd_args();
    if !is_dashboard_enabled() {
        println!("Command-line arguments: {:?}", args);
    }

    let enabled_modes = get_enabled_debug_modes();
    if !is_dashboard_enabled() {
        if enabled_modes.is_empty() {
            println!("No debug modes enabled");
        } else {
            println!("Enabled debug modes: {:?}", enabled_modes);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_args() {
        let test_args = vec![
            "screenerbot".to_string(),
            "--debug-trader".to_string(),
            "--mint".to_string(),
            "test_mint_address".to_string()
        ];

        set_cmd_args(test_args.clone());
        let retrieved_args = get_cmd_args();

        assert_eq!(retrieved_args, test_args);
    }

    #[test]
    fn test_has_arg() {
        set_cmd_args(vec!["screenerbot".to_string(), "--debug-trader".to_string()]);

        assert!(has_arg("--debug-trader"));
        assert!(!has_arg("--debug-profit"));
    }

    #[test]
    fn test_get_arg_value() {
        set_cmd_args(
            vec!["screenerbot".to_string(), "--mint".to_string(), "test_mint_address".to_string()]
        );

        assert_eq!(get_arg_value("--mint"), Some("test_mint_address".to_string()));
        assert_eq!(get_arg_value("--symbol"), None);
    }

    #[test]
    fn test_debug_flags() {
        set_cmd_args(
            vec![
                "screenerbot".to_string(),
                "--debug-trader".to_string(),
                "--debug-profit".to_string(),
                "--debug-summary-logging".to_string(),
                "--dry-run".to_string()
            ]
        );

        assert!(is_debug_trader_enabled());
        assert!(is_debug_profit_enabled());
        assert!(is_debug_summary_logging_enabled());
        assert!(!is_debug_filtering_enabled());
        assert!(is_dry_run_enabled());
        assert!(is_any_debug_enabled());

        let enabled_modes = get_enabled_debug_modes();
        assert!(enabled_modes.contains(&"trader"));
        assert!(enabled_modes.contains(&"profit"));
        assert!(enabled_modes.contains(&"summary-logging"));
        assert!(enabled_modes.contains(&"dry-run"));
        assert!(!enabled_modes.contains(&"filtering"));
    }

    #[test]
    fn test_patterns() {
        set_cmd_args(
            vec![
                "screenerbot".to_string(),
                "--help".to_string(),
                "--duration".to_string(),
                "300".to_string()
            ]
        );

        assert!(patterns::is_help_requested());
        assert_eq!(patterns::get_duration_seconds(), Some(300));
        assert!(!patterns::is_version_requested());
    }
}
