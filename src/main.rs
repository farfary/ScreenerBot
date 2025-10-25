use screenerbot::{
    arguments::{
        is_dry_run_enabled, is_force_enabled, is_reset_enabled, is_run_enabled, patterns,
        print_debug_info, print_help,
    },
    logger::{self as logger, LogTag},
};

/// Main entry point for ScreenerBot
///
/// Routes execution based on command-line arguments:
/// - `--help`: Display help information and exit
/// - `--reset [--force]`: Reset database state and exit
/// - `--run [--dry-run]`: Start the trading bot
#[tokio::main]
async fn main() {
    // Initialize logger system first (required for all operations)
    logger::init();

    // Check for help request first (before any other processing)
    if patterns::is_help_requested() {
        print_help();
        std::process::exit(0);
    }

    // Log startup information
    logger::info(LogTag::System, "🚀 ScreenerBot starting up...");

    // Print debug information if any debug modes are enabled
    print_debug_info();

    // Reset mode - execute and exit
    if is_reset_enabled() {
        logger::info(LogTag::System, "🔄 Reset mode enabled");

        let config = screenerbot::reset::ResetConfig {
            force: is_force_enabled(),
            ..Default::default()
        };

        match screenerbot::reset::execute_extended_reset(config) {
            Ok(()) => {
                logger::info(LogTag::System, "✅ Reset completed successfully");
                std::process::exit(0);
            }
            Err(e) => {
                logger::error(LogTag::System, &format!("❌ Reset failed: {}", e));
                std::process::exit(1);
            }
        }
    }

    // Validate argument combinations
    if let Err(e) = validate_arguments() {
        logger::error(
            LogTag::System,
            &format!("Argument validation failed: {}", e),
        );
        logger::error(LogTag::System, &format!("Error: {}", e));
        logger::info(LogTag::System, "Use --help to see all available options");
        std::process::exit(1);
    }

    // Route to appropriate bot state based on arguments
    let result = if is_run_enabled() {
        logger::info(LogTag::System, "🚀 Starting ScreenerBot in RUN mode");

        // Log dry-run status prominently if enabled
        if is_dry_run_enabled() {
            logger::info(
                LogTag::System,
                "🚫 DRY-RUN MODE ENABLED - NO ACTUAL TRADING WILL OCCUR",
            );
        }

        // Call the run function from run.rs
        screenerbot::run::run_bot().await
    } else {
        logger::error(LogTag::System, "No valid execution mode specified");
        logger::info(
            LogTag::System,
            "Use --run to start the bot, or --help to see all options",
        );
        print_help();
        std::process::exit(1);
    };

    // Handle the result
    match result {
        Ok(_) => {
            logger::info(LogTag::System, "✅ ScreenerBot completed successfully");
        }
        Err(e) => {
            logger::error(LogTag::System, &format!("❌ ScreenerBot failed: {}", e));
            std::process::exit(1);
        }
    }
}

/// Validates command line arguments for --run mode
fn validate_arguments() -> Result<(), String> {
    if !is_run_enabled() {
        return Err("--run flag is required".to_string());
    }

    Ok(())
}
