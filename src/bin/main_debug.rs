#![allow(warnings)]

/// ScreenerBot Debug Tool
///
/// Comprehensive debugging and testing tool for the ScreenerBot trading system.
/// This tool provides detailed analysis, monitoring, and debugging capabilities
/// for transactions, tokens, wallet balance, and system performance.
///
/// Features:
/// - Monitor wallet transactions in real-time
/// - Fetch new transactions from wallet (not in cache)
/// - Fetch all cached transactions (fast operation)
/// - Analyze specific transactions by signature
/// - Test transaction type detection
/// - Debug transaction caching system
/// - Validate transaction analysis
/// - Performance benchmarking
/// - Cache management and stats
/// - Execute real swap tests with transaction analysis
/// - Check wallet balance for SOL, tokens, and ATA accounts
/// - Get token information from database
/// - Combinable analysis flag for comprehensive insights
///
/// Usage Examples:
/// - Monitor wallet transactions: cargo run --bin main_debug -- --monitor
/// - Fetch new transactions: cargo run --bin main_debug -- --fetch-new
/// - Fetch limited transactions for testing: cargo run --bin main_debug -- --fetch 50
/// - Fetch ALL wallet transactions: cargo run --bin main_debug -- --fetch-all
/// - Analyze specific transaction: cargo run --bin main_debug -- --signature <SIG>
/// - Test analyzer on recent transactions: cargo run --bin main_debug -- --test-analyzer --count 10
/// - Debug cache system: cargo run --bin main_debug -- --debug-cache
/// - Update and re-analyze cache: cargo run --bin main_debug -- --update-cache --count 50 (preserves raw data)
/// - Clean cache files: cargo run --bin main_debug -- --clean-cache (removes calculated fields)
/// - Remove all cache files: cargo run --bin main_debug -- --clean (removes all JSON files)
/// - Analyze all swaps with PnL: cargo run --bin main_debug -- --analyze-swaps
/// - Filter swaps by SOL amount: cargo run --bin main_debug -- --analyze-swaps --min-sol 0.003 --max-sol 0.006
/// - Analyze position lifecycle: cargo run --bin main_debug -- --analyze-positions
/// - Analyze ALL transaction types: cargo run --bin main_debug -- --analyze-all --count 500
/// - Analyze ATA operations: cargo run --bin main_debug -- --analyze-ata --count 100
/// - Analyze transaction fees: cargo run --bin main_debug -- --analyze-fees --count 200
/// - Analyze specific transaction ATA: cargo run --bin main_debug -- --signature <SIG> --analyze-ata
/// - Deep analyze transaction: cargo run --bin main_debug -- --deep-analyze <SIGNATURE>
/// - Performance test: cargo run --bin main_debug -- --benchmark --count 100
/// - Fetch and analyze: cargo run --bin main_debug -- --fetch-new --analyze
/// - Monitor and analyze: cargo run --bin main_debug -- --monitor --analyze --duration 300
/// - Just analyze: cargo run --bin main_debug -- --analyze
/// - Test real swaps: cargo run --bin main_debug -- --test-swap --swap-type round-trip --token-mint <MINT> --sol-amount 0.002
/// - Test real position management: cargo run --bin main_debug -- --test-position --token-mint <MINT> --sol-amount 0.002
/// - Check wallet balance: cargo run --bin main_debug -- --check-balance
/// - Get token info: cargo run --bin main_debug -- --token-info <MINT_ADDRESS>
/// - Find mint by symbol: cargo run --bin main_debug -- --find-mint <SYMBOL>

use screenerbot::transactions::{
    TransactionsManager,
    get_transaction,
    initialize_global_transaction_manager,
};
use screenerbot::transactions_types::{
    Transaction,
    TransactionType,
    TransactionDirection,
    SwapPnLInfo,
    AtaOperationType,
};
use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::global::{ set_cmd_args };
use screenerbot::arguments::{ get_cmd_args, set_cmd_args as args_set_cmd_args };
use screenerbot::rpc::{ get_rpc_client, sol_to_lamports, TokenBalance };
use screenerbot::errors::ScreenerBotError;
use screenerbot::utils::get_wallet_address;
use solana_transaction_status::{
    UiInstruction,
    UiParsedInstruction,
    UiTransactionEncoding,
    EncodedConfirmedTransactionWithStatusMeta,
};
use solana_client::rpc_config::RpcTransactionConfig;
use screenerbot::tokens::types::PriceSourceType;
use screenerbot::tokens::{ Token, get_token_decimals_sync };
use screenerbot::swaps::{
    get_jupiter_quote,
    execute_jupiter_swap,
    get_gmgn_quote,
    JupiterSwapResult,
};
use screenerbot::positions;
use screenerbot::positions_types::Position;
use screenerbot::entry::get_profit_target;

use spl_associated_token_account::get_associated_token_address;
use clap::{ Arg, Command };
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::time::interval;
use chrono::{ DateTime, Utc };
use solana_sdk::pubkey::Pubkey;
use serde_json;

#[tokio::main]
async fn main() {
    // Initialize logger first
    init_file_logging();

    let matches = Command::new("ScreenerBot Debug Tool")
        .version("1.0")
        .about("Comprehensive debugging tool for ScreenerBot trading system")
        .after_help(
            "
COMMON USAGE EXAMPLES:

  Basic Analysis:
    cargo run --bin main_debug -- --analyze-swaps
    cargo run --bin main_debug -- --analyze-swaps --count 50 --min-sol 0.003
    cargo run --bin main_debug -- --analyze-fees --count 200
    cargo run --bin main_debug -- --analyze-swaps --filter-mint <MINT>

  Live Trading Tests:
    cargo run --bin main_debug -- --test-swap --dry-run
    cargo run --bin main_debug -- --test-swap --swap-type sol-to-token --sol-amount 0.001 --dry-run
    cargo run --bin main_debug -- --test-swap --token-mint CUSTOM_MINT --router jupiter

  Data Management:
    cargo run --bin main_debug -- --fetch-new --analyze
    cargo run --bin main_debug -- --monitor --duration 300 --analyze

  Deep Investigation:
    cargo run --bin main_debug -- --signature TRANSACTION_SIGNATURE
    cargo run --bin main_debug -- --signature TRANSACTION_SIGNATURE --debug-transactions
    cargo run --bin main_debug -- --deep-analyze TRANSACTION_SIGNATURE
    cargo run --bin main_debug -- --show-unknown --count 100
    cargo run --bin main_debug -- --analyze-all --filter-mint <MINT>

  Note: --deep-analyze provides comprehensive instruction-level transaction analysis
  Note: --debug-transactions enables verbose transaction verification logging

  Token Database Lookup:
    cargo run --bin main_debug -- --token-info DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263
    cargo run --bin main_debug -- --find-mint BONK

  Wallet Inspection:
    cargo run --bin main_debug -- --check-balance

IMPORTANT: Use --dry-run flag for safe testing without real transactions!
        "
        )

        // === DATA FETCHING & MONITORING ===
        .next_help_heading("Data Fetching & Monitoring")
        .arg(
            Arg::new("monitor")
                .long("monitor")
                .help("Monitor wallet transactions in real-time")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fetch-new")
                .long("fetch-new")
                .help("Fetch ALL new transactions from wallet (only uncached, no count limit)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("fetch")
                .long("fetch")
                .help("Fetch specified number of uncached transactions for testing")
                .value_name("COUNT")
                .value_parser(clap::value_parser!(usize))
        )
        .arg(
            Arg::new("fetch-all")
                .long("fetch-all")
                .help(
                    "Fetch ALL wallet transactions from blockchain (only uncached, no count limit)"
                )
                .action(clap::ArgAction::SetTrue)
        )

        // === TRANSACTION ANALYSIS ===
        .next_help_heading("Transaction Analysis")
        .arg(
            Arg::new("analyze-swaps")
                .long("analyze-swaps")
                .help("Analyze all swap transactions with comprehensive PnL")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("from")
                .long("from")
                .help(
                    "Filter swaps from this date/time (inclusive). Accepts RFC3339 (e.g., 2025-08-16T00:00:00Z), YYYY-MM-DD, or UNIX seconds."
                )
                .value_name("DATETIME")
        )
        .arg(
            Arg::new("to")
                .long("to")
                .help(
                    "Filter swaps up to this date/time (inclusive). Accepts RFC3339 (e.g., 2025-08-17T23:59:59Z), YYYY-MM-DD, or UNIX seconds."
                )
                .value_name("DATETIME")
        )
        .arg(
            Arg::new("analyze-positions")
                .long("analyze-positions")
                .help("Analyze position lifecycle with entry/exit tracking and PnL")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze-all")
                .long("analyze-all")
                .help("Analyze ALL transaction types (not just swaps) with comprehensive breakdown")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze-ata")
                .long("analyze-ata")
                .help(
                    "Analyze ATA operations across transactions or show detailed ATA analysis (use with --signature)"
                )
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze-fees")
                .long("analyze-fees")
                .help(
                    "Analyze transaction fees with comprehensive breakdown by type and statistics"
                )
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("show-unknown")
                .long("show-unknown")
                .help(
                    "Show only transactions with Unknown type for debugging classification issues"
                )
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("analyze")
                .long("analyze")
                .help("Perform comprehensive analysis (can be combined with other commands)")
                .action(clap::ArgAction::SetTrue)
        )

        // === INDIVIDUAL TRANSACTION DEBUGGING ===
        .next_help_heading("Individual Transaction Debugging")
        .arg(
            Arg::new("signature")
                .long("signature")
                .help("Analyze specific transaction by signature")
                .value_name("SIGNATURE")
        )
        .arg(
            Arg::new("deep-analyze")
                .long("deep-analyze")
                .help(
                    "Deep analyze specific transaction with comprehensive instruction-level details"
                )
                .value_name("SIGNATURE")
        )

        // === LIVE TRADING TESTS ===
        .next_help_heading("Live Trading Tests")
        .arg(
            Arg::new("test-swap")
                .long("test-swap")
                .help("Execute real swap test with transaction analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-position")
                .long("test-position")
                .help("Test real position management with transaction verification")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .help("Perform dry-run simulation without executing real transactions")
                .action(clap::ArgAction::SetTrue)
        )

        // === SWAP TEST CONFIGURATION ===
        .next_help_heading("Swap Test Configuration")
        .arg(
            Arg::new("swap-type")
                .long("swap-type")
                .help("Swap type: sol-to-token, token-to-sol, or round-trip")
                .value_name("TYPE")
                .default_value("round-trip")
                .value_parser(["sol-to-token", "token-to-sol", "round-trip"])
        )
        .arg(
            Arg::new("token-mint")
                .long("token-mint")
                .help("Token mint address for swap test")
                .value_name("MINT")
                .default_value("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263") // BONK
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("token-symbol")
                .long("token-symbol")
                .help("Token symbol for display purposes")
                .value_name("SYMBOL")
                .default_value("BONK")
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("sol-amount")
                .long("sol-amount")
                .help("SOL amount to trade (min: 0.001, max: 1.0)")
                .value_name("AMOUNT")
                .default_value("0.002")
                .value_parser(clap::value_parser!(f64))
        )
        .arg(
            Arg::new("slippage")
                .long("slippage")
                .help("Slippage tolerance percentage (min: 1.0, max: 50.0)")
                .value_name("PERCENT")
                .default_value("15.0")
                .value_parser(clap::value_parser!(f64))
        )
        .arg(
            Arg::new("router")
                .long("router")
                .help(
                    "Swap router: jupiter (recommended), gmgn, or raydium-cpmm (direct pool access)"
                )
                .value_name("ROUTER")
                .default_value("jupiter")
                .value_parser(["jupiter", "gmgn", "raydium-cpmm"])
        )

        // === ANALYSIS FILTERS ===
        .next_help_heading("Analysis Filters")
        .arg(
            Arg::new("min-sol")
                .long("min-sol")
                .help("Minimum SOL amount to include in swap analysis")
                .value_name("SOL_AMOUNT")
                .value_parser(clap::value_parser!(f64))
        )
        .arg(
            Arg::new("max-sol")
                .long("max-sol")
                .help("Maximum SOL amount to include in swap analysis")
                .value_name("SOL_AMOUNT")
                .value_parser(clap::value_parser!(f64))
        )
        .arg(
            Arg::new("filter-mint")
                .long("filter-mint")
                .help(
                    "Filter analysis to a specific token mint (applies to --analyze-swaps and --analyze-all)"
                )
                .value_name("MINT")
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("count")
                .long("count")
                .help("Number of transactions to process (min: 1, max: 10000)")
                .value_name("COUNT")
                .default_value("10")
                .value_parser(clap::value_parser!(usize))
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .help("Duration in seconds for monitoring (min: 10, max: 3600)")
                .value_name("SECONDS")
                .default_value("60")
                .value_parser(clap::value_parser!(u64))
        )

        // === CACHE & SYSTEM MANAGEMENT ===
        .next_help_heading("Cache & System Management")
        .arg(
            Arg::new("update-cache")
                .long("update-cache")
                .help("Re-analyze and update all cached transactions with new analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("clean-cache")
                .long("clean-cache")
                .help(
                    "Clean all cached transactions by removing calculated fields (keeps only raw blockchain data)"
                )
                .action(clap::ArgAction::SetTrue)
        )

        // === TESTING & DEBUGGING ===
        .next_help_heading("Testing & Debugging")
        .arg(
            Arg::new("test-analyzer")
                .long("test-analyzer")
                .help("Test transaction analyzer on recent transactions")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-cache")
                .long("debug-cache")
                .help("Debug the transaction cache system")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-transactions")
                .long("debug-transactions")
                .help("Enable verbose transaction verification and processing debug logs")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("benchmark")
                .long("benchmark")
                .help("Run performance benchmark tests")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("token-info")
                .long("token-info")
                .help("Get token information from database by mint address")
                .value_name("MINT_ADDRESS")
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("find-mint")
                .long("find-mint")
                .help("Find token mint address(es) by symbol")
                .value_name("SYMBOL")
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("check-balance")
                .long("check-balance")
                .help("Check wallet balance for SOL, tokens, and ATA accounts")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Enable verbose debug output")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Set command args for debug flags
    let mut args = vec!["main_debug".to_string()];
    if
        matches.get_flag("verbose") ||
        matches.get_one::<String>("signature").is_some() ||
        matches.get_flag("debug-transactions")
    {
        args.push("--debug-transactions".to_string());
    }
    set_cmd_args(args);

    log(LogTag::System, "INFO", "Starting ScreenerBot Debug Tool");

    // Initialize RPC client (it's automatically initialized when first used)
    let _rpc_client = get_rpc_client();

    // Load wallet configuration
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to load wallet: {}", e));
            std::process::exit(1);
        }
    };

    log(LogTag::System, "INFO", &format!("Loaded wallet: {}", wallet_pubkey));

    // Check for combinable analyze flag
    let should_analyze = matches.get_flag("analyze");
    // Optional token mint filter for analyses
    let filter_mint_for_analysis = matches.get_one::<String>("filter-mint").cloned();

    // Helper: parse datetime argument
    fn parse_datetime_arg(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
        use chrono::{ DateTime, NaiveDate, TimeZone, Utc };
        // Try UNIX seconds
        if let Ok(secs) = value.parse::<i64>() {
            return Some(chrono::DateTime::<Utc>::from_timestamp(secs, 0)?);
        }
        // Try RFC3339
        if let Ok(dt) = value.parse::<DateTime<Utc>>() {
            return Some(dt);
        }
        // Try YYYY-MM-DD
        if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            let dt = date.and_hms_opt(0, 0, 0)?;
            return Some(Utc.from_utc_datetime(&dt));
        }
        None
    }

    // Extract optional date filters up-front
    let from_dt = matches.get_one::<String>("from").and_then(|s| parse_datetime_arg(s));
    let to_dt = matches.get_one::<String>("to").and_then(|s| parse_datetime_arg(s));

    // Execute based on command line arguments
    if matches.get_flag("monitor") {
        let duration = *matches
            .get_one::<u64>("duration")
            .expect("duration should have default value");
        monitor_transactions(wallet_pubkey, duration).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after monitoring...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if let Some(signature) = matches.get_one::<String>("signature") {
        let analyze_ata = matches.get_flag("analyze-ata");
        analyze_specific_transaction(signature, analyze_ata).await;
    } else if let Some(signature) = matches.get_one::<String>("deep-analyze") {
        deep_analyze_transaction(signature).await;
    } else if matches.get_flag("fetch-new") {
        fetch_new_transactions(wallet_pubkey).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after fetching new transactions...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if let Some(count) = matches.get_one::<usize>("fetch") {
        // Validate fetch count range
        if *count < 1 || *count > 10000 {
            log(
                LogTag::System,
                "ERROR",
                &format!("Fetch count {} is out of range (min: 1, max: 10000)", count)
            );
            std::process::exit(1);
        }

        fetch_limited_transactions(wallet_pubkey, *count).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after fetching limited transactions...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if matches.get_flag("fetch-all") {
        fetch_all_wallet_transactions(wallet_pubkey).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after fetching all transactions...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if matches.get_flag("test-analyzer") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        test_transaction_analyzer(wallet_pubkey, count).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after testing analyzer...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if matches.get_flag("debug-cache") {
        debug_cache_system().await;
    } else if matches.get_flag("clean-cache") {
        clean_transaction_cache().await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after cleaning cache...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if matches.get_flag("benchmark") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        run_benchmark_tests(wallet_pubkey, count).await;
    } else if let Some(mint_address) = matches.get_one::<String>("token-info") {
        get_token_info_from_database(mint_address).await;
    } else if let Some(symbol) = matches.get_one::<String>("find-mint") {
        find_mint_by_symbol(symbol).await;
    } else if matches.get_flag("check-balance") {
        check_wallet_balance_comprehensive(wallet_pubkey).await;
    } else if matches.get_flag("analyze-swaps") {
        let count = matches.get_one::<usize>("count").copied();
        let min_sol = matches.get_one::<f64>("min-sol").copied();
        let max_sol = matches.get_one::<f64>("max-sol").copied();

        // Validate SOL amount ranges
        if let Some(min) = min_sol {
            if min < 0.000001 || min > 10.0 {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("min-sol {:.6} is out of range (min: 0.000001, max: 10.0)", min)
                );
                std::process::exit(1);
            }
        }
        if let Some(max) = max_sol {
            if max < 0.000001 || max > 10.0 {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("max-sol {:.6} is out of range (min: 0.000001, max: 10.0)", max)
                );
                std::process::exit(1);
            }
        }

        // Validate that min_sol <= max_sol if both are provided
        if let (Some(min), Some(max)) = (min_sol, max_sol) {
            if min > max {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("min-sol ({:.6}) cannot be greater than max-sol ({:.6})", min, max)
                );
                std::process::exit(1);
            }
        }

        // Validate count range
        if let Some(count_val) = count {
            if count_val < 1 || count_val > 10000 {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("count {} is out of range (min: 1, max: 10000)", count_val)
                );
                std::process::exit(1);
            }
        }

        analyze_swaps(
            wallet_pubkey,
            count,
            min_sol,
            max_sol,
            filter_mint_for_analysis.clone(),
            from_dt,
            to_dt
        ).await;
    } else if matches.get_flag("analyze-positions") {
        analyze_all_positions(wallet_pubkey).await;
    } else if matches.get_flag("analyze-all") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        analyze_all_transactions(wallet_pubkey, count, filter_mint_for_analysis.clone()).await;
    } else if matches.get_flag("analyze-ata") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        analyze_ata_operations(wallet_pubkey, count).await;
    } else if matches.get_flag("analyze-fees") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        analyze_transaction_fees(wallet_pubkey, count, filter_mint_for_analysis.clone()).await;
    } else if matches.get_flag("show-unknown") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        show_unknown_transactions(wallet_pubkey, count).await;
    } else if matches.get_flag("test-swap") {
        // Create and validate swap test configuration
        let config = match SwapTestConfig::from_matches(&matches) {
            Ok(config) => config,
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Invalid swap test configuration: {}", e));
                std::process::exit(1);
            }
        };

        // Log the validated configuration
        config.log_config();

        test_real_swap(
            wallet_pubkey,
            &config.swap_type,
            &config.token_mint,
            &config.token_symbol,
            config.sol_amount,
            config.slippage,
            &config.router,
            config.dry_run
        ).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after swap test...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if matches.get_flag("test-position") {
        // Validate and extract position test arguments with proper error handling
        let token_mint = matches
            .get_one::<String>("token-mint")
            .expect("token-mint should have default value");

        let token_symbol = matches
            .get_one::<String>("token-symbol")
            .expect("token-symbol should have default value");

        let sol_amount = *matches
            .get_one::<f64>("sol-amount")
            .expect("sol-amount should have default value");

        // Validate token mint format (basic validation)
        if token_mint.len() < 32 || token_mint.len() > 44 {
            log(
                LogTag::System,
                "ERROR",
                &format!("Invalid token mint format: {} (should be 32-44 characters)", token_mint)
            );
            std::process::exit(1);
        }

        // Log position test configuration
        log(
            LogTag::System,
            "POSITION_CONFIG",
            &format!(
                "Position test configuration validated:\n  • Token: {} ({})\n  • SOL Amount: {:.6}",
                token_symbol,
                screenerbot::utils::safe_truncate(&token_mint, 8),
                sol_amount
            )
        );

        test_real_position_management(wallet_pubkey, token_mint, token_symbol, sol_amount).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after position test...");
            analyze_all_positions(wallet_pubkey).await;
        }
    } else if matches.get_flag("update-cache") {
        let count = *matches.get_one::<usize>("count").expect("count should have default value");
        update_transaction_cache(wallet_pubkey, count).await;

        if should_analyze {
            log(LogTag::System, "INFO", "Running analysis after updating cache...");
            analyze_swaps(
                wallet_pubkey,
                None,
                None,
                None,
                filter_mint_for_analysis.clone(),
                from_dt,
                to_dt
            ).await;
        }
    } else if should_analyze {
        // If only --analyze is specified, run comprehensive analysis
        log(LogTag::System, "INFO", "Running comprehensive transaction analysis...");
        analyze_swaps(
            wallet_pubkey,
            None,
            None,
            None,
            filter_mint_for_analysis.clone(),
            from_dt,
            to_dt
        ).await;
    } else {
        log(LogTag::System, "ERROR", "No command specified. Use --help for usage information.");
        std::process::exit(1);
    }

    log(LogTag::System, "INFO", "ScreenerBot Debug Tool completed");
}

/// Swap test configuration with validation
#[derive(Debug, Clone)]
struct SwapTestConfig {
    pub swap_type: String,
    pub token_mint: String,
    pub token_symbol: String,
    pub sol_amount: f64,
    pub slippage: f64,
    pub router: String,
    pub dry_run: bool,
}

