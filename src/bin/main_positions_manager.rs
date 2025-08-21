#![allow(warnings)]

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
    get_open_positions,
    get_closed_positions,
    get_positions_handle,
    start_positions_manager_service,
    open_position_global,
    Position,
    calculate_position_pnl,
};
use screenerbot::tokens::{ get_token_from_db };
use screenerbot::tokens::price::{ initialize_price_service, get_token_price_blocking_safe };
use screenerbot::tokens::init_dexscreener_api;
use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::configs::{ read_configs, validate_configs };
use screenerbot::transactions::{
    start_transactions_service,
    get_transaction,
    is_transaction_verified,
};

use screenerbot::rpc::get_rpc_client;

use std::env;
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use colored::Colorize;
use tokio::sync::Notify;

/// Print comprehensive help menu for the Positions Manager Tool
fn print_help() {
    log(
        LogTag::Positions,
        "INFO",
        &format!("{}", "🎯 POSITIONS MANAGER TOOL".bright_blue().bold())
    );
    log(LogTag::Positions, "INFO", &format!("{}", "========================".bright_blue()));
    log(
        LogTag::Positions,
        "INFO",
        "Comprehensive trading positions management and monitoring tool"
    );
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "📋 LISTING COMMANDS:".bright_green().bold()));
    log(LogTag::Positions, "INFO", "  --list-open              List all open positions with P&L");
    log(
        LogTag::Positions,
        "INFO",
        "  --list-closed            List all closed positions with final P&L"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  --list-all               List both open and closed positions"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  --diagnostics            Show detailed diagnostics for all positions"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  --reverify               Force re-verification of all unverified transactions"
    );
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "🔍 STATUS COMMANDS:".bright_yellow().bold()));
    log(
        LogTag::Positions,
        "INFO",
        "  --status <MINT>          Show detailed status of specific position"
    );
    log(LogTag::Positions, "INFO", "  --summary                Show positions summary statistics");
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "📈 TRADING COMMANDS:".bright_cyan().bold()));
    log(
        LogTag::Positions,
        "INFO",
        "  --mint <ADDRESS>         Token mint address for position operations"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  --size <SOL_AMOUNT>      Position size in SOL (for opening positions)"
    );
    log(LogTag::Positions, "INFO", "  --action <open|close>    Action to perform on the position");
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "🔁 TESTING COMMANDS:".bright_purple().bold()));
    log(
        LogTag::Positions,
        "INFO",
        "  --test-loop              Run continuous open/close position testing"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  --test-iterations <N>    Number of test iterations (default: infinite)"
    );
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "⚙️  UTILITY COMMANDS:".bright_magenta().bold()));
    log(LogTag::Positions, "INFO", "  --help, -h               Show this help menu");
    log(LogTag::Positions, "INFO", "  --debug                  Enable debug logging for positions");
    log(LogTag::Positions, "INFO", "  --debug-positions        Verbose position lifecycle logs");
    log(
        LogTag::Positions,
        "INFO",
        "  --debug-transactions     Verbose transaction verification logs"
    );
    log(LogTag::Positions, "INFO", "  --debug-swaps            Verbose swap analysis logs");
    log(LogTag::Positions, "INFO", "  --dry-run               Simulate actions without executing");
    log(
        LogTag::Positions,
        "INFO",
        "  --verify-timeout <S>     Max seconds to wait for a tx (default 30)"
    );
    log(LogTag::Positions, "INFO", "  --verify-poll <S>        Poll interval seconds (default 1)");
    log(
        LogTag::Positions,
        "INFO",
        "  --max-exit-retries <N>   Max sell retries before queue (default 3)"
    );
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "📊 EXAMPLES:".bright_white().bold()));
    log(LogTag::Positions, "INFO", "  # List all open positions");
    log(LogTag::Positions, "INFO", "  cargo run --bin main_positions_manager -- --list-open");
    log(LogTag::Positions, "INFO", "");
    log(LogTag::Positions, "INFO", "  # Check specific token status");
    log(
        LogTag::Positions,
        "INFO",
        "  cargo run --bin main_positions_manager -- --status So11111111111111111111111111111111111111112"
    );
    log(LogTag::Positions, "INFO", "");
    log(LogTag::Positions, "INFO", "  # Open new 0.01 SOL position");
    log(
        LogTag::Positions,
        "INFO",
        "  cargo run --bin main_positions_manager -- --mint So11111111111111111111111111111111111111112 --size 0.01 --action open"
    );
    log(LogTag::Positions, "INFO", "");
    log(LogTag::Positions, "INFO", "  # Close existing position");
    log(
        LogTag::Positions,
        "INFO",
        "  cargo run --bin main_positions_manager -- --mint So11111111111111111111111111111111111111112 --action close"
    );
    log(LogTag::Positions, "INFO", "");

    log(LogTag::Positions, "INFO", &format!("{}", "⚠️  NOTES:".bright_red().bold()));
    log(
        LogTag::Positions,
        "INFO",
        "  • Positions are managed by the global PositionsManager service"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  • All operations require valid RPC connection and wallet configuration"
    );
    log(
        LogTag::Positions,
        "INFO",
        "  • Position sizes are specified in SOL (e.g., 0.01 = 0.01 SOL)"
    );
    log(LogTag::Positions, "INFO", "  • Use --dry-run to test commands without actual execution");
}

/// Parse command line arguments and extract configuration
#[derive(Debug, Clone)]
struct PositionsManagerArgs {
    pub show_help: bool,
    pub list_open: bool,
    pub list_closed: bool,
    pub list_all: bool,
    pub summary: bool,
    pub diagnostics: bool,
    pub reverify: bool,
    pub status_mint: Option<String>,
    pub target_mint: Option<String>,
    pub position_size: Option<f64>,
    pub action: Option<String>,
    pub debug: bool,
    // Fine-grained debug flags
    pub debug_positions: bool,
    pub debug_transactions: bool,
    pub debug_swaps: bool,
    pub dry_run: bool,
    pub test_loop: bool,
    pub test_iterations: Option<usize>,
    // Verification tuning
    pub verify_timeout_secs: u64,
    pub verify_poll_secs: u64,
    pub max_exit_retries: u32,
}

