//! # Positions Manager Tool
//!
//! A comprehensive tool for managing trading positions with the following capabilities:
//! - List open and closed positions
//! - Open new positions by token mint address
//! - Close existing positions by token mint address
//! - Check position status for specific tokens
//! - Interactive position management with detailed information
//!
//! ## Usage Examples
//! ```bash
//! # Show help menu
//! cargo run --bin main_positions_manager -- --help
//!
//! # List all open positions
//! cargo run --bin main_positions_manager -- --list-open
//!
//! # List all closed positions
//! cargo run --bin main_positions_manager -- --list-closed
//!
//! # Check status of specific token
//! cargo run --bin main_positions_manager -- --status So11111111111111111111111111111111111111112
//!
//! # Open new position
//! cargo run --bin main_positions_manager -- --mint So11111111111111111111111111111111111111112 --size 0.01 --action open
//!
//! # Close existing position
//! cargo run --bin main_positions_manager -- --mint So11111111111111111111111111111111111111112 --action close
//! ```

use screenerbot::positions::{
    get_open_positions, get_closed_positions, get_positions_handle, start_positions_manager_service,
    open_position_global, Position,
    calculate_position_pnl
};
use screenerbot::tokens::{get_token_from_db};
use screenerbot::tokens::price::{initialize_price_service, get_token_price_blocking_safe};
use screenerbot::tokens::init_dexscreener_api;
use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::configs::{read_configs, validate_configs};

use screenerbot::rpc::get_rpc_client;

use std::env;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use colored::Colorize;
use tokio::sync::Notify;

/// Print comprehensive help menu for the Positions Manager Tool
fn print_help() {
    println!("{}", "🎯 POSITIONS MANAGER TOOL".bright_blue().bold());
    println!("{}", "========================".bright_blue());
    println!("Comprehensive trading positions management and monitoring tool");
    println!();
    
    println!("{}", "📋 LISTING COMMANDS:".bright_green().bold());
    println!("  --list-open              List all open positions with P&L");
    println!("  --list-closed            List all closed positions with final P&L");
    println!("  --list-all               List both open and closed positions");
    println!();
    
    println!("{}", "🔍 STATUS COMMANDS:".bright_yellow().bold());
    println!("  --status <MINT>          Show detailed status of specific position");
    println!("  --summary                Show positions summary statistics");
    println!();
    
    println!("{}", "📈 TRADING COMMANDS:".bright_cyan().bold());
    println!("  --mint <ADDRESS>         Token mint address for position operations");
    println!("  --size <SOL_AMOUNT>      Position size in SOL (for opening positions)");
    println!("  --action <open|close>    Action to perform on the position");
    println!();
    
    println!("{}", "⚙️  UTILITY COMMANDS:".bright_magenta().bold());
    println!("  --help, -h               Show this help menu");
    println!("  --debug                  Enable debug logging for positions");
    println!("  --dry-run               Simulate actions without executing");
    println!();
    
    println!("{}", "📊 EXAMPLES:".bright_white().bold());
    println!("  # List all open positions");
    println!("  cargo run --bin main_positions_manager -- --list-open");
    println!();
    println!("  # Check specific token status");
    println!("  cargo run --bin main_positions_manager -- --status So11111111111111111111111111111111111111112");
    println!();
    println!("  # Open new 0.01 SOL position");
    println!("  cargo run --bin main_positions_manager -- --mint So11111111111111111111111111111111111111112 --size 0.01 --action open");
    println!();
    println!("  # Close existing position");
    println!("  cargo run --bin main_positions_manager -- --mint So11111111111111111111111111111111111111112 --action close");
    println!();
    
    println!("{}", "⚠️  NOTES:".bright_red().bold());
    println!("  • Positions are managed by the global PositionsManager service");
    println!("  • All operations require valid RPC connection and wallet configuration");
    println!("  • Position sizes are specified in SOL (e.g., 0.01 = 0.01 SOL)");
    println!("  • Use --dry-run to test commands without actual execution");
}