impl SwapTestConfig {
    /// Create and validate swap test configuration from command line arguments
    fn from_matches(matches: &clap::ArgMatches) -> Result<Self, String> {
        let swap_type = matches
            .get_one::<String>("swap-type")
            .expect("swap-type should have default value")
            .clone();

        let token_mint = matches
            .get_one::<String>("token-mint")
            .expect("token-mint should have default value")
            .clone();

        let token_symbol = matches
            .get_one::<String>("token-symbol")
            .expect("token-symbol should have default value")
            .clone();

        let sol_amount = *matches
            .get_one::<f64>("sol-amount")
            .expect("sol-amount should have default value");

        let slippage = *matches
            .get_one::<f64>("slippage")
            .expect("slippage should have default value");

        let router = matches
            .get_one::<String>("router")
            .expect("router should have default value")
            .clone();

        let dry_run = matches.get_flag("dry-run");

        let config = SwapTestConfig {
            swap_type,
            token_mint,
            token_symbol,
            sol_amount,
            slippage,
            router,
            dry_run,
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    fn validate(&self) -> Result<(), String> {
        // Validate token mint format
        if self.token_mint.len() < 32 || self.token_mint.len() > 44 {
            return Err(
                format!(
                    "Invalid token mint format: {} (should be 32-44 characters)",
                    self.token_mint
                )
            );
        }

        // Validate SOL amount range
        if self.sol_amount < 0.001 || self.sol_amount > 1.0 {
            return Err(
                format!("SOL amount {:.6} is out of range (min: 0.001, max: 1.0)", self.sol_amount)
            );
        }

        // Validate slippage range
        if self.slippage < 1.0 || self.slippage > 50.0 {
            return Err(
                format!("Slippage {:.1}% is out of range (min: 1.0%, max: 50.0%)", self.slippage)
            );
        }

        // Validate swap type (this should already be validated by clap, but double-check)
        match self.swap_type.as_str() {
            "sol-to-token" | "token-to-sol" | "round-trip" => {}
            _ => {
                return Err(format!("Invalid swap type: {}", self.swap_type));
            }
        }

        // Validate router (this should already be validated by clap, but double-check)
        match self.router.as_str() {
            "jupiter" | "gmgn" | "raydium-cpmm" => {}
            _ => {
                return Err(format!("Invalid router: {}", self.router));
            }
        }

        // Additional logic validation
        if self.swap_type == "token-to-sol" && !self.dry_run {
            log(LogTag::System, "WARNING", "token-to-sol swap requires existing token balance!");
        }

        Ok(())
    }

    /// Log the configuration for debugging
    fn log_config(&self) {
        log(
            LogTag::System,
            "SWAP_CONFIG",
            &format!(
                "Swap test configuration validated:\n  • Type: {}\n  • Token: {} ({})\n  • SOL Amount: {:.6}\n  • Slippage: {:.1}%\n  • Router: {}\n  • Dry Run: {}",
                self.swap_type,
                self.token_symbol,
                screenerbot::utils::safe_truncate(&self.token_mint, 8),
                self.sol_amount,
                self.slippage,
                self.router,
                self.dry_run
            )
        );
    }
}

/// Analyze swap transactions with comprehensive PnL and filtering
async fn analyze_swaps(
    wallet_pubkey: Pubkey,
    count: Option<usize>,
    min_sol: Option<f64>,
    max_sol: Option<f64>,
    filter_mint: Option<String>,
    from_dt: Option<chrono::DateTime<chrono::Utc>>,
    to_dt: Option<chrono::DateTime<chrono::Utc>>
) {
    log(
        LogTag::Transactions,
        "INFO",
        "Starting comprehensive swap analysis (includes automatic recalculation)"
    );

    if let Some(count_limit) = count {
        log(
            LogTag::Transactions,
            "FILTER",
            &format!("Limiting analysis to {} most recent transactions", count_limit)
        );
    }
    if let Some(min) = min_sol {
        log(
            LogTag::Transactions,
            "FILTER",
            &format!("Filtering swaps with SOL amount >= {:.6}", min)
        );
    }
    if let Some(max) = max_sol {
        log(
            LogTag::Transactions,
            "FILTER",
            &format!("Filtering swaps with SOL amount <= {:.6}", max)
        );
    }
    if let Some(ref mint) = filter_mint {
        let short = screenerbot::utils::safe_truncate(mint, 8);
        log(
            LogTag::Transactions,
            "FILTER",
            &format!("Filtering swaps by token mint: {}...", short)
        );
    }
    if let Some(dt) = from_dt {
        use chrono::SecondsFormat;
        log(
            LogTag::Transactions,
            "FILTER",
            &format!("From: {}", dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        );
    }
    if let Some(dt) = to_dt {
        use chrono::SecondsFormat;
        log(
            LogTag::Transactions,
            "FILTER",
            &format!("To:   {}", dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        );
    }

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get all swap transactions (now includes automatic recalculation and count limiting)
    match manager.get_recent_swaps(count.unwrap_or(20)).await {
        Ok(all_transactions) => {
            // Convert transactions to SwapPnLInfo objects
            let mut token_cache = std::collections::HashMap::new();
            let all_swaps: Vec<SwapPnLInfo> = all_transactions
                .iter()
                .filter_map(|tx| manager.convert_to_swap_pnl_info(tx, &token_cache, true))
                .collect();

            // Apply date/time and SOL amount filtering
            let filtered_swaps: Vec<_> = all_swaps
                .iter()
                .filter(|swap| {
                    let sol_amount = swap.sol_amount;
                    // Timestamp window filtering
                    if let Some(from) = from_dt {
                        if swap.timestamp < from {
                            return false;
                        }
                    }
                    if let Some(to) = to_dt {
                        if swap.timestamp > to {
                            return false;
                        }
                    }

                    // Check minimum filter
                    if let Some(min) = min_sol {
                        if sol_amount < min {
                            return false;
                        }
                    }

                    // Check maximum filter
                    if let Some(max) = max_sol {
                        if sol_amount > max {
                            return false;
                        }
                    }
                    // Check mint filter
                    if let Some(ref m) = filter_mint {
                        if swap.token_mint != *m {
                            return false;
                        }
                    }

                    true
                })
                .cloned()
                .collect();

            if let Some(count_limit) = count {
                log(
                    LogTag::Transactions,
                    "SUCCESS",
                    &format!(
                        "Found {} swap transactions (limited from {} total, filtered from {} count-limited)",
                        filtered_swaps.len(),
                        all_swaps.len(),
                        count_limit
                    )
                );
            } else {
                log(
                    LogTag::Transactions,
                    "SUCCESS",
                    &format!(
                        "Found {} swap transactions (filtered from {} total)",
                        filtered_swaps.len(),
                        all_swaps.len()
                    )
                );
            }

            // Display comprehensive analysis table with full signatures
            manager.display_swap_analysis_table_full_signatures(&filtered_swaps);

            // Additional statistics
            display_detailed_swap_statistics(&filtered_swaps);

            // ATA and SOL Flow Analysis
            display_ata_and_sol_flow_analysis(&filtered_swaps);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get swap transactions: {}", e));
        }
    }
}

/// Display detailed swap statistics
fn display_detailed_swap_statistics(swaps: &[SwapPnLInfo]) {
    if swaps.is_empty() {
        return;
    }

    log(LogTag::Transactions, "STATS", "=== DETAILED SWAP STATISTICS ===");

    let mut token_stats: std::collections::HashMap<
        String,
        TokenSwapStats
    > = std::collections::HashMap::new();
    let mut router_stats: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

    let mut total_profit_loss = 0.0;
    let mut profitable_swaps = 0;
    let mut loss_swaps = 0;

    for swap in swaps {
        // Token statistics
        let token_stat = token_stats
            .entry(swap.token_symbol.clone())
            .or_insert(TokenSwapStats::new());

        // Handle failed transactions separately - don't count them as buys or sells
        if swap.swap_type.starts_with("Failed") {
            // Failed transactions only contribute to fees, not to buy/sell counts or amounts
            token_stat.total_fees += swap.fee_sol;
            continue;
        }

        if swap.swap_type == "Buy" {
            token_stat.buy_count += 1;
            token_stat.total_sol_spent += swap.sol_amount;
        } else if swap.swap_type == "Sell" {
            token_stat.sell_count += 1;
            token_stat.total_sol_received += swap.sol_amount;
        }
        token_stat.total_fees += swap.fee_sol;

        // Router statistics (count all transactions including failed ones)
        *router_stats.entry(swap.router.clone()).or_insert(0) += 1;

        // Simplified PnL calculation (buy vs sell difference) - exclude failed transactions
        if swap.swap_type.starts_with("Failed") {
            // Failed transactions are just losses (fees paid for nothing)
            loss_swaps += 1;
            total_profit_loss -= swap.fee_sol; // Only count the fee as a loss
        } else if swap.swap_type == "Sell" {
            profitable_swaps += 1;
            total_profit_loss += swap.sol_amount;
        } else if swap.swap_type == "Buy" {
            loss_swaps += 1;
            total_profit_loss -= swap.sol_amount;
        }
    }

    // Display token statistics
    log(LogTag::Transactions, "STATS", "Token Trading Summary:");
    for (token, stats) in &token_stats {
        let net_sol = stats.total_sol_received - stats.total_sol_spent - stats.total_fees;
        log(
            LogTag::Transactions,
            "STATS",
            &format!(
                "  {}: {} buys ({:.3} SOL), {} sells ({:.3} SOL), fees: {:.6} SOL, net: {:.3} SOL",
                token,
                stats.buy_count,
                stats.total_sol_spent,
                stats.sell_count,
                stats.total_sol_received,
                stats.total_fees,
                net_sol
            )
        );
    }

    // Display router statistics
    log(LogTag::Transactions, "STATS", "Router Usage:");
    for (router, count) in &router_stats {
        log(LogTag::Transactions, "STATS", &format!("  {}: {} swaps", router, count));
    }

    // Display overall PnL
    log(
        LogTag::Transactions,
        "STATS",
        &format!(
            "Overall Performance: {} profitable, {} loss swaps, estimated P&L: {:.6} SOL",
            profitable_swaps,
            loss_swaps,
            total_profit_loss
        )
    );

    log(LogTag::Transactions, "STATS", "=== END STATISTICS ===");
}

/// Analyze all positions with comprehensive lifecycle tracking and PnL
async fn analyze_all_positions(wallet_pubkey: Pubkey) {
    log(
        LogTag::Transactions,
        "INFO",
        "Starting comprehensive position analysis for all transactions"
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Analyze positions
    match manager.analyze_positions(None).await {
        Ok(()) => {
            log(LogTag::Transactions, "SUCCESS", "Position analysis completed successfully");
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to analyze positions: {}", e));
        }
    }
}

#[derive(Debug)]
struct TokenSwapStats {
    buy_count: i32,
    sell_count: i32,
    total_sol_spent: f64,
    total_sol_received: f64,
    total_fees: f64,
}

impl TokenSwapStats {
    fn new() -> Self {
        Self {
            buy_count: 0,
            sell_count: 0,
            total_sol_spent: 0.0,
            total_sol_received: 0.0,
            total_fees: 0.0,
        }
    }
}

/// Analyze ALL transaction types (not just swaps) with comprehensive breakdown
async fn analyze_all_transactions(
    wallet_pubkey: Pubkey,
    max_count: usize,
    filter_mint: Option<String>
) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Starting comprehensive analysis of ALL transaction types (max {} transactions)", max_count)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get all cached transactions first
    match manager.fetch_all_wallet_transactions().await {
        Ok(transactions) => {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!(
                    "Loaded {} total transactions for comprehensive analysis",
                    transactions.len()
                )
            );

            if transactions.is_empty() {
                log(
                    LogTag::Transactions,
                    "WARN",
                    "No transactions found. Try fetching from wallet first with --fetch-new"
                );
                return;
            }

            // Optional filter by token mint (match any involvement in tx)
            let filtered: Vec<_> = if let Some(ref mint) = filter_mint {
                let short = screenerbot::utils::safe_truncate(mint, 8);
                log(
                    LogTag::Transactions,
                    "FILTER",
                    &format!("Filtering all transactions by mint: {}...", short)
                );
                transactions
                    .into_iter()
                    .filter(|tx| transaction_involves_mint(tx, mint))
                    .collect()
            } else {
                transactions
            };

            // Display comprehensive transaction analysis table
            display_all_transactions_table(&filtered);

            // Display detailed statistics breakdown
            display_comprehensive_transaction_statistics(&filtered);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load transactions: {}", e));
        }
    }
}

/// Display comprehensive table of ALL transaction types
fn display_all_transactions_table(transactions: &[screenerbot::transactions_types::Transaction]) {
    log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE TRANSACTION ANALYSIS ===");
    log(
        LogTag::Transactions,
        "TABLE",
        "Sig      Slot         Type                    Details                          SOL Change   Fee SOL      Success"
    );
    log(
        LogTag::Transactions,
        "TABLE",
        "-----------------------------------------------------------------------------------------------------------------------"
    );

    for transaction in transactions.iter() {
        // Show all transactions
        let slot = transaction.slot.unwrap_or(0);
        let sol_change_str = format!("{:+.6}", transaction.sol_balance_change);
        let fee_str = format!("{:.6}", transaction.fee_sol);
        let success_icon = if transaction.success { "✅" } else { "❌" };
        let sig_short = &transaction.signature[..(8).min(transaction.signature.len())];
        let _timestamp = transaction.timestamp.format("%H:%M:%S").to_string();

        let (tx_type, details) = match &transaction.transaction_type {
            screenerbot::transactions_types::TransactionType::SwapSolToToken {
                token_mint: _,
                sol_amount,
                token_amount,
                router,
            } => {
                (
                    "SOL->Token",
                    format!("{:.4} SOL -> {:.0} tokens via {}", sol_amount, token_amount, router),
                )
            }
            screenerbot::transactions_types::TransactionType::SwapTokenToSol {
                token_mint: _,
                token_amount,
                sol_amount,
                router,
            } => {
                (
                    "Token->SOL",
                    format!("{:.0} tokens -> {:.4} SOL via {}", token_amount, sol_amount, router),
                )
            }
            screenerbot::transactions_types::TransactionType::SwapTokenToToken {
                from_mint: _,
                to_mint: _,
                from_amount,
                to_amount,
                router,
            } => {
                ("Token->Token", format!("{:.0} -> {:.0} via {}", from_amount, to_amount, router))
            }
            screenerbot::transactions_types::TransactionType::SolTransfer { from, to, amount } => {
                let from_short = screenerbot::utils::safe_truncate(from, 8);
                let to_short = screenerbot::utils::safe_truncate(to, 8);
                ("SOL Transfer", format!("{:.4} SOL: {}...->{}...", amount, from_short, to_short))
            }
            screenerbot::transactions_types::TransactionType::TokenTransfer {
                mint: _,
                from,
                to,
                amount,
            } => {
                let from_short = screenerbot::utils::safe_truncate(from, 8);
                let to_short = screenerbot::utils::safe_truncate(to, 8);
                (
                    "Token Transfer",
                    format!("{:.0} tokens: {}...->{}...", amount, from_short, to_short),
                )
            }
            screenerbot::transactions_types::TransactionType::AtaClose {
                token_mint,
                recovered_sol,
            } => {
                let mint_short = screenerbot::utils::safe_truncate(token_mint, 8);
                ("ATA Close", format!("Recovered {:.6} SOL from {}...", recovered_sol, mint_short))
            }
            screenerbot::transactions_types::TransactionType::Other { description, details } => {
                ("Other", format!("{}: {}", description, details))
            }
            screenerbot::transactions_types::TransactionType::Unknown => {
                ("Unknown", "Unidentified transaction type".to_string())
            }
        };

        log(
            LogTag::Transactions,
            "TABLE",
            &format!(
                "{:<8} {:<12} {:<19} {:<32} {:<12} {:<12} {}",
                sig_short,
                slot,
                tx_type,
                if details.len() > 30 {
                    format!("{}...", screenerbot::utils::safe_truncate(&details, 27))
                } else {
                    details
                },
                sol_change_str,
                fee_str,
                success_icon
            )
        );
    }

    log(
        LogTag::Transactions,
        "TABLE",
        "-----------------------------------------------------------------------------------------------------------------------"
    );
    log(LogTag::Transactions, "TABLE", "=== END TRANSACTION TABLE ===");
}

/// Display comprehensive statistics for all transaction types
fn display_comprehensive_transaction_statistics(
    transactions: &[screenerbot::transactions_types::Transaction]
) {
    log(LogTag::Transactions, "STATS", "=== COMPREHENSIVE TRANSACTION STATISTICS ===");

    let mut type_counts = HashMap::new();
    let mut successful_count = 0;
    let mut failed_count = 0;
    let mut total_fees = 0.0;
    let mut total_sol_in = 0.0;
    let mut total_sol_out = 0.0;
    let mut oldest_timestamp = transactions[0].timestamp;
    let mut newest_timestamp = transactions[0].timestamp;

    // Count transaction types and calculate statistics
    for transaction in transactions {
        let tx_type = match &transaction.transaction_type {
            screenerbot::transactions_types::TransactionType::SwapSolToToken { .. } =>
                "Swap: SOL->Token",
            screenerbot::transactions_types::TransactionType::SwapTokenToSol { .. } =>
                "Swap: Token->SOL",
            screenerbot::transactions_types::TransactionType::SwapTokenToToken { .. } =>
                "Swap: Token->Token",
            screenerbot::transactions_types::TransactionType::SolTransfer { .. } => "SOL Transfer",
            screenerbot::transactions_types::TransactionType::TokenTransfer { .. } =>
                "Token Transfer",
            screenerbot::transactions_types::TransactionType::AtaClose { .. } => "ATA Close",
            screenerbot::transactions_types::TransactionType::Other { .. } => "Other",
            screenerbot::transactions_types::TransactionType::Unknown => "Unknown",
        };

        *type_counts.entry(tx_type.to_string()).or_insert(0) += 1;

        if transaction.success {
            successful_count += 1;
        } else {
            failed_count += 1;
        }

        total_fees += transaction.fee_sol;

        if transaction.sol_balance_change > 0.0 {
            total_sol_in += transaction.sol_balance_change;
        } else {
            total_sol_out += transaction.sol_balance_change.abs();
        }

        if transaction.timestamp < oldest_timestamp {
            oldest_timestamp = transaction.timestamp;
        }
        if transaction.timestamp > newest_timestamp {
            newest_timestamp = transaction.timestamp;
        }
    }

    // Display overall statistics
    log(LogTag::Transactions, "STATS", &format!("Total Transactions: {}", transactions.len()));
    log(
        LogTag::Transactions,
        "STATS",
        &format!(
            "Successful: {} ({:.1}%)",
            successful_count,
            ((successful_count as f64) / (transactions.len() as f64)) * 100.0
        )
    );
    log(
        LogTag::Transactions,
        "STATS",
        &format!(
            "Failed: {} ({:.1}%)",
            failed_count,
            ((failed_count as f64) / (transactions.len() as f64)) * 100.0
        )
    );
    log(
        LogTag::Transactions,
        "STATS",
        &format!("Time Range: {} to {}", oldest_timestamp, newest_timestamp)
    );

    let time_span = newest_timestamp.signed_duration_since(oldest_timestamp);
    log(LogTag::Transactions, "STATS", &format!("Time Span: {} days", time_span.num_days()));

    log(LogTag::Transactions, "STATS", &format!("Total Fees Paid: {:.6} SOL", total_fees));
    log(LogTag::Transactions, "STATS", &format!("Total SOL Received: +{:.6} SOL", total_sol_in));
    log(LogTag::Transactions, "STATS", &format!("Total SOL Spent: -{:.6} SOL", total_sol_out));
    log(
        LogTag::Transactions,
        "STATS",
        &format!("Net SOL Change: {:.6} SOL", total_sol_in - total_sol_out)
    );

    log(LogTag::Transactions, "STATS", "");
    log(LogTag::Transactions, "STATS", "Transaction Type Breakdown:");
    let mut sorted_types: Vec<_> = type_counts.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

    for (tx_type, count) in sorted_types {
        let percentage = ((*count as f64) / (transactions.len() as f64)) * 100.0;
        log(
            LogTag::Transactions,
            "STATS",
            &format!("  {}: {} ({:.1}%)", tx_type, count, percentage)
        );
    }

    log(LogTag::Transactions, "STATS", "=== END COMPREHENSIVE STATISTICS ===");
}

/// Determine if a transaction involves a given token mint in any capacity
fn transaction_involves_mint(
    tx: &screenerbot::transactions_types::Transaction,
    mint: &str
) -> bool {
    // Check token transfers first
    if tx.token_transfers.iter().any(|t| t.mint == mint) {
        return true;
    }
    // Check token balance changes
    if tx.token_balance_changes.iter().any(|b| b.mint == mint) {
        return true;
    }
    // Check analyzed transaction type fields
    match &tx.transaction_type {
        screenerbot::transactions_types::TransactionType::SwapSolToToken { token_mint, .. } =>
            token_mint == mint,
        screenerbot::transactions_types::TransactionType::SwapTokenToSol { token_mint, .. } =>
            token_mint == mint,
        screenerbot::transactions_types::TransactionType::SwapTokenToToken {
            from_mint,
            to_mint,
            ..
        } => from_mint == mint || to_mint == mint,
        screenerbot::transactions_types::TransactionType::TokenTransfer { mint: m, .. } =>
            m == mint,
        screenerbot::transactions_types::TransactionType::AtaClose { token_mint, .. } =>
            token_mint == mint,
        _ => false,
    }
}

/// Analyze ATA operations across multiple transactions
async fn analyze_ata_operations(wallet_pubkey: Pubkey, max_count: usize) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Starting comprehensive ATA operations analysis (max {} transactions)", max_count)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get all cached transactions and analyze ATA operations
    match manager.fetch_all_wallet_transactions().await {
        Ok(transactions) => {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!("Loaded {} transactions for ATA analysis", transactions.len())
            );

            if transactions.is_empty() {
                log(
                    LogTag::Transactions,
                    "WARN",
                    "No transactions found. Try fetching from wallet first with --fetch-new"
                );
                return;
            }

            analyze_and_display_ata_operations(&transactions);
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load transactions: {}", e));
        }
    }
}

/// Analyze and display detailed ATA operations from transactions
fn analyze_and_display_ata_operations(
    transactions: &[screenerbot::transactions_types::Transaction]
) {
    log(LogTag::Transactions, "ATA_ANALYSIS", "=== COMPREHENSIVE ATA OPERATIONS ANALYSIS ===");

    let mut total_transactions_with_ata = 0;
    let mut total_ata_creations = 0;
    let mut total_ata_closures = 0;
    let mut total_rent_spent = 0.0;
    let mut total_rent_recovered = 0.0;
    let mut swap_transactions_with_ata = 0;
    let mut non_swap_transactions_with_ata = 0;

    // Track ATA operations by transaction type
    let mut ata_by_tx_type: std::collections::HashMap<
        String,
        (u32, u32, f64, f64)
    > = std::collections::HashMap::new();

    // Track problematic ATA calculations
    let mut problematic_calculations = Vec::new();

    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(LogTag::Transactions, "ATA_ANALYSIS", "DETAILED ATA OPERATIONS BY TRANSACTION:");
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        "Signature  Type        SOL Change  ATA Creates  ATA Closes  Rent Spent  Rent Recovered  Net Impact  Raw Sol Change"
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        "----------------------------------------------------------------------------------------------------------------------"
    );

    for transaction in transactions {
        if let Some(ata_analysis) = &transaction.ata_analysis {
            if ata_analysis.total_ata_creations > 0 || ata_analysis.total_ata_closures > 0 {
                total_transactions_with_ata += 1;
                total_ata_creations += ata_analysis.total_ata_creations;
                total_ata_closures += ata_analysis.total_ata_closures;
                total_rent_spent += ata_analysis.total_rent_spent;
                total_rent_recovered += ata_analysis.total_rent_recovered;

                let tx_type = match &transaction.transaction_type {
                    screenerbot::transactions_types::TransactionType::SwapSolToToken {
                        router,
                        ..
                    } => {
                        swap_transactions_with_ata += 1;
                        format!("Buy ({})", screenerbot::utils::safe_truncate(router, 12))
                    }
                    screenerbot::transactions_types::TransactionType::SwapTokenToSol {
                        router,
                        ..
                    } => {
                        swap_transactions_with_ata += 1;
                        format!("Sell ({})", screenerbot::utils::safe_truncate(router, 12))
                    }
                    screenerbot::transactions_types::TransactionType::SwapTokenToToken {
                        router,
                        ..
                    } => {
                        swap_transactions_with_ata += 1;
                        format!("Swap ({})", screenerbot::utils::safe_truncate(router, 12))
                    }
                    _ => {
                        non_swap_transactions_with_ata += 1;
                        format!("{:?}", transaction.transaction_type)
                            .split('(')
                            .next()
                            .unwrap_or("Unknown")
                            .to_string()
                    }
                };

                // Track stats by transaction type
                let stats = ata_by_tx_type.entry(tx_type.clone()).or_insert((0, 0, 0.0, 0.0));
                stats.0 += ata_analysis.total_ata_creations;
                stats.1 += ata_analysis.total_ata_closures;
                stats.2 += ata_analysis.total_rent_spent;
                stats.3 += ata_analysis.total_rent_recovered;

                let sig_short = &transaction.signature[..(8).min(transaction.signature.len())];
                let net_impact = ata_analysis.total_rent_recovered - ata_analysis.total_rent_spent;

                // Check for problematic calculations
                let expected_sol_change_from_ata = net_impact;
                let actual_sol_change = transaction.sol_balance_change;
                let difference = (actual_sol_change - expected_sol_change_from_ata).abs();

                if difference > 0.001 && (tx_type.contains("Buy") || tx_type.contains("Sell")) {
                    problematic_calculations.push((
                        transaction.signature.clone(),
                        tx_type.clone(),
                        actual_sol_change,
                        expected_sol_change_from_ata,
                        difference,
                    ));
                }

                log(
                    LogTag::Transactions,
                    "ATA_ANALYSIS",
                    &format!(
                        "{:<10} {:<11} {:>10.6} {:>11} {:>10} {:>10.6} {:>14.6} {:>10.6} {:>14.6}",
                        sig_short,
                        screenerbot::utils::safe_truncate(&tx_type, 11),
                        transaction.sol_balance_change,
                        ata_analysis.total_ata_creations,
                        ata_analysis.total_ata_closures,
                        ata_analysis.total_rent_spent,
                        ata_analysis.total_rent_recovered,
                        net_impact,
                        actual_sol_change
                    )
                );

                // Show detailed ATA operations if there are any
                if !ata_analysis.detected_operations.is_empty() {
                    for operation in &ata_analysis.detected_operations {
                        let op_type = if
                            operation.operation_type ==
                            screenerbot::transactions_types::AtaOperationType::Creation
                        {
                            "CREATE"
                        } else {
                            "CLOSE"
                        };
                        let mint_short = screenerbot::utils::safe_truncate(
                            &operation.token_mint,
                            8
                        );
                        let wsol_flag = if operation.is_wsol { " (WSOL)" } else { "" };

                        log(
                            LogTag::Transactions,
                            "ATA_ANALYSIS",
                            &format!(
                                "           └─ {} {} for {}...{} ({:.6} SOL)",
                                op_type,
                                screenerbot::utils::safe_truncate(&operation.account_address, 8),
                                mint_short,
                                wsol_flag,
                                operation.rent_amount
                            )
                        );
                    }
                }
            }
        }
    }

    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        "----------------------------------------------------------------------------------------------------------------------"
    );
    log(LogTag::Transactions, "ATA_ANALYSIS", "");

    // Display summary statistics
    log(LogTag::Transactions, "ATA_ANALYSIS", "=== ATA OPERATIONS SUMMARY ===");
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total Transactions Analyzed: {}", transactions.len())
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Transactions with ATA Operations: {}", total_transactions_with_ata)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("  - Swap Transactions: {}", swap_transactions_with_ata)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("  - Non-Swap Transactions: {}", non_swap_transactions_with_ata)
    );
    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total ATA Creations: {}", total_ata_creations)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total ATA Closures: {}", total_ata_closures)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total Rent Spent: {:.6} SOL", total_rent_spent)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total Rent Recovered: {:.6} SOL", total_rent_recovered)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Net ATA Impact: {:.6} SOL", total_rent_recovered - total_rent_spent)
    );
    log(LogTag::Transactions, "ATA_ANALYSIS", "");

    // Display breakdown by transaction type
    log(LogTag::Transactions, "ATA_ANALYSIS", "ATA Operations by Transaction Type:");
    for (tx_type, (creates, closes, spent, recovered)) in &ata_by_tx_type {
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!(
                "  {}: {} creates, {} closes, {:.6} SOL spent, {:.6} SOL recovered, net {:.6} SOL",
                tx_type,
                creates,
                closes,
                spent,
                recovered,
                recovered - spent
            )
        );
    }

    // Display problematic calculations
    if !problematic_calculations.is_empty() {
        log(LogTag::Transactions, "ATA_ANALYSIS", "");
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            "⚠️  PROBLEMATIC ATA CALCULATIONS (difference > 0.001 SOL):"
        );
        for (
            signature,
            tx_type,
            actual_sol,
            expected_sol,
            difference,
        ) in &problematic_calculations {
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!(
                    "  {}: {} - Actual: {:.6} SOL, Expected from ATA: {:.6} SOL, Difference: {:.6} SOL",
                    signature,
                    tx_type,
                    actual_sol,
                    expected_sol,
                    difference
                )
            );
        }
        log(LogTag::Transactions, "ATA_ANALYSIS", "");
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            "💡 These transactions may need manual verification of SOL balance changes vs ATA operations"
        );
    }

    log(LogTag::Transactions, "ATA_ANALYSIS", "=== END ATA OPERATIONS ANALYSIS ===");
}