impl PositionsManagerArgs {
    pub fn from_args(args: Vec<String>) -> Self {
        let mut config = Self {
            show_help: false,
            list_open: false,
            list_closed: false,
            list_all: false,
            summary: false,
            diagnostics: false,
            reverify: false,
            status_mint: None,
            target_mint: None,
            position_size: None,
            action: None,
            debug: false,
            debug_positions: false,
            debug_transactions: false,
            debug_swaps: false,
            dry_run: false,
            test_loop: false,
            test_iterations: None,
            verify_timeout_secs: 30, // default 30s wait in tool loop
            verify_poll_secs: 1, // default poll every 1s
            max_exit_retries: 3, // default sell attempts
        };

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--help" | "-h" => {
                    config.show_help = true;
                }
                "--list-open" => {
                    config.list_open = true;
                }
                "--list-closed" => {
                    config.list_closed = true;
                }
                "--list-all" => {
                    config.list_all = true;
                }
                "--summary" => {
                    config.summary = true;
                }
                "--diagnostics" => {
                    config.diagnostics = true;
                }
                "--reverify" => {
                    config.reverify = true;
                }
                "--debug" => {
                    config.debug = true;
                }
                "--debug-positions" => {
                    config.debug_positions = true;
                }
                "--debug-transactions" => {
                    config.debug_transactions = true;
                }
                "--debug-swaps" => {
                    config.debug_swaps = true;
                }
                "--dry-run" => {
                    config.dry_run = true;
                }
                "--status" => {
                    if i + 1 < args.len() {
                        config.status_mint = Some(args[i + 1].clone());
                        i += 1;
                    }
                }
                "--verify-timeout" => {
                    if i + 1 < args.len() {
                        if let Ok(v) = args[i + 1].parse::<u64>() {
                            config.verify_timeout_secs = v.max(5).min(600);
                        }
                        i += 1;
                    }
                }
                "--verify-poll" => {
                    if i + 1 < args.len() {
                        if let Ok(v) = args[i + 1].parse::<u64>() {
                            config.verify_poll_secs = v.clamp(1, 30);
                        }
                        i += 1;
                    }
                }
                "--max-exit-retries" => {
                    if i + 1 < args.len() {
                        if let Ok(v) = args[i + 1].parse::<u32>() {
                            config.max_exit_retries = v.clamp(1, 10);
                        }
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
                "--test-loop" => {
                    config.test_loop = true;
                }
                "--test-iterations" => {
                    if i + 1 < args.len() {
                        if let Ok(iterations) = args[i + 1].parse::<usize>() {
                            config.test_iterations = Some(iterations);
                        }
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
    let config = PositionsManagerArgs::from_args(args.clone());

    // Show help and exit if requested
    if config.show_help {
        print_help();
        return Ok(());
    }

    // Set global command args so debug flags work in swap modules
    let mut global_args = args.clone();
    if config.debug_swaps {
        global_args.push("--debug-swaps".to_string());
    }
    if config.debug_positions {
        global_args.push("--debug-positions".to_string());
    }
    if config.debug_transactions {
        global_args.push("--debug-transactions".to_string());
    }
    screenerbot::arguments::set_cmd_args(global_args);

    // Validate arguments
    if let Err(e) = config.validate() {
        log(LogTag::Positions, "ERROR", &format!("{} {}", "❌ Error:".bright_red().bold(), e));
        log(LogTag::Positions, "INFO", "Use --help for usage information");
        std::process::exit(1);
    }

    // Initialize system (only if needed)
    let needs_service =
        !config.dry_run &&
        (config.list_open ||
            config.list_closed ||
            config.list_all ||
            config.summary ||
            config.status_mint.is_some() ||
            config.action.is_some());

    let (shutdown_handle, task_handles) = if needs_service {
        match initialize_system(&config).await {
            Ok((shutdown, handles)) => (Some(shutdown), handles),
            Err(e) => {
                log(
                    LogTag::Positions,
                    "ERROR",
                    &format!("{} {}", "❌ Initialization failed:".bright_red().bold(), e)
                );
                std::process::exit(1);
            }
        }
    } else {
        // Minimal initialization for dry-run or help operations - no background services needed
        if config.debug {
            log(
                LogTag::Positions,
                "INFO",
                "🧪 Dry-run mode: skipping background service initialization"
            );
        }
        (None, Vec::new())
    };

    // Execute requested operation
    match execute_operation(&config).await {
        Ok(_) => {
            if config.debug {
                log(LogTag::Positions, "INFO", "✅ Positions manager tool completed successfully");
            }
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "ERROR",
                &format!("{} {}", "❌ Operation failed:".bright_red().bold(), e)
            );
            std::process::exit(1);
        }
    }

    // Allow background tasks time to complete before shutdown
    if needs_service && !task_handles.is_empty() {
        if config.debug {
            log(LogTag::Positions, "INFO", "⏳ Allowing background tasks to complete...");
        }

        // Signal shutdown to background services first
        if let Some(shutdown) = shutdown_handle {
            if config.debug {
                log(LogTag::Positions, "INFO", "📤 Signaling shutdown to background services...");
            }
            shutdown.notify_waiters();
        }

        // Wait for background tasks to complete with proper timeout
        if config.debug {
            log(
                LogTag::Positions,
                "INFO",
                &format!("🔄 Waiting for {} background tasks to shutdown...", task_handles.len())
            );
        }

        let shutdown_timeout = tokio::time::timeout(
            Duration::from_secs(10), // Increased timeout for proper shutdown
            async {
                for (i, handle) in task_handles.into_iter().enumerate() {
                    if config.debug {
                        log(
                            LogTag::Positions,
                            "INFO",
                            &format!("🔄 Waiting for task {} to shutdown...", i + 1)
                        );
                    }
                    if let Err(e) = handle.await {
                        log(
                            LogTag::Positions,
                            "WARN",
                            &format!("Background task {} failed to shutdown cleanly: {}", i + 1, e)
                        );
                    } else if config.debug {
                        log(
                            LogTag::Positions,
                            "INFO",
                            &format!("✅ Task {} shutdown completed", i + 1)
                        );
                    }
                }
            }
        ).await;

        match shutdown_timeout {
            Ok(_) => {
                if config.debug {
                    log(LogTag::Positions, "INFO", "✅ All background tasks shutdown successfully");
                }
            }
            Err(_) => {
                log(
                    LogTag::Positions,
                    "WARN",
                    "⚠️  Background task shutdown timed out after 10 seconds"
                );
            }
        }
    }

    Ok(())
}

/// Initialize system components and validate prerequisites
async fn initialize_system(
    config: &PositionsManagerArgs
) -> Result<(Arc<Notify>, Vec<tokio::task::JoinHandle<()>>), String> {
    log(
        LogTag::Positions,
        "INFO",
        &format!("{}", "🔧 INITIALIZING POSITIONS MANAGER".bright_blue().bold())
    );
    log(
        LogTag::Positions,
        "INFO",
        &format!("{}", "=================================".bright_blue())
    );

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
                log(
                    LogTag::Positions,
                    "INFO",
                    "✅ Configuration loaded and validated successfully"
                );
                log(LogTag::Positions, "INFO", "✅ Configurations validated");
            }
        }
        Err(e) => {
            return Err(format!("Failed to load configuration: {}", e));
        }
    }

    // Check RPC connection
    if config.debug {
        log(LogTag::Positions, "INFO", "🌐 Testing RPC connection...");
    }

    let rpc_client = get_rpc_client();
    match rpc_client.get_latest_blockhash().await {
        Ok(_) => {
            if config.debug {
                log(LogTag::Positions, "INFO", "✅ RPC connection established");
                log(LogTag::Positions, "INFO", "✅ RPC connection verified");
            }
        }
        Err(e) => {
            return Err(format!("RPC connection failed: {}", e));
        }
    }

    // Initialize Price Service
    if config.debug {
        log(LogTag::Positions, "INFO", "💰 Initializing Price Service...");
        log(LogTag::Positions, "INFO", "💰 Initializing Price Service...");
    }

    match initialize_price_service().await {
        Ok(_) => {
            if config.debug {
                log(LogTag::Positions, "INFO", "✅ Price Service initialized successfully");
                log(LogTag::Positions, "INFO", "✅ Price Service initialized successfully");
            }
        }
        Err(e) => {
            return Err(format!("Price Service initialization failed: {}", e));
        }
    }

    // Initialize DexScreener API
    if config.debug {
        log(LogTag::Positions, "INFO", "🌐 Initializing DexScreener API...");
        log(LogTag::Positions, "INFO", "🌐 Initializing DexScreener API...");
    }

    match init_dexscreener_api().await {
        Ok(_) => {
            if config.debug {
                log(LogTag::Positions, "INFO", "✅ DexScreener API initialized successfully");
                log(LogTag::Positions, "INFO", "✅ DexScreener API initialized successfully");
            }
        }
        Err(e) => {
            return Err(format!("DexScreener API initialization failed: {}", e));
        }
    }