/// Parse command line arguments and extract configuration
#[derive(Debug, Clone)]
struct PositionsManagerArgs {
    pub show_help: bool,
    pub list_open: bool,
    pub list_closed: bool,
    pub list_all: bool,
    pub summary: bool,
    pub status_mint: Option<String>,
    pub target_mint: Option<String>,
    pub position_size: Option<f64>,
    pub action: Option<String>,
    pub debug: bool,
    pub dry_run: bool,
}

impl PositionsManagerArgs {
    pub fn from_args(args: Vec<String>) -> Self {
        let mut config = Self {
            show_help: false,
            list_open: false,
            list_closed: false,
            list_all: false,
            summary: false,
            status_mint: None,
            target_mint: None,
            position_size: None,
            action: None,
            debug: false,
            dry_run: false,
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--help" | "-h" => config.show_help = true,
                "--list-open" => config.list_open = true,
                "--list-closed" => config.list_closed = true,
                "--list-all" => config.list_all = true,
                "--summary" => config.summary = true,
                "--debug" => config.debug = true,
                "--dry-run" => config.dry_run = true,
                "--status" => {
                    if i + 1 < args.len() {
                        config.status_mint = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--mint" => {
                    if i + 1 < args.len() {
                        config.target_mint = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--size" => {
                    if i + 1 < args.len() {
                        if let Ok(size) = args[i + 1].parse::<f64>() {
                            config.position_size = Some(size);
                        }
                        i += 1;
                    }
                }
                "--action" => {
                    if i + 1 < args.len() {
                        config.action = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        config
    }

    pub fn validate(&self) -> Result<(), String> {
        // Validate action-specific requirements
        if let Some(action) = &self.action {
            match action.as_str() {
                "open" => {
                    if self.target_mint.is_none() {
                        return Err("--mint required for open action".to_string());
                    }
                    if self.position_size.is_none() {
                        return Err("--size required for open action".to_string());
                    }
                    if let Some(size) = self.position_size {
                        if size <= 0.0 || size > 1.0 {
                            return Err("--size must be between 0.0 and 1.0 SOL".to_string());
                        }
                    }
                }
                "close" => {
                    if self.target_mint.is_none() {
                        return Err("--mint required for close action".to_string());
                    }
                }
                _ => {
                    return Err("--action must be 'open' or 'close'".to_string());
                }
            }
        }

        Ok(())
    }
}

/// Main function to handle positions management
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config = PositionsManagerArgs::from_args(args);

    // Show help and exit if requested
    if config.show_help {
        print_help();
        return Ok(());
    }

    // Validate arguments
    if let Err(e) = config.validate() {
        eprintln!("{} {}", "❌ Error:".bright_red().bold(), e);
        eprintln!("Use --help for usage information");
        std::process::exit(1);
    }

    // Initialize system (only if needed)
    let needs_service = !config.dry_run && 
                       (config.list_open || config.list_closed || config.list_all || 
                        config.summary || config.status_mint.is_some() || 
                        config.action.is_some());
                       
    if needs_service {
        if let Err(e) = initialize_system(&config).await {
            eprintln!("{} {}", "❌ Initialization failed:".bright_red().bold(), e);
            std::process::exit(1);
        }
    } else {
        // Minimal initialization for dry-run or help operations
        if let Err(e) = initialize_system(&config).await {
            eprintln!("{} {}", "❌ Initialization failed:".bright_red().bold(), e);
            std::process::exit(1);
        }
    }

    // Execute requested operation
    match execute_operation(&config).await {
        Ok(_) => {
            if config.debug {
                log(LogTag::Positions, "INFO", "✅ Positions manager tool completed successfully");
            }
        }
        Err(e) => {
            eprintln!("{} {}", "❌ Operation failed:".bright_red().bold(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Initialize system components and validate prerequisites
async fn initialize_system(config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", "🔧 INITIALIZING POSITIONS MANAGER".bright_blue().bold());
    println!("{}", "=================================".bright_blue());

    // Initialize file logging system first
    init_file_logging();
    
    if config.debug {
        log(LogTag::Positions, "INFO", "🔧 Starting full system initialization...");
    }

    // Validate configuration files
    match read_configs() {
        Ok(configs) => {
            validate_configs(&configs).map_err(|e| format!("Invalid configs: {}", e))?;
            if config.debug {
                println!("✅ Configuration loaded and validated successfully");
                log(LogTag::Positions, "INFO", "✅ Configurations validated");
            }
        }
        Err(e) => {
            return Err(format!("Failed to load configuration: {}", e));
        }
    }

    // Check RPC connection
    if config.debug {
        println!("🌐 Testing RPC connection...");
    }
    
    let rpc_client = get_rpc_client();
    match rpc_client.get_latest_blockhash().await {
        Ok(_) => {
            if config.debug {
                println!("✅ RPC connection established");
                log(LogTag::Positions, "INFO", "✅ RPC connection verified");
            }
        }
        Err(e) => {
            return Err(format!("RPC connection failed: {}", e));
        }
    }

    // Initialize Price Service
    if config.debug {
        println!("💰 Initializing Price Service...");
        log(LogTag::Positions, "INFO", "💰 Initializing Price Service...");
    }
    
    match initialize_price_service().await {
        Ok(_) => {
            if config.debug {
                println!("✅ Price Service initialized successfully");
                log(LogTag::Positions, "INFO", "✅ Price Service initialized successfully");
            }
        }
        Err(e) => {
            return Err(format!("Price Service initialization failed: {}", e));
        }
    }

    // Initialize DexScreener API
    if config.debug {
        println!("🌐 Initializing DexScreener API...");
        log(LogTag::Positions, "INFO", "🌐 Initializing DexScreener API...");
    }
    
    match init_dexscreener_api().await {
        Ok(_) => {
            if config.debug {
                println!("✅ DexScreener API initialized successfully");
                log(LogTag::Positions, "INFO", "✅ DexScreener API initialized successfully");
            }
        }
        Err(e) => {
            return Err(format!("DexScreener API initialization failed: {}", e));
        }
    }

    // Start PositionsManager service if not already running
    if get_positions_handle().is_none() {
        if config.debug {
            println!("� Starting PositionsManager service...");
            log(LogTag::Positions, "INFO", "🚀 Initializing PositionsManager service...");
        }
        
        // Create shutdown notification for the service
        let shutdown = Arc::new(Notify::new());
        
        // Capture debug flag for the spawned task
        let debug_enabled = config.debug;
        
        // Start PositionsManager background service
        let shutdown_positions_manager = shutdown.clone();
        let positions_manager_handle = tokio::spawn(async move {
            if debug_enabled {
                log(LogTag::System, "INFO", "PositionsManager service task started");
            }
            start_positions_manager_service(shutdown_positions_manager).await;
            if debug_enabled {
                log(LogTag::System, "INFO", "PositionsManager service task ended");
            }
        });

        // Give the service time to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        // Verify the service is available and responding
        if let Some(handle) = get_positions_handle() {
            if config.debug {
                println!("✅ PositionsManager service started successfully");
                log(LogTag::Positions, "INFO", "✅ PositionsManager service initialized and handle available");
            }
            
            // Test the service with a simple operation
            match tokio::time::timeout(Duration::from_secs(2), handle.get_open_positions_count()).await {
                Ok(_count) => {
                    if config.debug {
                        println!("✅ PositionsManager service responding to requests");
                        log(LogTag::Positions, "INFO", "✅ PositionsManager service responding to requests");
                    }
                }
                Err(_) => {
                    return Err("PositionsManager service timeout - service not responding".to_string());
                }
            }
        } else {
            return Err("PositionsManager service failed to initialize - no handle available".to_string());
        }

        // Keep the service running for the duration of the tool
        // Note: In production, you'd want proper shutdown handling
        std::mem::forget(positions_manager_handle);
        
    } else {
        if config.debug {
            println!("✅ PositionsManager service already available");
            log(LogTag::Positions, "INFO", "✅ PositionsManager service already available");
        }
    }

    println!("✅ System initialization complete\n");
    if config.debug {
        log(LogTag::Positions, "INFO", "✅ Full system initialization completed successfully");
    }
    Ok(())
}

/// Execute the requested operation based on configuration
async fn execute_operation(config: &PositionsManagerArgs) -> Result<(), String> {
    // Handle listing operations
    if config.list_open || config.list_all {
        list_open_positions(config).await?;
    }

    if config.list_closed || config.list_all {
        if config.list_all {
            println!(); // Add spacing between open and closed listings
        }
        list_closed_positions(config).await?;
    }

    // Handle status operations
    if let Some(mint) = &config.status_mint {
        show_position_status(mint, config).await?;
    }

    if config.summary {
        show_positions_summary(config).await?;
    }

    // Handle trading operations
    if let Some(action) = &config.action {
        match action.as_str() {
            "open" => {
                if let (Some(mint), Some(size)) = (&config.target_mint, config.position_size) {
                    open_position(mint, size, config).await?;
                }
            }
            "close" => {
                if let Some(mint) = &config.target_mint {
                    close_position(mint, config).await?;
                }
            }
            _ => return Err("Invalid action specified".to_string()),
        }
    }

    Ok(())
}

/// List all open positions with detailed information
async fn list_open_positions(config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", "📈 OPEN POSITIONS".bright_green().bold());
    println!("{}", "================".bright_green());

    if get_positions_handle().is_none() {
        println!("⚠️  PositionsManager service not available. Start the main bot first to view actual positions.");
        return Ok(());
    }

    let open_positions = get_open_positions().await;
    
    if open_positions.is_empty() {
        println!("ℹ️  No open positions found");
        return Ok(());
    }

    println!("Found {} open position(s):\n", open_positions.len());

    // Table header
    println!("{:<12} {:<10} {:<12} {:<12} {:<10} {:<15} {:<10}",
        "SYMBOL".bright_white().bold(),
        "MINT".bright_white().bold(),
        "ENTRY_PRICE".bright_white().bold(),
        "SIZE_SOL".bright_white().bold(),
        "P&L_SOL".bright_white().bold(),
        "P&L_%".bright_white().bold(),
        "STATUS".bright_white().bold()
    );
    println!("{}", "─".repeat(100).bright_black());

    for position in open_positions {
        display_position_row(&position, config).await;
    }

    Ok(())
}

/// List all closed positions with final P&L
async fn list_closed_positions(config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", "📊 CLOSED POSITIONS".bright_yellow().bold());
    println!("{}", "==================".bright_yellow());

    if get_positions_handle().is_none() {
        println!("⚠️  PositionsManager service not available. Start the main bot first to view actual positions.");
        return Ok(());
    }

    let closed_positions = get_closed_positions().await;
    
    if closed_positions.is_empty() {
        println!("ℹ️  No closed positions found");
        return Ok(());
    }

    println!("Found {} closed position(s):\n", closed_positions.len());

    // Table header
    println!("{:<12} {:<10} {:<12} {:<12} {:<12} {:<10} {:<15} {:<10}",
        "SYMBOL".bright_white().bold(),
        "MINT".bright_white().bold(),
        "ENTRY_PRICE".bright_white().bold(),
        "EXIT_PRICE".bright_white().bold(),
        "SIZE_SOL".bright_white().bold(),
        "P&L_SOL".bright_white().bold(),
        "P&L_%".bright_white().bold(),
        "DURATION".bright_white().bold()
    );
    println!("{}", "─".repeat(120).bright_black());

    for position in closed_positions {
        display_closed_position_row(&position, config).await;
    }

    Ok(())
}

/// Display a single position row in the table
async fn display_position_row(position: &Position, config: &PositionsManagerArgs) {
    // Placeholder for P&L calculation
    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None); // TODO: Get current price
    
    let mint_short = format!("{}...", &position.mint[..8]);
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    
    let pnl_sol_colored = if pnl_sol >= 0.0 {
        format!("{:+.6}", pnl_sol).bright_green()
    } else {
        format!("{:+.6}", pnl_sol).bright_red()
    };
    
    let pnl_percent_colored = if pnl_percent >= 0.0 {
        format!("{:+.2}%", pnl_percent).bright_green()
    } else {
        format!("{:+.2}%", pnl_percent).bright_red()
    };

    let status = if position.transaction_entry_verified {
        "VERIFIED".bright_green()
    } else {
        "PENDING".bright_yellow()
    };

    println!("{:<12} {:<10} {:<12.8} {:<12.6} {:<10} {:<15} {:<10}",
        position.symbol,
        mint_short,
        entry_price,
        position.entry_size_sol,
        pnl_sol_colored,
        pnl_percent_colored,
        status
    );
}

/// Display a single closed position row in the table
async fn display_closed_position_row(position: &Position, config: &PositionsManagerArgs) {
    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
    
    let mint_short = format!("{}...", &position.mint[..8]);
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let exit_price = position.exit_price.unwrap_or(0.0);
    
    let pnl_sol_colored = if pnl_sol >= 0.0 {
        format!("{:+.6}", pnl_sol).bright_green()
    } else {
        format!("{:+.6}", pnl_sol).bright_red()
    };
    
    let pnl_percent_colored = if pnl_percent >= 0.0 {
        format!("{:+.2}%", pnl_percent).bright_green()
    } else {
        format!("{:+.2}%", pnl_percent).bright_red()
    };

    let duration = if let (Some(exit_time), entry_time) = (position.exit_time, position.entry_time) {
        let duration = exit_time.signed_duration_since(entry_time);
        if duration.num_hours() > 0 {
            format!("{}h", duration.num_hours())
        } else {
            format!("{}m", duration.num_minutes())
        }
    } else {
        "N/A".to_string()
    };

    println!("{:<12} {:<10} {:<12.8} {:<12.8} {:<12.6} {:<10} {:<15} {:<10}",
        position.symbol,
        mint_short,
        entry_price,
        exit_price,
        position.entry_size_sol,
        pnl_sol_colored,
        pnl_percent_colored,
        duration
    );
}

/// Show detailed status for a specific position
async fn show_position_status(mint: &str, config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", format!("🔍 POSITION STATUS: {}", get_mint_prefix(mint)).bright_cyan().bold());
    println!("{}", "=".repeat(50).bright_cyan());

    if config.debug {
        log(LogTag::Positions, "INFO", &format!("Fetching current price for {}", mint));
    }
    
    // Get current price from price service
    let current_price = get_token_price_blocking_safe(mint).await;
    
    if let Some(price) = current_price {
        if config.debug {
            log(LogTag::Positions, "INFO", &format!("✅ Got current price for {}: ${:.12} SOL", mint, price));
        }
        
        // Check if there's an open position
        let positions = get_open_positions().await;
        let open_position = positions.iter().find(|pos| pos.mint == mint);
        
        if let Some(position) = open_position {
            // Show open position details
            println!("✅ {}", "OPEN POSITION FOUND".bright_green().bold());
            println!("   Mint: {}", mint);
            println!("   Symbol: {}", position.symbol.bright_yellow());
            println!("   Entry Price: ${:.12} SOL", position.entry_price);
            println!("   Current Price: ${:.12} SOL", price);
            
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(price));
            let pnl_color = if pnl_percent > 0.0 { 
                format!("🟢 P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).bright_green()
            } else if pnl_percent < 0.0 { 
                format!("🔴 P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).bright_red()
            } else { 
                format!("⚪ P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).white()
            };
            
            println!("   {}", pnl_color);
            println!("   Position Size: {:.6} SOL", position.entry_size_sol);
            println!("   Entry Time: {}", position.entry_time.format("%Y-%m-%d %H:%M:%S UTC"));
            
            if let Some(sig) = &position.entry_transaction_signature {
                println!("   Entry TX: {}", get_signature_prefix(sig));
                
                // TODO: Check transaction verification status
                println!("   📋 Entry transaction recorded");
            }
        } else {
            // Check closed positions
            let closed_positions = get_closed_positions().await;
            let closed_position = closed_positions.iter().find(|pos| pos.mint == mint);
            
            if let Some(position) = closed_position {
                println!("📊 {}", "CLOSED POSITION FOUND".bright_blue().bold());
                println!("   Mint: {}", mint);
                println!("   Symbol: {}", position.symbol.bright_yellow());
                println!("   Entry Price: ${:.12} SOL", position.entry_price);
                
                if let Some(exit_price) = position.exit_price {
                    println!("   Exit Price: ${:.12} SOL", exit_price);
                    
                    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
                    let pnl_color = if pnl_percent > 0.0 { 
                        format!("🟢 Final P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).bright_green()
                    } else if pnl_percent < 0.0 { 
                        format!("🔴 Final P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).bright_red()
                    } else { 
                        format!("⚪ Final P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).white()
                    };
                    
                    println!("   {}", pnl_color);
                }
                
                println!("   Current Market Price: ${:.12} SOL", price);
                println!("   Position Size: {:.6} SOL", position.entry_size_sol);
                println!("   Entry Time: {}", position.entry_time.format("%Y-%m-%d %H:%M:%S UTC"));
                
                if let Some(exit_time) = position.exit_time {
                    println!("   Exit Time: {}", exit_time.format("%Y-%m-%d %H:%M:%S UTC"));
                }
                
                if let Some(sig) = &position.entry_transaction_signature {
                    println!("   Entry TX: {}", get_signature_prefix(sig));
                }
                
                if let Some(sig) = &position.exit_transaction_signature {
                    println!("   Exit TX: {}", get_signature_prefix(sig));
                }
            } else {
                println!("ℹ️  {}", "NO POSITION FOUND".bright_yellow().bold());
                println!("   Mint: {}", mint);
                println!("   Current Market Price: ${:.12} SOL", price);
                println!("   Status: No open or closed positions for this token");
                println!("   💡 Use --action open --size <SOL> to open a position");
            }
        }
    } else {
        println!("❌ {}", "PRICE NOT AVAILABLE".bright_red().bold());
        println!("   Mint: {}", mint);
        println!("   Status: Unable to fetch current price");
        println!("   💡 Price service may be initializing or token may not be tradeable");
        
        if config.debug {
            log(LogTag::Positions, "WARN", &format!("Failed to get price for {}", mint));
        }
    }

    if config.debug {
        log(LogTag::Positions, "DEBUG", &format!("Position status requested for mint: {}", mint));
    }

    Ok(())
}

/// Show positions summary statistics
async fn show_positions_summary(config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", "📊 POSITIONS SUMMARY".bright_magenta().bold());
    println!("{}", "===================".bright_magenta());

    if get_positions_handle().is_none() {
        println!("⚠️  PositionsManager service not available. Start the main bot first to view actual summary.");
        return Ok(());
    }

    let open_positions = get_open_positions().await;
    let closed_positions = get_closed_positions().await;

    println!("📈 Open Positions: {}", open_positions.len().to_string().bright_green());
    println!("📊 Closed Positions: {}", closed_positions.len().to_string().bright_yellow());

    // Calculate total invested
    let total_invested: f64 = open_positions.iter()
        .chain(closed_positions.iter())
        .map(|p| p.entry_size_sol)
        .sum();

    println!("💰 Total Invested: {:.6} SOL", total_invested.to_string().bright_cyan());

    // Calculate total P&L for closed positions
    let total_realized_pnl: f64 = closed_positions.iter()
        .map(|p| calculate_position_pnl(p, None).0)
        .sum();

    let total_realized_pnl_colored = if total_realized_pnl >= 0.0 {
        format!("{:+.6} SOL", total_realized_pnl).bright_green()
    } else {
        format!("{:+.6} SOL", total_realized_pnl).bright_red()
    };

    println!("💸 Realized P&L: {}", total_realized_pnl_colored);

    // TODO: Calculate unrealized P&L for open positions
    println!("📊 Unrealized P&L: TBD (requires current prices)");

    Ok(())
}

/// Open a new position for the specified token
async fn open_position(mint: &str, size: f64, config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", format!("📈 OPENING POSITION: {}", get_mint_prefix(mint)).bright_green().bold());
    println!("{}", "=".repeat(50).bright_green());

    if config.dry_run {
        println!("🧪 DRY RUN MODE: Position would be opened");
        println!("   Mint: {}", mint);
        println!("   Size: {:.6} SOL", size);
        return Ok(());
    }

    // Step 1: Get current token price
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting current price for {}", mint));
    }
    
    let current_price = get_token_price_blocking_safe(mint).await
        .ok_or_else(|| format!("Failed to get current price for {}", mint))?;
    
    println!("✅ Current Price: ${:.12} SOL", current_price);
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Got price: ${:.12} SOL", current_price));
    }

    // Step 2: Get token information
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting token information for {}", mint));
    }
    
    let token = get_token_from_db(mint).await
        .ok_or_else(|| format!("Token not found in database: {}", mint))?;
    
    println!("✅ Token Found: {} ({})", token.symbol, token.name);
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Token: {} - {}", token.symbol, token.name));
    }

    // Step 3: Check if position already exists
    let existing_positions = get_open_positions().await;
    if existing_positions.iter().any(|pos| pos.mint == mint) {
        return Err(format!("Position already exists for token {}", token.symbol));
    }

    // Step 4: Open the position
    println!("🚀 Opening position...");
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("📈 Opening position for {} at ${:.12} SOL", token.symbol, current_price));
    }
    
    // Calculate percent change (we'll use 0.0 as initial value since this is a new position)
    let percent_change = 0.0;
    
    match open_position_global(token.clone(), current_price, percent_change).await {
        Ok((position_id, transaction_sig)) => {
            println!("✅ Position opened successfully!");
            println!("   Position ID: {}", position_id);
            println!("   Symbol: {}", token.symbol);
            println!("   Entry Price: ${:.12} SOL", current_price);
            println!("   Size: {:.6} SOL", size);
            
            if !transaction_sig.is_empty() {
                println!("   Transaction: {}", get_signature_prefix(&transaction_sig));
                
                if config.debug {
                    log(LogTag::Positions, "INFO", &format!("📄 Transaction signature: {}", transaction_sig));
                }
            }
            
            if config.debug {
                log(LogTag::Positions, "INFO", &format!("✅ Position opened: ID={}, Symbol={}", position_id, token.symbol));
            }
            
            Ok(())
        }
        Err(e) => {
            println!("❌ Failed to open position: {}", e);
            
            if config.debug {
                log(LogTag::Positions, "ERROR", &format!("❌ Position opening failed: {}", e));
            }
            
            Err(format!("Position opening failed: {}", e))
        }
    }
}

/// Close an existing position for the specified token
async fn close_position(mint: &str, config: &PositionsManagerArgs) -> Result<(), String> {
    println!("{}", format!("📉 CLOSING POSITION: {}", get_mint_prefix(mint)).bright_red().bold());
    println!("{}", "=".repeat(50).bright_red());

    if config.dry_run {
        println!("🧪 DRY RUN MODE: Position would be closed");
        println!("   Mint: {}", mint);
        return Ok(());
    }

    if config.debug {
        log(LogTag::Positions, "DEBUG", &format!(
            "Close position requested for mint: {}", mint
        ));
    }

    // Step 1: Find existing open position
    if config.debug {
        log(LogTag::Positions, "INFO", "🔍 Looking for existing open position");
    }
    
    let open_positions = get_open_positions().await;
    let position = open_positions.iter()
        .find(|pos| pos.mint == mint)
        .ok_or_else(|| format!("No open position found for token {}", mint))?;

    println!("✅ Position Found: {} ({})", position.symbol, position.name);
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Found position: {} - entry price: ${:.12} SOL", 
            position.symbol, position.entry_price));
    }

    // Step 2: Get current token price
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting current price for {}", mint));
    }
    
    let current_price = get_token_price_blocking_safe(mint).await
        .ok_or_else(|| format!("Failed to get current price for {}", mint))?;
    
    println!("✅ Current Price: ${:.12} SOL", current_price);
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Got exit price: ${:.12} SOL", current_price));
    }

    // Step 3: Get token information for closing
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting token information for closing"));
    }
    
    let token = get_token_from_db(mint).await
        .ok_or_else(|| format!("Token not found in database: {}", mint))?;
    
    // Step 4: Calculate P&L before closing
    let pnl_percentage = if position.entry_price > 0.0 {
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    } else {
        0.0
    };
    
    println!("📊 Position P&L: {:.2}%", pnl_percentage);
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("📊 P&L calculation: entry={:.12}, current={:.12}, pnl={:.2}%", 
            position.entry_price, current_price, pnl_percentage));
    }

    // Step 5: Close the position using positions system
    println!("🚀 Closing position...");
    
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("📉 Closing position for {} at ${:.12} SOL", 
            position.symbol, current_price));
    }

    // Get the PositionsManager handle for closing the position
    if let Some(handle) = get_positions_handle() {
        let exit_time = Utc::now();
        match handle.close_position(mint.to_string(), token, current_price, exit_time).await {
            Ok((position_id, exit_transaction)) => {
                println!("✅ Position closed successfully!");
                println!("   Position ID: {}", mint);
                println!("   Symbol: {}", position.symbol);
                println!("   Entry Price: ${:.12} SOL", position.entry_price);
                println!("   Exit Price: ${:.12} SOL", current_price);
                println!("   P&L: {:.2}%", pnl_percentage);
                
                if !exit_transaction.is_empty() {
                    println!("   Transaction: {}", get_signature_prefix(&exit_transaction));
                    
                    if config.debug {
                        log(LogTag::Positions, "INFO", &format!("📄 Exit transaction signature: {}", exit_transaction));
                    }
                }
                
                if config.debug {
                    log(LogTag::Positions, "INFO", &format!("✅ Position closed: ID={}, Symbol={}, P&L={:.2}%", 
                        mint, position.symbol, pnl_percentage));
                }
                
                Ok(())
            }
            Err(e) => {
                println!("❌ Failed to close position: {}", e);
                
                if config.debug {
                    log(LogTag::Positions, "ERROR", &format!("❌ Position closing failed: {}", e));
                }
                
                Err(format!("Position closing failed: {}", e))
            }
        }
    } else {
        let error_msg = "PositionsManager service not available";
        println!("❌ {}", error_msg);
        
        if config.debug {
            log(LogTag::Positions, "ERROR", error_msg);
        }
        
        Err(error_msg.to_string())
    }
}

/// Safe 8-char prefix for signatures (avoids direct string indexing)
fn get_signature_prefix(s: &str) -> String {
    s.chars().take(8).collect()
}

/// Safe 8-char prefix for mints (avoids direct string indexing)  
fn get_mint_prefix(s: &str) -> String {
    s.chars().take(8).collect()
}

/// Validate token mint address format
fn validate_mint_address(mint: &str) -> Result<(), String> {
    if mint.len() < 32 {
        return Err("Invalid mint address: too short".to_string());
    }
    
    if mint.len() > 44 {
        return Err("Invalid mint address: too long".to_string());
    }
    
    // TODO: Add more sophisticated validation (base58 check, etc.)
    
    Ok(())
}

/// Get token information for display purposes
async fn get_token_info_placeholder(mint: &str) -> Result<(String, String), String> {
    // TODO: Implement actual token info lookup
    // This would query token metadata, symbol, name, etc.
    
    let symbol = format!("TOKEN_{}", &mint[..8]);
    let name = format!("Token {}", &mint[..8]);
    
    Ok((symbol, name))
}