/// Show only transactions with Unknown type for debugging classification issues
async fn show_unknown_transactions(wallet_pubkey: Pubkey, max_count: usize) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Analyzing transactions with Unknown type (max {} transactions)", max_count)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get all cached transactions
    match manager.fetch_all_wallet_transactions().await {
        Ok(transactions) => {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!(
                    "Loaded {} total transactions for Unknown type analysis",
                    transactions.len()
                )
            );

            if transactions.is_empty() {
                log(
                    LogTag::Transactions,
                    "WARN",
                    "No transactions found. Try fetching from wallet first with --fetch-new"
                );
                return;
            }

            // Filter only Unknown transactions
            let unknown_transactions: Vec<_> = transactions
                .iter()
                .filter(|tx|
                    matches!(
                        tx.transaction_type,
                        screenerbot::transactions_types::TransactionType::Unknown
                    )
                )
                .collect();

            if unknown_transactions.is_empty() {
                log(
                    LogTag::Transactions,
                    "SUCCESS",
                    "🎉 Great! No Unknown transactions found - all transactions are properly classified!"
                );
                return;
            }

            log(
                LogTag::Transactions,
                "INFO",
                &format!(
                    "Found {} Unknown transactions out of {} total ({:.1}%)",
                    unknown_transactions.len(),
                    transactions.len(),
                    ((unknown_transactions.len() as f64) / (transactions.len() as f64)) * 100.0
                )
            );

            // Display table header
            log(LogTag::Transactions, "TABLE", "=== UNKNOWN TRANSACTIONS ANALYSIS ===");
            log(
                LogTag::Transactions,
                "TABLE",
                "Sig      Slot         Timestamp        SOL Change   Fee SOL      Success   Programs"
            );
            log(
                LogTag::Transactions,
                "TABLE",
                "-----------------------------------------------------------------------------------------------------------------------"
            );

            for transaction in unknown_transactions.iter() {
                let slot = transaction.slot.unwrap_or(0);
                let sol_change_str = format!("{:+.6}", transaction.sol_balance_change);
                let fee_str = format!("{:.6}", transaction.fee_sol);
                let success_icon = if transaction.success { "✅" } else { "❌" };
                let sig_short = &transaction.signature[..(8).min(transaction.signature.len())];
                let timestamp = transaction.timestamp.format("%H:%M:%S").to_string();

                // Extract program IDs from raw transaction data if available
                let programs = if let Some(raw_data) = &transaction.raw_transaction_data {
                    if let Some(transaction_obj) = raw_data.get("transaction") {
                        if let Some(message) = transaction_obj.get("message") {
                            if let Some(account_keys) = message.get("accountKeys") {
                                if let Some(keys_array) = account_keys.as_array() {
                                    let program_ids: Vec<String> = keys_array
                                        .iter()
                                        .take(5) // Show first 5 program IDs
                                        .filter_map(|key| key.as_str())
                                        .map(|pk_str| {
                                            if pk_str.len() >= 8 {
                                                format!("{}...", &pk_str[..8])
                                            } else {
                                                pk_str.to_string()
                                            }
                                        })
                                        .collect();
                                    program_ids.join(",")
                                } else {
                                    "Keys not array".to_string()
                                }
                            } else {
                                "No account keys".to_string()
                            }
                        } else {
                            "No message".to_string()
                        }
                    } else {
                        "No transaction obj".to_string()
                    }
                } else {
                    "No raw data".to_string()
                };

                log(
                    LogTag::Transactions,
                    "TABLE",
                    &format!(
                        "{:<8} {:<12} {:<16} {:<12} {:<12} {:<9} {}",
                        sig_short,
                        slot,
                        timestamp,
                        sol_change_str,
                        fee_str,
                        success_icon,
                        if programs.len() > 30 {
                            format!("{}...", &programs[..27])
                        } else {
                            programs
                        }
                    )
                );
            }

            log(
                LogTag::Transactions,
                "TABLE",
                "-----------------------------------------------------------------------------------------------------------------------"
            );

            // Display detailed analysis for first few unknown transactions
            log(
                LogTag::Transactions,
                "DETAIL",
                "=== DETAILED ANALYSIS OF FIRST 3 UNKNOWN TRANSACTIONS ==="
            );

            for (i, transaction) in unknown_transactions.iter().take(3).enumerate() {
                log(
                    LogTag::Transactions,
                    "DETAIL",
                    &format!("--- Unknown Transaction #{} ---", i + 1)
                );
                log(
                    LogTag::Transactions,
                    "DETAIL",
                    &format!("Signature: {}", transaction.signature)
                );
                log(LogTag::Transactions, "DETAIL", &format!("Slot: {:?}", transaction.slot));
                log(LogTag::Transactions, "DETAIL", &format!("Success: {}", transaction.success));
                log(
                    LogTag::Transactions,
                    "DETAIL",
                    &format!("SOL Change: {:+.6}", transaction.sol_balance_change)
                );
                log(
                    LogTag::Transactions,
                    "DETAIL",
                    &format!("Fee: {:.6} SOL", transaction.fee_sol)
                );

                if let Some(raw_data) = &transaction.raw_transaction_data {
                    if let Some(transaction_obj) = raw_data.get("transaction") {
                        if let Some(message) = transaction_obj.get("message") {
                            if let Some(account_keys) = message.get("accountKeys") {
                                if let Some(keys_array) = account_keys.as_array() {
                                    log(LogTag::Transactions, "DETAIL", "Program IDs involved:");
                                    for (j, key) in keys_array.iter().take(10).enumerate() {
                                        if let Some(key_str) = key.as_str() {
                                            log(
                                                LogTag::Transactions,
                                                "DETAIL",
                                                &format!("  [{}] {}", j, key_str)
                                            );
                                        }
                                    }
                                }
                            }

                            if let Some(instructions) = message.get("instructions") {
                                if let Some(instr_array) = instructions.as_array() {
                                    log(
                                        LogTag::Transactions,
                                        "DETAIL",
                                        &format!("Instructions count: {}", instr_array.len())
                                    );
                                    for (j, instruction) in instr_array.iter().take(3).enumerate() {
                                        if
                                            let Some(program_id_index) =
                                                instruction.get("programIdIndex")
                                        {
                                            if let Some(accounts) = instruction.get("accounts") {
                                                if let Some(accounts_array) = accounts.as_array() {
                                                    log(
                                                        LogTag::Transactions,
                                                        "DETAIL",
                                                        &format!(
                                                            "  Instruction {}: Program {} with {} accounts",
                                                            j,
                                                            program_id_index,
                                                            accounts_array.len()
                                                        )
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    log(LogTag::Transactions, "DETAIL", "No raw transaction data available");
                }

                log(LogTag::Transactions, "DETAIL", "");
            }

            // Provide debugging tips
            log(LogTag::Transactions, "INFO", "=== DEBUGGING TIPS ===");
            log(LogTag::Transactions, "INFO", "To debug specific unknown transactions:");
            log(
                LogTag::Transactions,
                "INFO",
                "1. Use --signature <sig> to analyze individual transactions in detail"
            );
            log(
                LogTag::Transactions,
                "INFO",
                "2. Check if the program IDs above match any known DEX routers or protocols"
            );
            log(
                LogTag::Transactions,
                "INFO",
                "3. Look for patterns in SOL balance changes that might indicate specific operation types"
            );
            log(
                LogTag::Transactions,
                "INFO",
                "4. Failed transactions (❌) might have different instruction patterns"
            );
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load transactions: {}", e));
        }
    }
}

/// Fetch new transactions from the wallet that are not yet in cache
async fn fetch_new_transactions(wallet_pubkey: Pubkey) {
    log(
        LogTag::Transactions,
        "INFO",
        "Fetching ALL new transactions from wallet (no limit, skipping cached)"
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Initialize known signatures to detect new ones
    if let Err(e) = manager.initialize_known_signatures().await {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!("Failed to initialize known signatures: {}", e)
        );
        return;
    }

    log(
        LogTag::Transactions,
        "INFO",
        &format!("Loaded {} known signatures from cache", manager.known_signatures.len())
    );

    // Get new transactions
    match manager.check_new_transactions().await {
        Ok(new_signatures) => {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!("Found {} new transactions", new_signatures.len())
            );

            if new_signatures.is_empty() {
                log(LogTag::Transactions, "INFO", "No new transactions found");
                return;
            }

            let mut processed_count = 0;
            let mut error_count = 0;
            let start_time = Instant::now();

            for (index, signature) in new_signatures.iter().enumerate() {
                log(
                    LogTag::Transactions,
                    "PROGRESS",
                    &format!(
                        "Processing new transaction {}/{}: {}...",
                        index + 1,
                        new_signatures.len(),
                        &signature[..8]
                    )
                );

                match manager.process_transaction(signature).await {
                    Ok(_) => {
                        processed_count += 1;
                        log(
                            LogTag::Transactions,
                            "SUCCESS",
                            &format!("✅ Processed {}", &signature[..8])
                        );
                    }
                    Err(e) => {
                        error_count += 1;
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("❌ Failed to process {}: {}", &signature[..8], e)
                        );
                    }
                }

                // Add small delay to avoid overwhelming the system
                if index % 5 == 0 && index > 0 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            let total_time = start_time.elapsed();
            log(LogTag::Transactions, "RESULTS", "=== FETCH NEW TRANSACTIONS RESULTS ===");
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("New Transactions Found: {}", new_signatures.len())
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Successfully Processed: {}", processed_count)
            );
            log(LogTag::Transactions, "RESULTS", &format!("Errors: {}", error_count));
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!(
                    "Success Rate: {:.1}%",
                    ((processed_count as f64) / (new_signatures.len() as f64)) * 100.0
                )
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Processing Time: {:.2}s", total_time.as_secs_f64())
            );
            log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to fetch new transactions: {}", e));
        }
    }
}

/// Fetch ALL wallet transactions from blockchain (no limit, replaces cache)
async fn fetch_all_wallet_transactions(wallet_pubkey: Pubkey) {
    log(
        LogTag::Transactions,
        "INFO",
        "Fetching ALL wallet transactions from blockchain (no limit, skipping already-cached)"
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create transaction manager: {}", e)
            );
            return;
        }
    };

    // Fetch ALL transactions from blockchain (no count limit)
    match manager.fetch_all_wallet_transactions().await {
        Ok(transactions) => {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!("Successfully fetched {} transactions from blockchain", transactions.len())
            );

            // Display summary statistics
            display_transaction_summary(&transactions);
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to fetch wallet transactions: {}", e)
            );
        }
    }
}

/// Fetch limited number of transactions from blockchain (for testing)
async fn fetch_limited_transactions(wallet_pubkey: Pubkey, count: usize) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Fetching up to {} uncached transactions from blockchain for testing", count)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create transaction manager: {}", e)
            );
            return;
        }
    };

    // Fetch limited transactions from blockchain
    match manager.fetch_limited_wallet_transactions(count).await {
        Ok(transactions) => {
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!("Successfully fetched {} transactions from blockchain", transactions.len())
            );

            // Display summary statistics
            display_transaction_summary(&transactions);
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to fetch wallet transactions: {}", e)
            );
        }
    }
}

/// Display transaction summary statistics
fn display_transaction_summary(transactions: &[Transaction]) {
    if transactions.is_empty() {
        return;
    }

    log(LogTag::Transactions, "SUMMARY", "=== TRANSACTION SUMMARY ===");

    let mut type_counts = HashMap::new();
    let mut successful_count = 0;
    let mut total_fees = 0.0;
    let mut total_sol_volume = 0.0;

    let mut oldest_timestamp = transactions[0].timestamp;
    let mut newest_timestamp = transactions[0].timestamp;

    for transaction in transactions {
        // Count transaction types
        let tx_type = format!("{:?}", transaction.transaction_type)
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        *type_counts.entry(tx_type).or_insert(0) += 1;

        if transaction.success {
            successful_count += 1;
        }

        total_fees += transaction.fee_sol;
        total_sol_volume += transaction.sol_balance_change.abs();

        if transaction.timestamp < oldest_timestamp {
            oldest_timestamp = transaction.timestamp;
        }
        if transaction.timestamp > newest_timestamp {
            newest_timestamp = transaction.timestamp;
        }
    }

    log(LogTag::Transactions, "SUMMARY", &format!("Total Transactions: {}", transactions.len()));
    log(
        LogTag::Transactions,
        "SUMMARY",
        &format!(
            "Successful: {} ({:.1}%)",
            successful_count,
            ((successful_count as f64) / (transactions.len() as f64)) * 100.0
        )
    );
    log(
        LogTag::Transactions,
        "SUMMARY",
        &format!("Time Range: {} to {}", oldest_timestamp, newest_timestamp)
    );

    let time_span = newest_timestamp.signed_duration_since(oldest_timestamp);
    log(LogTag::Transactions, "SUMMARY", &format!("Time Span: {} days", time_span.num_days()));

    log(LogTag::Transactions, "SUMMARY", &format!("Total Fees Paid: {:.6} SOL", total_fees));
    log(LogTag::Transactions, "SUMMARY", &format!("Total SOL Volume: {:.6} SOL", total_sol_volume));

    log(LogTag::Transactions, "SUMMARY", "Transaction Types:");
    for (tx_type, count) in &type_counts {
        let percentage = ((count * 100) as f64) / (transactions.len() as f64);
        log(
            LogTag::Transactions,
            "SUMMARY",
            &format!("  {}: {} ({:.1}%)", tx_type, count, percentage)
        );
    }

    log(LogTag::Transactions, "SUMMARY", "=== END SUMMARY ===");
}

/// Load wallet pubkey from configuration
async fn load_wallet_pubkey() -> Result<Pubkey, Box<dyn std::error::Error>> {
    let wallet_address_str = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;

    Pubkey::from_str(&wallet_address_str).map_err(|e|
        format!("Invalid wallet address: {}", e).into()
    )
}

/// Monitor wallet transactions in real-time
async fn monitor_transactions(wallet_pubkey: Pubkey, duration_seconds: u64) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Starting real-time transaction monitoring for {} seconds", duration_seconds)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Initialize known signatures
    if let Err(e) = manager.initialize_known_signatures().await {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!("Failed to initialize known signatures: {}", e)
        );
        return;
    }

    log(
        LogTag::Transactions,
        "INFO",
        &format!("Loaded {} known signatures from cache", manager.known_signatures.len())
    );

    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(duration_seconds);
    let mut check_interval = interval(Duration::from_secs(5));

    let mut total_new_transactions = 0;
    let mut total_processed = 0;

    while Instant::now() < end_time {
        tokio::select! {
            _ = check_interval.tick() => {
                match manager.check_new_transactions().await {
                    Ok(new_signatures) => {
                        if !new_signatures.is_empty() {
                            total_new_transactions += new_signatures.len();
                            log(LogTag::Transactions, "NEW", &format!(
                                "Found {} new transactions", new_signatures.len()
                            ));

                            // Process each new transaction
                            for signature in new_signatures {
                                match manager.process_transaction(&signature).await {
                                    Ok(transaction) => {
                                        total_processed += 1;
                                        log_transaction_summary(&transaction);
                                    }
                                    Err(e) => {
                                        log(LogTag::Transactions, "ERROR", &format!(
                                            "Failed to process transaction {}: {}", 
                                            &signature[..8], e
                                        ));
                                    }
                                }
                            }
                        } else {
                            log(LogTag::Transactions, "DEBUG", "No new transactions found");
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Failed to check new transactions: {}", e));
                    }
                }

                // Display stats
                let elapsed = start_time.elapsed().as_secs();
                let remaining = duration_seconds.saturating_sub(elapsed);
                log(LogTag::Transactions, "STATS", &format!(
                    "Elapsed: {}s | Remaining: {}s | New: {} | Processed: {}",
                    elapsed, remaining, total_new_transactions, total_processed
                ));
            }
        }
    }

    log(
        LogTag::Transactions,
        "INFO",
        &format!(
            "Monitoring completed. Total new transactions: {}, Total processed: {}",
            total_new_transactions,
            total_processed
        )
    );
}

/// Analyze a specific transaction by signature
async fn analyze_specific_transaction(signature: &str, analyze_ata: bool) {
    log(LogTag::Transactions, "INFO", &format!("Analyzing transaction: {}", signature));

    // Always perform fresh analysis since we calculate on each call

    // Load wallet and create manager
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet: {}", e));
            return;
        }
    };

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(mut manager) => {
            // Enable debug mode for enhanced ATA analysis when requested
            if analyze_ata {
                manager.debug_enabled = true;
                log(LogTag::Transactions, "INFO", "Debug mode enabled for enhanced ATA analysis");

                // Add debug-transactions flag to enable enhanced ATA analysis logging
                let mut current_args = get_cmd_args();
                current_args.push("--debug-transactions".to_string());
                args_set_cmd_args(current_args);
            }
            manager
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Process the transaction with comprehensive analysis
    match manager.process_transaction(signature).await {
        Ok(transaction) => {
            log(LogTag::Transactions, "SUCCESS", "Transaction analyzed successfully");

            // Always run comprehensive analysis for complete fee breakdown
            log(
                LogTag::Transactions,
                "INFO",
                "Running additional comprehensive analysis for complete fee breakdown"
            );

            display_detailed_transaction_info(&transaction);

            // If ATA analysis was requested, provide detailed breakdown
            if analyze_ata {
                display_detailed_ata_analysis(&transaction);
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to analyze transaction: {}", e));
        }
    }
}

/// Deep analyze a specific transaction with comprehensive instruction-level details
async fn deep_analyze_transaction(signature: &str) {
    log(
        LogTag::Transactions,
        "DEEP_ANALYSIS",
        &format!("🔍 Starting DEEP ANALYSIS for transaction: {}", signature)
    );
    log(
        LogTag::Transactions,
        "DEEP_ANALYSIS",
        "This will detect ALL operations, instructions, and account changes"
    );

    // Load wallet and create manager with debug mode enabled
    let wallet_pubkey = match load_wallet_pubkey().await {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet: {}", e));
            return;
        }
    };

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(mut manager) => {
            manager.debug_enabled = true; // Enable maximum debugging
            manager
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get RPC client for raw transaction data
    let rpc_client = get_rpc_client();

    // Parse signature
    let sig = match signature.parse::<solana_sdk::signature::Signature>() {
        Ok(sig) => sig,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Invalid signature format: {}", e));
            return;
        }
    };

    // Fetch raw transaction with maximum details
    log(
        LogTag::Transactions,
        "DEEP_ANALYSIS",
        "📡 Fetching raw transaction data from Solana RPC..."
    );

    // Use our internal transaction details method instead
    match rpc_client.get_transaction_details(signature).await {
        Ok(tx_details) => {
            log(LogTag::Transactions, "DEEP_ANALYSIS", "✅ Transaction fetched successfully");
            display_simple_transaction_analysis(&tx_details, signature).await;
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to fetch transaction details: {}", e)
            );

            // Fallback to our internal analysis only
            log(LogTag::Transactions, "DEEP_ANALYSIS", "");
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                "🤖 === SCREENERBOT INTERNAL ANALYSIS (FALLBACK) ==="
            );

            match get_transaction(signature).await {
                Ok(Some(internal_tx)) => {
                    log(LogTag::Transactions, "DEEP_ANALYSIS", "✅ Found in ScreenerBot database");
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("Type: {:?}", internal_tx.transaction_type)
                    );
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("Status: {}", if internal_tx.success {
                            "✅ SUCCESS"
                        } else {
                            "❌ FAILED"
                        })
                    );
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("Direction: {:?}", internal_tx.direction)
                    );

                    if internal_tx.sol_balance_change != 0.0 {
                        log(
                            LogTag::Transactions,
                            "DEEP_ANALYSIS",
                            &format!(
                                "SOL Balance Change: {:.9} SOL",
                                internal_tx.sol_balance_change
                            )
                        );
                    }
                }
                Ok(None) => {
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        "❌ Not found in ScreenerBot database"
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("⚠️ Error checking internal database: {}", e)
                    );
                }
            }
        }
    }
}

/// Test transaction analyzer on recent transactions
async fn test_transaction_analyzer(wallet_pubkey: Pubkey, count: usize) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Testing transaction analyzer on {} recent transactions", count)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get recent transactions
    match manager.check_new_transactions().await {
        Ok(signatures) => {
            let test_signatures: Vec<_> = signatures.into_iter().take(count).collect();

            log(
                LogTag::Transactions,
                "INFO",
                &format!("Found {} signatures to test", test_signatures.len())
            );

            let mut stats = AnalyzerTestStats::new();
            let start_time = Instant::now();

            for (index, signature) in test_signatures.iter().enumerate() {
                let tx_start = Instant::now();

                match manager.process_transaction(signature).await {
                    Ok(transaction) => {
                        let processing_time = tx_start.elapsed();
                        stats.record_success(&transaction, processing_time);

                        log(
                            LogTag::Transactions,
                            "TEST",
                            &format!(
                                "[{}/{}] {} - {:?} - {:.2}ms",
                                index + 1,
                                test_signatures.len(),
                                &signature[..8],
                                transaction.transaction_type,
                                processing_time.as_millis()
                            )
                        );
                    }
                    Err(e) => {
                        stats.record_error(&e);
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!(
                                "[{}/{}] {} - Error: {}",
                                index + 1,
                                test_signatures.len(),
                                &signature[..8],
                                e
                            )
                        );
                    }
                }
            }

            let total_time = start_time.elapsed();
            stats.display_results(total_time);
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to get recent transactions: {}", e)
            );
        }
    }
}

/// Debug the transaction cache system
async fn debug_cache_system() {
    log(LogTag::Transactions, "INFO", "Debugging transaction cache system");

    log(LogTag::Transactions, "INFO", "Legacy JSON cache inspection disabled (migrated to DB)");
}

/// Clean all cached transactions by removing calculated fields
/// This keeps only raw blockchain data and is useful during development
async fn clean_transaction_cache() {
    log(LogTag::Transactions, "INFO", "No JSON cache to clean (DB only)");
}

/// Update and re-analyze all cached transactions (now uses database)
async fn update_transaction_cache(wallet_pubkey: Pubkey, max_count: usize) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Updating transaction cache from database (max {} transactions)", max_count)
    );

    // Create manager for re-analysis
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    log(LogTag::Transactions, "INFO", "Fetching transactions from database for re-analysis");

    let start_time = Instant::now();
    let mut updated_count = 0;
    let mut swap_count = 0;
    let mut unknown_count = 0;

    // Use database instead of JSON files
    match manager.fetch_all_wallet_transactions().await {
        Ok(transactions) => {
            let total_signatures = transactions.len();
            log(
                LogTag::Transactions,
                "INFO",
                &format!("Processing {} transactions from database", total_signatures)
            );

            for (index, transaction) in transactions.iter().enumerate() {
                log(
                    LogTag::Transactions,
                    "PROGRESS",
                    &format!(
                        "Processing transaction {}/{}: {}...",
                        index + 1,
                        total_signatures,
                        &transaction.signature[..8]
                    )
                );

                updated_count += 1;

                // Log transaction type for statistics
                match &transaction.transaction_type {
                    | TransactionType::SwapSolToToken { router, .. }
                    | TransactionType::SwapTokenToSol { router, .. }
                    | TransactionType::SwapTokenToToken { router, .. } => {
                        swap_count += 1;
                        log(
                            LogTag::Transactions,
                            "SWAP",
                            &format!(
                                "✅ Updated swap via {}: {}",
                                router,
                                &transaction.signature[..8]
                            )
                        );
                    }
                    TransactionType::Unknown => {
                        unknown_count += 1;
                        log(
                            LogTag::Transactions,
                            "UNKNOWN",
                            &format!(
                                "❓ Updated unknown transaction: {}",
                                &transaction.signature[..8]
                            )
                        );
                    }
                    _ => {
                        log(
                            LogTag::Transactions,
                            "OTHER",
                            &format!("ℹ️ Updated transaction: {}", &transaction.signature[..8])
                        );
                    }
                }

                // Add small delay to avoid overwhelming the system
                if index % 10 == 0 && index > 0 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            let total_time = start_time.elapsed();

            log(LogTag::Transactions, "RESULTS", "=== CACHE UPDATE RESULTS ===");
            log(LogTag::Transactions, "RESULTS", &format!("Total Processed: {}", total_signatures));
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Successfully Updated: {}", updated_count)
            );
            log(LogTag::Transactions, "RESULTS", &format!("Swap Transactions: {}", swap_count));
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Unknown Transactions: {}", unknown_count)
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Other Transactions: {}", updated_count - swap_count - unknown_count)
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Processing Time: {:.2}s", total_time.as_secs_f64())
            );

            if updated_count > 0 {
                let avg_time = total_time / (updated_count as u32);
                log(
                    LogTag::Transactions,
                    "RESULTS",
                    &format!("Avg Time per Transaction: {:.2}ms", avg_time.as_millis())
                );
            }

            log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");

            // After updating cache, show comprehensive swap analysis if any swaps were found
            if swap_count > 0 {
                log(
                    LogTag::Transactions,
                    "INFO",
                    "Performing comprehensive swap analysis on updated cache..."
                );

                match manager.get_recent_swaps(50).await {
                    Ok(transactions) => {
                        // Convert transactions to SwapPnLInfo objects
                        let mut token_cache = std::collections::HashMap::new();
                        let swaps: Vec<SwapPnLInfo> = transactions
                            .iter()
                            .filter_map(|tx|
                                manager.convert_to_swap_pnl_info(tx, &token_cache, true)
                            )
                            .collect();

                        log(
                            LogTag::Transactions,
                            "SUCCESS",
                            &format!("Found {} total swap transactions for analysis", swaps.len())
                        );
                        manager.display_swap_analysis_table(&swaps);
                        display_detailed_swap_statistics(&swaps);
                    }
                    Err(e) => {
                        log(
                            LogTag::Transactions,
                            "ERROR",
                            &format!("Failed to analyze updated swaps: {}", e)
                        );
                    }
                }
            }
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to recalculate transactions from database: {}", e)
            );
        }
    }
}