    // Create shutdown notification for services
    let shutdown = Arc::new(Notify::new());
    let mut task_handles = Vec::new();

    // Start PositionsManager service if not already running
    if get_positions_handle().await.is_none() {
        if config.debug {
            log(LogTag::Positions, "INFO", "🚀 Starting PositionsManager service...");
            log(LogTag::Positions, "INFO", "🚀 Initializing PositionsManager service...");
        }

        // Capture debug flag for the spawned task
        let debug_enabled = config.debug;

        // Start TransactionManager background service FIRST (CRITICAL for verification)
        if config.debug {
            log(LogTag::Positions, "INFO", "⚡ Starting TransactionManager service first...");
        }

        let shutdown_transaction_manager = shutdown.clone();
        let transaction_manager_handle = tokio::spawn(async move {
            if debug_enabled {
                log(LogTag::System, "INFO", "TransactionManager service task started");
            }
            start_transactions_service(shutdown_transaction_manager).await;
            if debug_enabled {
                log(LogTag::System, "INFO", "TransactionManager service task ended");
            }
        });

        // Wait for TransactionsManager to be fully initialized BEFORE starting PositionsManager
        if config.debug {
            log(LogTag::Positions, "INFO", "🔍 Waiting for TransactionsManager to initialize...");
        }

        let mut tx_manager_ready = false;
        for attempt in 1..=20 {
            use screenerbot::transactions::GLOBAL_TRANSACTION_MANAGER;
            let manager_guard = GLOBAL_TRANSACTION_MANAGER.lock().await;
            if manager_guard.is_some() {
                tx_manager_ready = true;
                if config.debug {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("✅ TransactionsManager ready after {} attempts", attempt)
                    );
                }
                drop(manager_guard);
                break;
            }
            drop(manager_guard);

            if config.debug && attempt % 5 == 0 {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("⏳ Still waiting for TransactionsManager (attempt {}/20)...", attempt)
                );
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        if !tx_manager_ready {
            return Err(
                "TransactionsManager failed to initialize - position operations require transaction verification".to_string()
            );
        }

        // Now start PositionsManager service (it can now safely verify transactions)
        if config.debug {
            log(
                LogTag::Positions,
                "INFO",
                "🚀 Starting PositionsManager service (TransactionsManager ready)..."
            );
        }

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

        // Store handles for proper shutdown
        task_handles.push(transaction_manager_handle);
        task_handles.push(positions_manager_handle);

        // Give the services time to initialize properly
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Verify both services are available and responding
        if let Some(handle) = get_positions_handle().await {
            if config.debug {
                log(LogTag::Positions, "INFO", "✅ PositionsManager service started successfully");
                log(
                    LogTag::Positions,
                    "INFO",
                    "✅ PositionsManager service initialized and handle available"
                );
            }

            // Test the PositionsManager service with a simple operation
            match
                tokio::time::timeout(
                    Duration::from_secs(2),
                    handle.get_open_positions_count()
                ).await
            {
                Ok(_count) => {
                    if config.debug {
                        log(
                            LogTag::Positions,
                            "INFO",
                            "✅ PositionsManager service responding to requests"
                        );
                        log(
                            LogTag::Positions,
                            "INFO",
                            "✅ PositionsManager service responding to requests"
                        );
                    }
                }
                Err(_) => {
                    return Err(
                        "PositionsManager service timeout - service not responding".to_string()
                    );
                }
            }

            if config.debug {
                log(
                    LogTag::Positions,
                    "INFO",
                    "✅ Both PositionsManager and TransactionsManager are ready for operations"
                );
            }
        } else {
            return Err(
                "PositionsManager service failed to initialize - no handle available".to_string()
            );
        }

        // Keep the services running for the duration of the tool
        // No need to spawn tasks here since we already stored the handles
    } else {
        if config.debug {
            log(LogTag::Positions, "INFO", "✅ PositionsManager service already available");
            log(LogTag::Positions, "INFO", "✅ PositionsManager service already available");
        }
    }

    log(LogTag::Positions, "INFO", "✅ System initialization complete\n");
    if config.debug {
        log(LogTag::Positions, "INFO", "✅ Full system initialization completed successfully");
    }
    Ok((shutdown, task_handles))
}

/// Execute the requested operation based on configuration
async fn execute_operation(config: &PositionsManagerArgs) -> Result<(), String> {
    // Handle listing operations
    if config.list_open || config.list_all {
        list_open_positions(config).await?;
    }

    if config.list_closed || config.list_all {
        if config.list_all {
            log(LogTag::Positions, "INFO", ""); // Add spacing between open and closed listings
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

    // Diagnostics (after summary so it can build upon it)
    if config.diagnostics {
        show_positions_diagnostics(config).await?;
    }

    // Force reverify unverified transactions
    if config.reverify {
        force_reverify_positions(config).await?;
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
            _ => {
                return Err("Invalid action specified".to_string());
            }
        }
    }

    // Handle test loop operations
    if config.test_loop {
        run_continuous_test_loop(config).await?;
    }

    Ok(())
}

/// List all open positions with detailed information
async fn list_open_positions(config: &PositionsManagerArgs) -> Result<(), String> {
    log(LogTag::Positions, "INFO", &format!("{}", "📈 OPEN POSITIONS".bright_green().bold()));
    log(LogTag::Positions, "INFO", &format!("{}", "================".bright_green()));

    if get_positions_handle().await.is_none() {
        log(
            LogTag::Positions,
            "WARN",
            "⚠️  PositionsManager service not available. Start the main bot first to view actual positions."
        );
        return Ok(());
    }

    let open_positions = get_open_positions().await;

    if open_positions.is_empty() {
        log(LogTag::Positions, "INFO", "ℹ️  No open positions found");
        return Ok(());
    }

    log(LogTag::Positions, "INFO", &format!("Found {} open position(s):\n", open_positions.len()));

    // Table header
    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "{:<12} {:<10} {:<12} {:<12} {:<10} {:<15} {:<10}",
            "SYMBOL".bright_white().bold(),
            "MINT".bright_white().bold(),
            "ENTRY_PRICE".bright_white().bold(),
            "SIZE_SOL".bright_white().bold(),
            "P&L_SOL".bright_white().bold(),
            "P&L_%".bright_white().bold(),
            "STATUS".bright_white().bold()
        )
    );
    log(LogTag::Positions, "INFO", &format!("{}", "─".repeat(100).bright_black()));

    for position in open_positions {
        display_position_row(&position, config).await;
    }

    Ok(())
}

/// List all closed positions with final P&L
async fn list_closed_positions(config: &PositionsManagerArgs) -> Result<(), String> {
    log(LogTag::Positions, "INFO", &format!("{}", "📊 CLOSED POSITIONS".bright_yellow().bold()));
    log(LogTag::Positions, "INFO", &format!("{}", "==================".bright_yellow()));

    if get_positions_handle().await.is_none() {
        log(
            LogTag::Positions,
            "WARN",
            "⚠️  PositionsManager service not available. Start the main bot first to view actual positions."
        );
        return Ok(());
    }

    let closed_positions = get_closed_positions().await;

    if closed_positions.is_empty() {
        log(LogTag::Positions, "INFO", "ℹ️  No closed positions found");
        return Ok(());
    }

    log(
        LogTag::Positions,
        "INFO",
        &format!("Found {} closed position(s):\n", closed_positions.len())
    );

    // Table header
    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "{:<12} {:<10} {:<12} {:<12} {:<12} {:<10} {:<15} {:<10}",
            "SYMBOL".bright_white().bold(),
            "MINT".bright_white().bold(),
            "ENTRY_PRICE".bright_white().bold(),
            "EXIT_PRICE".bright_white().bold(),
            "SIZE_SOL".bright_white().bold(),
            "P&L_SOL".bright_white().bold(),
            "P&L_%".bright_white().bold(),
            "DURATION".bright_white().bold()
        )
    );
    log(LogTag::Positions, "INFO", &format!("{}", "─".repeat(120).bright_black()));

    for position in closed_positions {
        display_closed_position_row(&position, config).await;
    }

    Ok(())
}

