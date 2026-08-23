//! Process boot sequence: CLI early-exits, logger, config, panic hook,
//! shutdown signal, then the bot lifecycle.

use crate::{
    arguments::{print_banner, print_help, print_version, set_cmd_args},
    config::utils::load_config,
    logger::{self, LogTag},
};

/// Full process boot sequence: CLI early-exits, banner, logger, config, panic
/// hook, ctrl-c shutdown signal, one-shot wallet reset, run the bot, and
/// restart-after-graceful-shutdown handling. Called once from `main()`.
pub async fn boot() {
    // Store command line arguments
    set_cmd_args(std::env::args().collect());

    // Handle help flag
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    // Handle version flag
    if std::env::args().any(|arg| arg == "--version" || arg == "-v") {
        print_version();
        return;
    }

    print_banner();

    // Initialize logger
    crate::logger::init();
    logger::info(
        LogTag::System,
        "Logger initialized, attempting to load config...",
    );

    // Load configuration
    if let Err(e) = load_config() {
        logger::error(
            LogTag::System,
            &format!("Failed to load configuration: {e}"),
        );
        return;
    }
    logger::info(LogTag::System, "Configuration loaded successfully");

    // Detect and log the system/network proxy once (used by HTTP, RPC and WS).
    // Must run AFTER config load so `[network] proxy` is available.
    crate::net::log_detected_proxy();

    // Set up panic hook for crash notifications (after config is loaded)
    crate::process::panic_hook::install();

    logger::info(LogTag::System, "ScreenerBot starting...");

    // Set up shutdown signal handler
    super::shutdown::spawn_initial_ctrl_c_listener();

    // Handle one-shot wallet-data reset (safe: backs up before clearing) then
    // continue into a normal boot, so a single relaunch fixes a wallet mismatch.
    // This is what the GUI's "Reset wallet data & restart" and the documented
    // `screenerbot --clean-wallet-data` both trigger.
    if crate::arguments::is_clean_wallet_data_enabled() {
        match crate::wallets::recovery::backup_and_clean_wallet_data() {
            Ok(backup_dir) => logger::info(
                LogTag::System,
                &format!(
                    "Wallet data reset complete (backup: {}). Continuing startup.",
                    backup_dir.display()
                ),
            ),
            Err(e) => logger::error(
                LogTag::System,
                &format!("Wallet data reset failed: {e}. Continuing startup anyway."),
            ),
        }
    }

    // Run the bot in headless mode. On a fatal startup failure, surface a
    // structured error to the terminal/log file and the GUI shell, then exit
    // with a non-zero code so callers (Electron, systemd) can tell a failed
    // boot apart from a clean shutdown.
    if let Err(e) = super::run_bot().await {
        e.emit();
        std::process::exit(1);
    }

    if crate::global::is_restart_requested() {
        crate::process::restart::restart_after_graceful_shutdown();
    }

    logger::info(LogTag::System, "ScreenerBot shutdown complete");
}
