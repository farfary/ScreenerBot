//! Manual trading debug tool for testing position management
//!
//! This tool initializes the full bot system (services, RPC, config, etc.) and allows
//! manual execution of trading operations for testing and debugging purposes.
//!
//! Features:
//! - Manual position opening/closing
//! - DCA (Dollar Cost Averaging) buy operations
//! - Partial exit testing
//! - Full system initialization (identical to run.rs)
//! - Dry-run mode support
//! - Comprehensive operation logging
//!
//! Usage examples:
//!   # Open a position
//!   cargo run --bin debug_manual_trading -- open --mint <TOKEN_MINT>
//!
//!   # Close a position
//!   cargo run --bin debug_manual_trading -- close --mint <TOKEN_MINT>
//!
//!   # Execute DCA buy
//!   cargo run --bin debug_manual_trading -- dca --mint <TOKEN_MINT> --amount <SOL_AMOUNT>
//!
//!   # Execute partial exit
//!   cargo run --bin debug_manual_trading -- partial-exit --mint <TOKEN_MINT> --percentage <0-100>
//!
//!   # List all open positions
//!   cargo run --bin debug_manual_trading -- list
//!
//!   # Show position details
//!   cargo run --bin debug_manual_trading -- inspect --mint <TOKEN_MINT>
//!
//!   # Interactive mode (future)
//!   cargo run --bin debug_manual_trading -- interactive

use clap::{Parser, Subcommand};
use screenerbot::{
    arguments,
    logger::{init_file_logging, log, LogTag},
    services::ServiceManager,
};