/// Display a single position row in the table
async fn display_position_row(position: &Position, config: &PositionsManagerArgs) {
    // Cache price service access to avoid repeated calls
    static PRICE_SERVICE_CACHE: std::sync::OnceLock<Option<std::sync::Arc<screenerbot::tokens::price::TokenPriceService>>> = std::sync::OnceLock::new();

    // Get current price for P&L calculation (with caching to reduce API calls)
    let current_price = PRICE_SERVICE_CACHE.get_or_init(|| {
        screenerbot::tokens::price::PRICE_SERVICE.get().cloned()
    });

    let price = if let Some(price_service) = current_price {
        price_service.get_token_price(&position.mint).await
    } else {
        // Fallback to DexScreener API only if price service is not available
        None // Skip expensive fallback for table display
    };

    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, price).await;

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

    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "{:<12} {:<10} {:<12.8} {:<12.6} {:<10} {:<15} {:<10}",
            position.symbol,
            mint_short,
            entry_price,
            position.entry_size_sol,
            pnl_sol_colored,
            pnl_percent_colored,
            status
        )
    );
}

/// Display a single closed position row in the table
async fn display_closed_position_row(position: &Position, config: &PositionsManagerArgs) {
    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None).await;

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

    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "{:<12} {:<10} {:<12.8} {:<12.8} {:<12.6} {:<10} {:<15} {:<10}",
            position.symbol,
            mint_short,
            entry_price,
            exit_price,
            position.entry_size_sol,
            pnl_sol_colored,
            pnl_percent_colored,
            duration
        )
    );
}