/// Run performance benchmark tests
async fn run_benchmark_tests(wallet_pubkey: Pubkey, count: usize) {
    log(
        LogTag::Transactions,
        "INFO",
        &format!("Running performance benchmark with {} transactions", count)
    );

    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Get signatures for testing
    let signatures = match manager.check_new_transactions().await {
        Ok(sigs) => sigs.into_iter().take(count).collect::<Vec<_>>(),
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get signatures: {}", e));
            return;
        }
    };

    if signatures.is_empty() {
        log(LogTag::Transactions, "WARN", "No signatures available for benchmarking");
        return;
    }

    let mut benchmark = BenchmarkStats::new();
    let start_time = Instant::now();

    log(LogTag::Transactions, "INFO", &format!("Benchmarking {} signatures", signatures.len()));

    for (index, signature) in signatures.iter().enumerate() {
        let tx_start = Instant::now();

        match manager.process_transaction(signature).await {
            Ok(transaction) => {
                let processing_time = tx_start.elapsed();
                benchmark.record_transaction(&transaction, processing_time);

                if (index + 1) % 10 == 0 {
                    log(
                        LogTag::Transactions,
                        "PROGRESS",
                        &format!("Processed {}/{} transactions", index + 1, signatures.len())
                    );
                }
            }
            Err(e) => {
                benchmark.record_error();
                log(LogTag::Transactions, "ERROR", &format!("Benchmark error: {}", e));
            }
        }
    }

    let total_time = start_time.elapsed();
    benchmark.display_results(total_time, signatures.len());
}

/// Display transaction summary for monitoring
fn log_transaction_summary(transaction: &Transaction) {
    let tx_type_str = match &transaction.transaction_type {
        TransactionType::SwapSolToToken { token_mint: _, sol_amount, token_amount, router } => {
            format!(
                "SOL->Token: {:.4} SOL -> {:.2} tokens via {}",
                sol_amount,
                token_amount,
                router
            )
        }
        TransactionType::SwapTokenToSol { token_mint: _, token_amount, sol_amount, router } => {
            format!(
                "Token->SOL: {:.2} tokens -> {:.4} SOL via {}",
                token_amount,
                sol_amount,
                router
            )
        }
        TransactionType::SwapTokenToToken {
            from_mint: _,
            to_mint: _,
            from_amount,
            to_amount,
            router,
        } => {
            format!("Token->Token: {:.2} -> {:.2} via {}", from_amount, to_amount, router)
        }
        TransactionType::SolTransfer { amount, .. } => {
            format!("SOL Transfer: {:.4} SOL", amount)
        }
        TransactionType::TokenTransfer { amount, .. } => {
            format!("Token Transfer: {:.2} tokens", amount)
        }
        TransactionType::AtaClose { token_mint, recovered_sol } => {
            let mint_short = if token_mint.len() >= 8 { &token_mint[..8] } else { token_mint };
            format!("ATA Close: {:.6} SOL from {}...", recovered_sol, mint_short)
        }
        TransactionType::Other { description, .. } => { format!("Other: {}", description) }
        TransactionType::Unknown => "Unknown".to_string(),
    };

    let direction_emoji = match transaction.direction {
        TransactionDirection::Incoming => "⬇️",
        TransactionDirection::Outgoing => "⬆️",
        TransactionDirection::Internal => "🔄",
    };

    log(
        LogTag::Transactions,
        "TX",
        &format!(
            "{} {} - {} - Fee: {:.6} SOL - {}",
            direction_emoji,
            &transaction.signature[..8],
            tx_type_str,
            transaction.fee_sol,
            if transaction.success {
                "✅"
            } else {
                "❌"
            }
        )
    );
}

/// Display detailed ATA analysis information for a transaction
fn display_detailed_ata_analysis(transaction: &Transaction) {
    log(LogTag::Transactions, "ATA_ANALYSIS", "=== DETAILED ATA OPERATIONS ANALYSIS ===");

    // Check if we have ATA analysis data
    if let Some(ata_analysis) = &transaction.ata_analysis {
        // Summary information
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total ATA Creations: {}", ata_analysis.total_ata_creations)
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total ATA Closures: {}", ata_analysis.total_ata_closures)
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total Rent Spent: {:.9} SOL", ata_analysis.total_rent_spent)
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total Rent Recovered: {:.9} SOL", ata_analysis.total_rent_recovered)
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Net Rent Impact: {:.9} SOL", ata_analysis.net_rent_impact)
        );

        // WSOL specific operations
        if ata_analysis.wsol_ata_creations > 0 || ata_analysis.wsol_ata_closures > 0 {
            log(LogTag::Transactions, "ATA_ANALYSIS", "--- WSOL ATA Operations ---");
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("WSOL ATA Creations: {}", ata_analysis.wsol_ata_creations)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("WSOL ATA Closures: {}", ata_analysis.wsol_ata_closures)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("WSOL Rent Spent: {:.9} SOL", ata_analysis.wsol_rent_spent)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("WSOL Rent Recovered: {:.9} SOL", ata_analysis.wsol_rent_recovered)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("WSOL Net Impact: {:.9} SOL", ata_analysis.wsol_net_rent_impact)
            );
        }

        // Token specific operations
        if ata_analysis.token_ata_creations > 0 || ata_analysis.token_ata_closures > 0 {
            log(LogTag::Transactions, "ATA_ANALYSIS", "--- Token ATA Operations ---");
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("Token ATA Creations: {}", ata_analysis.token_ata_creations)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("Token ATA Closures: {}", ata_analysis.token_ata_closures)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("Token Rent Spent: {:.9} SOL", ata_analysis.token_rent_spent)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("Token Rent Recovered: {:.9} SOL", ata_analysis.token_rent_recovered)
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!("Token Net Impact: {:.9} SOL", ata_analysis.token_net_rent_impact)
            );
        }

        // Detailed operation list
        if !ata_analysis.detected_operations.is_empty() {
            log(LogTag::Transactions, "ATA_ANALYSIS", "--- Detailed ATA Operations ---");
            for (i, operation) in ata_analysis.detected_operations.iter().enumerate() {
                let op_type = match operation.operation_type {
                    screenerbot::transactions_types::AtaOperationType::Creation => "Creation",
                    screenerbot::transactions_types::AtaOperationType::Closure => "Closure",
                };

                let token_type = if operation.is_wsol { "WSOL" } else { "Token" };

                log(
                    LogTag::Transactions,
                    "ATA_ANALYSIS",
                    &format!(
                        "Operation #{}: {} - {} ATA for mint {} - {:.9} SOL",
                        i + 1,
                        op_type,
                        token_type,
                        operation.token_mint,
                        operation.rent_amount
                    )
                );
            }
        }

        // SOL calculations
        log(LogTag::Transactions, "ATA_ANALYSIS", "--- SOL Amount Calculation ---");
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total SOL Change: {:.9} SOL", transaction.sol_balance_change)
        );

        // Calculate pure trading amount
        let pure_sol_amount = transaction.sol_balance_change - ata_analysis.net_rent_impact;
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!(
                "Pure Trading Amount: {:.9} SOL (SOL Change - Net ATA Impact)",
                pure_sol_amount
            )
        );

        // Check for WSOL operations and provide additional insights
        if ata_analysis.wsol_ata_creations > 0 || ata_analysis.wsol_ata_closures > 0 {
            let wsol_adjusted = transaction.sol_balance_change - ata_analysis.token_net_rent_impact;
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                &format!(
                    "WSOL-Adjusted Trading: {:.9} SOL (excludes WSOL ATA operations)",
                    wsol_adjusted
                )
            );
        }

        // Recommendations
        log(LogTag::Transactions, "ATA_ANALYSIS", "--- Analysis & Recommendations ---");
        if ata_analysis.total_ata_creations > 0 || ata_analysis.total_ata_closures > 0 {
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                "• This transaction includes ATA operations that affect the SOL calculation"
            );
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                "• When calculating trade amounts, consider adjusting for ATA rent costs"
            );

            if ata_analysis.wsol_ata_creations > 0 || ata_analysis.wsol_ata_closures > 0 {
                log(
                    LogTag::Transactions,
                    "ATA_ANALYSIS",
                    "• WSOL operations detected - these are typically temporary and shouldn't be counted as costs"
                );
            }
        } else {
            log(
                LogTag::Transactions,
                "ATA_ANALYSIS",
                "• No ATA operations detected in this transaction"
            );
        }
    } else {
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            "No ATA analysis data available for this transaction"
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            "Transaction analysis will automatically include ATA data when available"
        );
    }

    log(LogTag::Transactions, "ATA_ANALYSIS", "=== END ATA OPERATIONS ANALYSIS ===");
}

/// Display detailed transaction information
fn display_detailed_transaction_info(transaction: &Transaction) {
    log(LogTag::Transactions, "DETAIL", "=== TRANSACTION DETAILS ===");
    log(LogTag::Transactions, "DETAIL", &format!("Signature: {}", transaction.signature));

    // Use blockchain timestamp if available, otherwise fall back to transaction timestamp
    let display_timestamp = if let Some(block_time) = transaction.block_time {
        DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
    } else {
        transaction.timestamp
    };
    log(LogTag::Transactions, "DETAIL", &format!("Timestamp: {}", display_timestamp));
    log(LogTag::Transactions, "DETAIL", &format!("Success: {}", transaction.success));
    log(LogTag::Transactions, "DETAIL", &format!("Status: {:?}", transaction.status));
    log(LogTag::Transactions, "DETAIL", &format!("Direction: {:?}", transaction.direction));
    log(LogTag::Transactions, "DETAIL", &format!("Fee (SOL): {:.9}", transaction.fee_sol));
    log(
        LogTag::Transactions,
        "DETAIL",
        &format!("SOL Balance Change: {:.9}", transaction.sol_balance_change)
    );

    // Display ATA analysis if available (simpler fee information)
    if let Some(ata_analysis) = &transaction.ata_analysis {
        log(
            LogTag::Transactions,
            "DETAIL",
            &format!("ATA Creation Cost: {:.9} SOL", ata_analysis.total_rent_spent)
        );
        log(
            LogTag::Transactions,
            "DETAIL",
            &format!("ATA Rent Recovery: {:.9} SOL", ata_analysis.total_rent_recovered)
        );
        log(
            LogTag::Transactions,
            "DETAIL",
            &format!("Net ATA Impact: {:.9} SOL", ata_analysis.net_rent_impact)
        );
        log(
            LogTag::Transactions,
            "DETAIL",
            &format!(
                "Infrastructure Costs: {:.9} SOL (one-time setup)",
                ata_analysis.total_rent_spent
            )
        );
    }

    // Transaction type details
    match &transaction.transaction_type {
        TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, router } => {
            log(LogTag::Transactions, "DETAIL", &format!("Type: SOL to Token Swap"));
            log(LogTag::Transactions, "DETAIL", &format!("  Router: {}", router));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Mint: {}", token_mint));
            log(LogTag::Transactions, "DETAIL", &format!("  SOL Amount: {:.6}", sol_amount));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Amount: {:.2}", token_amount));
        }
        TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, router } => {
            log(LogTag::Transactions, "DETAIL", &format!("Type: Token to SOL Swap"));
            log(LogTag::Transactions, "DETAIL", &format!("  Router: {}", router));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Mint: {}", token_mint));
            log(LogTag::Transactions, "DETAIL", &format!("  Token Amount: {:.2}", token_amount));
            log(LogTag::Transactions, "DETAIL", &format!("  SOL Amount: {:.6}", sol_amount));
        }
        _ => {
            log(
                LogTag::Transactions,
                "DETAIL",
                &format!("Type: {:?}", transaction.transaction_type)
            );
        }
    }

    // Token transfers
    if !transaction.token_transfers.is_empty() {
        log(LogTag::Transactions, "DETAIL", "Token Transfers:");
        for transfer in &transaction.token_transfers {
            let from_display = if transfer.from.len() >= 8 {
                &transfer.from[..8]
            } else {
                &transfer.from
            };
            let to_display = if transfer.to.len() >= 8 { &transfer.to[..8] } else { &transfer.to };
            let mint_display = if transfer.mint.len() >= 8 {
                &transfer.mint[..8]
            } else {
                &transfer.mint
            };

            log(
                LogTag::Transactions,
                "DETAIL",
                &format!(
                    "  {} -> {}: {:.6} ({})",
                    from_display,
                    to_display,
                    transfer.amount,
                    mint_display
                )
            );
        }
    }

    // Instructions
    if !transaction.instructions.is_empty() {
        log(
            LogTag::Transactions,
            "DETAIL",
            &format!("Instructions: {}", transaction.instructions.len())
        );
        for (i, instruction) in transaction.instructions.iter().enumerate() {
            log(
                LogTag::Transactions,
                "DETAIL",
                &format!(
                    "  [{}] {} - {} - {} accounts",
                    i,
                    &instruction.program_id[..8],
                    instruction.instruction_type,
                    instruction.accounts.len()
                )
            );
        }
    }

    if let Some(error) = &transaction.error_message {
        log(LogTag::Transactions, "DETAIL", &format!("Error: {}", error));
    }

    log(LogTag::Transactions, "DETAIL", "=== END DETAILS ===");
}

/// Analyze a transaction by signature (now uses database)
async fn analyze_transaction_by_signature(
    signature: &str
) -> Result<Transaction, Box<dyn std::error::Error>> {
    // This function is no longer needed as we work directly with the database
    Err("JSON cache analysis deprecated - use database queries instead".into())
}

/// Statistics for analyzer testing
#[derive(Debug)]
struct AnalyzerTestStats {
    total_processed: usize,
    successful: usize,
    errors: usize,
    transaction_types: HashMap<String, usize>,
    total_processing_time: Duration,
    min_time: Duration,
    max_time: Duration,
}

impl AnalyzerTestStats {
    fn new() -> Self {
        Self {
            total_processed: 0,
            successful: 0,
            errors: 0,
            transaction_types: HashMap::new(),
            total_processing_time: Duration::ZERO,
            min_time: Duration::MAX,
            max_time: Duration::ZERO,
        }
    }

    fn record_success(&mut self, transaction: &Transaction, processing_time: Duration) {
        self.total_processed += 1;
        self.successful += 1;
        self.total_processing_time += processing_time;

        if processing_time < self.min_time {
            self.min_time = processing_time;
        }
        if processing_time > self.max_time {
            self.max_time = processing_time;
        }

        let tx_type = format!("{:?}", transaction.transaction_type)
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        *self.transaction_types.entry(tx_type).or_insert(0) += 1;
    }

    fn record_error(&mut self, _error: &str) {
        self.total_processed += 1;
        self.errors += 1;
    }

    fn display_results(&self, total_time: Duration) {
        log(LogTag::Transactions, "RESULTS", "=== ANALYZER TEST RESULTS ===");
        log(LogTag::Transactions, "RESULTS", &format!("Total Processed: {}", self.total_processed));
        log(LogTag::Transactions, "RESULTS", &format!("Successful: {}", self.successful));
        log(LogTag::Transactions, "RESULTS", &format!("Errors: {}", self.errors));
        log(
            LogTag::Transactions,
            "RESULTS",
            &format!(
                "Success Rate: {:.1}%",
                ((self.successful as f64) / (self.total_processed as f64)) * 100.0
            )
        );

        if self.successful > 0 {
            let avg_time = self.total_processing_time / (self.successful as u32);
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Avg Processing Time: {:.2}ms", avg_time.as_millis())
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Min Processing Time: {:.2}ms", self.min_time.as_millis())
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("Max Processing Time: {:.2}ms", self.max_time.as_millis())
            );
        }

        log(
            LogTag::Transactions,
            "RESULTS",
            &format!("Total Test Time: {:.2}s", total_time.as_secs_f64())
        );

        log(LogTag::Transactions, "RESULTS", "Transaction Types:");
        for (tx_type, count) in &self.transaction_types {
            log(LogTag::Transactions, "RESULTS", &format!("  {}: {}", tx_type, count));
        }

        log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");
    }
}

/// Statistics for cache analysis
#[derive(Debug)]
struct CacheStats {
    total_files: usize,
    valid_files: usize,
    invalid_files: usize,
    transaction_types: HashMap<String, usize>,
    oldest_transaction: Option<DateTime<Utc>>,
    newest_transaction: Option<DateTime<Utc>>,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            total_files: 0,
            valid_files: 0,
            invalid_files: 0,
            transaction_types: HashMap::new(),
            oldest_transaction: None,
            newest_transaction: None,
        }
    }

    fn record_transaction(&mut self, transaction: &Transaction) {
        self.total_files += 1;
        self.valid_files += 1;

        let tx_type = format!("{:?}", transaction.transaction_type)
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        *self.transaction_types.entry(tx_type).or_insert(0) += 1;

        if
            self.oldest_transaction.is_none() ||
            transaction.timestamp < self.oldest_transaction.unwrap()
        {
            self.oldest_transaction = Some(transaction.timestamp);
        }
        if
            self.newest_transaction.is_none() ||
            transaction.timestamp > self.newest_transaction.unwrap()
        {
            self.newest_transaction = Some(transaction.timestamp);
        }
    }

    fn record_error(&mut self) {
        self.total_files += 1;
        self.invalid_files += 1;
    }

    fn display_results(&self) {
        log(LogTag::Transactions, "CACHE", "=== CACHE ANALYSIS RESULTS ===");
        log(LogTag::Transactions, "CACHE", &format!("Total Files: {}", self.total_files));
        log(LogTag::Transactions, "CACHE", &format!("Valid Files: {}", self.valid_files));
        log(LogTag::Transactions, "CACHE", &format!("Invalid Files: {}", self.invalid_files));

        if let (Some(oldest), Some(newest)) = (self.oldest_transaction, self.newest_transaction) {
            log(LogTag::Transactions, "CACHE", &format!("Oldest Transaction: {}", oldest));
            log(LogTag::Transactions, "CACHE", &format!("Newest Transaction: {}", newest));

            let time_span = newest.signed_duration_since(oldest);
            log(
                LogTag::Transactions,
                "CACHE",
                &format!("Time Span: {} days", time_span.num_days())
            );
        }

        log(LogTag::Transactions, "CACHE", "Transaction Types in Cache:");
        for (tx_type, count) in &self.transaction_types {
            log(LogTag::Transactions, "CACHE", &format!("  {}: {}", tx_type, count));
        }

        log(LogTag::Transactions, "CACHE", "=== END CACHE ANALYSIS ===");
    }
}

/// Execute real swap test with comprehensive transaction analysis
async fn test_real_swap(
    wallet_pubkey: Pubkey,
    swap_type: &str,
    token_mint: &str,
    token_symbol: &str,
    sol_amount: f64,
    slippage: f64,
    router: &str,
    dry_run: bool
) {
    log(LogTag::Transactions, "SWAP_TEST", "=== REAL SWAP TEST STARTING ===");
    log(
        LogTag::Transactions,
        "SWAP_TEST",
        &format!(
            "Test Configuration:\n  • Swap Type: {}\n  • Token: {} ({})\n  • SOL Amount: {:.6} SOL\n  • Slippage: {:.1}%\n  • Router: {}\n  • Dry Run: {}",
            swap_type,
            token_symbol,
            &token_mint[..8],
            sol_amount,
            slippage,
            router,
            dry_run
        )
    );

    if dry_run {
        log(
            LogTag::Transactions,
            "DRY_RUN",
            "DRY RUN MODE: Simulating swap without real transactions"
        );
        log(
            LogTag::Transactions,
            "DRY_RUN",
            "All operations will be simulated only - no real SOL will be spent"
        );
    } else {
        // Safety warning
        log(
            LogTag::Transactions,
            "WARNING",
            "This test performs REAL blockchain transactions with REAL SOL!"
        );
        log(LogTag::Transactions, "WARNING", "Starting in 5 seconds... Press Ctrl+C to cancel!");

        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    // Create transactions manager for monitoring
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create TransactionsManager: {}", e)
            );
            return;
        }
    };

    // Pre-flight checks
    if let Err(e) = perform_preflight_checks(wallet_pubkey, sol_amount, token_mint, router).await {
        log(LogTag::Transactions, "ERROR", &format!("Pre-flight check failed: {}", e));
        return;
    }

    log(LogTag::Transactions, "SUCCESS", "✅ All pre-flight checks passed");

    // Load token with updated information from tokens module
    let test_token = match load_token_with_updated_info(token_mint, token_symbol).await {
        Ok(token) => token,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load token info: {}", e));
            log(LogTag::Transactions, "INFO", "Creating basic token for testing...");
            create_basic_token(token_mint, token_symbol)
        }
    };

    match swap_type {
        "sol-to-token" => {
            execute_sol_to_token_test(
                &mut manager,
                &test_token,
                sol_amount,
                slippage,
                router,
                dry_run
            ).await;
        }
        "token-to-sol" => {
            execute_token_to_sol_test(&mut manager, &test_token, slippage, router, dry_run).await;
        }
        "round-trip" => {
            execute_round_trip_test(
                &mut manager,
                &test_token,
                sol_amount,
                slippage,
                router,
                dry_run
            ).await;
        }
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown swap type: {}", swap_type));
        }
    }

    log(LogTag::Transactions, "SWAP_TEST", "=== REAL SWAP TEST COMPLETED ===");
}

/// Perform pre-flight safety checks before executing swaps
async fn perform_preflight_checks(
    wallet_pubkey: Pubkey,
    sol_amount: f64,
    token_mint: &str,
    router: &str
) -> Result<(), String> {
    log(LogTag::Transactions, "PREFLIGHT", "🔍 Performing pre-flight checks...");

    let slippage = 1.0; // 1% slippage for testing

    // Check wallet SOL balance
    let rpc_client = get_rpc_client();
    let sol_balance = match rpc_client.get_sol_balance(&wallet_pubkey.to_string()).await {
        Ok(balance) => balance,
        Err(e) => {
            return Err(format!("Failed to get wallet balance: {}", e));
        }
    };

    let minimum_required = sol_amount + 0.01; // Buffer for fees
    if sol_balance < minimum_required {
        return Err(
            format!(
                "Insufficient SOL balance: {:.6} SOL, required: {:.6} SOL",
                sol_balance,
                minimum_required
            )
        );
    }

    log(
        LogTag::Transactions,
        "PREFLIGHT",
        &format!(
            "✅ Wallet balance check: {:.6} SOL (required: {:.6} SOL)",
            sol_balance,
            minimum_required
        )
    );

    // Test quote availability
    let wallet_address = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;
    let lamport_amount = sol_to_lamports(sol_amount);

    let quote_result = match router {
        "jupiter" => {
            get_jupiter_quote(
                "So11111111111111111111111111111111111111112", // SOL mint
                token_mint,
                lamport_amount,
                slippage
            ).await
        }
        "gmgn" => {
            get_gmgn_quote(
                "So11111111111111111111111111111111111111112",
                token_mint,
                lamport_amount,
                &wallet_address,
                slippage
            ).await
        }
        "raydium-cpmm" => {
            // Skip quote test for Raydium CPMM as it doesn't use traditional quotes
            log(
                LogTag::Transactions,
                "PREFLIGHT",
                "✅ Raydium CPMM: Direct pool access, skipping quote test"
            );

            // Check if the token is the supported test token
            if token_mint != "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t" {
                return Err(
                    format!("Raydium CPMM only supports test token 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t, got: {}", token_mint)
                );
            }

            // Return a dummy success result since we skipped the actual quote test
            return Ok(());
        }
        _ => {
            return Err(format!("Unknown router: {}", router));
        }
    };

    match quote_result {
        Ok(quote) => {
            log(
                LogTag::Transactions,
                "PREFLIGHT",
                &format!(
                    "✅ {} quote test: {} SOL -> {} tokens",
                    router,
                    quote.quote.in_amount,
                    quote.quote.out_amount
                )
            );
        }
        Err(e) => {
            return Err(format!("{} quote test failed: {}", router, e));
        }
    }

    log(LogTag::Transactions, "PREFLIGHT", "✅ All pre-flight checks completed successfully");
    Ok(())
}

