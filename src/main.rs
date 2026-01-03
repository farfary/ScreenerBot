//! ScreenerBot - Automated Solana DeFi Trading Bot
//!
//! This is the main entry point for the ScreenerBot application.
//! The bot runs as a headless server with a web-based dashboard.

use screenerbot::arguments::{print_help, print_version, set_cmd_args};
use screenerbot::config::utils::load_config;
use screenerbot::logger::{error, info, LogTag};
use screenerbot::run::run_bot;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Global flag to signal shutdown
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Check if shutdown was requested
pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_FLAG.load(Ordering::SeqCst)
}

/// Request application shutdown
pub fn request_shutdown() {
    SHUTDOWN_FLAG.store(true, Ordering::SeqCst);
}

#[tokio::main]
async fn main() {
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

    // Initialize logger
    screenerbot::logger::init();

    // Load configuration
    if let Err(e) = load_config() {
        error(
            LogTag::System,
            &format!("Failed to load configuration: {e}"),
        );
        return;
    }

    info(LogTag::System, "ScreenerBot starting...");

    // Set up shutdown signal handler
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
        info(LogTag::System, "Shutdown signal received");
        shutdown_flag_clone.store(true, Ordering::SeqCst);
        request_shutdown();
    });

    // Run the bot in headless mode
    if let Err(e) = run_bot().await {
        error(LogTag::System, &format!("Bot error: {e}"));
    }

    info(LogTag::System, "ScreenerBot shutdown complete");
}