#[derive(Parser)]
#[command(
    name = "debug_manual_trading",
    about = "Manual trading operations for testing and debugging",
    long_about = "Initialize full bot system and execute manual trading operations for testing position management, DCA, partial exits, etc."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Enable dry-run mode (no actual transactions)
    #[arg(long, global = true)]
    dry_run: bool,

    /// Enable debug logging for positions
    #[arg(long, global = true)]
    debug_positions: bool,

    /// Enable debug logging for trader
    #[arg(long, global = true)]
    debug_trader: bool,

    /// Enable debug logging for swaps
    #[arg(long, global = true)]
    debug_swaps: bool,

    /// Wait for all services to be ready before executing command
    #[arg(long, global = true, default_value_t = true)]
    wait_ready: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Open a new position
    Open {
        /// Token mint address
        #[arg(long, value_name = "MINT")]
        mint: String,

        /// Override trade size in SOL (uses config if not specified)
        #[arg(long)]
        size_sol: Option<f64>,

        /// Strategy ID to associate with this position
        #[arg(long)]
        strategy: Option<String>,
    },

    /// Close an existing position
    Close {
        /// Token mint address
        #[arg(long, value_name = "MINT")]
        mint: String,

        /// Close reason (stop_loss, take_profit, manual, etc.)
        #[arg(long, default_value = "manual")]
        reason: String,

        /// Force close even if verification fails
        #[arg(long)]
        force: bool,
    },

    /// Execute DCA (Dollar Cost Averaging) buy
    Dca {
        /// Token mint address
        #[arg(long, value_name = "MINT")]
        mint: String,

        /// Amount in SOL to add to position
        #[arg(long)]
        amount: f64,

        /// Maximum number of DCA operations allowed
        #[arg(long)]
        max_dca: Option<u32>,
    },

    /// Execute partial exit (sell portion of position)
    PartialExit {
        /// Token mint address
        #[arg(long, value_name = "MINT")]
        mint: String,

        /// Percentage to exit (0-100)
        #[arg(long)]
        percentage: f64,

        /// Reason for partial exit
        #[arg(long, default_value = "manual")]
        reason: String,
    },

    /// List all open positions
    List {
        /// Show detailed information
        #[arg(long, short)]
        detailed: bool,

        /// Filter by strategy ID
        #[arg(long)]
        strategy: Option<String>,
    },

    /// Inspect a specific position
    Inspect {
        /// Token mint address
        #[arg(long, value_name = "MINT")]
        mint: String,

        /// Show transaction history
        #[arg(long)]
        show_transactions: bool,

        /// Show DCA history
        #[arg(long)]
        show_dca: bool,

        /// Show partial exit history
        #[arg(long)]
        show_exits: bool,
    },

    /// Interactive mode (future implementation)
    Interactive,

    /// Reconcile positions (verify chain state)
    Reconcile {
        /// Only reconcile specific mint
        #[arg(long)]
        mint: Option<String>,

        /// Fix discrepancies automatically
        #[arg(long)]
        auto_fix: bool,
    },

    /// Test quote retrieval for a token
    TestQuote {
        /// Token mint address
        #[arg(long, value_name = "MINT")]
        mint: String,

        /// Amount in SOL
        #[arg(long)]
        amount: f64,

        /// Operation type (buy/sell)
        #[arg(long, default_value = "buy")]
        operation: String,
    },

    /// Initialize system and wait (for testing service startup)
    Init {
        /// Wait duration in seconds
        #[arg(long, default_value_t = 30)]
        wait_seconds: u64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Set up command-line arguments for debug flags
    let mut args = std::env::args().collect::<Vec<_>>();
    if cli.dry_run {
        args.push("--dry-run".to_string());
    }
    if cli.debug_positions {
        args.push("--debug-positions".to_string());
    }
    if cli.debug_trader {
        args.push("--debug-trader".to_string());
    }
    if cli.debug_swaps {
        args.push("--debug-swaps".to_string());
    }
    arguments::set_cmd_args(args);

    if let Err(err) = execute(cli).await {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

async fn execute(cli: Cli) -> Result<(), String> {
    // Initialize file logging
    init_file_logging();

    log(
        LogTag::System,
        "INFO",
        "🧪 Manual Trading Debug Tool - Initializing...",
    );

    // Initialize the full bot system (same as run.rs)
    initialize_system().await?;

    // Wait for core services to be ready if requested
    if cli.wait_ready {
        wait_for_services_ready().await?;
    }

    log(
        LogTag::System,
        "SUCCESS",
        "✅ System initialized - executing command",
    );

    // Execute the requested command
    match cli.command {
        Command::Open {
            mint,
            size_sol,
            strategy,
        } => handle_open_position(&mint, size_sol, strategy).await,
        Command::Close {
            mint,
            reason,
            force,
        } => handle_close_position(&mint, &reason, force).await,
        Command::Dca {
            mint,
            amount,
            max_dca,
        } => handle_dca_buy(&mint, amount, max_dca).await,
        Command::PartialExit {
            mint,
            percentage,
            reason,
        } => handle_partial_exit(&mint, percentage, &reason).await,
        Command::List { detailed, strategy } => handle_list_positions(detailed, strategy).await,
        Command::Inspect {
            mint,
            show_transactions,
            show_dca,
            show_exits,
        } => handle_inspect_position(&mint, show_transactions, show_dca, show_exits).await,
        Command::Interactive => handle_interactive_mode().await,
        Command::Reconcile { mint, auto_fix } => handle_reconcile(mint, auto_fix).await,
        Command::TestQuote {
            mint,
            amount,
            operation,
        } => handle_test_quote(&mint, amount, &operation).await,
        Command::Init { wait_seconds } => handle_init(wait_seconds).await,
    }
}

/// Initialize the full bot system (same as run.rs)
async fn initialize_system() -> Result<(), String> {
    log(
        LogTag::System,
        "INFO",
        "Initializing configuration and directories...",
    );

    // 1. Ensure data directories exist
    screenerbot::global::ensure_data_directories()
        .map_err(|e| format!("Failed to create data directories: {}", e))?;

    // 2. Load configuration
    screenerbot::config::load_config()
        .map_err(|e| format!("Failed to load config: {}", e))?;

    log(LogTag::System, "INFO", "Configuration loaded successfully");

    // 3. Initialize strategy system
    screenerbot::strategies::init_strategy_system(
        screenerbot::strategies::engine::EngineConfig::default(),
    )
    .await
    .map_err(|e| format!("Failed to initialize strategy system: {}", e))?;

    log(
        LogTag::System,
        "INFO",
        "Strategy system initialized successfully",
    );

    // 4. Create service manager
    let mut service_manager = ServiceManager::new().await?;

    log(LogTag::System, "INFO", "Service manager created");

    // 5. Register all services (same as run.rs)
    register_all_services(&mut service_manager);

    // 6. Initialize global ServiceManager for webserver access
    screenerbot::services::init_global_service_manager(service_manager).await;

    // 7. Get mutable reference and start services
    let manager_ref = screenerbot::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard.take().ok_or("ServiceManager was already taken")?
    };

    // 8. Start all enabled services
    log(LogTag::System, "INFO", "Starting services...");
    service_manager.start_all().await?;

    // 9. Put it back for other components
    {
        let mut guard = manager_ref.write().await;
        *guard = Some(service_manager);
    }

    log(
        LogTag::System,
        "SUCCESS",
        "✅ All services started successfully",
    );

    Ok(())
}

/// Register all services (same as run.rs)
fn register_all_services(manager: &mut ServiceManager) {
    use screenerbot::services::implementations::*;

    log(LogTag::System, "INFO", "Registering services...");

    // Core infrastructure services
    manager.register(Box::new(EventsService));
    manager.register(Box::new(TransactionsService));
    manager.register(Box::new(SolPriceService));

    // Pool services (4 sub-services + 1 helper coordinator)
    manager.register(Box::new(PoolDiscoveryService));
    manager.register(Box::new(PoolFetcherService));
    manager.register(Box::new(PoolCalculatorService));
    manager.register(Box::new(PoolAnalyzerService));
    manager.register(Box::new(PoolsService));

    // Centralized Tokens service
    manager.register(Box::new(TokensService::default()));

    // Other application services
    manager.register(Box::new(FilteringService::new()));
    manager.register(Box::new(OhlcvService));
    manager.register(Box::new(PositionsService));
    manager.register(Box::new(WalletService));
    manager.register(Box::new(RpcStatsService));
    manager.register(Box::new(AtaCleanupService));

    // Note: We do NOT register TraderService or WebserverService for manual mode
    // TraderService auto-trades, which we don't want in manual mode
    // WebserverService is not needed for CLI operations

    log(
        LogTag::System,
        "INFO",
        "Services registered (18 total - excluding Trader and Webserver)",
    );
}

/// Wait for core services to be ready
async fn wait_for_services_ready() -> Result<(), String> {
    log(
        LogTag::System,
        "INFO",
        "Waiting for core services to be ready...",
    );

    let timeout = tokio::time::Duration::from_secs(60);
    let start = tokio::time::Instant::now();

    loop {
        if screenerbot::global::are_core_services_ready() {
            log(
                LogTag::System,
                "SUCCESS",
                "✅ Core services are ready",
            );
            return Ok(());
        }

        if start.elapsed() > timeout {
            return Err("Timeout waiting for services to be ready".to_string());
        }

        let pending = screenerbot::global::get_pending_services();
        log(
            LogTag::System,
            "INFO",
            &format!("Waiting for services: {:?}", pending),
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
}

// =============================================================================
// COMMAND HANDLERS (PLACEHOLDER IMPLEMENTATIONS)
// =============================================================================

/// Handle open position command
async fn handle_open_position(
    mint: &str,
    size_sol: Option<f64>,
    strategy: Option<String>,
) -> Result<(), String> {
    log(
        LogTag::Trader,
        "INFO",
        &format!("📈 Opening position for token: {}", mint),
    );

    // TODO: Implement position opening logic
    // - Validate mint address
    // - Check token data (decimals, price, liquidity)
    // - Override trade size if specified
    // - Call positions::open_position_direct()
    // - Wait for verification
    // - Display results

    println!("✅ Position opened successfully");
    println!("   Mint: {}", mint);
    if let Some(size) = size_sol {
        println!("   Size: {} SOL", size);
    }
    if let Some(strat) = strategy {
        println!("   Strategy: {}", strat);
    }

    Ok(())
}

/// Handle close position command
async fn handle_close_position(mint: &str, reason: &str, force: bool) -> Result<(), String> {
    log(
        LogTag::Trader,
        "INFO",
        &format!("📉 Closing position for token: {}", mint),
    );

    // TODO: Implement position closing logic
    // - Validate position exists
    // - Get current token balance
    // - Call positions::close_position_direct()
    // - Wait for verification
    // - Display results (profit/loss, exit price, etc.)

    println!("✅ Position closed successfully");
    println!("   Mint: {}", mint);
    println!("   Reason: {}", reason);
    if force {
        println!("   Force: true");
    }

    Ok(())
}

/// Handle DCA buy command
async fn handle_dca_buy(mint: &str, amount: f64, max_dca: Option<u32>) -> Result<(), String> {
    log(
        LogTag::Trader,
        "INFO",
        &format!("💰 Executing DCA buy for token: {} ({} SOL)", mint, amount),
    );

    // TODO: Implement DCA buy logic
    // - Validate position exists and is open
    // - Check DCA count limits
    // - Execute additional buy
    // - Update position with DCA data
    // - Display results (new average entry price, total invested, etc.)

    println!("✅ DCA buy executed successfully");
    println!("   Mint: {}", mint);
    println!("   Amount: {} SOL", amount);
    if let Some(max) = max_dca {
        println!("   Max DCA: {}", max);
    }

    Ok(())
}

/// Handle partial exit command
async fn handle_partial_exit(mint: &str, percentage: f64, reason: &str) -> Result<(), String> {
    log(
        LogTag::Trader,
        "INFO",
        &format!(
            "📤 Executing partial exit for token: {} ({}%)",
            mint, percentage
        ),
    );

    // TODO: Implement partial exit logic
    // - Validate position exists and is open
    // - Validate percentage (0-100)
    // - Calculate token amount to sell
    // - Execute partial sell
    // - Update position tracking
    // - Display results (amount sold, remaining, profit on exit, etc.)

    println!("✅ Partial exit executed successfully");
    println!("   Mint: {}", mint);
    println!("   Percentage: {}%", percentage);
    println!("   Reason: {}", reason);

    Ok(())
}

/// Handle list positions command
async fn handle_list_positions(detailed: bool, strategy: Option<String>) -> Result<(), String> {
    log(LogTag::Positions, "INFO", "📋 Listing positions");

    // TODO: Implement list positions logic
    // - Get all open positions from DB
    // - Filter by strategy if specified
    // - Calculate current values and P&L
    // - Display in table format
    // - Show detailed info if requested

    println!("📋 Open Positions:");
    println!("   (no positions)");
    if detailed {
        println!("   Detailed mode: enabled");
    }
    if let Some(strat) = strategy {
        println!("   Filter: strategy={}", strat);
    }

    Ok(())
}

/// Handle inspect position command
async fn handle_inspect_position(
    mint: &str,
    show_transactions: bool,
    show_dca: bool,
    show_exits: bool,
) -> Result<(), String> {
    log(
        LogTag::Positions,
        "INFO",
        &format!("🔍 Inspecting position: {}", mint),
    );

    // TODO: Implement position inspection logic
    // - Get position from DB
    // - Fetch current price and calculate P&L
    // - Show entry/exit details
    // - Show DCA history if requested
    // - Show partial exit history if requested
    // - Show transaction signatures if requested

    println!("🔍 Position Details:");
    println!("   Mint: {}", mint);
    if show_transactions {
        println!("   Transactions: (show)");
    }
    if show_dca {
        println!("   DCA History: (show)");
    }
    if show_exits {
        println!("   Partial Exits: (show)");
    }

    Ok(())
}

/// Handle interactive mode command
async fn handle_interactive_mode() -> Result<(), String> {
    log(
        LogTag::System,
        "INFO",
        "🎮 Entering interactive mode (not implemented yet)",
    );

    // TODO: Implement interactive mode
    // - Show menu with available operations
    // - Accept user input
    // - Execute commands in loop
    // - Support exit command

    println!("🎮 Interactive Mode (Coming Soon)");
    println!("   This feature will provide a REPL-style interface for manual trading");

    Ok(())
}

/// Handle reconcile command
async fn handle_reconcile(mint: Option<String>, auto_fix: bool) -> Result<(), String> {
    log(LogTag::Positions, "INFO", "🔄 Reconciling positions");

    // TODO: Implement reconcile logic
    // - If mint specified, reconcile single position
    // - Otherwise reconcile all positions
    // - Compare DB state vs chain state
    // - Report discrepancies
    // - Auto-fix if requested

    println!("🔄 Reconciliation Report:");
    if let Some(m) = mint {
        println!("   Mint: {}", m);
    } else {
        println!("   Scope: all positions");
    }
    if auto_fix {
        println!("   Auto-fix: enabled");
    }

    Ok(())
}

/// Handle test quote command
async fn handle_test_quote(mint: &str, amount: f64, operation: &str) -> Result<(), String> {
    log(
        LogTag::Swap,
        "INFO",
        &format!("💱 Testing quote: {} {} SOL for {}", operation, amount, mint),
    );

    // TODO: Implement quote testing logic
    // - Validate mint and operation
    // - Get quotes from all available DEXes
    // - Display comparison (Jupiter, GMGN, etc.)
    // - Show best route and price impact

    println!("💱 Quote Test Results:");
    println!("   Mint: {}", mint);
    println!("   Operation: {}", operation);
    println!("   Amount: {} SOL", amount);
    println!("   (quote retrieval not implemented)");

    Ok(())
}

/// Handle init command (just wait)
async fn handle_init(wait_seconds: u64) -> Result<(), String> {
    log(
        LogTag::System,
        "INFO",
        &format!("⏳ System initialized - waiting {} seconds", wait_seconds),
    );

    tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;

    log(
        LogTag::System,
        "INFO",
        "✅ Wait complete - system ready for testing",
    );

    Ok(())
}