/// Execute SOL to Token swap test
async fn execute_sol_to_token_test(
    manager: &mut TransactionsManager,
    token: &Token,
    sol_amount: f64,
    slippage: f64,
    router: &str,
    dry_run: bool
) {
    log(
        LogTag::Transactions,
        "BUY_TEST",
        &format!(
            "Starting {} BUY test (SOL -> {}) - Dry Run: {}",
            router.to_uppercase(),
            token.symbol,
            dry_run
        )
    );

    let start_time = Instant::now();

    if dry_run {
        log(LogTag::Transactions, "DRY_RUN", "DRY RUN: Simulating swap execution...");
        log(
            LogTag::Transactions,
            "DRY_RUN",
            &format!(
                "Would execute {} swap: {:.6} SOL -> {} tokens",
                router.to_uppercase(),
                sol_amount,
                token.symbol
            )
        );

        let execution_time = start_time.elapsed();
        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "DRY RUN: {} BUY simulation completed in {:.2}s!",
                router.to_uppercase(),
                execution_time.as_secs_f64()
            )
        );
        return;
    }

    let swap_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, sol_amount, slippage, true).await,
        "gmgn" => execute_gmgn_swap_test(token, sol_amount, slippage, true).await,
        "raydium-cpmm" => execute_raydium_cpmm_swap_test(token, sol_amount, slippage, true).await,
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    match swap_result {
        Ok(result) => {
            let execution_time = start_time.elapsed();
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!(
                    "✅ {} BUY completed in {:.2}s!",
                    router.to_uppercase(),
                    execution_time.as_secs_f64()
                )
            );

            if let Some(signature) = &result.transaction_signature {
                log(
                    LogTag::Transactions,
                    "BUY_RESULT",
                    &format!(
                        "• Signature: {}\n  • Input: {} SOL\n  • Output: {} tokens\n  • Price Impact: {}%\n  • Fee: {} lamports",
                        &signature[..12],
                        result.input_amount,
                        result.output_amount,
                        result.price_impact,
                        result.fee_lamports
                    )
                );

                // Wait for transaction confirmation and analyze
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "BUY").await;
            }
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("❌ {} BUY failed: {}", router.to_uppercase(), e)
            );
        }
    }
}

/// Execute Token to SOL swap test
async fn execute_token_to_sol_test(
    manager: &mut TransactionsManager,
    token: &Token,
    slippage: f64,
    router: &str,
    dry_run: bool
) {
    log(
        LogTag::Transactions,
        "SELL_TEST",
        &format!(
            "Starting {} SELL test ({} -> SOL) - Dry Run: {}",
            router.to_uppercase(),
            token.symbol,
            dry_run
        )
    );

    // Get wallet address for balance check
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get wallet address: {}", e));
            return;
        }
    };

    // Check existing token balance
    let token_balance = match
        screenerbot::utils::get_token_balance(&wallet_address, &token.mint).await
    {
        Ok(balance) => balance,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get token balance: {}", e));
            return;
        }
    };

    if token_balance == 0 {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!(
                "No {} balance found in wallet. Cannot execute token-to-sol test.",
                token.symbol
            )
        );
        return;
    }

    // Get token decimals for proper amount calculation
    let token_decimals = match get_token_decimals_sync(&token.mint) {
        Some(decimals) => decimals,
        None => {
            log(
                LogTag::Transactions,
                "WARN",
                "Could not get token decimals from cache, using default 9"
            );
            9
        }
    };

    let token_amount_raw = (token_balance as f64) / (10_f64).powi(token_decimals as i32);

    log(
        LogTag::Transactions,
        "SELL_TEST",
        &format!(
            "🔍 Found token balance: {} raw tokens ({:.6} decimal-adjusted tokens)",
            token_balance,
            token_amount_raw
        )
    );

    // Use all available tokens for the sell test
    let tokens_to_sell = token_amount_raw;

    let start_time = Instant::now();

    if dry_run {
        log(LogTag::Transactions, "DRY_RUN", "DRY RUN: Simulating sell execution...");
        log(
            LogTag::Transactions,
            "DRY_RUN",
            &format!(
                "Would execute {} swap: {:.6} {} tokens -> SOL",
                router.to_uppercase(),
                tokens_to_sell,
                token.symbol
            )
        );

        let execution_time = start_time.elapsed();
        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "DRY RUN: {} SELL simulation completed in {:.2}s!",
                router.to_uppercase(),
                execution_time.as_secs_f64()
            )
        );
        return;
    }

    let swap_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, tokens_to_sell, slippage, false).await,
        "gmgn" => execute_gmgn_swap_test(token, tokens_to_sell, slippage, false).await,
        "raydium-cpmm" => {
            log(
                LogTag::Transactions,
                "WARNING",
                "Raydium CPMM does not support token-to-SOL swaps in this test version"
            );
            return;
        }
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    match swap_result {
        Ok(result) => {
            let execution_time = start_time.elapsed();
            log(
                LogTag::Transactions,
                "SUCCESS",
                &format!(
                    "✅ {} SELL completed in {:.2}s!",
                    router.to_uppercase(),
                    execution_time.as_secs_f64()
                )
            );

            if let Some(signature) = &result.transaction_signature {
                log(
                    LogTag::Transactions,
                    "SELL_RESULT",
                    &format!(
                        "• Signature: {}\n  • Input: {} tokens\n  • Output: {} SOL\n  • Price Impact: {}%\n  • Fee: {} lamports",
                        &signature[..12],
                        result.input_amount,
                        result.output_amount,
                        result.price_impact,
                        result.fee_lamports
                    )
                );

                // Wait for transaction confirmation and analyze
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "SELL").await;
            }
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("❌ {} SELL failed: {}", router.to_uppercase(), e)
            );
        }
    }
}

/// Execute round-trip swap test (SOL -> Token -> SOL)
async fn execute_round_trip_test(
    manager: &mut TransactionsManager,
    token: &Token,
    sol_amount: f64,
    slippage: f64,
    router: &str,
    dry_run: bool
) {
    log(
        LogTag::Transactions,
        "ROUND_TRIP",
        &format!(
            "Starting {} ROUND-TRIP test (SOL -> {} -> SOL) - Dry Run: {}",
            router.to_uppercase(),
            token.symbol,
            dry_run
        )
    );

    let mut test_results = SwapTestResults::new();

    // Phase 1: SOL -> Token (BUY)
    log(
        LogTag::Transactions,
        "BUY_PHASE",
        &format!("Phase 1: {} BUY (SOL -> {})", router.to_uppercase(), token.symbol)
    );

    let buy_start = Instant::now();

    if dry_run {
        log(LogTag::Transactions, "DRY_RUN", "DRY RUN: Simulating round-trip test...");
        log(
            LogTag::Transactions,
            "DRY_RUN",
            &format!("Phase 1 simulation: {:.6} SOL -> {} tokens", sol_amount, token.symbol)
        );
        log(
            LogTag::Transactions,
            "DRY_RUN",
            &format!("Phase 2 simulation: {} tokens -> SOL", token.symbol)
        );

        let execution_time = buy_start.elapsed();
        log(
            LogTag::Transactions,
            "SUCCESS",
            &format!(
                "DRY RUN: {} ROUND-TRIP simulation completed in {:.2}s!",
                router.to_uppercase(),
                execution_time.as_secs_f64()
            )
        );
        return;
    }

    let buy_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, sol_amount, slippage, true).await,
        "gmgn" => execute_gmgn_swap_test(token, sol_amount, slippage, true).await,
        "raydium-cpmm" => execute_raydium_cpmm_swap_test(token, sol_amount, slippage, true).await,
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    let mut tokens_received = 0.0;
    let mut _buy_signature = String::new();

    match buy_result {
        Ok(result) => {
            let buy_time = buy_start.elapsed();
            test_results.buy_success = true;
            test_results.buy_execution_time = buy_time.as_secs_f64();

            if let Some(signature) = &result.transaction_signature {
                _buy_signature = signature.clone();
                test_results.buy_signature = Some(signature.clone());

                tokens_received = result.output_amount.parse::<f64>().unwrap_or(0.0);
                test_results.tokens_received = tokens_received;
                test_results.sol_spent = result.input_amount.parse::<f64>().unwrap_or(0.0);

                log(
                    LogTag::Transactions,
                    "BUY_SUCCESS",
                    &format!(
                        "✅ BUY completed in {:.2}s: {} SOL -> {} tokens ({})",
                        buy_time.as_secs_f64(),
                        result.input_amount,
                        result.output_amount,
                        &signature[..12]
                    )
                );

                // Analyze buy transaction
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "BUY").await;
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ BUY phase failed: {}", e));
            test_results.display_results();
            return;
        }
    }

    // Wait a bit between phases
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Phase 2: Token -> SOL (SELL)
    log(
        LogTag::Transactions,
        "SELL_PHASE",
        &format!("🔴 Phase 2: {} SELL ({} -> SOL)", router.to_uppercase(), token.symbol)
    );

    if tokens_received <= 0.0 {
        log(
            LogTag::Transactions,
            "ERROR",
            "❌ No tokens received from buy phase, cannot proceed with sell"
        );
        test_results.display_results();
        return;
    }

    let sell_start = Instant::now();
    let sell_result = match router {
        "jupiter" => execute_jupiter_swap_test(token, tokens_received, slippage, false).await,
        "gmgn" => execute_gmgn_swap_test(token, tokens_received, slippage, false).await,
        "raydium-cpmm" => {
            log(
                LogTag::Transactions,
                "WARNING",
                "Raydium CPMM does not support token-to-SOL swaps in this test version"
            );
            return;
        }
        _ => {
            log(LogTag::Transactions, "ERROR", &format!("Unknown router: {}", router));
            return;
        }
    };

    match sell_result {
        Ok(result) => {
            let sell_time = sell_start.elapsed();
            test_results.sell_success = true;
            test_results.sell_execution_time = sell_time.as_secs_f64();

            if let Some(signature) = &result.transaction_signature {
                test_results.sell_signature = Some(signature.clone());
                test_results.sol_received = result.output_amount.parse::<f64>().unwrap_or(0.0);

                log(
                    LogTag::Transactions,
                    "SELL_SUCCESS",
                    &format!(
                        "✅ SELL completed in {:.2}s: {} tokens -> {} SOL ({})",
                        sell_time.as_secs_f64(),
                        result.input_amount,
                        result.output_amount,
                        &signature[..12]
                    )
                );

                // Analyze sell transaction
                tokio::time::sleep(Duration::from_secs(10)).await;
                analyze_swap_transaction(manager, signature, "SELL").await;
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ SELL phase failed: {}", e));
        }
    }

    // Display comprehensive results
    test_results.display_results();
}

/// Execute Jupiter swap test
async fn execute_jupiter_swap_test(
    token: &Token,
    amount: f64,
    slippage: f64,
    is_buy: bool // true for SOL->Token, false for Token->SOL
) -> Result<JupiterSwapResult, ScreenerBotError> {
    let wallet_address = get_wallet_address()?;
    let sol_mint = "So11111111111111111111111111111111111111112";

    let token_decimals = match get_token_decimals_sync(&token.mint) {
        Some(decimals) => decimals,
        None => {
            log(
                LogTag::Transactions,
                "WARN",
                "Could not get token decimals from cache, using default 9"
            );
            9
        }
    };

    let (input_mint, output_mint, input_amount) = if is_buy {
        // SOL -> Token
        (sol_mint.to_string(), token.mint.clone(), sol_to_lamports(amount))
    } else {
        // Token -> SOL
        let token_amount = (amount * (10_f64).powi(token_decimals as i32)) as u64;
        (token.mint.clone(), sol_mint.to_string(), token_amount)
    };

    // Get quote first
    let quote = get_jupiter_quote(&input_mint, &output_mint, input_amount, slippage).await?;

    // Execute the swap
    execute_jupiter_swap(token, &input_mint, &output_mint, quote).await
}

/// Execute GMGN swap test
async fn execute_gmgn_swap_test(
    token: &Token,
    amount: f64,
    slippage: f64,
    is_buy: bool
) -> Result<JupiterSwapResult, ScreenerBotError> {
    let wallet_address = get_wallet_address()?;
    let sol_mint = "So11111111111111111111111111111111111111112";

    let token_decimals = match get_token_decimals_sync(&token.mint) {
        Some(decimals) => decimals,
        None => {
            log(
                LogTag::Transactions,
                "WARN",
                "Could not get token decimals from cache, using default 9"
            );
            9
        }
    };

    let (input_mint, output_mint, input_amount) = if is_buy {
        (sol_mint.to_string(), token.mint.clone(), sol_to_lamports(amount))
    } else {
        let token_amount = (amount * (10_f64).powi(token_decimals as i32)) as u64;
        (token.mint.clone(), sol_mint.to_string(), token_amount)
    };

    // Get quote first
    let _quote = get_gmgn_quote(
        &input_mint,
        &output_mint,
        input_amount,
        &wallet_address,
        slippage
    ).await?;

    // Get quote first to create swap data
    let quote = get_gmgn_quote(
        &input_mint,
        &output_mint,
        input_amount,
        &wallet_address,
        slippage
    ).await?;

    // Execute the swap through GMGN
    let swap_result = screenerbot::swaps::gmgn::execute_gmgn_swap(
        token,
        &input_mint,
        &output_mint,
        input_amount,
        quote
    ).await?;

    // Convert to JupiterSwapResult format for consistency
    Ok(JupiterSwapResult {
        success: swap_result.success,
        transaction_signature: swap_result.transaction_signature,
        input_amount: swap_result.input_amount,
        output_amount: swap_result.output_amount,
        price_impact: swap_result.price_impact,
        fee_lamports: swap_result.fee_lamports,
        execution_time: swap_result.execution_time,
        effective_price: swap_result.effective_price,
        swap_data: swap_result.swap_data,
        error: swap_result.error,
    })
}

/// Execute Raydium CPMM direct swap test (DEPRECATED - Raydium direct API is no longer available)
async fn execute_raydium_cpmm_swap_test(
    token: &Token,
    sol_amount: f64,
    slippage: f64,
    is_buy: bool
) -> Result<JupiterSwapResult, ScreenerBotError> {
    // Raydium direct API is deprecated - use Jupiter aggregator which includes Raydium routes
    Err(
        ScreenerBotError::Configuration(screenerbot::errors::ConfigurationError::Generic {
            message: "Raydium CPMM direct test is deprecated. Use Jupiter aggregator which includes Raydium routes.".to_string(),
        })
    )
}

/// Analyze a specific swap transaction
async fn analyze_swap_transaction(
    manager: &mut TransactionsManager,
    signature: &str,
    swap_type: &str
) {
    log(
        LogTag::Transactions,
        "ANALYSIS",
        &format!("📊 Analyzing {} transaction: {}...", swap_type, &signature[..12])
    );

    tokio::time::sleep(Duration::from_secs(2)).await; // Wait for RPC propagation

    match manager.process_transaction(signature).await {
        Ok(transaction) => {
            log(
                LogTag::Transactions,
                "ANALYSIS_SUCCESS",
                &format!("✅ Transaction analysis completed for {}", &signature[..12])
            );

            // Display comprehensive transaction details
            display_detailed_transaction_info(&transaction);

            // Add to swap analysis if it's a swap
            if
                matches!(
                    transaction.transaction_type,
                    TransactionType::SwapSolToToken { .. } |
                        TransactionType::SwapTokenToSol { .. } |
                        TransactionType::SwapTokenToToken { .. }
                )
            {
                log(
                    LogTag::Transactions,
                    "SWAP_DETECTED",
                    "✅ Transaction confirmed as swap and analyzed"
                );
            }
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ANALYSIS_ERROR",
                &format!("❌ Failed to analyze transaction {}: {}", &signature[..12], e)
            );
        }
    }
}

/// Test results tracking structure
struct SwapTestResults {
    buy_success: bool,
    sell_success: bool,
    buy_signature: Option<String>,
    sell_signature: Option<String>,
    buy_execution_time: f64,
    sell_execution_time: f64,
    sol_spent: f64,
    tokens_received: f64,
    sol_received: f64,
}

impl SwapTestResults {
    fn new() -> Self {
        Self {
            buy_success: false,
            sell_success: false,
            buy_signature: None,
            sell_signature: None,
            buy_execution_time: 0.0,
            sell_execution_time: 0.0,
            sol_spent: 0.0,
            tokens_received: 0.0,
            sol_received: 0.0,
        }
    }

    fn display_results(&self) {
        log(LogTag::Transactions, "RESULTS", "📊 === COMPLETE SWAP TEST RESULTS ===");

        log(LogTag::Transactions, "RESULTS", " 🔵 BUY PHASE:");
        if self.buy_success {
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • Status: ✅ Success ({:.2}s)", self.buy_execution_time)
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • SOL Spent: {:.6} SOL", self.sol_spent)
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • Tokens Received: {:.2} tokens", self.tokens_received)
            );
            if let Some(sig) = &self.buy_signature {
                log(LogTag::Transactions, "RESULTS", &format!("  • TX: {}...", &sig[..12]));
            }
        } else {
            log(LogTag::Transactions, "RESULTS", "  • Status: ❌ Failed");
        }

        log(LogTag::Transactions, "RESULTS", " 🔴 SELL PHASE:");
        if self.sell_success {
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • Status: ✅ Success ({:.2}s)", self.sell_execution_time)
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • Tokens Sold: {:.2} tokens", self.tokens_received)
            );
            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • SOL Received: {:.6} SOL", self.sol_received)
            );
            if let Some(sig) = &self.sell_signature {
                log(LogTag::Transactions, "RESULTS", &format!("  • TX: {}...", &sig[..12]));
            }
        } else {
            log(LogTag::Transactions, "RESULTS", "  • Status: ❌ Failed");
        }

        log(LogTag::Transactions, "RESULTS", " 💰 NET RESULT:");
        if self.buy_success && self.sell_success {
            let net_sol = self.sol_received - self.sol_spent;
            let success_indicator = if net_sol >= -0.001 { "✅ Good" } else { "⚠️ High Cost" };

            log(
                LogTag::Transactions,
                "RESULTS",
                &format!("  • Net SOL Change: {:.6} SOL", net_sol)
            );
            log(LogTag::Transactions, "RESULTS", &format!("  • Success: {}", success_indicator));

            if self.tokens_received > 0.0 {
                let effective_price = self.sol_spent / self.tokens_received;
                log(
                    LogTag::Transactions,
                    "RESULTS",
                    &format!("  • Effective Price: {:.12} SOL per token", effective_price)
                );
            }
        } else {
            log(LogTag::Transactions, "RESULTS", "  • Net SOL Change: N/A (incomplete test)");
        }

        log(LogTag::Transactions, "RESULTS", " 📋 SIGNATURES:");
        if let Some(buy_sig) = &self.buy_signature {
            log(LogTag::Transactions, "RESULTS", &format!("  • Buy TX: {}", buy_sig));
        }
        if let Some(sell_sig) = &self.sell_signature {
            log(LogTag::Transactions, "RESULTS", &format!("  • Sell TX: {}", sell_sig));
        }

        log(LogTag::Transactions, "RESULTS", "=== END RESULTS ===");
    }
}

/// Statistics for benchmark testing
#[derive(Debug)]
struct BenchmarkStats {
    successful: usize,
    errors: usize,
    total_processing_time: Duration,
    processing_times: Vec<Duration>,
    transaction_types: HashMap<String, usize>,
}

impl BenchmarkStats {
    fn new() -> Self {
        Self {
            successful: 0,
            errors: 0,
            total_processing_time: Duration::ZERO,
            processing_times: Vec::new(),
            transaction_types: HashMap::new(),
        }
    }

    fn record_transaction(&mut self, transaction: &Transaction, processing_time: Duration) {
        self.successful += 1;
        self.total_processing_time += processing_time;
        self.processing_times.push(processing_time);

        let tx_type = format!("{:?}", transaction.transaction_type)
            .split('{')
            .next()
            .unwrap_or("Unknown")
            .to_string();
        *self.transaction_types.entry(tx_type).or_insert(0) += 1;
    }

    fn record_error(&mut self) {
        self.errors += 1;
    }

    fn display_results(&self, total_time: Duration, total_transactions: usize) {
        log(LogTag::Transactions, "BENCHMARK", "=== BENCHMARK RESULTS ===");
        log(
            LogTag::Transactions,
            "BENCHMARK",
            &format!("Total Transactions: {}", total_transactions)
        );
        log(LogTag::Transactions, "BENCHMARK", &format!("Successful: {}", self.successful));
        log(LogTag::Transactions, "BENCHMARK", &format!("Errors: {}", self.errors));
        log(
            LogTag::Transactions,
            "BENCHMARK",
            &format!(
                "Success Rate: {:.1}%",
                ((self.successful as f64) / (total_transactions as f64)) * 100.0
            )
        );

        if !self.processing_times.is_empty() {
            let avg_time = self.total_processing_time / (self.processing_times.len() as u32);
            let min_time = self.processing_times.iter().min().unwrap();
            let max_time = self.processing_times.iter().max().unwrap();

            // Calculate percentiles
            let mut sorted_times = self.processing_times.clone();
            sorted_times.sort();
            let p50 = sorted_times[sorted_times.len() / 2];
            let p95 = sorted_times[(sorted_times.len() * 95) / 100];

            log(
                LogTag::Transactions,
                "BENCHMARK",
                &format!("Avg Processing Time: {:.2}ms", avg_time.as_millis())
            );
            log(
                LogTag::Transactions,
                "BENCHMARK",
                &format!("Min Processing Time: {:.2}ms", min_time.as_millis())
            );
            log(
                LogTag::Transactions,
                "BENCHMARK",
                &format!("Max Processing Time: {:.2}ms", max_time.as_millis())
            );
            log(
                LogTag::Transactions,
                "BENCHMARK",
                &format!("P50 Processing Time: {:.2}ms", p50.as_millis())
            );
            log(
                LogTag::Transactions,
                "BENCHMARK",
                &format!("P95 Processing Time: {:.2}ms", p95.as_millis())
            );
        }

        log(
            LogTag::Transactions,
            "BENCHMARK",
            &format!("Total Benchmark Time: {:.2}s", total_time.as_secs_f64())
        );

        if total_time.as_secs() > 0 {
            let throughput = (self.successful as f64) / total_time.as_secs_f64();
            log(
                LogTag::Transactions,
                "BENCHMARK",
                &format!("Throughput: {:.2} tx/sec", throughput)
            );
        }

        log(LogTag::Transactions, "BENCHMARK", "Transaction Types:");
        for (tx_type, count) in &self.transaction_types {
            log(LogTag::Transactions, "BENCHMARK", &format!("  {}: {}", tx_type, count));
        }

        log(LogTag::Transactions, "BENCHMARK", "=== END BENCHMARK ===");
    }
}

