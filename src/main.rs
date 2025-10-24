use screenerbot::{
    arguments::{is_dry_run_enabled, is_run_enabled, patterns, print_debug_info, print_help},
    logger::{self as logger, LogTag},
};

/// Main entry point for ScreenerBot
///
/// This function handles argument routing to different bot states:
/// - --run: Main bot execution
/// - --help: Display help information
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
        let error_msg = "No valid mode specified";
        logger::error(LogTag::System, error_msg);
        logger::error(LogTag::System, &format!("Error: {}", error_msg));
        logger::info(LogTag::System, "Use --help to see all available options");
        print_help();
        std::process::exit(1);
    };

    // Handle the result
    match result {
        Ok(_) => {
            logger::info(LogTag::System, "✅ ScreenerBot completed successfully");
        }
        Err(e) => {
            logger::error(LogTag::System, &format!("ScreenerBot failed: {}", e));
            logger::error(LogTag::System, &format!("Error: {}", e));
            std::process::exit(1);
        }
    }
}

/// Validates command line arguments for consistency and conflicts
fn validate_arguments() -> Result<(), String> {
    // Validate that --run is specified
    if !is_run_enabled() {
        return Err("No execution mode specified. Use --run".to_string());
    }

    Ok(())
}
