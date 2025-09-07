use screenerbot::{
    arguments::{
        is_clear_all_enabled, is_dry_run_enabled, is_get_list_tools_enabled,
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
        println!("Error: {}", e);
        println!("Use --help to see all available options");
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
            println!("Clear all functionality not yet implemented");
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
            println!("Positions sell all functionality not yet implemented");
            Ok(())
        }
        BotMode::GetListTools => {
            log(
                LogTag::System,
                "INFO",
                "🔧 Starting ScreenerBot in GET-LIST-TOOLS mode",
            );

            // Display available MCP tools
            display_mcp_tools();
            Ok(())
        }
        BotMode::None => {
            let error_msg = "No valid mode specified";
            log(LogTag::System, "ERROR", error_msg);
            println!("Error: {}", error_msg);
            println!("Use --help to see all available options");
            println!();
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
            println!("Error: {}", e);
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
    GetListTools,
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
    } else if is_get_list_tools_enabled() {
        BotMode::GetListTools
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
    if is_get_list_tools_enabled() {
        mode_count += 1;
    }

    if mode_count == 0 {
        return Err(
            "No execution mode specified. Use --run, --clear-all, --positions-sell-all, or --get-list-tools"
                .to_string(),
        );
    }

    if mode_count > 1 {
        return Err("Multiple execution modes specified. Use only one of: --run, --clear-all, --positions-sell-all, --get-list-tools".to_string());
    }

    // Validate that --dry-run is only used with --run
    if is_dry_run_enabled() && !is_run_enabled() {
        return Err("--dry-run can only be used with --run mode".to_string());
    }

    // Validate that --dashboard and --summary are only used with --run
    if (screenerbot::arguments::is_dashboard_enabled()
        || screenerbot::arguments::is_summary_enabled())
        && !is_run_enabled()
    {
        return Err("--dashboard and --summary can only be used with --run mode".to_string());
    }

    Ok(())
}

/// Displays available MCP tools by reading the MCP server configuration
fn display_mcp_tools() {
    use std::fs;
    use std::path::Path;

    println!("🔧 Available MCP Tools:");
    println!();

    // Try to read the MCP tools from the TypeScript file
    let mcp_file_path = Path::new("mcp/src/index.ts");

    if mcp_file_path.exists() {
        match fs::read_to_string(mcp_file_path) {
            Ok(content) => {
                // Extract tool names from the tools array
                let mut tools = Vec::new();
                let lines: Vec<&str> = content.lines().collect();

                for line in lines.iter() {
                    if line.trim().starts_with("name: \"") {
                        // Extract tool name from line like: name: "tool_name",
                        if let Some(start) = line.find("\"") {
                            if let Some(end) = line[start + 1..].find("\"") {
                                let tool_name = &line[start + 1..start + 1 + end];
                                tools.push(tool_name.to_string());
                            }
                        }
                    }
                }

                if tools.is_empty() {
                    println!("❌ No tools found in MCP configuration");
                } else {
                    println!("📋 Found {} MCP tools:", tools.len());
                    println!();

                    for (i, tool) in tools.iter().enumerate() {
                        println!("  {}. {}", i + 1, tool);
                    }
                }
            }
            Err(e) => {
                println!("❌ Error reading MCP configuration: {}", e);
            }
        }
    } else {
        println!(
            "❌ MCP configuration file not found at: {}",
            mcp_file_path.display()
        );
        println!("   Expected location: mcp/src/index.ts");
    }

    println!();
    println!("💡 To use these tools, start the MCP server and connect via Claude Desktop");
    println!("   or use the MCP client tools directly.");
}