/// Show detailed status for a specific position
async fn show_position_status(mint: &str, config: &PositionsManagerArgs) -> Result<(), String> {
    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "{}",
            format!("🔍 POSITION STATUS: {}", get_mint_prefix(mint)).bright_cyan().bold()
        )
    );
    log(LogTag::Positions, "INFO", &format!("{}", "=".repeat(50).bright_cyan()));

    if config.debug {
        log(LogTag::Positions, "INFO", &format!("Fetching current price for {}", mint));
    }

    // Get current price from price service
    let current_price = get_token_price_blocking_safe(mint).await;

    if let Some(price) = current_price {
        if config.debug {
            log(
                LogTag::Positions,
                "INFO",
                &format!("✅ Got current price for {}: ${:.12} SOL", mint, price)
            );
        }

        // Check if there's an open position
        let positions = get_open_positions().await;
        let open_position = positions.iter().find(|pos| pos.mint == mint);

        if let Some(position) = open_position {
            // Show open position details
            log(
                LogTag::Positions,
                "INFO",
                &format!("✅ {}", "OPEN POSITION FOUND".bright_green().bold())
            );
            log(LogTag::Positions, "INFO", &format!("   Mint: {}", mint));
            log(
                LogTag::Positions,
                "INFO",
                &format!("   Symbol: {}", position.symbol.bright_yellow())
            );
            log(
                LogTag::Positions,
                "INFO",
                &format!("   Entry Price: ${:.12} SOL", position.entry_price)
            );
            log(LogTag::Positions, "INFO", &format!("   Current Price: ${:.12} SOL", price));

            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(price)).await;
            let pnl_color = if pnl_percent > 0.0 {
                format!("🟢 P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).bright_green()
            } else if pnl_percent < 0.0 {
                format!("🔴 P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).bright_red()
            } else {
                format!("⚪ P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).white()
            };

            log(LogTag::Positions, "INFO", &format!("   {}", pnl_color));
            log(
                LogTag::Positions,
                "INFO",
                &format!("   Position Size: {:.6} SOL", position.entry_size_sol)
            );
            log(
                LogTag::Positions,
                "INFO",
                &format!("   Entry Time: {}", position.entry_time.format("%Y-%m-%d %H:%M:%S UTC"))
            );

            if let Some(sig) = &position.entry_transaction_signature {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Entry TX: {}", get_signature_prefix(sig))
                );

                // Check transaction verification status
                let verification_status = if position.transaction_entry_verified {
                    "✅ Verified".bright_green()
                } else {
                    "⏳ Pending verification".bright_yellow()
                };
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   📋 Entry status: {}", verification_status)
                );
            }
        } else {
            // Check closed positions
            let closed_positions = get_closed_positions().await;
            let closed_position = closed_positions.iter().find(|pos| pos.mint == mint);

            if let Some(position) = closed_position {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("📊 {}", "CLOSED POSITION FOUND".bright_blue().bold())
                );
                log(LogTag::Positions, "INFO", &format!("   Mint: {}", mint));
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Symbol: {}", position.symbol.bright_yellow())
                );
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Entry Price: ${:.12} SOL", position.entry_price)
                );

                if let Some(exit_price) = position.exit_price {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("   Exit Price: ${:.12} SOL", exit_price)
                    );

                    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None).await;
                    let pnl_color = if pnl_percent > 0.0 {
                        format!(
                            "🟢 Final P&L: {:.6} SOL ({:+.2}%)",
                            pnl_sol,
                            pnl_percent
                        ).bright_green()
                    } else if pnl_percent < 0.0 {
                        format!(
                            "🔴 Final P&L: {:.6} SOL ({:+.2}%)",
                            pnl_sol,
                            pnl_percent
                        ).bright_red()
                    } else {
                        format!("⚪ Final P&L: {:.6} SOL ({:+.2}%)", pnl_sol, pnl_percent).white()
                    };

                    log(LogTag::Positions, "INFO", &format!("   {}", pnl_color));
                }

                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Current Market Price: ${:.12} SOL", price)
                );
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Position Size: {:.6} SOL", position.entry_size_sol)
                );
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!(
                        "   Entry Time: {}",
                        position.entry_time.format("%Y-%m-%d %H:%M:%S UTC")
                    )
                );

                if let Some(exit_time) = position.exit_time {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("   Exit Time: {}", exit_time.format("%Y-%m-%d %H:%M:%S UTC"))
                    );
                }

                if let Some(sig) = &position.entry_transaction_signature {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("   Entry TX: {}", get_signature_prefix(sig))
                    );
                }

                if let Some(sig) = &position.exit_transaction_signature {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("   Exit TX: {}", get_signature_prefix(sig))
                    );
                }
            } else {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("ℹ️  {}", "NO POSITION FOUND".bright_yellow().bold())
                );
                log(LogTag::Positions, "INFO", &format!("   Mint: {}", mint));
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Current Market Price: ${:.12} SOL", price)
                );
                log(
                    LogTag::Positions,
                    "INFO",
                    "   Status: No open or closed positions for this token"
                );
                log(
                    LogTag::Positions,
                    "INFO",
                    "   💡 Use --action open --size <SOL> to open a position"
                );
            }
        }
    } else {
        log(
            LogTag::Positions,
            "INFO",
            &format!("❌ {}", "PRICE NOT AVAILABLE".bright_red().bold())
        );
        log(LogTag::Positions, "INFO", &format!("   Mint: {}", mint));
        log(LogTag::Positions, "INFO", "   Status: Unable to fetch current price");
        log(
            LogTag::Positions,
            "INFO",
            "   💡 Price service may be initializing or token may not be tradeable"
        );

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
    log(LogTag::Positions, "INFO", &format!("{}", "📊 POSITIONS SUMMARY".bright_magenta().bold()));
    log(LogTag::Positions, "INFO", &format!("{}", "===================".bright_magenta()));

    if get_positions_handle().await.is_none() {
        log(
            LogTag::Positions,
            "WARN",
            "⚠️  PositionsManager service not available. Start the main bot first to view actual summary."
        );
        return Ok(());
    }

    let open_positions = get_open_positions().await;
    let closed_positions = get_closed_positions().await;

    log(
        LogTag::Positions,
        "INFO",
        &format!("📈 Open Positions: {}", open_positions.len().to_string().bright_green())
    );
    log(
        LogTag::Positions,
        "INFO",
        &format!("📊 Closed Positions: {}", closed_positions.len().to_string().bright_yellow())
    );

    // Calculate total invested
    let total_invested: f64 = open_positions
        .iter()
        .chain(closed_positions.iter())
        .map(|p| p.entry_size_sol)
        .sum();

    log(
        LogTag::Positions,
        "INFO",
        &format!("💰 Total Invested: {:.6} SOL", total_invested.to_string().bright_cyan())
    );

    // Calculate total P&L for closed positions
    let mut total_realized_pnl = 0.0;
    for position in &closed_positions {
        let (pnl_sol, _) = calculate_position_pnl(position, None).await;
        total_realized_pnl += pnl_sol;
    }

    let total_realized_pnl_colored = if total_realized_pnl >= 0.0 {
        format!("{:+.6} SOL", total_realized_pnl).bright_green()
    } else {
        format!("{:+.6} SOL", total_realized_pnl).bright_red()
    };

    log(LogTag::Positions, "INFO", &format!("💸 Realized P&L: {}", total_realized_pnl_colored));

    // Calculate unrealized P&L for open positions (with rate limiting)
    let open_positions = get_open_positions().await;
    let mut total_unrealized_pnl = 0.0;
    let mut unrealized_positions_count = 0;

    // Limit the number of price lookups to avoid API spam
    let max_price_lookups = 10; // Limit to first 10 positions
    let mut price_lookups = 0;

    for position in open_positions.iter().take(max_price_lookups) {
        if let Some(price_service) = screenerbot::tokens::price::PRICE_SERVICE.get() {
            if let Some(current_price) = price_service.get_token_price(&position.mint).await {
                let (pnl_sol, _pnl_percent) = calculate_position_pnl(
                    position,
                    Some(current_price)
                ).await;
                total_unrealized_pnl += pnl_sol;
                unrealized_positions_count += 1;
                price_lookups += 1;

                // Small delay to avoid overwhelming price service
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        }
    }

    let total_open_positions = open_positions.len();
    let unrealized_display = if unrealized_positions_count < total_open_positions {
        format!(
            "📊 Unrealized P&L: {} ({}/{} positions with prices)",
            if total_unrealized_pnl >= 0.0 {
                format!("{:+.6} SOL", total_unrealized_pnl).bright_green()
            } else {
                format!("{:+.6} SOL", total_unrealized_pnl).bright_red()
            },
            unrealized_positions_count,
            total_open_positions
        )
    } else {
        format!(
            "📊 Unrealized P&L: {} ({} positions)",
            if total_unrealized_pnl >= 0.0 {
                format!("{:+.6} SOL", total_unrealized_pnl).bright_green()
            } else {
                format!("{:+.6} SOL", total_unrealized_pnl).bright_red()
            },
            unrealized_positions_count
        )
    };

    log(LogTag::Positions, "INFO", &unrealized_display);

    Ok(())
}

/// Show verbose diagnostics for all positions (open + closed + closing)
async fn show_positions_diagnostics(config: &PositionsManagerArgs) -> Result<(), String> {
    log(
        LogTag::Positions,
        "INFO",
        &format!("{}", "🧪 POSITIONS DIAGNOSTICS".bright_white().bold())
    );
    log(LogTag::Positions, "INFO", &format!("{}", "======================".bright_white()));

    if get_positions_handle().await.is_none() {
        log(LogTag::Positions, "WARN", "⚠️  PositionsManager service not available.");
        return Ok(());
    }

    let open_positions = get_open_positions().await;
    let closed_positions = get_closed_positions().await;

    log(
        LogTag::Positions,
        "INFO",
        &format!("Open/Closing: {} | Closed: {}", open_positions.len(), closed_positions.len())
    );
    log(LogTag::Positions, "INFO", "");

    for p in open_positions.iter().chain(closed_positions.iter()) {
        log(
            LogTag::Positions,
            "INFO",
            &format!("{} {}", if p.exit_price.is_some() { "📊" } else { "📈" }, p.symbol.bold())
        );
        log(LogTag::Positions, "INFO", &format!("  Mint: {}", get_mint_prefix(&p.mint)));
        log(
            LogTag::Positions,
            "INFO",
            &format!("  Entry: {:.10} SOL @ {}", p.entry_price, p.entry_time.format("%H:%M:%S"))
        );
        if let Some(exit_p) = p.exit_price {
            log(LogTag::Positions, "INFO", &format!("  Exit:  {:.10} SOL", exit_p));
        }
        log(LogTag::Positions, "INFO", &format!("  Size: {:.6} SOL", p.entry_size_sol));
        log(
            LogTag::Positions,
            "INFO",
            &format!(
                "  Tx Entry: {:?} (verified={})",
                p.entry_transaction_signature.as_ref().map(|s| get_signature_prefix(s)),
                p.transaction_entry_verified
            )
        );
        if p.exit_transaction_signature.is_some() {
            log(
                LogTag::Positions,
                "INFO",
                &format!(
                    "  Tx Exit:  {:?} (verified={})",
                    p.exit_transaction_signature.as_ref().map(|s| get_signature_prefix(s)),
                    p.transaction_exit_verified
                )
            );
        }
        log(
            LogTag::Positions,
            "INFO",
            &format!("  High/Low: {:.10} / {:.10}", p.price_highest, p.price_lowest)
        );
        if p.phantom_remove {
            log(LogTag::Positions, "INFO", "  Flag: PHANTOM_REMOVE");
        }
        log(LogTag::Positions, "INFO", "");
    }

    Ok(())
}

/// Force reverification of all unverified transactions
async fn force_reverify_positions(config: &PositionsManagerArgs) -> Result<(), String> {
    log(
        LogTag::Positions,
        "INFO",
        &format!("{}", "🔄 FORCE REVERIFY TRANSACTIONS".bright_yellow().bold())
    );
    log(LogTag::Positions, "INFO", &format!("{}", "=============================".bright_yellow()));

    if get_positions_handle().await.is_none() {
        log(LogTag::Positions, "WARN", "⚠️  PositionsManager service not available.");
        return Ok(());
    }

    log(
        LogTag::Positions,
        "INFO",
        "⏳ Requesting forced reverification of all unverified transactions..."
    );

    if config.debug || config.debug_transactions {
        log(LogTag::Positions, "DEBUG", "🔄 Triggering force reverification via positions handle");
    }

    match get_positions_handle().await.unwrap().force_reverify_all().await {
        count if count > 0 => {
            log(
                LogTag::Positions,
                "INFO",
                &format!("✅ {} unverified transactions re-queued for verification", count)
            );
            log(
                LogTag::Positions,
                "INFO",
                "   Verification will occur in the background every 10 seconds"
            );

            if config.debug || config.debug_transactions {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("✅ Force reverification completed: {} transactions re-queued", count)
                );
            }
        }
        0 => {
            log(
                LogTag::Positions,
                "INFO",
                "ℹ️  No unverified transactions found - all positions are already verified"
            );
        }
        _ => {
            log(LogTag::Positions, "WARN", "❌ Unexpected result from reverification command");
        }
    }

    Ok(())
}

