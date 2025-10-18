use screenerbot::{
    arguments::{is_dry_run_enabled, is_run_enabled, patterns, print_debug_info, print_help},
    logger::{init_file_logging, log, LogTag},
};

/// Main entry point for ScreenerBot
///
/// This function handles argument routing to different bot states:
/// - --run: Main bot execution
/// - --help: Display help information
#[tokio::main]
async fn main() {
    // Initialize file logging system first (required for all operations)
    init_file_logging();

    // Check for help request first (before any other processing)
    if patterns::is_help_requested() {
        print_help();
        std::process::exit(0);
    }

    // Log startup information
    log(LogTag::System, "INFO", "🚀 ScreenerBot starting up...");

    // Print debug information if any debug modes are enabled
    print_debug_info();

    // Validate argument combinations
    if let Err(e) = validate_arguments() {
        log(
            LogTag::System,
            "ERROR",
            &format!("Argument validation failed: {}", e),
        );
        log(LogTag::System, "ERROR", &format!("Error: {}", e));
        log(
            LogTag::System,
            "INFO",
            "Use --help to see all available options",
        );
        std::process::exit(1);
    }

    // Route to appropriate bot state based on arguments
    let result = if is_run_enabled() {
        log(
            LogTag::System,
            "INFO",
            "🚀 Starting ScreenerBot in RUN mode",
        );

        // Log dry-run status prominently if enabled
        if is_dry_run_enabled() {
            log(
                LogTag::System,
                "CRITICAL",
                "🚫 DRY-RUN MODE ENABLED - NO ACTUAL TRADING WILL OCCUR",
            );
        }

        // Call the run function from run.rs
        screenerbot::run::run_bot().await
    } else {
        let error_msg = "No valid mode specified";
        log(LogTag::System, "ERROR", error_msg);
        log(LogTag::System, "ERROR", &format!("Error: {}", error_msg));
        log(
            LogTag::System,
            "INFO",
            "Use --help to see all available options",
        );
        print_help();
        std::process::exit(1);
    };

    // Handle the result
    match result {
        Ok(_) => {
            log(
                LogTag::System,
                "INFO",
                "✅ ScreenerBot completed successfully",
            );
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("ScreenerBot failed: {}", e),
            );
            log(LogTag::System, "ERROR", &format!("Error: {}", e));
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
