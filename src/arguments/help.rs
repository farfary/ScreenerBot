//! Help display and debug info output.

use crate::logger::{self, LogTag};
use super::{get_cmd_args, modes::*};

/// Displays the help menu with all available flags and their descriptions.
pub fn print_help() {
    println!("ScreenerBot - Advanced Solana DeFi Trading Bot");
    println!();
    println!("USAGE:");
    println!("    screenerbot [OPTIONS]");
    println!();
    println!("    By default, ScreenerBot starts the trading bot with webserver on http://localhost:8080");
    println!();
    println!("SPECIAL MODES (execute and exit):");
    println!(
        "    --reset                     Reset pending verifications and delete database files"
    );
    println!("    --reset-default-configs     Reset all config to defaults (preserves wallet + RPC URLs)");
    println!("    --clean-wallet-data         Clean all wallet-specific databases (use when switching wallets)");
    println!("    --help, -h                  Show this help message");
    println!();
    println!("DISPLAY OPTIONS:");
    println!("    --gui                       Launch with desktop GUI window");
    println!(
        "                                Without --gui, runs headless with webserver on port 8080"
    );
    println!("    --dashboard-demo            Show hardcoded demo data for screenshots/marketing");
    println!(
        "    --dashboard-onboarding      Force show onboarding screens (resets onboarding state)"
    );
    println!();
    println!("WEBSERVER CONFIGURATION:");
    println!("    --port <PORT>               Override webserver port (1-65535, default: 8080)");
    println!("                                Invalid values will cause bot to exit with error");
    println!("                                Note: GUI mode ignores this and uses dynamic port");
    println!("    --host <HOST>               Override webserver host (default: 127.0.0.1)");
    println!("                                IPv4 addresses only. Use 0.0.0.0 for remote access");
    println!();
    println!("    Examples:");
    println!("      screenerbot --port 9000");
    println!("      screenerbot --host 0.0.0.0");
    println!("      screenerbot --port 3000 --host 0.0.0.0");
    println!();
    println!("MODIFIERS:");
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
    println!("    screenerbot                                  # Start bot (headless, webserver on :8080)");
    println!(
        "    screenerbot --gui                            # Start bot with desktop GUI window"
    );
    println!("    screenerbot --gui --dashboard-demo           # Demo mode for screenshots");
    println!("    screenerbot --debug-trader                   # Start bot with trader debug logs");
    println!("    screenerbot --reset                          # Reset with confirmation prompt");
    println!("    screenerbot --reset --force                  # Reset without confirmation");
    println!("    screenerbot --reset-default-configs          # Reset config to defaults");
    println!(
        "    screenerbot --clean-wallet-data              # Clean databases when switching wallets"
    );
    println!();
    println!("BUILDING:");
    println!("    cargo build                                  # Build complete binary (GUI always included)");
    println!("    cargo build --release                        # Build optimized release version");
}

/// Prints version information to stdout (for --version flag).
/// This MUST print to stdout (not logger) so install scripts can parse it.
pub fn print_version() {
    println!("ScreenerBot v{}", env!("CARGO_PKG_VERSION"));
}

/// Prints debug information about current arguments and enabled debug modes.
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

/// Gets a list of all enabled debug modes by checking command-line arguments.
pub fn get_enabled_debug_modes() -> Vec<String> {
    let mut modes = Vec::new();
    let args = get_cmd_args();

    for arg in &args {
        if let Some(module) = arg.strip_prefix("--debug-") {
            modes.push(format!("debug-{module}"));
        } else if let Some(module) = arg.strip_prefix("--verbose-") {
            modes.push(format!("verbose-{module}"));
        }
    }

    if is_reset_enabled() {
        modes.push("reset".to_owned());
    }
    if is_gui_enabled() {
        modes.push("gui".to_owned());
    }
    if is_dashboard_demo_enabled() {
        modes.push("dashboard-demo".to_owned());
    }
    if is_force_enabled() {
        modes.push("force".to_owned());
    }
    if is_cache_only_enabled() {
        modes.push("cache-only".to_owned());
    }
    if is_force_refresh_enabled() {
        modes.push("force-refresh".to_owned());
    }
    if is_profile_cpu_enabled() {
        modes.push("profile-cpu".to_owned());
    }
    if is_profile_tokio_console_enabled() {
        modes.push("profile-tokio-console".to_owned());
    }
    if is_profile_tracing_enabled() {
        modes.push("profile-tracing".to_owned());
    }

    modes
}