/// Open a new position for the specified token
async fn open_position(mint: &str, size: f64, config: &PositionsManagerArgs) -> Result<(), String> {
    log(
        LogTag::Positions,
        "INFO",
        &format!(
            "{}",
            format!("📈 OPENING POSITION: {}", get_mint_prefix(mint)).bright_green().bold()
        )
    );
    log(LogTag::Positions, "INFO", &format!("{}", "=".repeat(50).bright_green()));

    if config.dry_run {
        log(LogTag::Positions, "INFO", "🧪 DRY RUN MODE: Position would be opened");
        log(LogTag::Positions, "INFO", &format!("   Mint: {}", mint));
        log(LogTag::Positions, "INFO", &format!("   Size: {:.6} SOL", size));
        return Ok(());
    }

    // Step 1: Get current token price
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting current price for {}", mint));
    }

    let current_price = get_token_price_blocking_safe(mint).await.ok_or_else(||
        format!("Failed to get current price for {}", mint)
    )?;

    log(LogTag::Positions, "INFO", &format!("✅ Current Price: ${:.12} SOL", current_price));

    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Got price: ${:.12} SOL", current_price));
    }

    // Step 2: Get token information
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting token information for {}", mint));
    }

    let token = get_token_from_db(mint).await.ok_or_else(||
        format!("Token not found in database: {}", mint)
    )?;

    log(LogTag::Positions, "INFO", &format!("✅ Token Found: {} ({})", token.symbol, token.name));

    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Token: {} - {}", token.symbol, token.name));
    }

    // Step 3: Check if position already exists
    let existing_positions = get_open_positions().await;
    if existing_positions.iter().any(|pos| pos.mint == mint) {
        return Err(format!("Position already exists for token {}", token.symbol));
    }

    // Step 4: Open the position
    log(LogTag::Positions, "INFO", "🚀 Opening position...");

    if config.debug {
        log(
            LogTag::Positions,
            "INFO",
            &format!(
                "📈 Opening position for {} at ${:.12} SOL with size {:.6} SOL",
                token.symbol,
                current_price,
                size
            )
        );
    }

    // Calculate percent change (we'll use 0.0 as initial value since this is a new position)
    let percent_change = 0.0;

    match open_position_global(token.clone(), current_price, percent_change, size).await {
        Ok((position_id, transaction_sig)) => {
            log(LogTag::Positions, "INFO", "✅ Position opened successfully!");
            log(LogTag::Positions, "INFO", &format!("   Position ID: {}", position_id));
            log(LogTag::Positions, "INFO", &format!("   Symbol: {}", token.symbol));
            log(LogTag::Positions, "INFO", &format!("   Entry Price: ${:.12} SOL", current_price));
            log(LogTag::Positions, "INFO", &format!("   Size: {:.6} SOL", size));

            if !transaction_sig.is_empty() {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Transaction: {}", get_signature_prefix(&transaction_sig))
                );

                if config.debug {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("📄 Transaction signature: {}", transaction_sig)
                    );
                }

                // Step 5: Wait for transaction verification (CRITICAL)
                log(LogTag::Positions, "INFO", "⏳ Waiting for transaction verification...");

                if config.debug {
                    log(
                        LogTag::Positions,
                        "INFO",
                        "⏳ Waiting for transaction to be confirmed on-chain"
                    );
                }

                // Simple verification waiting loop (configurable)
                let mut verification_attempts = 0;
                let max_attempts = if config.verify_poll_secs > 0 {
                    (config.verify_timeout_secs / config.verify_poll_secs).max(1)
                } else {
                    30
                };
                let mut verified = false;

                while verification_attempts < max_attempts && !verified {
                    tokio::time::sleep(Duration::from_secs(config.verify_poll_secs)).await;
                    verification_attempts += 1;

                    match get_transaction(&transaction_sig).await {
                        Ok(Some(transaction)) => {
                            if transaction.success {
                                log(
                                    LogTag::Positions,
                                    "INFO",
                                    "✅ Transaction verified successfully!"
                                );
                                verified = true;
                                if config.debug {
                                    log(
                                        LogTag::Positions,
                                        "INFO",
                                        &format!(
                                            "✅ Transaction {} verified successfully",
                                            get_signature_prefix(&transaction_sig)
                                        )
                                    );
                                }
                            } else {
                                log(
                                    LogTag::Positions,
                                    "ERROR",
                                    "❌ Transaction found but marked as failed!"
                                );
                                log(
                                    LogTag::Positions,
                                    "ERROR",
                                    &format!(
                                        "❌ Transaction {} found but marked as failed",
                                        get_signature_prefix(&transaction_sig)
                                    )
                                );
                                return Err(
                                    "Transaction failed on-chain - position may be phantom".to_string()
                                );
                            }
                        }
                        Ok(None) => {
                            if
                                (config.debug || config.debug_transactions) &&
                                verification_attempts % 10 == 0
                            {
                                log(
                                    LogTag::Positions,
                                    "INFO",
                                    &format!(
                                        "⏳ Still waiting for transaction... (attempt {}/{})",
                                        verification_attempts,
                                        max_attempts
                                    )
                                );
                            }
                        }
                        Err(e) => {
                            if config.debug {
                                log(
                                    LogTag::Positions,
                                    "WARNING",
                                    &format!(
                                        "Transaction verification error (attempt {}): {}",
                                        verification_attempts,
                                        e
                                    )
                                );
                            }
                        }
                    }
                }

                if !verified {
                    log(
                        LogTag::Positions,
                        "WARN",
                        &format!(
                            "⚠️  Transaction verification timed out after {} seconds",
                            config.verify_timeout_secs
                        )
                    );
                    log(
                        LogTag::Positions,
                        "WARNING",
                        &format!(
                            "⚠️  Transaction {} verification timed out",
                            get_signature_prefix(&transaction_sig)
                        )
                    );
                    return Err(
                        "Transaction verification timed out - position may be phantom".to_string()
                    );
                }
            }

            if config.debug {
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("✅ Position opened: ID={}, Symbol={}", position_id, token.symbol)
                );
            }

            Ok(())
        }
        Err(e) => {
            log(LogTag::Positions, "ERROR", &format!("❌ Failed to open position: {}", e));

            if config.debug {
                log(LogTag::Positions, "ERROR", &format!("❌ Position opening failed: {}", e));
            }

            Err(format!("Position opening failed: {}", e))
        }
    }
}