/// Test real position management with transaction verification (like main bot)
async fn test_real_position_management(
    wallet_pubkey: Pubkey,
    token_mint: &str,
    token_symbol: &str,
    sol_amount: f64
) {
    log(LogTag::Transactions, "POSITION_TEST", "=== REAL POSITION MANAGEMENT TEST ===");
    log(
        LogTag::Transactions,
        "POSITION_TEST",
        &format!(
            "📋 Test Configuration:\n  • Token: {} ({})\n  • SOL Amount: {:.6} SOL\n  • This test mimics main bot position management",
            token_symbol,
            &token_mint[..8],
            sol_amount
        )
    );

    // Safety warning
    log(
        LogTag::Transactions,
        "WARNING",
        "⚠️ This test performs REAL blockchain transactions with REAL SOL!"
    );
    log(
        LogTag::Transactions,
        "WARNING",
        "⚠️ This test will open and close a position like the main bot!"
    );
    log(LogTag::Transactions, "WARNING", "⚠️ Starting in 10 seconds... Press Ctrl+C to cancel!");

    tokio::time::sleep(Duration::from_secs(10)).await;

    // Initialize token system and price service
    log(LogTag::Transactions, "POSITION_TEST", "🔧 Initializing token systems...");

    // Initialize token database
    let _token_database = match screenerbot::tokens::TokenDatabase::new() {
        Ok(db) => {
            log(LogTag::Transactions, "POSITION_TEST", "✅ Token database initialized");
            db
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to initialize token database: {}", e)
            );
            return;
        }
    };

    // Initialize price service
    if let Err(e) = screenerbot::tokens::initialize_price_service().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize price service: {}", e));
        return;
    }
    log(LogTag::Transactions, "POSITION_TEST", "✅ Price service initialized");

    // Initialize DexScreener API
    if let Err(e) = screenerbot::tokens::init_dexscreener_api().await {
        log(LogTag::Transactions, "WARN", &format!("Failed to initialize DexScreener API: {}", e));
        // Continue anyway as this is not critical for position testing
    } else {
        log(LogTag::Transactions, "POSITION_TEST", "✅ DexScreener API initialized");
    }

    // Initialize global transaction manager for monitoring
    if let Err(e) = initialize_global_transaction_manager(wallet_pubkey).await {
        log(
            LogTag::Transactions,
            "ERROR",
            &format!("Failed to initialize transaction manager: {}", e)
        );
        return;
    }

    // Start lightweight transaction monitoring for the test
    log(
        LogTag::Transactions,
        "POSITION_TEST",
        "🔄 Starting transaction monitoring for position test..."
    );
    let shutdown_monitor = Arc::new(tokio::sync::Notify::new());
    let monitor_handle = {
        let shutdown_clone = shutdown_monitor.clone();
        tokio::spawn(async move {
            // Run monitoring for 5 minutes max (longer than position test)
            tokio::select! {
                _ = shutdown_clone.notified() => {
                    log(LogTag::Transactions, "POSITION_TEST", "Transaction monitoring stopped");
                }
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    log(LogTag::Transactions, "POSITION_TEST", "Transaction monitoring timeout (5 minutes)");
                }
                _ = start_lightweight_transaction_monitoring(wallet_pubkey, shutdown_clone.clone()) => {
                    log(LogTag::Transactions, "POSITION_TEST", "Transaction monitoring completed");
                }
            }
        })
    };

    // Load token with updated information from tokens module
    let test_token = match load_token_with_updated_info(token_mint, token_symbol).await {
        Ok(token) => {
            log(
                LogTag::Transactions,
                "POSITION_TEST",
                &format!(
                    "✅ Loaded token: {} ({}) with updated info - Price: {} SOL, Liquidity: {} USD",
                    token.symbol,
                    &token.mint[..8],
                    token.price_dexscreener_sol
                        .map(|p| format!("{:.12}", p))
                        .unwrap_or("N/A".to_string()),
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .map(|l| format!("{:.0}", l))
                        .unwrap_or("N/A".to_string())
                )
            );
            token
        }
        Err(e) => {
            log(LogTag::Transactions, "WARNING", &format!("Failed to load token info: {}", e));
            log(LogTag::Transactions, "INFO", "Creating basic token for testing...");
            create_basic_token(token_mint, token_symbol)
        }
    };

    // Get current price for the test
    let current_price = sol_amount / 1000.0; // Simulate price for testing

    log(
        LogTag::Transactions,
        "POSITION_TEST",
        "🟢 STEP 1: Opening position with transaction verification..."
    );

    // Get profit targets and liquidity tier for test
    let (profit_min, profit_max) = get_profit_target(&test_token).await;
    let liquidity_tier = Some("TEST".to_string()); // Test tier for debug

    // Open position using the PositionsManager
    if
        let Err(e) = positions::open_position_direct(
            &test_token,
            current_price,
            -5.0,
            sol_amount,
            liquidity_tier,
            profit_min,
            profit_max
        ).await
    {
        log(LogTag::Transactions, "ERROR", &format!("Failed to open position: {}", e));
        return;
    }

    // Wait a moment for position to be created
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check if position was created
    let open_positions = positions::get_open_positions().await;
    let test_position = open_positions.iter().find(|p| p.mint == token_mint);

    if let Some(position) = test_position {
        log(
            LogTag::Transactions,
            "POSITION_TEST",
            &format!(
                "✅ Position opened successfully: {} | Entry: {:.9} SOL | TX: {}",
                position.symbol,
                position.entry_price,
                position.entry_transaction_signature.as_ref().unwrap_or(&"None".to_string())
            )
        );

        log(
            LogTag::Transactions,
            "POSITION_TEST",
            "⏳ Waiting 10 seconds before closing position..."
        );
        tokio::time::sleep(Duration::from_secs(10)).await;

        log(
            LogTag::Transactions,
            "POSITION_TEST",
            "🔴 STEP 2: Closing position with transaction verification..."
        );

        // Get the position again in case it was updated
        let open_positions = positions::get_open_positions().await;
        if
            let Some(position) = open_positions
                .iter()
                .find(|p| p.mint == token_mint)
                .cloned()
        {
            let exit_price = current_price * 1.02; // Simulate 2% profit
            let exit_time = chrono::Utc::now();

            // Create token object for debug test
            let token = screenerbot::tokens::Token {
                mint: position.mint.clone(),
                symbol: position.symbol.clone(),
                name: position.name.clone(),
                chain: "solana".to_string(),
                logo_url: None,
                coingecko_id: None,
                website: None,
                description: None,
                tags: vec![],
                is_verified: false,
                created_at: None,
                price_dexscreener_sol: None,
                price_dexscreener_usd: None,
                price_pool_sol: None,
                price_pool_usd: None,
                dex_id: None,
                pair_address: None,
                pair_url: None,
                labels: vec![],
                fdv: None,
                market_cap: None,
                txns: None,
                volume: None,
                price_change: None,
                liquidity: None,
                info: None,
                boosts: None,
                decimals: None,
                rugcheck_data: None,
            };

            match
                positions::close_position_direct(
                    &position.mint,
                    &token,
                    exit_price,
                    "Debug test".to_string(),
                    exit_time
                ).await
            {
                Ok(_) => {
                    log(
                        LogTag::Transactions,
                        "POSITION_TEST",
                        &format!(
                            "✅ Position closed successfully: {} | Exit: {:.9} SOL | TX: {}",
                            position.symbol,
                            exit_price,
                            position.exit_transaction_signature
                                .as_ref()
                                .unwrap_or(&"None".to_string())
                        )
                    );

                    // Generate comprehensive test report
                    generate_comprehensive_position_test_report(
                        &test_token,
                        token_symbol,
                        token_mint,
                        sol_amount,
                        wallet_pubkey,
                        &position
                    ).await;
                }
                Err(e) => {
                    log(
                        LogTag::Transactions,
                        "POSITION_TEST",
                        &format!(
                            "❌ Failed to send close position request for {}: {}",
                            position.symbol,
                            e
                        )
                    );

                    // Generate report even if closing failed
                    generate_comprehensive_position_test_report(
                        &test_token,
                        token_symbol,
                        token_mint,
                        sol_amount,
                        wallet_pubkey,
                        &position
                    ).await;
                }
            }
        } else {
            log(LogTag::Transactions, "POSITION_TEST", "❌ Position not found for closing");
        }
    } else {
        log(LogTag::Transactions, "POSITION_TEST", "❌ Position was not created");
    }

    // Stop transaction monitoring
    shutdown_monitor.notify_one();

    // Give monitor a moment to stop
    tokio::time::sleep(Duration::from_secs(2)).await;

    log(LogTag::Transactions, "POSITION_TEST", "=== REAL POSITION MANAGEMENT TEST COMPLETED ===");
}

/// Load token with updated information from tokens module
async fn load_token_with_updated_info(
    token_mint: &str,
    token_symbol: &str
) -> Result<Token, String> {
    log(
        LogTag::Transactions,
        "TOKEN_LOAD",
        &format!("Loading token {} ({}) with updated info...", token_symbol, &token_mint[..8])
    );

    // Initialize tokens system if not already done
    if let Err(e) = screenerbot::tokens::initialize_tokens_system().await {
        log(LogTag::Transactions, "WARNING", &format!("Failed to initialize tokens system: {}", e));
    }

    // Try to get token from database first
    if let Some(mut token) = screenerbot::tokens::get_token_from_db(token_mint).await {
        log(
            LogTag::Transactions,
            "TOKEN_LOAD",
            &format!("✅ Found token in database: {}", token.symbol)
        );

        // Update with current price if available
        if let Some(current_price) = screenerbot::tokens::get_current_token_price(token_mint).await {
            token.price_dexscreener_sol = Some(current_price);
            log(
                LogTag::Transactions,
                "TOKEN_LOAD",
                &format!("✅ Updated current price: {:.12} SOL", current_price)
            );
        }

        // Get token decimals and ensure they are set
        if token.price_dexscreener_sol.is_none() {
            log(
                LogTag::Transactions,
                "TOKEN_LOAD",
                "⚠️ No price available, fetching decimals for safety..."
            );
        }

        // Ensure decimals are available
        if let Some(decimals) = screenerbot::tokens::get_token_decimals(token_mint).await {
            log(LogTag::Transactions, "TOKEN_LOAD", &format!("✅ Token decimals: {}", decimals));
        }

        return Ok(token);
    }

    // If not in database, try to fetch from discovery system
    log(LogTag::Transactions, "TOKEN_LOAD", "Token not in database, attempting discovery...");

    // Run discovery to fetch the token
    if let Err(e) = screenerbot::tokens::discover_tokens_once().await {
        log(LogTag::Transactions, "WARNING", &format!("Discovery failed: {}", e));
    }

    // Try again after discovery
    if let Some(token) = screenerbot::tokens::get_token_from_db(token_mint).await {
        log(
            LogTag::Transactions,
            "TOKEN_LOAD",
            &format!("✅ Found token after discovery: {}", token.symbol)
        );
        return Ok(token);
    }

    // If still not found, create a basic token but try to get current info
    log(
        LogTag::Transactions,
        "TOKEN_LOAD",
        "Token not found in discovery, creating with current data..."
    );
    let mut basic_token = create_basic_token(token_mint, token_symbol);

    // Try to get current price
    if let Some(current_price) = screenerbot::tokens::get_current_token_price(token_mint).await {
        basic_token.price_dexscreener_sol = Some(current_price);
        log(
            LogTag::Transactions,
            "TOKEN_LOAD",
            &format!("✅ Got current price: {:.12} SOL", current_price)
        );
    }

    Ok(basic_token)
}

/// Create a basic token structure for testing
fn create_basic_token(token_mint: &str, token_symbol: &str) -> Token {
    Token {
        mint: token_mint.to_string(),
        symbol: token_symbol.to_string(),
        name: token_symbol.to_string(),
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: None,
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: None,
        info: None,
        boosts: None,
        decimals: None,
        rugcheck_data: None,
    }
}

async fn generate_comprehensive_position_test_report(
    original_token: &Token,
    token_symbol: &str,
    token_mint: &str,
    sol_amount: f64,
    wallet_pubkey: Pubkey,
    position: &Position
) {
    println!("\n{}", "=".repeat(80));
    println!("🔍 COMPREHENSIVE POSITION TEST REPORT");
    println!("{}", "=".repeat(80));

    // Get current wallet balance
    let rpc_client = get_rpc_client();
    let current_sol_balance = match rpc_client.get_sol_balance(&wallet_pubkey.to_string()).await {
        Ok(balance) => balance,
        Err(_) => 0.0,
    };

    // Position Overview
    println!("\n📊 POSITION OVERVIEW:");
    println!("Token Address: {}", token_mint);
    println!("Token Symbol: {}", token_symbol);
    println!("Token Name: {}", original_token.name);
    println!("Test SOL Amount: {:.6} SOL", sol_amount);

    // Price Analysis
    println!("\n💰 PRICE ANALYSIS:");
    println!("Entry Price: {:.9} SOL", position.entry_price);
    if let Some(exit_price) = position.exit_price {
        println!("Exit Price: {:.9} SOL", exit_price);

        let price_change = ((exit_price - position.entry_price) / position.entry_price) * 100.0;
        let price_change_emoji = if price_change > 0.0 { "📈" } else { "📉" };
        println!("{} Price Change: {:.2}%", price_change_emoji, price_change);
    }

    // Token Amount Analysis
    println!("\n🪙 TOKEN AMOUNT ANALYSIS:");
    if let Some(token_amount) = position.token_amount {
        // Token amount is stored in raw units, need to convert to human readable
        let human_readable = (token_amount as f64) / 1_000_000.0; // Assuming 6 decimals for most tokens
        println!("Token Amount (raw units): {}", token_amount);
        println!("Token Amount (human readable): {:.6}", human_readable);
    }

    // SOL Balance Analysis
    println!("\n💎 SOL BALANCE ANALYSIS:");
    println!("Entry SOL Size: {:.9} SOL", position.entry_size_sol);
    println!("Total Position SOL: {:.9} SOL", position.total_size_sol);
    println!("Current Wallet Balance: {:.9} SOL", current_sol_balance);

    if let Some(sol_received) = position.sol_received {
        println!("SOL Received on Exit: {:.9} SOL", sol_received);
        let sol_change = sol_received - position.entry_size_sol;
        let sol_change_emoji = if sol_change > 0.0 { "📈" } else { "📉" };
        println!("{} SOL P&L: {:.9} SOL", sol_change_emoji, sol_change);

        if position.entry_size_sol > 0.0 {
            let sol_change_percent = (sol_change / position.entry_size_sol) * 100.0;
            println!("SOL P&L Percentage: {:.2}%", sol_change_percent);
        }
    }

    // Price Tracking
    println!("\n� PRICE TRACKING:");
    println!("Highest Price: {:.9} SOL", position.price_highest);
    println!("Lowest Price: {:.9} SOL", position.price_lowest);

    let highest_gain =
        ((position.price_highest - position.entry_price) / position.entry_price) * 100.0;
    let lowest_loss =
        ((position.price_lowest - position.entry_price) / position.entry_price) * 100.0;
    println!("Max Gain Potential: {:.2}%", highest_gain);
    println!("Max Loss Experienced: {:.2}%", lowest_loss);

    // Transaction Analysis
    println!("\n📋 TRANSACTION ANALYSIS:");
    if let Some(entry_sig) = &position.entry_transaction_signature {
        println!("Entry Transaction: https://solscan.io/tx/{}", entry_sig);
    }
    if let Some(exit_sig) = &position.exit_transaction_signature {
        println!("Exit Transaction: https://solscan.io/tx/{}", exit_sig);
    }

    // Verification Status
    println!("\n✅ VERIFICATION STATUS:");
    println!("Entry Transaction Verified: {}", if position.transaction_entry_verified {
        "✅ Yes"
    } else {
        "❌ No"
    });

    // Position Timing
    println!("\n⏱️ TIMING ANALYSIS:");
    println!("Position Opened: {}", position.entry_time.format("%Y-%m-%d %H:%M:%S UTC"));
    if let Some(exit_time) = position.exit_time {
        println!("Position Closed: {}", exit_time.format("%Y-%m-%d %H:%M:%S UTC"));

        if let Ok(duration) = exit_time.signed_duration_since(position.entry_time).to_std() {
            println!("Position Duration: {:.2} seconds", duration.as_secs_f64());
        }
    }

    // Performance Summary
    println!("\n🎯 PERFORMANCE SUMMARY:");
    if let (Some(exit_price), Some(sol_received)) = (position.exit_price, position.sol_received) {
        let price_pnl = ((exit_price - position.entry_price) / position.entry_price) * 100.0;
        let sol_pnl = sol_received - position.entry_size_sol;
        let sol_pnl_percent = (sol_pnl / position.entry_size_sol) * 100.0;

        let performance_emoji = if sol_pnl > 0.0 { "🟢" } else { "🔴" };
        println!("{} Price P&L: {:.2}%", performance_emoji, price_pnl);
        println!("{} SOL P&L: {:.9} SOL ({:.2}%)", performance_emoji, sol_pnl, sol_pnl_percent);

        // Risk assessment
        let risk_level = if sol_pnl_percent.abs() > 10.0 {
            "HIGH"
        } else if sol_pnl_percent.abs() > 5.0 {
            "MEDIUM"
        } else {
            "LOW"
        };
        println!("Risk Level: {}", risk_level);
    }

    // Position Status
    println!("\n📈 POSITION STATUS:");
    if position.transaction_exit_verified {
        println!("✅ Position closed successfully");
        println!("Status: COMPLETED");
    } else {
        println!("⚠️ Position still open or closure failed");
        println!("Status: OPEN/FAILED");
    }

    // Smart Targeting Analysis
    println!("\n🎯 SMART TARGETING:");
    if let Some(min_target) = position.profit_target_min {
        println!("Minimum Profit Target: {:.2}%", min_target);
    }
    if let Some(max_target) = position.profit_target_max {
        println!("Maximum Profit Target: {:.2}%", max_target);
    }
    if let Some(liquidity_tier) = &position.liquidity_tier {
        println!("Liquidity Tier: {}", liquidity_tier);
    }

    // ATA Information
    println!("\n🏦 ATA (Associated Token Account) STATUS:");
    println!(
        "Token ATA Address: {}",
        get_associated_token_address(
            &wallet_pubkey,
            &Pubkey::from_str(token_mint).unwrap_or_default()
        )
    );

    // Additional Insights
    println!("\n💡 INSIGHTS & ANALYSIS:");

    // Transaction verification insights
    if position.transaction_entry_verified {
        println!("✅ Transaction verification system worked correctly");
    } else {
        println!("⚠️ Transaction verification may have issues");
    }

    // Effective pricing insights
    if let Some(effective_entry) = position.effective_entry_price {
        println!("Effective Entry Price: {:.9} SOL", effective_entry);
        let slippage = ((effective_entry - position.entry_price) / position.entry_price) * 100.0;
        if slippage.abs() < 1.0 {
            println!("✅ Low slippage detected ({:.2}%)", slippage);
        } else {
            println!("⚠️ High slippage detected ({:.2}%)", slippage);
        }
    }

    if let Some(effective_exit) = position.effective_exit_price {
        println!("Effective Exit Price: {:.9} SOL", effective_exit);
    }

    // Position size analysis
    if position.total_size_sol > position.entry_size_sol {
        println!("📊 Position was accumulated (total > entry size)");
    } else {
        println!("📊 Single entry position");
    }

    // Market timing insights
    if let (Some(exit_price), Some(exit_time)) = (position.exit_price, position.exit_time) {
        let hold_duration = exit_time.signed_duration_since(position.entry_time);
        if let Ok(duration) = hold_duration.to_std() {
            if duration.as_secs() < 60 {
                println!("⚡ Very short position (< 1 minute) - High frequency strategy");
            } else if duration.as_secs() < 300 {
                println!("🕐 Short position (< 5 minutes) - Quick scalp trade");
            } else {
                println!("🕰️ Extended position (> 5 minutes) - Swing trade");
            }
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("📊 END OF COMPREHENSIVE POSITION TEST REPORT");
    println!("{}", "=".repeat(80));
}

/// Lightweight transaction monitoring for position tests
async fn start_lightweight_transaction_monitoring(
    wallet_pubkey: Pubkey,
    shutdown: Arc<tokio::sync::Notify>
) {
    log(LogTag::Transactions, "MONITOR", "Starting lightweight transaction monitoring...");

    // Create a monitoring manager
    let mut manager = match
        screenerbot::transactions::TransactionsManager::new(wallet_pubkey).await
    {
        Ok(manager) => manager,
        Err(e) => {
            log(
                LogTag::Transactions,
                "ERROR",
                &format!("Failed to create monitoring manager: {}", e)
            );
            return;
        }
    };

    // Initialize known signatures
    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize monitoring: {}", e));
        return;
    }

    log(
        LogTag::Transactions,
        "MONITOR",
        &format!(
            "Monitoring initialized with {} known transactions",
            manager.known_signatures.len()
        )
    );

    // Monitor frequently for position tests (every 2 seconds)
    let mut interval = tokio::time::interval(Duration::from_secs(2));

    loop {
        // Wait for either the next tick or shutdown
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Transactions, "MONITOR", "Shutdown received - stopping lightweight monitoring loop");
                break;
            }
            _ = interval.tick() => {}
        }

        // Check for new transactions
        match manager.check_new_transactions().await {
            Ok(new_signatures) => {
                if !new_signatures.is_empty() {
                    log(
                        LogTag::Transactions,
                        "MONITOR",
                        &format!("Found {} new transactions, processing...", new_signatures.len())
                    );

                    // Process each new transaction, but respect shutdown between items
                    for signature in new_signatures {
                        // Fast shutdown check between items
                        if
                            screenerbot::utils::check_shutdown_or_delay(
                                &shutdown,
                                Duration::from_millis(0)
                            ).await
                        {
                            log(
                                LogTag::Transactions,
                                "MONITOR",
                                "Stopping processing new signatures due to shutdown"
                            );
                            break;
                        }
                        if let Err(e) = manager.process_transaction(&signature).await {
                            log(
                                LogTag::Transactions,
                                "WARN",
                                &format!("Failed to process transaction {}: {}", &signature[..8], e)
                            );
                        } else {
                            log(
                                LogTag::Transactions,
                                "SUCCESS",
                                &format!("Successfully processed transaction {}", &signature[..8])
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log(LogTag::Transactions, "WARN", &format!("Monitoring cycle failed: {}", e));
            }
        }

        // Priority transaction system removed in favor of positions manager
    }
}

/// Get token information from the database by mint address
async fn get_token_info_from_database(mint_address: &str) {
    log(
        LogTag::System,
        "TOKEN_INFO",
        &format!("Looking up token information for mint: {}", mint_address)
    );

    // Validate mint address format
    if mint_address.len() < 32 || mint_address.len() > 44 {
        log(
            LogTag::System,
            "ERROR",
            &format!("Invalid mint address format: {} (should be 32-44 characters)", mint_address)
        );
        return;
    }

    // Try to parse as Pubkey to validate format
    match Pubkey::from_str(mint_address) {
        Ok(mint_pubkey) => {
            log(LogTag::System, "INFO", &format!("Valid mint address: {}", mint_pubkey));
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Invalid mint address format: {}", e));
            return;
        }
    }

    // Initialize the tokens system
    log(LogTag::System, "INIT", "Initializing tokens system for database lookup...");

    // Initialize tokens database
    match screenerbot::tokens::initialize_tokens_system().await {
        Ok(_) => {
            log(LogTag::System, "SUCCESS", "Tokens system initialized successfully");
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to initialize tokens system: {}", e));
            return;
        }
    }

    // Get token from database
    match screenerbot::tokens::get_token_from_db(mint_address).await {
        Some(token) => {
            log(LogTag::System, "SUCCESS", "Token found in database!");
            display_token_database_info(&token);
        }
        None => {
            log(LogTag::System, "INFO", "Token not found in database");

            // Try to get token decimals from cache
            if let Some(decimals) = get_token_decimals_sync(mint_address) {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("Found token decimals in cache: {}", decimals)
                );
            } else {
                log(LogTag::System, "INFO", "Token decimals not found in cache either");
            }

            // Suggest fetching from external sources
            log(LogTag::System, "INFO", "To add this token to the database, you can:");
            log(LogTag::System, "INFO", "1. Run a swap test with this token");
            log(LogTag::System, "INFO", "2. Use the main bot to discover this token");
            log(LogTag::System, "INFO", "3. Wait for the token discovery service to find it");
        }
    }
}

/// Display comprehensive token information from database
fn display_token_database_info(token: &screenerbot::tokens::Token) {
    log(LogTag::System, "TOKEN_INFO", "=== TOKEN DATABASE INFORMATION ===");
    log(LogTag::System, "TOKEN_INFO", &format!("Symbol: {}", token.symbol));
    log(LogTag::System, "TOKEN_INFO", &format!("Name: {}", token.name));
    log(LogTag::System, "TOKEN_INFO", &format!("Mint: {}", token.mint));
    log(LogTag::System, "TOKEN_INFO", &format!("Chain: {}", token.chain));

    // Try to get decimals from cache
    if let Some(decimals) = get_token_decimals_sync(&token.mint) {
        log(LogTag::System, "TOKEN_INFO", &format!("Decimals: {}", decimals));
    } else {
        log(LogTag::System, "TOKEN_INFO", "Decimals: Not available in cache");
    }

    // Display price information
    if let Some(price_sol) = token.price_dexscreener_sol {
        log(LogTag::System, "TOKEN_INFO", &format!("DexScreener Price (SOL): {:.12}", price_sol));
    }
    if let Some(price_usd) = token.price_dexscreener_usd {
        log(LogTag::System, "TOKEN_INFO", &format!("DexScreener Price (USD): ${:.6}", price_usd));
    }
    if let Some(price_sol) = token.price_pool_sol {
        log(LogTag::System, "TOKEN_INFO", &format!("Pool Price (SOL): {:.12}", price_sol));
    }
    if let Some(price_usd) = token.price_pool_usd {
        log(LogTag::System, "TOKEN_INFO", &format!("Pool Price (USD): ${:.6}", price_usd));
    }

    // Display market data
    if let Some(market_cap) = token.market_cap {
        log(LogTag::System, "TOKEN_INFO", &format!("Market Cap: ${:.2}", market_cap));
    }

    if let Some(fdv) = token.fdv {
        log(LogTag::System, "TOKEN_INFO", &format!("Fully Diluted Valuation: ${:.2}", fdv));
    }

    // Display volume statistics
    if let Some(ref volume) = token.volume {
        log(LogTag::System, "TOKEN_INFO", "Volume Statistics:");
        if let Some(h24) = volume.h24 {
            log(LogTag::System, "TOKEN_INFO", &format!("  24h Volume: ${:.2}", h24));
        }
        if let Some(h6) = volume.h6 {
            log(LogTag::System, "TOKEN_INFO", &format!("  6h Volume: ${:.2}", h6));
        }
        if let Some(h1) = volume.h1 {
            log(LogTag::System, "TOKEN_INFO", &format!("  1h Volume: ${:.2}", h1));
        }
    }

    // Display transaction statistics
    if let Some(ref txns) = token.txns {
        log(LogTag::System, "TOKEN_INFO", "Transaction Statistics:");
        if let Some(ref h24) = txns.h24 {
            if let (Some(buys), Some(sells)) = (h24.buys, h24.sells) {
                log(
                    LogTag::System,
                    "TOKEN_INFO",
                    &format!("  24h: {} buys, {} sells", buys, sells)
                );
            }
        }
        if let Some(ref h6) = txns.h6 {
            if let (Some(buys), Some(sells)) = (h6.buys, h6.sells) {
                log(LogTag::System, "TOKEN_INFO", &format!("  6h: {} buys, {} sells", buys, sells));
            }
        }
    }

    // Display price change statistics
    if let Some(ref price_change) = token.price_change {
        log(LogTag::System, "TOKEN_INFO", "Price Changes:");
        if let Some(h24) = price_change.h24 {
            log(LogTag::System, "TOKEN_INFO", &format!("  24h: {:.2}%", h24));
        }
        if let Some(h6) = price_change.h6 {
            log(LogTag::System, "TOKEN_INFO", &format!("  6h: {:.2}%", h6));
        }
        if let Some(h1) = price_change.h1 {
            log(LogTag::System, "TOKEN_INFO", &format!("  1h: {:.2}%", h1));
        }
    }

    // Display liquidity information
    if let Some(ref liquidity) = token.liquidity {
        log(LogTag::System, "TOKEN_INFO", "Liquidity:");
        if let Some(usd) = liquidity.usd {
            log(LogTag::System, "TOKEN_INFO", &format!("  USD: ${:.2}", usd));
        }
        if let Some(base) = liquidity.base {
            log(LogTag::System, "TOKEN_INFO", &format!("  Base: {:.6}", base));
        }
        if let Some(quote) = liquidity.quote {
            log(LogTag::System, "TOKEN_INFO", &format!("  Quote: {:.6}", quote));
        }
    }

    // Display timestamps
    if let Some(created_at) = token.created_at {
        log(LogTag::System, "TOKEN_INFO", &format!("Created At: {}", created_at));
    }

    // Display DEX information
    if let Some(ref dex_id) = token.dex_id {
        log(LogTag::System, "TOKEN_INFO", &format!("DEX: {}", dex_id));
    }

    if let Some(ref pair_address) = token.pair_address {
        log(LogTag::System, "TOKEN_INFO", &format!("Pair Address: {}", pair_address));
    }

    if let Some(ref pair_url) = token.pair_url {
        log(LogTag::System, "TOKEN_INFO", &format!("Pair URL: {}", pair_url));
    }

    // Display flags and metadata
    log(LogTag::System, "TOKEN_INFO", &format!("Is Verified: {}", token.is_verified));

    if !token.labels.is_empty() {
        log(LogTag::System, "TOKEN_INFO", &format!("Labels: {}", token.labels.join(", ")));
    }

    if !token.tags.is_empty() {
        log(LogTag::System, "TOKEN_INFO", &format!("Tags: {}", token.tags.join(", ")));
    }

    // Display additional metadata if available
    if let Some(ref description) = token.description {
        if !description.is_empty() {
            log(LogTag::System, "TOKEN_INFO", &format!("Description: {}", description));
        }
    }

    if let Some(ref website) = token.website {
        if !website.is_empty() {
            log(LogTag::System, "TOKEN_INFO", &format!("Website: {}", website));
        }
    }

    if let Some(ref logo_url) = token.logo_url {
        if !logo_url.is_empty() {
            log(LogTag::System, "TOKEN_INFO", &format!("Logo URL: {}", logo_url));
        }
    }

    if let Some(ref coingecko_id) = token.coingecko_id {
        if !coingecko_id.is_empty() {
            log(LogTag::System, "TOKEN_INFO", &format!("CoinGecko ID: {}", coingecko_id));
        }
    }

    // Display social information if available
    if let Some(ref info) = token.info {
        if !info.socials.is_empty() {
            log(LogTag::System, "TOKEN_INFO", "Social Links:");
            for social in &info.socials {
                log(
                    LogTag::System,
                    "TOKEN_INFO",
                    &format!("  {}: {}", social.link_type, social.url)
                );
            }
        }
        if !info.websites.is_empty() {
            log(LogTag::System, "TOKEN_INFO", "Websites:");
            for website in &info.websites {
                let label = website.label.as_deref().unwrap_or("Website");
                log(LogTag::System, "TOKEN_INFO", &format!("  {}: {}", label, website.url));
            }
        }
    }

    log(LogTag::System, "TOKEN_INFO", "=== END TOKEN INFORMATION ===");
}

/// Find token mint address(es) by symbol
async fn find_mint_by_symbol(symbol: &str) {
    log(LogTag::System, "SYMBOL_SEARCH", &format!("Searching for tokens with symbol: {}", symbol));

    // Initialize the tokens system
    log(LogTag::System, "INIT", "Initializing tokens system for database lookup...");

    match screenerbot::tokens::initialize_tokens_system().await {
        Ok(_) => {
            log(LogTag::System, "SUCCESS", "Tokens system initialized successfully");
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to initialize tokens system: {}", e));
            return;
        }
    }

    // Get all tokens from database and filter by symbol
    match screenerbot::tokens::get_all_tokens_by_liquidity().await {
        Ok(tokens) => {
            let matching_tokens: Vec<_> = tokens
                .iter()
                .filter(|token| token.symbol.to_lowercase() == symbol.to_lowercase())
                .collect();

            if matching_tokens.is_empty() {
                log(LogTag::System, "WARN", &format!("No tokens found with symbol: {}", symbol));
                log(
                    LogTag::System,
                    "INFO",
                    "Try fetching recent transactions to update the token database"
                );

                // Show similar symbols as suggestions
                let similar_tokens: Vec<_> = tokens
                    .iter()
                    .filter(
                        |token|
                            token.symbol.to_lowercase().contains(&symbol.to_lowercase()) ||
                            symbol.to_lowercase().contains(&token.symbol.to_lowercase())
                    )
                    .take(5)
                    .collect();

                if !similar_tokens.is_empty() {
                    log(LogTag::System, "SUGGESTION", "Similar symbols found:");
                    for token in similar_tokens {
                        log(
                            LogTag::System,
                            "SUGGESTION",
                            &format!("  {} ({}): {}", token.symbol, &token.mint, &token.name)
                        );
                    }
                }
            } else {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Found {} token(s) with symbol '{}':", matching_tokens.len(), symbol)
                );

                for (index, token) in matching_tokens.iter().enumerate() {
                    log(LogTag::System, "RESULT", &format!("=== MATCH {} ===", index + 1));
                    log(LogTag::System, "RESULT", &format!("Symbol: {}", token.symbol));
                    log(LogTag::System, "RESULT", &format!("Mint: {}", token.mint));
                    log(LogTag::System, "RESULT", &format!("Name: {}", &token.name));

                    if let Some(liquidity) = &token.liquidity {
                        if let Some(usd) = liquidity.usd {
                            log(LogTag::System, "RESULT", &format!("Liquidity: ${:.2}", usd));
                        }
                    }

                    log(LogTag::System, "RESULT", &format!("Price USD: ${:.9}", token.price_usd));

                    if let Some(price_sol) = token.price_sol {
                        log(LogTag::System, "RESULT", &format!("Price SOL: {:.12}", price_sol));
                    }

                    log(
                        LogTag::System,
                        "RESULT",
                        &format!("Pair Address: {}", &token.pair_address)
                    );

                    log(LogTag::System, "RESULT", "");
                }

                // If multiple matches, show a summary
                if matching_tokens.len() > 1 {
                    log(LogTag::System, "SUMMARY", "Quick mint reference:");
                    for (index, token) in matching_tokens.iter().enumerate() {
                        log(LogTag::System, "SUMMARY", &format!("  {}: {}", index + 1, token.mint));
                    }
                }
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to search token database: {}", e));
        }
    }
}

/// Check comprehensive wallet balance including SOL, tokens, and ATA accounts
async fn check_wallet_balance_comprehensive(wallet_pubkey: Pubkey) {
    const ATA_RENT_COST_SOL: f64 = 0.00203928; // Standard ATA creation/closure cost

    let wallet_address = wallet_pubkey.to_string();
    log(
        LogTag::System,
        "BALANCE_CHECK",
        &format!("Starting comprehensive balance check for wallet: {}", &wallet_address[..8])
    );

    // Check SOL balance first
    match screenerbot::utils::get_sol_balance(&wallet_address).await {
        Ok(sol_balance) => {
            log(
                LogTag::System,
                "BALANCE_CHECK",
                &format!("💰 SOL Balance: {:.6} SOL ({:.2} USD)", sol_balance, sol_balance * 200.0)
            );
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get SOL balance: {}", e));
            return;
        }
    }

    // Get all token accounts
    log(LogTag::System, "BALANCE_CHECK", "Fetching all token accounts...");
    match screenerbot::utils::get_all_token_accounts(&wallet_address).await {
        Ok(token_accounts) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Found {} token accounts", token_accounts.len())
            );

            if token_accounts.is_empty() {
                log(
                    LogTag::System,
                    "BALANCE_CHECK",
                    "No token accounts found - wallet only holds SOL"
                );
                return;
            }

            analyze_token_accounts(&token_accounts, ATA_RENT_COST_SOL).await;
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token accounts: {}", e));
        }
    }
}

