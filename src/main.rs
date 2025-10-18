use screenerbot::{
    arguments::{
        get_blacklist_mint, is_add_to_blacklist_enabled, is_clear_all_enabled, is_dry_run_enabled,
        is_positions_sell_all_enabled, is_run_enabled, patterns, print_debug_info, print_help,
    },
    logger::{init_file_logging, log, LogTag},
};

/// Main entry point for ScreenerBot
///
/// This function handles argument routing to different bot states:
/// - --run: Main bot execution
/// - --clear-all: Clear all data and reset system
/// - --positions-sell-all: Sell all open positions
/// - --add-to-blacklist <mint>: Add mint address to blacklist
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
    let result = match get_bot_mode() {
        BotMode::Run => {
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
        }
        BotMode::ClearAll => {
            log(
                LogTag::System,
                "INFO",
                "🧹 Starting ScreenerBot in CLEAR-ALL mode",
            );

            // TODO: Implement clear all functionality
            log(
                LogTag::System,
                "INFO",
                "Clear all functionality not yet implemented",
            );
            Ok(())
        }
        BotMode::PositionsSellAll => {
            log(
                LogTag::System,
                "INFO",
                "💰 Starting ScreenerBot in POSITIONS-SELL-ALL mode",
            );

            // TODO: Implement positions sell all functionality
            log(
                LogTag::System,
                "INFO",
                "Positions sell all functionality not yet implemented",
            );
            Ok(())
        }
        BotMode::AddToBlacklist => {
            log(
                LogTag::System,
                "INFO",
                "📃 Starting ScreenerBot in ADD-TO-BLACKLIST mode",
            );

            // Handle blacklist addition
            handle_add_to_blacklist().await
        }
        BotMode::None => {
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
        }
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

/// Represents the different bot execution modes
#[derive(Debug, Clone, Copy)]
enum BotMode {
    Run,
    ClearAll,
    PositionsSellAll,
    AddToBlacklist,
    None,
}

/// Determines which bot mode should be executed based on command line arguments
fn get_bot_mode() -> BotMode {
    if is_run_enabled() {
        BotMode::Run
    } else if is_clear_all_enabled() {
        BotMode::ClearAll
    } else if is_positions_sell_all_enabled() {
        BotMode::PositionsSellAll
    } else if is_add_to_blacklist_enabled() {
        BotMode::AddToBlacklist
    } else {
        BotMode::None
    }
}

/// Validates command line arguments for consistency and conflicts
fn validate_arguments() -> Result<(), String> {
    // Check for conflicting modes
    let mut mode_count = 0;
    if is_run_enabled() {
        mode_count += 1;
    }
    if is_clear_all_enabled() {
        mode_count += 1;
    }
    if is_positions_sell_all_enabled() {
        mode_count += 1;
    }
    if is_add_to_blacklist_enabled() {
        mode_count += 1;
    }

    if mode_count == 0 {
        return Err(
            "No execution mode specified. Use --run, --clear-all, --positions-sell-all, or --add-to-blacklist".to_string()
        );
    }

    if mode_count > 1 {
        return Err(
            "Multiple execution modes specified. Use only one of: --run, --clear-all, --positions-sell-all, --add-to-blacklist".to_string()
        );
    }

    // Validate that --dry-run is only used with --run
    if is_dry_run_enabled() && !is_run_enabled() {
        return Err("--dry-run can only be used with --run mode".to_string());
    }

    Ok(())
}

/// Handle adding a mint address to the blacklist
async fn handle_add_to_blacklist() -> Result<(), String> {
    // Get the mint address from command line arguments
    let mint = match get_blacklist_mint() {
        Some(mint) => mint,
        None => {
            return Err(
                "No mint address provided. Usage: --add-to-blacklist <mint_address>".to_string(),
            );
        }
    };

    // Validate mint address format (should be 44 characters for base58)
    if mint.len() != 44 {
        return Err(format!(
            "Invalid mint address format: {}. Expected 44-character base58 string",
            mint
        ));
    }

    // Try to parse as Pubkey to validate it's a proper Solana address
    if let Err(_) = mint.parse::<solana_sdk::pubkey::Pubkey>() {
        return Err(format!("Invalid Solana mint address: {}", mint));
    }

    log(
        LogTag::System,
        "INFO",
        &format!("Adding mint {} to blacklist...", mint),
    );

    // Initialize blacklist system
    if let Err(e) = screenerbot::tokens::blacklist::initialize_blacklist_system() {
        return Err(format!("Failed to initialize blacklist system: {}", e));
    }

    // Add to blacklist with ManualBlacklist reason
    let success = screenerbot::tokens::blacklist::add_to_blacklist_db(
        &mint,
        "Manual", // Symbol placeholder for manual additions
        screenerbot::tokens::blacklist::BlacklistReason::ManualBlacklist,
    );

    if success {
        log(
            LogTag::System,
            "INFO",
            &format!("✅ Successfully added {} to blacklist", mint),
        );
        println!("✅ Successfully added {} to blacklist", mint);
        Ok(())
    } else {
        let error_msg = format!("❌ Failed to add {} to blacklist", mint);
        log(LogTag::System, "ERROR", &error_msg);
        Err(error_msg)
    }
}