/// Close an existing position for the specified token
async fn close_position(mint: &str, config: &PositionsManagerArgs) -> Result<(), String> {
    log(LogTag::Positions, "INFO", &format!("📉 CLOSING POSITION: {}", get_mint_prefix(mint)));
    log(LogTag::Positions, "INFO", &"=".repeat(50));

    if config.dry_run {
        log(LogTag::Positions, "INFO", "🧪 DRY RUN MODE: Position would be closed");
        log(LogTag::Positions, "INFO", &format!("   Mint: {}", mint));
        return Ok(());
    }

    if config.debug {
        log(LogTag::Positions, "DEBUG", &format!("Close position requested for mint: {}", mint));
    }

    // Step 1: Find existing open position
    if config.debug {
        log(LogTag::Positions, "INFO", "🔍 Looking for existing open position");
    }

    let open_positions = get_open_positions().await;
    let position = open_positions
        .iter()
        .find(|pos| pos.mint == mint)
        .ok_or_else(|| format!("No open position found for token {}", mint))?;

    log(
        LogTag::Positions,
        "INFO",
        &format!("✅ Position Found: {} ({})", position.symbol, position.name)
    );

    if config.debug {
        log(
            LogTag::Positions,
            "INFO",
            &format!(
                "✅ Found position: {} - entry price: ${:.12} SOL",
                position.symbol,
                position.entry_price
            )
        );
    }

    // Step 2: Get current token price
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting current price for {}", mint));
    }

    let current_price = get_token_price_blocking_safe(mint).await.ok_or_else(||
        format!("Failed to get current price for {}", mint)
    )?;

    log(LogTag::Positions, "INFO", &format!("✅ Current Price: ${:.12} SOL", current_price));

    if config.debug {
        log(LogTag::Positions, "INFO", &format!("✅ Got exit price: ${:.12} SOL", current_price));
    }

    // Step 3: Get token information for closing
    if config.debug {
        log(LogTag::Positions, "INFO", &format!("🔍 Getting token information for closing"));
    }

    let token = get_token_from_db(mint).await.ok_or_else(||
        format!("Token not found in database: {}", mint)
    )?;

    // Step 4: Calculate P&L before closing
    let pnl_percentage = if position.entry_price > 0.0 {
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    } else {
        0.0
    };

    log(LogTag::Positions, "INFO", &format!("📊 Position P&L: {:.2}%", pnl_percentage));

    if config.debug {
        log(
            LogTag::Positions,
            "INFO",
            &format!(
                "📊 P&L calculation: entry={:.12}, current={:.12}, pnl={:.2}%",
                position.entry_price,
                current_price,
                pnl_percentage
            )
        );
    }

    // Step 5: Close the position using positions system
    log(LogTag::Positions, "INFO", "🚀 Closing position...");

    if config.debug {
        log(
            LogTag::Positions,
            "INFO",
            &format!("📉 Closing position for {} at ${:.12} SOL", position.symbol, current_price)
        );
    }

    // Get the PositionsManager handle for closing the position
    if let Some(handle) = get_positions_handle().await {
        let exit_time = Utc::now();
        match handle.close_position(mint.to_string(), token, current_price, exit_time).await {
            Ok((position_id, exit_transaction)) => {
                log(LogTag::Positions, "INFO", "✅ Position closed successfully!");
                log(LogTag::Positions, "INFO", &format!("   Position ID: {}", mint));
                log(LogTag::Positions, "INFO", &format!("   Symbol: {}", position.symbol));
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Entry Price: ${:.12} SOL", position.entry_price)
                );
                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("   Exit Price: ${:.12} SOL", current_price)
                );
                log(LogTag::Positions, "INFO", &format!("   P&L: {:.2}%", pnl_percentage));

                if !exit_transaction.is_empty() {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("   Transaction: {}", get_signature_prefix(&exit_transaction))
                    );

                    if config.debug {
                        log(
                            LogTag::Positions,
                            "INFO",
                            &format!("📄 Exit transaction signature: {}", exit_transaction)
                        );
                    }

                    // Step 6: Wait for exit transaction verification (CRITICAL)
                    log(
                        LogTag::Positions,
                        "INFO",
                        "⏳ Waiting for exit transaction verification..."
                    );

                    if config.debug {
                        log(
                            LogTag::Positions,
                            "INFO",
                            "⏳ Waiting for exit transaction to be confirmed on-chain"
                        );
                    }

                    // Simple verification waiting loop for exit transaction (configurable)
                    let mut verification_attempts = 0;
                    let max_attempts = if config.verify_poll_secs > 0 {
                        (config.verify_timeout_secs / config.verify_poll_secs).max(1)
                    } else {
                        30
                    };
                    let mut verified = false;

                    while verification_attempts < max_attempts && !verified {
                        tokio::time::sleep(Duration::from_secs(config.verify_poll_secs)).await;
                        verification_attempts += 1;

                        match get_transaction(&exit_transaction).await {
                            Ok(Some(transaction)) => {
                                if transaction.success {
                                    log(
                                        LogTag::Positions,
                                        "INFO",
                                        "✅ Exit transaction verified successfully!"
                                    );
                                    verified = true;
                                    if config.debug {
                                        log(
                                            LogTag::Positions,
                                            "INFO",
                                            &format!(
                                                "✅ Exit transaction {} verified successfully",
                                                get_signature_prefix(&exit_transaction)
                                            )
                                        );
                                    }
                                } else {
                                    log(
                                        LogTag::Positions,
                                        "ERROR",
                                        "❌ Exit transaction found but marked as failed!"
                                    );
                                    log(
                                        LogTag::Positions,
                                        "ERROR",
                                        &format!(
                                            "❌ Exit transaction {} found but marked as failed",
                                            get_signature_prefix(&exit_transaction)
                                        )
                                    );
                                    return Err("Exit transaction failed on-chain".to_string());
                                }
                            }
                            Ok(None) => {
                                if
                                    (config.debug || config.debug_transactions) &&
                                    verification_attempts % 10 == 0
                                {
                                    log(
                                        LogTag::Positions,
                                        "INFO",
                                        &format!(
                                            "⏳ Still waiting for exit transaction... (attempt {}/{})",
                                            verification_attempts,
                                            max_attempts
                                        )
                                    );
                                }
                            }
                            Err(e) => {
                                if config.debug {
                                    log(
                                        LogTag::Positions,
                                        "WARNING",
                                        &format!(
                                            "Exit transaction verification error (attempt {}): {}",
                                            verification_attempts,
                                            e
                                        )
                                    );
                                }
                            }
                        }
                    }

                    if !verified {
                        log(
                            LogTag::Positions,
                            "WARN",
                            &format!(
                                "⚠️  Exit transaction verification timed out after {} seconds",
                                config.verify_timeout_secs
                            )
                        );
                        log(
                            LogTag::Positions,
                            "WARNING",
                            &format!(
                                "⚠️  Exit transaction {} verification timed out",
                                get_signature_prefix(&exit_transaction)
                            )
                        );
                        return Err("Exit transaction verification timed out".to_string());
                    }
                }

                if config.debug {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!(
                            "✅ Position closed: ID={}, Symbol={}, P&L={:.2}%",
                            mint,
                            position.symbol,
                            pnl_percentage
                        )
                    );
                }

                Ok(())
            }
            Err(e) => {
                log(LogTag::Positions, "ERROR", &format!("❌ Failed to close position: {}", e));

                if config.debug {
                    log(LogTag::Positions, "ERROR", &format!("❌ Position closing failed: {}", e));
                }

                Err(format!("Position closing failed: {}", e))
            }
        }
    } else {
        let error_msg = "PositionsManager service not available";
        log(LogTag::Positions, "ERROR", &format!("❌ {}", error_msg));

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

    // Validate base58 encoding and proper length
    if mint.len() < 32 || mint.len() > 44 {
        return Err("Invalid mint address: wrong length".to_string());
    }

    // Basic character validation for base58
    let valid_chars = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    for ch in mint.chars() {
        if !valid_chars.contains(ch) {
            return Err("Invalid mint address: contains invalid characters".to_string());
        }
    }

    Ok(())
}