/// Analyze token accounts and categorize them
async fn analyze_token_accounts(
    token_accounts: &[screenerbot::rpc::TokenAccountInfo],
    ata_rent_cost_sol: f64
) {
    log(LogTag::System, "BALANCE_CHECK", "=== TOKEN ACCOUNT ANALYSIS ===");

    let mut empty_accounts = Vec::new();
    let mut non_empty_accounts = Vec::new();
    let mut total_rent_reclaimable = 0.0;
    let mut total_token_value_sol = 0.0;
    let mut token_type_counts = std::collections::HashMap::new();

    for account in token_accounts {
        // Count token types
        let token_type = if account.is_token_2022 { "Token-2022" } else { "SPL Token" };
        *token_type_counts.entry(token_type).or_insert(0) += 1;

        if account.balance == 0 {
            empty_accounts.push(account);
            total_rent_reclaimable += ata_rent_cost_sol;
        } else {
            non_empty_accounts.push(account);
        }
    }

    // Display summary statistics
    log(LogTag::System, "BALANCE_CHECK", "ACCOUNT SUMMARY:");
    log(LogTag::System, "BALANCE_CHECK", &format!("  Total Accounts: {}", token_accounts.len()));
    log(LogTag::System, "BALANCE_CHECK", &format!("  Non-Empty: {}", non_empty_accounts.len()));
    log(LogTag::System, "BALANCE_CHECK", &format!("  Empty: {}", empty_accounts.len()));

    for (token_type, count) in &token_type_counts {
        log(LogTag::System, "BALANCE_CHECK", &format!("  {}: {}", token_type, count));
    }

    if !empty_accounts.is_empty() {
        log(
            LogTag::System,
            "BALANCE_CHECK",
            &format!(
                "  💸 Reclaimable Rent: {:.6} SOL (~${:.2})",
                total_rent_reclaimable,
                total_rent_reclaimable * 200.0
            )
        );
    }

    // Display non-empty accounts with token info
    if !non_empty_accounts.is_empty() {
        log(LogTag::System, "BALANCE_CHECK", "");
        log(LogTag::System, "BALANCE_CHECK", "=== NON-EMPTY TOKEN ACCOUNTS ===");
        display_non_empty_token_accounts(&non_empty_accounts).await;
    }

    // Display empty accounts
    if !empty_accounts.is_empty() {
        log(LogTag::System, "BALANCE_CHECK", "");
        log(LogTag::System, "BALANCE_CHECK", "=== EMPTY TOKEN ACCOUNTS (Reclaimable) ===");
        display_empty_token_accounts(&empty_accounts, ata_rent_cost_sol);
    }

    // Display actionable recommendations
    display_balance_recommendations(&non_empty_accounts, &empty_accounts, total_rent_reclaimable);
}

/// Display detailed information about non-empty token accounts
async fn display_non_empty_token_accounts(accounts: &[&screenerbot::rpc::TokenAccountInfo]) {
    log(
        LogTag::System,
        "BALANCE_CHECK",
        "Account              Token                Symbol      Raw Balance    UI Balance       Price (SOL)    Value (SOL)    Type"
    );
    log(
        LogTag::System,
        "BALANCE_CHECK",
        "-------------------------------------------------------------------------------------------------------------------------------"
    );

    let mut total_portfolio_value_sol = 0.0;

    for account in accounts {
        // Get token decimals
        let decimals = screenerbot::tokens::get_token_decimals_sync(&account.mint).unwrap_or(9);
        let ui_balance = (account.balance as f64) / (10_f64).powi(decimals as i32);

        // Try to get token info from database
        let (symbol, price_sol) = if
            let Some(token) = screenerbot::tokens::get_token_from_db(&account.mint).await
        {
            let price = token.price_dexscreener_sol.or(token.price_pool_sol).unwrap_or(0.0);
            (token.symbol, price)
        } else {
            ("UNKNOWN".to_string(), 0.0)
        };

        let value_sol = ui_balance * price_sol;
        total_portfolio_value_sol += value_sol;
        let token_type = if account.is_token_2022 { "Token-2022" } else { "SPL" };

        log(
            LogTag::System,
            "BALANCE_CHECK",
            &format!(
                "{}  {}  {:>10}  {:>13}  {:>13.6}  {:>12.9}  {:>12.6}  {}",
                &account.account[..8],
                &account.mint[..8],
                if symbol.len() > 10 {
                    &symbol[..10]
                } else {
                    &symbol
                },
                format_large_number(account.balance),
                ui_balance,
                price_sol,
                value_sol,
                token_type
            )
        );
    }

    log(
        LogTag::System,
        "BALANCE_CHECK",
        "-------------------------------------------------------------------------------------------------------------------------------"
    );
    log(
        LogTag::System,
        "BALANCE_CHECK",
        &format!(
            "💎 Total Token Portfolio Value: {:.6} SOL (~${:.2})",
            total_portfolio_value_sol,
            total_portfolio_value_sol * 200.0
        )
    );
}

/// Display information about empty token accounts
fn display_empty_token_accounts(
    accounts: &[&screenerbot::rpc::TokenAccountInfo],
    ata_rent_cost_sol: f64
) {
    log(
        LogTag::System,
        "BALANCE_CHECK",
        "Account              Token                Type         Rent (SOL)"
    );
    log(
        LogTag::System,
        "BALANCE_CHECK",
        "------------------------------------------------------------------------"
    );

    for account in accounts {
        let token_type = if account.is_token_2022 { "Token-2022" } else { "SPL Token" };

        log(
            LogTag::System,
            "BALANCE_CHECK",
            &format!(
                "{}  {}  {:>11}  {:>10.6}",
                &account.account[..8],
                &account.mint[..8],
                token_type,
                ata_rent_cost_sol
            )
        );
    }

    log(
        LogTag::System,
        "BALANCE_CHECK",
        "------------------------------------------------------------------------"
    );
}

/// Display actionable recommendations based on balance analysis
fn display_balance_recommendations(
    non_empty_accounts: &[&screenerbot::rpc::TokenAccountInfo],
    empty_accounts: &[&screenerbot::rpc::TokenAccountInfo],
    total_rent_reclaimable: f64
) {
    log(LogTag::System, "BALANCE_CHECK", "");
    log(LogTag::System, "BALANCE_CHECK", "=== RECOMMENDATIONS ===");

    if !empty_accounts.is_empty() {
        log(
            LogTag::System,
            "BALANCE_CHECK",
            &format!(
                "💡 Empty ATA Cleanup: You can reclaim {:.6} SOL by closing {} empty accounts",
                total_rent_reclaimable,
                empty_accounts.len()
            )
        );
        log(
            LogTag::System,
            "BALANCE_CHECK",
            "   Run: cargo run --bin main_ata_cleanup -- --wallet-from-config --dry-run"
        );
        log(
            LogTag::System,
            "BALANCE_CHECK",
            "   Then: cargo run --bin main_ata_cleanup -- --wallet-from-config (for real cleanup)"
        );
    }

    if !non_empty_accounts.is_empty() {
        log(
            LogTag::System,
            "BALANCE_CHECK",
            &format!(
                "🔍 Token Holdings: You have {} tokens with balances",
                non_empty_accounts.len()
            )
        );
        log(
            LogTag::System,
            "BALANCE_CHECK",
            "   Consider checking token prices and market conditions"
        );

        // Check for dust balances
        let mut dust_tokens = 0;
        for account in non_empty_accounts {
            let decimals = screenerbot::tokens::get_token_decimals_sync(&account.mint).unwrap_or(9);
            let ui_balance = (account.balance as f64) / (10_f64).powi(decimals as i32);
            if ui_balance < 0.001 {
                dust_tokens += 1;
            }
        }

        if dust_tokens > 0 {
            log(
                LogTag::System,
                "BALANCE_CHECK",
                &format!("   ⚠️  {} tokens have very small balances (dust) - consider if worth keeping", dust_tokens)
            );
        }
    }

    if non_empty_accounts.is_empty() && empty_accounts.is_empty() {
        log(LogTag::System, "BALANCE_CHECK", "✅ Clean wallet: Only holds SOL, no token accounts");
    }

    log(LogTag::System, "BALANCE_CHECK", "");
    log(
        LogTag::System,
        "BALANCE_CHECK",
        "💡 For detailed token analysis, use: --token-info <MINT_ADDRESS>"
    );
    log(LogTag::System, "BALANCE_CHECK", "💡 For transaction history, use: --analyze-swaps");
    log(LogTag::System, "BALANCE_CHECK", "=== END BALANCE CHECK ===");
}

/// Format large numbers with appropriate suffixes
fn format_large_number(num: u64) -> String {
    if num >= 1_000_000_000 {
        format!("{:.1}B", (num as f64) / 1_000_000_000.0)
    } else if num >= 1_000_000 {
        format!("{:.1}M", (num as f64) / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", (num as f64) / 1_000.0)
    } else {
        num.to_string()
    }
}

/// Helper function to safely get signature prefix for logging
fn get_signature_prefix(signature: &str) -> &str {
    if signature.len() >= 8 { &signature[..8] } else { signature }
}

/// Helper function to safely get mint address prefix for logging
fn get_mint_prefix(mint: &str) -> &str {
    if mint.len() >= 8 { &mint[..8] } else { mint }
}

/// Analyze transaction fees with comprehensive breakdown and statistics
async fn analyze_transaction_fees(
    wallet_pubkey: Pubkey,
    max_count: usize,
    filter_mint: Option<String>
) {
    log(LogTag::System, "FEE_ANALYSIS", "=== STARTING FEE ANALYSIS ===");
    log(
        LogTag::System,
        "FEE_ANALYSIS",
        &format!("Analyzing fees for up to {} transactions", max_count)
    );

    if let Some(ref mint) = filter_mint {
        log(
            LogTag::System,
            "FEE_ANALYSIS",
            &format!("Filtering to mint: {}", get_mint_prefix(mint))
        );
    }

    // Initialize transaction manager
    let manager_result = TransactionsManager::new(wallet_pubkey).await;
    let mut manager = match manager_result {
        Ok(m) => m,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to create transaction manager: {}", e));
            return;
        }
    };

    // Get all transactions
    let all_transactions_result = manager.get_recent_transactions(max_count).await;
    let all_transactions = match all_transactions_result {
        Ok(txns) => txns,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get transactions: {}", e));
            return;
        }
    };

    log(
        LogTag::System,
        "FEE_ANALYSIS",
        &format!("Retrieved {} transactions for analysis", all_transactions.len())
    );

    // Filter transactions by mint if specified
    let filtered_transactions: Vec<&Transaction> = if let Some(ref mint) = filter_mint {
        all_transactions
            .iter()
            .filter(|tx| transaction_involves_mint(tx, mint))
            .collect()
    } else {
        all_transactions.iter().collect()
    };

    if filtered_transactions.is_empty() {
        log(LogTag::System, "FEE_ANALYSIS", "No transactions found for analysis");
        return;
    }

    log(
        LogTag::System,
        "FEE_ANALYSIS",
        &format!("Analyzing fees for {} transactions", filtered_transactions.len())
    );

    // Initialize fee statistics
    let mut fee_stats = FeeStatistics::new();
    let mut transaction_type_fees: HashMap<String, FeeTypeStats> = HashMap::new();
    let mut monthly_fees: HashMap<String, f64> = HashMap::new();
    let mut expensive_transactions: Vec<(&Transaction, f64)> = Vec::new();

    // Process each transaction for fee analysis
    for transaction in filtered_transactions.iter() {
        let fee_amount = transaction.fee_sol;
        fee_stats.total_fees += fee_amount;
        fee_stats.transaction_count += 1;

        // Track by transaction type
        let tx_type = format!("{:?}", transaction.transaction_type);
        let type_stat = transaction_type_fees
            .entry(tx_type.clone())
            .or_insert_with(|| FeeTypeStats::new(tx_type));
        type_stat.total_fees += fee_amount;
        type_stat.transaction_count += 1;
        if fee_amount > type_stat.max_fee {
            type_stat.max_fee = fee_amount;
        }
        if type_stat.min_fee == 0.0 || fee_amount < type_stat.min_fee {
            type_stat.min_fee = fee_amount;
        }

        // Track monthly fees
        let month_key = transaction.timestamp.format("%Y-%m").to_string();
        *monthly_fees.entry(month_key).or_insert(0.0) += fee_amount;

        // Track expensive transactions (top 10)
        expensive_transactions.push((transaction, fee_amount));
        if expensive_transactions.len() > 50 {
            expensive_transactions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            expensive_transactions.truncate(10);
        }

        // Update min/max
        if fee_amount > fee_stats.max_fee {
            fee_stats.max_fee = fee_amount;
        }
        if fee_stats.min_fee == 0.0 || fee_amount < fee_stats.min_fee {
            fee_stats.min_fee = fee_amount;
        }
    }

    // Sort expensive transactions
    expensive_transactions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    expensive_transactions.truncate(10);

    // Calculate averages
    if fee_stats.transaction_count > 0 {
        fee_stats.average_fee = fee_stats.total_fees / (fee_stats.transaction_count as f64);
    }

    for type_stat in transaction_type_fees.values_mut() {
        if type_stat.transaction_count > 0 {
            type_stat.average_fee = type_stat.total_fees / (type_stat.transaction_count as f64);
        }
    }

    // Display comprehensive fee analysis
    display_fee_analysis_summary(&fee_stats);
    display_fee_by_transaction_type(&transaction_type_fees);
    display_monthly_fee_trends(&monthly_fees);
    display_expensive_transactions(&expensive_transactions);

    log(LogTag::System, "FEE_ANALYSIS", "=== FEE ANALYSIS COMPLETE ===");
}

/// Statistics structure for fee analysis
#[derive(Debug)]
struct FeeStatistics {
    total_fees: f64,
    transaction_count: usize,
    average_fee: f64,
    min_fee: f64,
    max_fee: f64,
}

impl FeeStatistics {
    fn new() -> Self {
        Self {
            total_fees: 0.0,
            transaction_count: 0,
            average_fee: 0.0,
            min_fee: 0.0,
            max_fee: 0.0,
        }
    }
}

/// Fee statistics by transaction type
#[derive(Debug)]
struct FeeTypeStats {
    transaction_type: String,
    total_fees: f64,
    transaction_count: usize,
    average_fee: f64,
    min_fee: f64,
    max_fee: f64,
}

impl FeeTypeStats {
    fn new(transaction_type: String) -> Self {
        Self {
            transaction_type,
            total_fees: 0.0,
            transaction_count: 0,
            average_fee: 0.0,
            min_fee: 0.0,
            max_fee: 0.0,
        }
    }
}

/// Display fee analysis summary
fn display_fee_analysis_summary(stats: &FeeStatistics) {
    log(LogTag::System, "FEE_ANALYSIS", "");
    log(LogTag::System, "FEE_ANALYSIS", "📊 === FEE ANALYSIS SUMMARY ===");
    log(
        LogTag::System,
        "FEE_ANALYSIS",
        &format!("Total Transactions: {}", stats.transaction_count)
    );
    log(LogTag::System, "FEE_ANALYSIS", &format!("Total Fees Paid: {:.9} SOL", stats.total_fees));
    log(LogTag::System, "FEE_ANALYSIS", &format!("Average Fee: {:.9} SOL", stats.average_fee));
    log(LogTag::System, "FEE_ANALYSIS", &format!("Minimum Fee: {:.9} SOL", stats.min_fee));
    log(LogTag::System, "FEE_ANALYSIS", &format!("Maximum Fee: {:.9} SOL", stats.max_fee));

    // Convert to USD estimate (assuming $200 SOL for context)
    let total_usd = stats.total_fees * 200.0;
    let avg_usd = stats.average_fee * 200.0;
    log(
        LogTag::System,
        "FEE_ANALYSIS",
        &format!("Estimated Total Cost: ${:.2} USD (at $200/SOL)", total_usd)
    );
    log(
        LogTag::System,
        "FEE_ANALYSIS",
        &format!("Estimated Avg Cost: ${:.4} USD per transaction", avg_usd)
    );
}

/// Display fees broken down by transaction type
fn display_fee_by_transaction_type(type_fees: &HashMap<String, FeeTypeStats>) {
    log(LogTag::System, "FEE_ANALYSIS", "");
    log(LogTag::System, "FEE_ANALYSIS", "💰 === FEES BY TRANSACTION TYPE ===");

    // Sort by total fees (highest first)
    let mut sorted_types: Vec<&FeeTypeStats> = type_fees.values().collect();
    sorted_types.sort_by(|a, b| b.total_fees.partial_cmp(&a.total_fees).unwrap());

    for type_stat in sorted_types {
        let percentage = if !type_fees.is_empty() {
            let total_all_fees: f64 = type_fees
                .values()
                .map(|s| s.total_fees)
                .sum();
            (type_stat.total_fees / total_all_fees) * 100.0
        } else {
            0.0
        };

        log(
            LogTag::System,
            "FEE_ANALYSIS",
            &format!(
                "  {} ({} txns): {:.9} SOL ({:.1}% of total) | Avg: {:.9} | Range: {:.9}-{:.9}",
                type_stat.transaction_type,
                type_stat.transaction_count,
                type_stat.total_fees,
                percentage,
                type_stat.average_fee,
                type_stat.min_fee,
                type_stat.max_fee
            )
        );
    }
}