/// Get token information for display purposes
async fn get_token_info_placeholder(mint: &str) -> Result<(String, String), String> {
    // Try to get token info from various sources

    // First try from positions data (if we have a position for this token)
    let positions = screenerbot::positions::get_all_positions().await;
    if let Some(position) = positions.iter().find(|p| p.mint == mint) {
        return Ok((position.symbol.clone(), format!("{} (from position)", position.symbol)));
    }

    // Try to get from DexScreener API
    match screenerbot::tokens::dexscreener::get_token_price_from_global_api(mint).await {
        Some(_) => {
            // If we can get price data, the token exists
            // For now return mint as symbol (could be enhanced to fetch metadata)
            let short_mint = format!("{}...", &mint[..8]);
            return Ok((short_mint.clone(), short_mint));
        }
        None => {
            // Fallback to mint address format
            let short_mint = format!("{}...", &mint[..8]);
            return Ok((short_mint.clone(), format!("{} (unknown token)", short_mint)));
        }
    }

    let symbol = format!("TOKEN_{}", &mint[..8]);
    let name = format!("Token {}", &mint[..8]);

    Ok((symbol, name))
}

/// Run continuous test loop for opening and closing positions with verification
async fn run_continuous_test_loop(config: &PositionsManagerArgs) -> Result<(), String> {
    log(LogTag::Positions, "INFO", "🔁 CONTINUOUS POSITION TESTING LOOP");
    log(LogTag::Positions, "INFO", &"=".repeat(50));

    // Test configuration
    let test_sol_amount = 0.005; // Fixed 0.005 SOL for all tests
    let default_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(); // BONK token - good for testing
    let test_mint = config.target_mint.as_ref().unwrap_or(&default_mint);
    let max_iterations = config.test_iterations.unwrap_or(usize::MAX); // Default: infinite

    log(LogTag::Positions, "INFO", "🎯 Test Configuration:");
    log(LogTag::Positions, "INFO", &format!("   Token Mint: {}", get_mint_prefix(test_mint)));
    log(LogTag::Positions, "INFO", &format!("   Position Size: {:.6} SOL", test_sol_amount));
    log(
        LogTag::Positions,
        "INFO",
        &format!("   Max Iterations: {}", if max_iterations == usize::MAX {
            "infinite".to_string()
        } else {
            max_iterations.to_string()
        })
    );
    log(LogTag::Positions, "INFO", &format!("   Debug Mode: {}", config.debug));
    log(LogTag::Positions, "INFO", "");

    if config.debug {
        log(
            LogTag::Positions,
            "INFO",
            &format!(
                "🔁 Starting continuous test loop: mint={}, size={:.6}, max_iter={}",
                test_mint,
                test_sol_amount,
                max_iterations
            )
        );
    }

    let mut iteration = 0;
    let mut successful_cycles = 0;
    let mut failed_cycles = 0;

    loop {
        iteration += 1;

        if iteration > max_iterations {
            log(LogTag::Positions, "INFO", "✅ Test loop completed: reached maximum iterations");
            break;
        }

        log(
            LogTag::Positions,
            "INFO",
            &format!("🔄 ITERATION {} / {}", iteration, if max_iterations == usize::MAX {
                "∞".to_string()
            } else {
                max_iterations.to_string()
            })
        );
        log(LogTag::Positions, "INFO", &"=".repeat(40));

        let start_time = std::time::Instant::now();

        // Check if we already have an open position
        let existing_positions = get_open_positions().await;
        let has_open_position = existing_positions.iter().any(|pos| pos.mint == *test_mint);

        if has_open_position {
            log(LogTag::Positions, "INFO", "🔍 Found existing open position, closing first...");
            if let Err(e) = close_position(test_mint, config).await {
                log(
                    LogTag::Positions,
                    "ERROR",
                    &format!("❌ Failed to close existing position: {}", e)
                );
                failed_cycles += 1;

                if config.debug {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!("❌ Iteration {} failed - close error: {}", iteration, e)
                    );
                }

                // Wait before retrying
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Wait a bit between close and open
            log(LogTag::Positions, "INFO", "⏳ Waiting 3 seconds between close and open...");
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        // Step 1: Open position with verification
        log(LogTag::Positions, "INFO", "📈 PHASE 1: Opening position...");
        match open_position(test_mint, test_sol_amount, config).await {
            Ok(_) => {
                log(LogTag::Positions, "INFO", "✅ Position opened and verified successfully!");

                if config.debug {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("✅ Iteration {} - position opened successfully", iteration)
                    );
                }
            }
            Err(e) => {
                log(LogTag::Positions, "ERROR", &format!("❌ Failed to open position: {}", e));
                failed_cycles += 1;

                if config.debug {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!("❌ Iteration {} failed - open error: {}", iteration, e)
                    );
                }

                // Wait before retrying
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        }

        // Wait between open and close operations
        log(LogTag::Positions, "INFO", "⏳ Waiting 10 seconds before closing position...");
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Step 2: Close position with verification
        log(LogTag::Positions, "INFO", "📉 PHASE 2: Closing position...");
        match close_position(test_mint, config).await {
            Ok(_) => {
                log(LogTag::Positions, "INFO", "✅ Position closed and verified successfully!");
                successful_cycles += 1;

                if config.debug {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("✅ Iteration {} - position closed successfully", iteration)
                    );
                }
            }
            Err(e) => {
                log(LogTag::Positions, "ERROR", &format!("❌ Failed to close position: {}", e));
                failed_cycles += 1;

                if config.debug {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!("❌ Iteration {} failed - close error: {}", iteration, e)
                    );
                }

                // Continue to next iteration even if close fails
            }
        }

        let cycle_duration = start_time.elapsed();

        // Print iteration summary
        log(LogTag::Positions, "INFO", "");
        log(LogTag::Positions, "INFO", "📊 ITERATION SUMMARY");
        log(
            LogTag::Positions,
            "INFO",
            &format!("   Iteration: {} / {}", iteration, if max_iterations == usize::MAX {
                "∞".to_string()
            } else {
                max_iterations.to_string()
            })
        );
        log(
            LogTag::Positions,
            "INFO",
            &format!("   Duration: {:.2}s", cycle_duration.as_secs_f64())
        );
        log(LogTag::Positions, "INFO", &format!("   Successful Cycles: {}", successful_cycles));
        log(LogTag::Positions, "INFO", &format!("   Failed Cycles: {}", failed_cycles));
        log(
            LogTag::Positions,
            "INFO",
            &format!("   Success Rate: {:.1}%", if iteration > 0 {
                ((successful_cycles as f64) / (iteration as f64)) * 100.0
            } else {
                0.0
            })
        );
        log(LogTag::Positions, "INFO", "");

        // Wait between iterations
        log(LogTag::Positions, "INFO", "⏳ Waiting 15 seconds before next iteration...");
        tokio::time::sleep(Duration::from_secs(15)).await;
    }

    // Final summary
    log(LogTag::Positions, "INFO", "🎉 FINAL TEST SUMMARY");
    log(LogTag::Positions, "INFO", &"=".repeat(50));
    log(LogTag::Positions, "INFO", &format!("   Total Iterations: {}", iteration));
    log(LogTag::Positions, "INFO", &format!("   Successful Cycles: {}", successful_cycles));
    log(LogTag::Positions, "INFO", &format!("   Failed Cycles: {}", failed_cycles));
    log(
        LogTag::Positions,
        "INFO",
        &format!("   Final Success Rate: {:.1}%", if iteration > 0 {
            ((successful_cycles as f64) / (iteration as f64)) * 100.0
        } else {
            0.0
        })
    );

    if config.debug {
        log(
            LogTag::Positions,
            "INFO",
            &format!(
                "🎉 Test loop completed: {} iterations, {} successful, {} failed",
                iteration,
                successful_cycles,
                failed_cycles
            )
        );
    }

    Ok(())
}