/// Display monthly fee trends
fn display_monthly_fee_trends(monthly_fees: &HashMap<String, f64>) {
    log(LogTag::System, "FEE_ANALYSIS", "");
    log(LogTag::System, "FEE_ANALYSIS", "📅 === MONTHLY FEE TRENDS ===");

    // Sort by month
    let mut sorted_months: Vec<(&String, &f64)> = monthly_fees.iter().collect();
    sorted_months.sort_by(|a, b| a.0.cmp(b.0));

    for (month, total_fee) in sorted_months {
        let usd_estimate = total_fee * 200.0;
        log(
            LogTag::System,
            "FEE_ANALYSIS",
            &format!("  {}: {:.9} SOL (≈${:.2} USD)", month, total_fee, usd_estimate)
        );
    }
}

/// Display most expensive transactions by fees
fn display_expensive_transactions(expensive_txns: &[(&Transaction, f64)]) {
    log(LogTag::System, "FEE_ANALYSIS", "");
    log(LogTag::System, "FEE_ANALYSIS", "💸 === TOP 10 MOST EXPENSIVE TRANSACTIONS ===");

    for (i, (transaction, fee)) in expensive_txns.iter().enumerate() {
        let usd_estimate = fee * 200.0;
        let signature_prefix = get_signature_prefix(&transaction.signature);
        let tx_type = format!("{:?}", transaction.transaction_type);
        let success_indicator = if transaction.success { "✅" } else { "❌" };

        log(
            LogTag::System,
            "FEE_ANALYSIS",
            &format!(
                "  {}. {} {}...{} ({}) - {:.9} SOL (≈${:.4} USD) {}",
                i + 1,
                transaction.timestamp.format("%Y-%m-%d %H:%M"),
                signature_prefix,
                &transaction.signature[transaction.signature.len() - 4..],
                tx_type,
                fee,
                usd_estimate,
                success_indicator
            )
        );
    }
}

/// Display comprehensive ATA operations and SOL flow analysis
fn display_ata_and_sol_flow_analysis(swaps: &[screenerbot::transactions_types::SwapPnLInfo]) {
    if swaps.is_empty() {
        return;
    }

    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(LogTag::Transactions, "ATA_ANALYSIS", "🏦 === ATA OPERATIONS & SOL FLOW ANALYSIS ===");

    // Initialize tracking variables
    // Use EFFECTIVE amounts which already exclude ATA rent effects (but include fees in fee field)
    let mut total_effective_sol_spent = 0.0; // sum of effective_sol_spent for buys
    let mut total_effective_sol_received = 0.0; // sum of effective_sol_received for sells
    let mut total_fees = 0.0;
    // Informational only: ATA rent flows captured during transaction analysis
    let mut total_ata_rent_net = 0.0; // net across all swaps (can be +/-)
    let mut total_ata_rent_gross = 0.0; // sum of absolute rent flows (magnitude only)

    // Track per-token ATA operations
    let mut token_ata_ops: std::collections::HashMap<
        String,
        TokenATAOperations
    > = std::collections::HashMap::new();

    // Analyze each swap
    for (i, swap) in swaps.iter().enumerate() {
        let token_ops = token_ata_ops
            .entry(swap.token_symbol.clone())
            .or_insert(TokenATAOperations::new());

        // Track SOL flow using EFFECTIVE amounts
        total_fees += swap.fee_sol;
        if swap.swap_type == "Buy" || swap.swap_type.starts_with("Failed Buy") {
            total_effective_sol_spent += swap.effective_sol_spent.max(0.0);
            token_ops.buys += 1;
            token_ops.sol_spent += swap.effective_sol_spent.max(0.0);
        } else if swap.swap_type == "Sell" || swap.swap_type.starts_with("Failed Sell") {
            total_effective_sol_received += swap.effective_sol_received.max(0.0);
            token_ops.sells += 1;
            token_ops.sol_received += swap.effective_sol_received.max(0.0);
        }
        token_ops.fees += swap.fee_sol;
        // Track token-specific ATA create/close counts
        token_ops.ata_created += swap.ata_created_count;
        token_ops.ata_closed += swap.ata_closed_count;
        // Track ATA rent flows (informational only; NOT used in P&L to avoid double counting)
        total_ata_rent_net += swap.ata_rents;
        total_ata_rent_gross += swap.ata_rents.abs();
    }

    // Calculate net SOL flow using effective amounts only
    let net_sol_flow = total_effective_sol_received - total_effective_sol_spent - total_fees;
    let gross_sol_flow = total_effective_sol_received - total_effective_sol_spent;

    // Display overall SOL flow summary
    log(LogTag::Transactions, "ATA_ANALYSIS", "💰 === OVERALL SOL FLOW (Begin to End) ===");
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total SOL Spent:     -{:.9} SOL (buying tokens)", total_effective_sol_spent)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Total SOL Received:  +{:.9} SOL (selling tokens)", total_effective_sol_received)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Gross SOL Flow:      {:+.9} SOL (before fees)", gross_sol_flow)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Transaction Fees:    -{:.9} SOL", total_fees)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!(
            "ATA Rent Flow (info): {:+.9} SOL net, {:.9} SOL gross (infra; excluded from P&L)",
            total_ata_rent_net,
            total_ata_rent_gross
        )
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("NET SOL FLOW:        {:+.9} SOL (total profit/loss)", net_sol_flow)
    );

    // Display ATA operations summary
    // Optional: ATA operations summary omitted (heuristics were misleading); rely on per-transaction analysis logs.

    // Display per-token breakdown
    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(LogTag::Transactions, "ATA_ANALYSIS", "📊 === PER-TOKEN ATA & SOL ANALYSIS ===");
    for (token, ops) in &token_ata_ops {
        let net_sol_per_token = ops.sol_received - ops.sol_spent - ops.fees;
        // Do not adjust by heuristic ATA numbers; effective amounts already exclude ATA rent
        let final_net = net_sol_per_token;

        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!(
                "{}: {} buys ({:.6} SOL) | {} sells ({:.6} SOL) | {} ATA created | {} ATA closed | Net: {:+.6} SOL",
                token,
                ops.buys,
                ops.sol_spent,
                ops.sells,
                ops.sol_received,
                ops.ata_created,
                ops.ata_closed,
                final_net
            )
        );
    }

    // Display cost breakdown analysis
    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(LogTag::Transactions, "ATA_ANALYSIS", "💸 === COST BREAKDOWN ANALYSIS ===");
    let total_costs = total_fees; // Only trading fees count toward costs here; ATA rent is infra and excluded
    if total_costs > 0.0 {
        let fee_percentage = (total_fees / total_costs) * 100.0;

        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!(
                "Transaction Fees:    {:.9} SOL ({:.1}% of total costs)",
                total_fees,
                fee_percentage
            )
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total Trading Costs: {:.9} SOL", total_costs)
        );
        // Provide ATA rent separately as an informational line
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!(
                "ATA Rent (info):     net {:+.9} SOL, gross {:.9} SOL",
                total_ata_rent_net,
                total_ata_rent_gross
            )
        );
    }

    // Display efficiency metrics
    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(LogTag::Transactions, "ATA_ANALYSIS", "📈 === EFFICIENCY METRICS ===");
    let total_volume = total_effective_sol_spent + total_effective_sol_received;
    if total_volume > 0.0 {
        let cost_ratio = (total_costs / total_volume) * 100.0;
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Total Volume:        {:.6} SOL (spent + received)", total_volume)
        );
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("Cost Efficiency:     {:.3}% (costs as % of volume)", cost_ratio)
        );
    }

    if net_sol_flow > 0.0 {
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("🟢 PROFIT: +{:.6} SOL overall gain", net_sol_flow)
        );
    } else {
        log(
            LogTag::Transactions,
            "ATA_ANALYSIS",
            &format!("🔴 LOSS: {:.6} SOL overall loss", net_sol_flow)
        );
    }

    // Realized P&L (FIFO) — excludes open inventory to match user expectation
    // Compute across tokens using FIFO lots and per-swap effective amounts (fees handled per swap)
    #[derive(Clone, Debug)]
    struct BuyLot {
        qty: f64,
        remaining: f64,
        cost_sol: f64,
        fee_sol: f64,
    }

    // Sort swaps chronologically by slot, then timestamp as fallback
    let mut sorted = swaps.to_vec();
    sorted.sort_by(|a, b| {
        let sa = a.slot.unwrap_or(0);
        let sb = b.slot.unwrap_or(0);
        sa.cmp(&sb).then(a.timestamp.cmp(&b.timestamp))
    });

    use std::collections::{ HashMap, VecDeque };
    let mut books: HashMap<String, VecDeque<BuyLot>> = HashMap::new(); // mint -> FIFO buy lots
    let mut realized_spent = 0.0f64;
    let mut realized_received = 0.0f64;
    let mut realized_fees = 0.0f64;

    for s in &sorted {
        let mint = &s.token_mint;
        if s.swap_type == "Buy" {
            if s.token_amount > 0.0 && s.effective_sol_spent > 0.0 {
                books.entry(mint.clone()).or_default().push_back(BuyLot {
                    qty: s.token_amount,
                    remaining: s.token_amount,
                    cost_sol: s.effective_sol_spent,
                    fee_sol: s.fee_sol,
                });
            }
        } else if s.swap_type == "Sell" {
            let sell_qty = s.token_amount.abs();
            if sell_qty <= 0.0 || s.effective_sol_received <= 0.0 {
                continue;
            }
            realized_received += s.effective_sol_received;
            realized_fees += s.fee_sol; // full sell fee

            let mut qty_to_match = sell_qty;
            let lots = books.entry(mint.clone()).or_default();
            while qty_to_match > 0.0 {
                if let Some(front) = lots.front_mut() {
                    if front.remaining <= 0.0 {
                        lots.pop_front();
                        continue;
                    }
                    let take = front.remaining.min(qty_to_match);
                    let cost_per_token = if front.qty > 0.0 {
                        front.cost_sol / front.qty
                    } else {
                        0.0
                    };
                    realized_spent += cost_per_token * take;
                    // allocate proportional buy fee to realized
                    let fee_alloc = if front.qty > 0.0 {
                        front.fee_sol * (take / front.qty)
                    } else {
                        0.0
                    };
                    realized_fees += fee_alloc;
                    front.remaining -= take;
                    qty_to_match -= take;
                    if front.remaining <= 0.0 {
                        lots.pop_front();
                    }
                } else {
                    // No inventory to match; treat as oversold (no cost basis available)
                    break;
                }
            }
        }
    }

    let realized_net = realized_received - realized_spent - realized_fees;
    // Compute open inventory cost basis (unrealized) from remaining lots
    let mut open_inventory_cost = 0.0f64;
    for (_mint, lots) in &books {
        for lot in lots {
            if lot.remaining > 0.0 && lot.qty > 0.0 {
                let cost_per_token = lot.cost_sol / lot.qty;
                open_inventory_cost += cost_per_token * lot.remaining;
            }
        }
    }
    log(LogTag::Transactions, "ATA_ANALYSIS", "");
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        "🧮 === REALIZED P&L (FIFO, excludes open inventory) ==="
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Realized SOL Spent:    -{:.9} SOL", realized_spent)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Realized SOL Received: +{:.9} SOL", realized_received)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("Realized Fees:         -{:.9} SOL", realized_fees)
    );
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!("REALIZED NET P&L:      {:+.9} SOL", realized_net)
    );
    // Reconcile realized P&L with net SOL flow: net_sol_flow ≈ realized_net - open_inventory_cost
    log(
        LogTag::Transactions,
        "ATA_ANALYSIS",
        &format!(
            "Inventory Reconciliation: open inventory cost ≈ {:.9} SOL; realized + open ≈ {:+.9} SOL",
            open_inventory_cost,
            realized_net - open_inventory_cost
        )
    );

    log(LogTag::Transactions, "ATA_ANALYSIS", "=== END ATA & SOL FLOW ANALYSIS ===");
}

// Helper struct for tracking token ATA operations
#[derive(Debug, Clone)]
struct TokenATAOperations {
    buys: u32,
    sells: u32,
    sol_spent: f64,
    sol_received: f64,
    fees: f64,
    ata_created: u32,
    ata_closed: u32,
}

impl TokenATAOperations {
    fn new() -> Self {
        Self {
            buys: 0,
            sells: 0,
            sol_spent: 0.0,
            sol_received: 0.0,
            fees: 0.0,
            ata_created: 0,
            ata_closed: 0,
        }
    }
}

/// Display comprehensive deep analysis of a transaction with instruction-level details
/// Display comprehensive deep analysis of a transaction with instruction-level details
async fn display_simple_transaction_analysis(
    tx_details: &screenerbot::rpc::TransactionDetails,
    signature: &str
) {
    log(LogTag::Transactions, "DEEP_ANALYSIS", "");
    log(LogTag::Transactions, "DEEP_ANALYSIS", "🔍 === DEEP TRANSACTION ANALYSIS ===");
    log(LogTag::Transactions, "DEEP_ANALYSIS", &format!("Signature: {}", signature));

    // Basic transaction info
    log(LogTag::Transactions, "DEEP_ANALYSIS", "");
    log(LogTag::Transactions, "DEEP_ANALYSIS", "📋 === BASIC TRANSACTION INFO ===");
    log(LogTag::Transactions, "DEEP_ANALYSIS", &format!("Slot: {}", tx_details.slot));

    if let Some(ref meta) = tx_details.meta {
        let success = meta.err.is_none();
        log(
            LogTag::Transactions,
            "DEEP_ANALYSIS",
            &format!("Status: {}", if success { "✅ SUCCESS" } else { "❌ FAILED" })
        );

        if let Some(ref error) = meta.err {
            log(LogTag::Transactions, "DEEP_ANALYSIS", &format!("Error: {:?}", error));
        }

        // Fee analysis
        log(
            LogTag::Transactions,
            "DEEP_ANALYSIS",
            &format!("Fee: {} lamports ({:.9} SOL)", meta.fee, (meta.fee as f64) / 1_000_000_000.0)
        );

        // Account changes analysis
        log(LogTag::Transactions, "DEEP_ANALYSIS", "");
        log(LogTag::Transactions, "DEEP_ANALYSIS", "💰 === ACCOUNT CHANGES ANALYSIS ===");

        let pre_balances = &meta.pre_balances;
        let post_balances = &meta.post_balances;

        if pre_balances.len() == post_balances.len() {
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("Total Accounts: {}", pre_balances.len())
            );

            let mut total_net_change = 0i64;
            let mut meaningful_changes = 0;

            for i in 0..pre_balances.len() {
                let pre_balance = pre_balances[i];
                let post_balance = post_balances[i];
                let net_change = (post_balance as i64) - (pre_balance as i64);

                if net_change != 0 {
                    meaningful_changes += 1;
                    total_net_change += net_change;

                    let change_str = if net_change > 0 {
                        format!(
                            "+{} lamports (+{:.9} SOL)",
                            net_change,
                            (net_change as f64) / 1_000_000_000.0
                        )
                    } else {
                        format!(
                            "{} lamports ({:.9} SOL)",
                            net_change,
                            (net_change as f64) / 1_000_000_000.0
                        )
                    };

                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!(
                            "  Account[{}] {} → {} | {}",
                            i,
                            pre_balance,
                            post_balance,
                            change_str
                        )
                    );
                }
            }

            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("Accounts with balance changes: {}", meaningful_changes)
            );
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!(
                    "Total Net SOL Change: {} lamports ({:.9} SOL)",
                    total_net_change,
                    (total_net_change as f64) / 1_000_000_000.0
                )
            );
        }

        // Token changes analysis (Enhanced with pre/post comparison)
        if let Some(ref pre_token_balances) = meta.pre_token_balances {
            if let Some(ref post_token_balances) = meta.post_token_balances {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "");
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    "🪙 === TOKEN BALANCE CHANGES (COMPREHENSIVE) ==="
                );

                // Create a map of account_index -> pre_balance for easy lookup
                use std::collections::HashMap;
                let mut pre_balance_map: HashMap<u32, &TokenBalance> = HashMap::new();
                for pre_balance in pre_token_balances {
                    pre_balance_map.insert(pre_balance.account_index, pre_balance);
                }

                let mut meaningful_changes = 0;

                // Analyze all post-transaction token balances
                for post_balance in post_token_balances {
                    let account_index = post_balance.account_index;
                    let mint_short = if post_balance.mint.len() > 8 {
                        &post_balance.mint[..8]
                    } else {
                        &post_balance.mint
                    };

                    if let Some(pre_balance) = pre_balance_map.get(&account_index) {
                        // Compare pre and post balances
                        let pre_amount = pre_balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        let post_amount = post_balance.ui_token_amount.ui_amount.unwrap_or(0.0);
                        let change = post_amount - pre_amount;

                        if change.abs() > 0.000001 {
                            meaningful_changes += 1;
                            let change_str = if change > 0.0 {
                                format!("+{:.6}", change)
                            } else {
                                format!("{:.6}", change)
                            };

                            log(
                                LogTag::Transactions,
                                "DEEP_ANALYSIS",
                                &format!(
                                    "  Account[{}] | {}... | {:.6} → {:.6} | Change: {} tokens (decimals: {})",
                                    account_index,
                                    mint_short,
                                    pre_amount,
                                    post_amount,
                                    change_str,
                                    post_balance.ui_token_amount.decimals
                                )
                            );

                            // Show owner if available
                            if let Some(ref owner) = post_balance.owner {
                                let owner_short = if owner.len() > 8 { &owner[..8] } else { owner };
                                log(
                                    LogTag::Transactions,
                                    "DEEP_ANALYSIS",
                                    &format!("    Owner: {}...", owner_short)
                                );
                            }
                        }
                    } else {
                        // New token account (no pre-balance)
                        if let Some(post_amount) = post_balance.ui_token_amount.ui_amount {
                            if post_amount.abs() > 0.000001 {
                                meaningful_changes += 1;
                                log(
                                    LogTag::Transactions,
                                    "DEEP_ANALYSIS",
                                    &format!(
                                        "  Account[{}] | {}... | NEW ACCOUNT → {:.6} tokens (decimals: {})",
                                        account_index,
                                        mint_short,
                                        post_amount,
                                        post_balance.ui_token_amount.decimals
                                    )
                                );
                            }
                        }
                    }
                }

                // Check for closed token accounts (in pre but not in post)
                for pre_balance in pre_token_balances {
                    let account_found_in_post = post_token_balances
                        .iter()
                        .any(|post| post.account_index == pre_balance.account_index);

                    if !account_found_in_post {
                        if let Some(pre_amount) = pre_balance.ui_token_amount.ui_amount {
                            if pre_amount.abs() > 0.000001 {
                                meaningful_changes += 1;
                                let mint_short = if pre_balance.mint.len() > 8 {
                                    &pre_balance.mint[..8]
                                } else {
                                    &pre_balance.mint
                                };
                                log(
                                    LogTag::Transactions,
                                    "DEEP_ANALYSIS",
                                    &format!(
                                        "  Account[{}] | {}... | {:.6} → CLOSED (-{:.6} tokens)",
                                        pre_balance.account_index,
                                        mint_short,
                                        pre_amount,
                                        pre_amount
                                    )
                                );
                            }
                        }
                    }
                }

                if meaningful_changes == 0 {
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        "  No significant token balance changes found"
                    );
                } else {
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("  Total meaningful token changes: {}", meaningful_changes)
                    );
                }
            } else {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "");
                log(LogTag::Transactions, "DEEP_ANALYSIS", "🪙 === TOKEN BALANCE CHANGES ===");
                log(LogTag::Transactions, "DEEP_ANALYSIS", "  No post-token balances available");
            }
        } else {
            log(LogTag::Transactions, "DEEP_ANALYSIS", "");
            log(LogTag::Transactions, "DEEP_ANALYSIS", "🪙 === TOKEN BALANCE CHANGES ===");
            log(LogTag::Transactions, "DEEP_ANALYSIS", "  No token balance changes detected");
        }

        // Program logs analysis (ALL logs, not just first 10)
        if let Some(ref log_messages) = meta.log_messages {
            if !log_messages.is_empty() {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "");
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    "📝 === PROGRAM LOGS ANALYSIS (ALL LOGS) ==="
                );
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    &format!("Total Log Messages: {}", log_messages.len())
                );

                for (i, log_msg) in log_messages.iter().enumerate() {
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("  Log #{}: {}", i + 1, log_msg)
                    );
                }
            }
        }

        // Inner instructions analysis
        if let Some(ref inner_instructions) = meta.inner_instructions {
            if !inner_instructions.is_empty() {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "");
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    "🔧 === INNER INSTRUCTIONS ANALYSIS ==="
                );
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    &format!("Total Inner Instruction Groups: {}", inner_instructions.len())
                );

                for (group_idx, inner_group) in inner_instructions.iter().enumerate() {
                    if let Some(index) = inner_group.get("index") {
                        log(
                            LogTag::Transactions,
                            "DEEP_ANALYSIS",
                            &format!("  Group #{} (Instruction Index: {})", group_idx + 1, index)
                        );
                    }

                    if let Some(instructions) = inner_group.get("instructions") {
                        if let Some(instructions_array) = instructions.as_array() {
                            for (inner_idx, instruction) in instructions_array.iter().enumerate() {
                                log(
                                    LogTag::Transactions,
                                    "DEEP_ANALYSIS",
                                    &format!("    Inner #{}: {}", inner_idx + 1, instruction)
                                );
                            }
                        }
                    }
                }
            }
        }
    } else {
        log(LogTag::Transactions, "DEEP_ANALYSIS", "❌ No transaction metadata available");
    }

    // Main transaction instructions analysis
    log(LogTag::Transactions, "DEEP_ANALYSIS", "");
    log(LogTag::Transactions, "DEEP_ANALYSIS", "⚙️ === MAIN TRANSACTION INSTRUCTIONS ===");

    // Parse instructions from transaction message
    if let Some(message) = tx_details.transaction.message.as_object() {
        if let Some(instructions) = message.get("instructions") {
            if let Some(instructions_array) = instructions.as_array() {
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    &format!("Total Instructions: {}", instructions_array.len())
                );

                for (i, instruction) in instructions_array.iter().enumerate() {
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("  Instruction #{}: {}", i + 1, instruction)
                    );
                }
            } else {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "  Instructions field is not an array");
            }
        } else {
            log(LogTag::Transactions, "DEEP_ANALYSIS", "  No instructions field found in message");
        }

        // Account keys analysis
        if let Some(account_keys) = message.get("accountKeys") {
            if let Some(keys_array) = account_keys.as_array() {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "");
                log(LogTag::Transactions, "DEEP_ANALYSIS", "🔑 === ACCOUNT KEYS ===");
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    &format!("Total Account Keys: {}", keys_array.len())
                );

                for (i, key) in keys_array.iter().enumerate() {
                    if let Some(key_str) = key.as_str() {
                        let key_short = if key_str.len() > 8 { &key_str[..8] } else { key_str };
                        log(
                            LogTag::Transactions,
                            "DEEP_ANALYSIS",
                            &format!("  Account[{}]: {}...", i, key_short)
                        );
                    }
                }
            }
        }

        // Recent block hash
        if let Some(recent_blockhash) = message.get("recentBlockhash") {
            log(LogTag::Transactions, "DEEP_ANALYSIS", "");
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("Recent Blockhash: {}", recent_blockhash)
            );
        }
    } else {
        log(
            LogTag::Transactions,
            "DEEP_ANALYSIS",
            "  Transaction message is not a valid JSON object"
        );
    }

    // Our internal analysis
    log(LogTag::Transactions, "DEEP_ANALYSIS", "");
    log(LogTag::Transactions, "DEEP_ANALYSIS", "🤖 === SCREENERBOT INTERNAL ANALYSIS ===");

    // Try to get our internal transaction analysis
    match get_transaction(signature).await {
        Ok(Some(internal_tx)) => {
            log(LogTag::Transactions, "DEEP_ANALYSIS", "✅ Found in ScreenerBot database");
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("Type: {:?}", internal_tx.transaction_type)
            );
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("Status: {}", if internal_tx.success { "✅ SUCCESS" } else { "❌ FAILED" })
            );
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("Direction: {:?}", internal_tx.direction)
            );

            if internal_tx.sol_balance_change != 0.0 {
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    &format!("SOL Balance Change: {:.9} SOL", internal_tx.sol_balance_change)
                );
            }

            if !internal_tx.token_transfers.is_empty() {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "Token Transfers Detected:");
                for transfer in &internal_tx.token_transfers {
                    let mint_short = if transfer.mint.len() > 8 {
                        &transfer.mint[..8]
                    } else {
                        &transfer.mint
                    };
                    log(
                        LogTag::Transactions,
                        "DEEP_ANALYSIS",
                        &format!("  {}...: {} tokens", mint_short, transfer.amount)
                    );
                }
            }

            if let Some(ref ata_analysis) = internal_tx.ata_analysis {
                log(LogTag::Transactions, "DEEP_ANALYSIS", "ATA Operations Detected:");
                log(
                    LogTag::Transactions,
                    "DEEP_ANALYSIS",
                    &format!(
                        "  Created: {}, Closed: {}, Net Cost: {:.9} SOL",
                        ata_analysis.total_ata_creations,
                        ata_analysis.total_ata_closures,
                        ata_analysis.net_rent_impact
                    )
                );
            }
        }
        Ok(None) => {
            log(LogTag::Transactions, "DEEP_ANALYSIS", "❌ Not found in ScreenerBot database");
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                "   This transaction may not involve our wallet or hasn't been processed yet"
            );
        }
        Err(e) => {
            log(
                LogTag::Transactions,
                "DEEP_ANALYSIS",
                &format!("⚠️ Error checking internal database: {}", e)
            );
        }
    }

    log(LogTag::Transactions, "DEEP_ANALYSIS", "");
    log(LogTag::Transactions, "DEEP_ANALYSIS", "=== END DEEP TRANSACTION ANALYSIS ===");
}
