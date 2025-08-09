#![allow(warnings)]

use screenerbot::{
    utils::{ get_sol_balance, get_token_balance, get_wallet_address },
    swaps::{ buy_token, sell_token },
    rpc::lamports_to_sol,
    tokens::{
        types::Token,
        price::{get_token_price_safe, initialize_price_service},
        decimals::get_token_decimals_from_chain,
        api::{ init_dexscreener_api, get_token_from_mint_global_api },
        pool::{ init_pool_service, get_pool_service },
    },
    swaps::{
        gmgn::get_gmgn_quote,
        jupiter::get_jupiter_quote,
        config::{SOL_MINT},
        transaction::TransactionMonitoringService,
    },
    global::{ set_cmd_args },
    logger::{ log, LogTag, init_file_logging },
    rpc::init_rpc_client,
};

use std::env;
use tokio;

/// Simple formatters to avoid printing `Some(..)`/`None` in logs
fn fmt_opt_u64(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => "N/A".to_string(),
    }
}

fn fmt_opt_f64(v: Option<f64>, precision: usize) -> String {
    match v {
        Some(x) => format!("{x:.prec$}", x = x, prec = precision),
        None => "N/A".to_string(),
    }
}

fn fmt_opt_signature(sig: &Option<String>) -> String {
    match sig {
        Some(s) => s.clone(),
        None => "N/A".to_string(),
    }
}

/// Print comprehensive help menu for the Enhanced Debug Swap Tool
fn print_help() {
    println!("🚀 Enhanced Debug Swap Tool");
    println!("=====================================");
    println!("Advanced testing and debugging tool for swap operations with multi-token");
    println!("testing, comprehensive price analysis, and ATA rent calculation validation.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_debug_swap_enhanced -- <COMMAND> [OPTIONS]");
    println!("");
    println!("COMMANDS:");
    println!("    test-all               Run comprehensive tests on all predefined tokens");
    println!("    <TOKEN_MINT>           Test specific token mint address");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h             Show this help message");
    println!("    --dry-run              API testing only - no actual swaps executed");
    println!("    --debug-swap           Enable detailed swap operation logging");
    println!("    --debug-wallet         Enable detailed wallet balance tracking");
    println!("");
    println!("EXAMPLES:");
    println!("    # Test all predefined tokens with multiple amounts");
    println!("    cargo run --bin tool_debug_swap_enhanced -- test-all");
    println!("");
    println!("    # Test specific token (Bonk)");
    println!("    cargo run --bin tool_debug_swap_enhanced -- DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263");
    println!("");
    println!("    # API testing only (no actual swaps)");
    println!("    cargo run --bin tool_debug_swap_enhanced -- DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 --dry-run");
    println!("");
    println!("    # Full debug mode with detailed logging");
    println!("    cargo run --bin tool_debug_swap_enhanced -- test-all --debug-swap --debug-wallet");
    println!("");
    println!("PREDEFINED TEST TOKENS:");
    println!("    • DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 (Bonk)");
    println!("    • EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm (dogwifhat)");
    println!("    • 6p6xGHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN");
    println!("    • pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H8Dfn");
    println!("");
    println!("TEST AMOUNTS: 0.001, 0.002, 0.003 SOL");
    println!("");
    println!("TESTING WORKFLOW:");
    println!("    1. Validates token metadata and price data from multiple sources");
    println!("    2. Compares API prices with DexScreener data");
    println!("    3. Records initial wallet balances (SOL + tokens)");
    println!("    4. Executes buy transaction with test amount");
    println!("    5. Analyzes effective price calculations and ATA handling");
    println!("    6. Validates post-buy balances and token acquisition");
    println!("    7. Executes sell transaction with acquired tokens");
    println!("    8. Analyzes sell effective price and ATA rent recovery");
    println!("    9. Compares final vs initial balances with detailed metrics");
    println!("");
    println!("SAFETY FEATURES:");
    println!("    • Small test amounts to minimize risk");
    println!("    • {}% maximum slippage protection", MAX_PRICE_SLIPPAGE);
    println!("    • Comprehensive balance validation at each step");
    println!("    • Multi-source price validation");
    println!("    • Automatic ATA detection and rent calculation");
    println!("    • Transaction failure analysis and recovery");
    println!("");
    println!("PRICE ANALYSIS:");
    println!("    • Compare API vs DexScreener prices");
    println!("    • Effective price calculation validation");
    println!("    • Price impact analysis");
    println!("    • Slippage tolerance verification");
    println!("    • ATA rent impact on effective prices");
    println!("");
}

/// Test configuration - FOCUSED ON SINGLE TOKEN DEBUGGING
const TEST_SOL_AMOUNTS: [f64; 1] = [0.001]; // Single small test amount for debugging
const MAX_PRICE_SLIPPAGE: f64 = 10.0; // 10% maximum acceptable slippage

/// SINGLE Test token for debugging price calculation issues  
const TEST_TOKENS: [&str; 1] = [
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // Bonk - most liquid and stable for testing
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_file_logging();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        if args.len() < 2 {
            std::process::exit(1);
        } else {
            std::process::exit(0);
        }
    }

    // Set up debug flags for global access
    set_cmd_args(args.clone());

    // Check for special test modes
    if args.len() >= 2 && args[1] == "test-all" {
        // Run comprehensive tests on all predefined tokens
        return run_comprehensive_tests().await;
    }
    
    // Check for test-mode argument
    let mut test_mode_comprehensive = false;
    let mut dry_run_mode = false;
    let mut token_mint_arg = None;
    
    for arg in &args[1..] {
        if arg.starts_with("--test-mode=comprehensive") {
            test_mode_comprehensive = true;
        } else if arg == "--dry-run" {
            dry_run_mode = true;
        } else if !arg.starts_with("--") {
            token_mint_arg = Some(arg);
        }
    }
    
    if test_mode_comprehensive {
        // Run comprehensive tests on all predefined tokens
        return run_comprehensive_tests().await;
    }
    
    let token_mint = token_mint_arg.ok_or("No token mint provided. Use --help for usage information.")?;

    log(LogTag::System, "INFO", "🚀 Starting enhanced swap debug tool");
    log(LogTag::System, "INFO", &format!("Target token mint: {}", token_mint));
    
    if dry_run_mode {
        log(LogTag::System, "INFO", "🔍 DRY RUN MODE: API testing only - no actual swaps will be executed");
    }

    // Initialize systems
    initialize_systems().await?;

    let wallet_address = get_wallet_address()?;
    log(LogTag::System, "INFO", &format!("Using wallet: {}...{}", &wallet_address[..8], &wallet_address[wallet_address.len() - 8..]));

    // Check initial SOL balance
    let initial_sol_balance = check_initial_balance(&wallet_address).await?;

    // Get token information
    let token = get_token_info(token_mint).await?;

    // Run single token test with first amount
    run_single_token_test(&token, TEST_SOL_AMOUNTS[0], &wallet_address, initial_sol_balance, dry_run_mode).await
}

/// Initialize all required systems
async fn initialize_systems() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Initializing ScreenerBot systems...");
    
    // Initialize RPC client
    let _ = init_rpc_client()
        .map_err(|e| format!("Failed to initialize RPC client: {}", e))?;
    
    // Initialize DexScreener API
    init_dexscreener_api().await
        .map_err(|e| format!("Failed to initialize DexScreener API: {}", e))?;
    
    // Initialize price service
    initialize_price_service().await;
    
    // Initialize pool service
    let _ = init_pool_service();
    
    // Initialize transaction monitoring service
    TransactionMonitoringService::init_global_service().await
        .map_err(|e| format!("Failed to initialize transaction monitoring service: {}", e))?;
    
    // Start the background monitoring service as a separate task
    let shutdown_notify = std::sync::Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown_notify.clone();
    tokio::spawn(async move {
        if let Err(e) = TransactionMonitoringService::start_monitoring_service(shutdown_clone).await {
            log(LogTag::System, "ERROR", &format!("Transaction monitoring service failed: {}", e));
        }
    });
    
    println!("✅ Systems initialized successfully");
    Ok(())
}

/// Check initial wallet balance
async fn check_initial_balance(wallet_address: &str) -> Result<f64, Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "💰 Checking initial wallet balance...");
    let max_test_amount = TEST_SOL_AMOUNTS.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let required_balance = max_test_amount + 0.005; // Extra for fees
    
    let initial_sol_balance = get_sol_balance(wallet_address).await.map_err(|e| {
        log(LogTag::System, "ERROR", &format!("Failed to get SOL balance: {}", e));
        e
    })?;

    log(LogTag::System, "INFO", &format!("Initial SOL balance: {:.6} SOL", initial_sol_balance));
    
    if initial_sol_balance < required_balance {
        let error_msg = format!(
            "Insufficient SOL balance. Need at least {:.6} SOL, have {:.6} SOL",
            required_balance,
            initial_sol_balance
        );
        log(LogTag::System, "ERROR", &error_msg);
        return Err(error_msg.into());
    }

    Ok(initial_sol_balance)
}

/// Get token information with comprehensive validation
async fn get_token_info(token_mint: &str) -> Result<Token, Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "📊 Fetching token information...");

    // Get token from API (returns complete Token object)
    let token = get_token_from_mint_global_api(token_mint).await.map_err(|e| {
        log(LogTag::System, "ERROR", &format!("❌ Failed to fetch token info: {}", e));
        e
    })?.ok_or_else(|| {
        log(LogTag::System, "ERROR", "❌ Token not found in DexScreener API");
        "Token not found"
    })?;

    log(LogTag::System, "SUCCESS", &format!("✅ Token found: {} ({})", token.symbol, token.name));
    log(LogTag::System, "INFO", &format!("DEX: {}", token.dex_id.as_ref().unwrap_or(&"Unknown".to_string())));
    log(LogTag::System, "INFO", &format!("Pair address: {}", token.pair_address.as_ref().unwrap_or(&"None".to_string())));
    
    if let Some(liquidity) = &token.liquidity {
        if let Some(usd) = liquidity.usd {
            log(LogTag::System, "INFO", &format!("Liquidity: ${:.2}", usd));
        }
    }

    Ok(token)
}

/// Run comprehensive tests on all predefined tokens
async fn run_comprehensive_tests() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "🧪 Running comprehensive swap tests on all predefined tokens");
    
    // Initialize everything
    initialize_systems().await?;
    let wallet_address = get_wallet_address()?;
    let initial_sol_balance = check_initial_balance(&wallet_address).await?;
    
    log(LogTag::System, "INFO", &format!("Wallet: {}...", &wallet_address[..8]));
    log(LogTag::System, "INFO", &format!("Initial balance: {:.6} SOL", initial_sol_balance));

    let mut test_results = Vec::new();

    // Test each token with different amounts
    for token_mint in TEST_TOKENS.iter() {
        log(LogTag::System, "INFO", "");
        log(LogTag::System, "INFO", &format!("🎯 Testing token: {}", token_mint));
        log(LogTag::System, "INFO", &"=".repeat(80));

        // Get token info
        let token = match get_token_info(token_mint).await {
            Ok(token) => token,
            Err(e) => {
                log(LogTag::System, "WARNING", &format!("Token {} failed to load: {}, skipping", token_mint, e));
                continue;
            }
        };

        // Test with multiple amounts
        for &amount in TEST_SOL_AMOUNTS.iter() {
            log(LogTag::System, "INFO", "");
            log(LogTag::System, "INFO", &format!("💰 Testing {} with {:.6} SOL", token.symbol, amount));
            log(LogTag::System, "INFO", &"-".repeat(50));

            match test_single_swap(&token, amount, &wallet_address, false).await {
                Ok(result) => {
                    log(LogTag::System, "SUCCESS", &format!("✅ {} test with {:.6} SOL completed", token.symbol, amount));
                    test_results.push((token.symbol.clone(), amount, true, result));
                }
                Err(e) => {
                    log(LogTag::System, "ERROR", &format!("❌ {} test with {:.6} SOL failed: {}", token.symbol, amount, e));
                    test_results.push((token.symbol.clone(), amount, false, format!("Error: {}", e)));
                }
            }

            // Add position analysis summary for this test
            log(LogTag::System, "INFO", "");
            log(LogTag::System, "INFO", &format!("📊 {} POSITION ANALYSIS SUMMARY:", token.symbol.to_uppercase()));
            log(LogTag::System, "INFO", &format!("  Test Amount: {:.6} SOL", amount));
            log(LogTag::System, "INFO", &format!("  Token: {} ({})", token.symbol, token.name));
            log(LogTag::System, "INFO", &format!("  Mint: {}", token.mint));
            if let Some(price) = token.price_dexscreener_sol {
                log(LogTag::System, "INFO", &format!("  DexScreener Price: {:.12} SOL/token", price));
            }
            log(LogTag::System, "INFO", &"-".repeat(50));

            // Wait between tests
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    // Print summary
    print_test_summary(&test_results);

    log(LogTag::System, "SUCCESS", "🎉 Comprehensive tests completed!");
    Ok(())
}

/// Print comprehensive test summary
fn print_test_summary(results: &[(String, f64, bool, String)]) {
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "📊 TEST SUMMARY");
    log(LogTag::System, "INFO", &"=".repeat(80));

    let total_tests = results.len();
    let successful_tests = results.iter().filter(|(_, _, success, _)| *success).count();
    let failed_tests = total_tests - successful_tests;

    log(LogTag::System, "INFO", &format!("Total tests: {}", total_tests));
    log(LogTag::System, "INFO", &format!("Successful: {} ({}%)", successful_tests, (successful_tests * 100) / total_tests));
    log(LogTag::System, "INFO", &format!("Failed: {} ({}%)", failed_tests, (failed_tests * 100) / total_tests));
    log(LogTag::System, "INFO", "");

    for (symbol, amount, success, result) in results {
        let status = if *success { "✅" } else { "❌" };
        log(LogTag::System, "INFO", &format!("{} {} @ {:.6} SOL: {}", status, symbol, amount, result));
    }
}

/// Run test for a single token with specified amount
async fn run_single_token_test(
    token: &Token,
    test_amount: f64,
    wallet_address: &str,
    initial_sol_balance: f64,
    dry_run_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", &format!("🎯 Testing {} with {:.6} SOL", token.symbol, test_amount));
    
    // Check balance is sufficient
    if initial_sol_balance < test_amount + 0.005 {
        let error_msg = format!("Insufficient balance for test amount {:.6} SOL", test_amount);
        log(LogTag::System, "ERROR", &error_msg);
        return Err(error_msg.into());
    }

    let result = test_single_swap(token, test_amount, wallet_address, dry_run_mode).await?;
    log(LogTag::System, "SUCCESS", &format!("✅ Single token test completed: {}", result));
    Ok(())
}

/// Test a single swap cycle with specified token and amount
async fn test_single_swap(
    token: &Token,
    test_amount: f64,
    wallet_address: &str,
    dry_run_mode: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    // Get token decimals
    let token_decimals = get_token_decimals_from_chain(&token.mint).await.map_err(|e| {
        log(LogTag::System, "ERROR", &format!("Failed to get token decimals: {}", e));
        e
    })?;

    log(LogTag::System, "INFO", &format!("Token decimals: {}", token_decimals));

    // Get comprehensive price comparison from multiple sources
    let mut price_comparison = get_comprehensive_price_comparison(&token.mint, wallet_address, test_amount).await;
    
    // Add DexScreener price from token object
    price_comparison.dexscreener_price = token.price_dexscreener_sol;
    if let Some(price) = token.price_dexscreener_sol {
        log(LogTag::System, "INFO", &format!("  🔵 DexScreener Price: {:.10} SOL", price));
        // Update best price if needed
        if price_comparison.best_price.is_none() {
            price_comparison.best_price = Some(price);
        }
    } else {
        log(LogTag::System, "WARNING", "  🔴 DexScreener Price: Not available");
    }
    
    // Use the best available price for validation
    let expected_price = price_comparison.best_price;

    // Check initial balances
    let initial_sol_balance = get_sol_balance(wallet_address).await.unwrap_or(0.0);
    let initial_token_balance = get_token_balance(wallet_address, &token.mint).await.unwrap_or(0);

    log(LogTag::System, "INFO", "📊 Initial balances:");
    log(LogTag::System, "INFO", &format!("  SOL: {:.6}", initial_sol_balance));
    log(LogTag::System, "INFO", &format!("  {}: {} tokens", token.symbol, initial_token_balance));

    // STEP 1: Buy tokens with SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 1: Buying tokens with SOL");
    log(LogTag::System, "INFO", &"=".repeat(50));

    if dry_run_mode {
        log(LogTag::System, "INFO", "🔍 DRY RUN: Skipping actual buy transaction");
        log(LogTag::System, "INFO", "📊 API and quote analysis completed successfully");
        return Ok("DRY RUN COMPLETED - API and price analysis finished".to_string());
    }

    let buy_result = buy_token(&token, test_amount, expected_price).await.map_err(|e| {
        log(LogTag::System, "ERROR", &format!("❌ Buy transaction failed: {}", e));
        e
    })?;

    log(LogTag::System, "SUCCESS", "✅ Buy transaction successful!");
    log_swap_result(&buy_result, "BUY");

    // 🔍 DEBUG: Analyze buy transaction using transaction service BLOCKING verification
    if let Some(signature) = &buy_result.transaction_signature {
        log(LogTag::System, "DEBUG", "🔍 ANALYZING BUY TRANSACTION WITH BLOCKING VERIFICATION...");
        log(LogTag::System, "DEBUG", &format!("📝 Transaction signature: {}", signature));
        
        // Use blocking verification for immediate results (no background service dependency)
        match TransactionMonitoringService::verify_transaction_blocking(
            signature,
            &token.mint,
            "buy",
            SOL_MINT,
            &token.mint,
            std::time::Duration::from_secs(120) // 2 minute timeout
        ).await {
            Ok(final_state) => {
                log(LogTag::System, "DEBUG", &format!("✅ BLOCKING verification completed with state: {:?}", final_state));
                
                match final_state {
                    screenerbot::swaps::transaction::TransactionState::Verified { .. } => {
                        log(LogTag::System, "SUCCESS", "✅ Buy transaction fully verified!");
                    }
                    screenerbot::swaps::transaction::TransactionState::Failed { error, .. } => {
                        log(LogTag::System, "ERROR", &format!("❌ Buy transaction verification failed: {}", error));
                    }
                    _ => {
                        log(LogTag::System, "WARNING", &format!("⚠️ Buy transaction in unexpected state: {:?}", final_state));
                    }
                }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("❌ BLOCKING verification failed: {}", e));
            }
        }
        
        log(LogTag::System, "DEBUG", "🔍 BUY TRANSACTION BLOCKING ANALYSIS COMPLETE");
        log(LogTag::System, "DEBUG", &"=".repeat(60));
    }

    // Wait for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 10 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Check balances after buy
    let sol_balance_after_buy = get_sol_balance(wallet_address).await.unwrap_or(0.0);
    let token_balance_after_buy = get_token_balance(wallet_address, &token.mint).await.unwrap_or(0);

    log(LogTag::System, "INFO", "📊 Balances after buy:");
    log(LogTag::System, "INFO", &format!("  SOL: {:.6} (change: {:.6})", sol_balance_after_buy, sol_balance_after_buy - initial_sol_balance));
    log(LogTag::System, "INFO", &format!("  {}: {} tokens (change: {})", token.symbol, token_balance_after_buy, token_balance_after_buy - initial_token_balance));

    // Calculate tokens received
    let tokens_received = token_balance_after_buy - initial_token_balance;
    let tokens_received_swap = buy_result.output_amount.parse::<u64>().unwrap_or(0);

    let tokens_to_sell = if tokens_received > 0 {
        tokens_received
    } else if tokens_received_swap > 0 {
        log(LogTag::System, "WARNING", "⚠️ Using swap result data for tokens received");
        tokens_received_swap
    } else {
        return Err("No tokens received from buy transaction".into());
    };

    log(LogTag::System, "SUCCESS", &format!("✅ Successfully bought {} tokens", tokens_to_sell));

    // STEP 2: Sell tokens back to SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 2: Selling tokens back to SOL");
    log(LogTag::System, "INFO", &"=".repeat(50));

    // Calculate expected SOL output for validation
    let expected_sol_output = if let Some(price) = expected_price {
        let actual_tokens = (tokens_to_sell as f64) / (10_f64).powi(token_decimals as i32);
        let estimated_sol = price * actual_tokens;
        log(LogTag::System, "INFO", &format!("Expected SOL output: {:.6} SOL", estimated_sol));
        Some(estimated_sol)
    } else {
        None
    };

    let sell_result = sell_token(&token, tokens_to_sell, expected_sol_output).await.map_err(|e| {
        log(LogTag::System, "ERROR", &format!("❌ Sell transaction failed: {}", e));
        e
    })?;

    log(LogTag::System, "SUCCESS", "✅ Sell transaction successful!");
    log_swap_result(&sell_result, "SELL");

    // 📊 DETAILED SELL TRANSACTION ANALYSIS
    log(LogTag::System, "INFO", "🔍 Analyzing sell transaction details...");
    
    if let Some(ref swap_data) = sell_result.swap_data {
        // Log quote information
        log(LogTag::System, "INFO", &format!("📈 Sell Quote Analysis:"));
        log(LogTag::System, "INFO", &format!("  - Input Token: {}", swap_data.quote.input_mint));
        log(LogTag::System, "INFO", &format!("  - Output Token: {}", swap_data.quote.output_mint));
        log(LogTag::System, "INFO", &format!("  - Input Amount (quote): {} (raw: {}, decimals: {})", 
            swap_data.quote.in_amount.parse::<f64>().unwrap_or(0.0) / 10f64.powi(swap_data.quote.in_decimals as i32),
            swap_data.quote.in_amount, swap_data.quote.in_decimals));
        log(LogTag::System, "INFO", &format!("  - Output Amount (quote): {} (raw: {}, decimals: {})",
            swap_data.quote.out_amount.parse::<f64>().unwrap_or(0.0) / 10f64.powi(swap_data.quote.out_decimals as i32), 
            swap_data.quote.out_amount, swap_data.quote.out_decimals));
        log(LogTag::System, "INFO", &format!("  - Price Impact: {}%", swap_data.quote.price_impact_pct));
    } else {
        log(LogTag::System, "WARNING", "⚠️  No swap_data available in sell result!");
    }
    
    // Log transaction analysis from SwapResult
    log(LogTag::System, "INFO", &format!("📊 Sell Transaction Analysis:"));
    log(LogTag::System, "INFO", &format!("  - Signature: {}", fmt_opt_signature(&sell_result.transaction_signature)));
    log(LogTag::System, "INFO", &format!("  - Input Amount (parsed): {}", sell_result.input_amount));
    log(LogTag::System, "INFO", &format!("  - Output Amount (parsed): {}", sell_result.output_amount));
    log(LogTag::System, "INFO", &format!("  - Price Impact: {}", sell_result.price_impact));
    log(LogTag::System, "INFO", &format!("  - Effective Price: {}", fmt_opt_f64(sell_result.effective_price, 12)));
    log(LogTag::System, "INFO", &format!("  - Fee (lamports): {}", sell_result.fee_lamports));
    log(LogTag::System, "INFO", &format!("  - Execution Time: {:.2}s", sell_result.execution_time));
    
    // Compare quote vs parsed amounts if both are available
    if let Some(ref swap_data) = sell_result.swap_data {
        let quote_input_raw = swap_data.quote.in_amount.parse::<f64>().unwrap_or(0.0);
        let quote_output_raw = swap_data.quote.out_amount.parse::<f64>().unwrap_or(0.0);
        let parsed_input_raw = sell_result.input_amount.parse::<f64>().unwrap_or(0.0);
        let parsed_output_raw = sell_result.output_amount.parse::<f64>().unwrap_or(0.0);
        
        log(LogTag::System, "INFO", &format!("🔍 Sell Amount Verification:"));
        log(LogTag::System, "INFO", &format!("  - Quote Input (raw): {:.0}", quote_input_raw));
        log(LogTag::System, "INFO", &format!("  - Parsed Input (raw): {:.0}", parsed_input_raw));
        log(LogTag::System, "INFO", &format!("  - Input Difference: {:.0} ({:.2}%)", 
            (parsed_input_raw - quote_input_raw).abs(),
            if quote_input_raw > 0.0 { 
                ((parsed_input_raw - quote_input_raw).abs() / quote_input_raw) * 100.0 
            } else { 0.0 }));
        
        log(LogTag::System, "INFO", &format!("  - Quote Output (raw): {:.0}", quote_output_raw));
        log(LogTag::System, "INFO", &format!("  - Parsed Output (raw): {:.0}", parsed_output_raw));
        log(LogTag::System, "INFO", &format!("  - Output Difference: {:.0} ({:.2}%)", 
            (parsed_output_raw - quote_output_raw).abs(),
            if quote_output_raw > 0.0 { 
                ((parsed_output_raw - quote_output_raw).abs() / quote_output_raw) * 100.0 
            } else { 0.0 }));
    }

    // Wait for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 5 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Final analysis
    let analysis_result = analyze_swap_cycle(&buy_result, &sell_result, token, test_amount, expected_price).await;

    // STEP 3: Advanced Transaction & Position Analysis using Core Functions
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🧪 STEP 3: Advanced Transaction & Position Analysis");
    log(LogTag::System, "INFO", &"=".repeat(80));
    
    // Analyze buy transaction using transaction.rs verification
    if let Some(buy_signature) = &buy_result.transaction_signature {
        log(LogTag::System, "INFO", "🔍 ANALYZING BUY TRANSACTION WITH TRANSACTION.RS:");
        
        match screenerbot::swaps::transaction::verify_position_entry_transaction(
            buy_signature,
            &token.mint,
            test_amount
        ).await {
            Ok(entry_verification) => {
                log(LogTag::System, "SUCCESS", "✅ Position Entry Verification Results:");
                log(LogTag::System, "INFO", &format!("  • Transaction: {}", entry_verification.transaction_signature));
                log(LogTag::System, "INFO", &format!("  • Success: {}", entry_verification.success));
                log(LogTag::System, "INFO", &format!("  • Entry Verified: {}", entry_verification.entry_transaction_verified));
                log(LogTag::System, "INFO", &format!("  • Tokens Received: {} raw", entry_verification.token_amount_received));
                log(LogTag::System, "INFO", &format!("  • SOL Spent: {} lamports ({:.9} SOL)", 
                    entry_verification.sol_spent, 
                    screenerbot::rpc::lamports_to_sol(entry_verification.sol_spent)));
                log(LogTag::System, "INFO", &format!("  • Effective Entry Price: {:.12} SOL/token", entry_verification.effective_entry_price));
                log(LogTag::System, "INFO", &format!("  • ATA Created: {}", entry_verification.ata_created));
                log(LogTag::System, "INFO", &format!("  • ATA Rent Paid: {} lamports ({:.9} SOL)", 
                    entry_verification.ata_rent_paid,
                    screenerbot::rpc::lamports_to_sol(entry_verification.ata_rent_paid)));
                log(LogTag::System, "INFO", &format!("  • Transaction Fee: {} lamports ({:.9} SOL)", 
                    entry_verification.transaction_fee,
                    screenerbot::rpc::lamports_to_sol(entry_verification.transaction_fee)));
                log(LogTag::System, "INFO", &format!("  • Total Cost: {:.9} SOL", entry_verification.total_cost_sol));
                
                // Compare with swap result
                if let Some(swap_effective_price) = buy_result.effective_price {
                    let price_diff = ((entry_verification.effective_entry_price - swap_effective_price) / swap_effective_price * 100.0).abs();
                    log(LogTag::System, "INFO", "");
                    log(LogTag::System, "INFO", "🔍 PRICE COMPARISON:");
                    log(LogTag::System, "INFO", &format!("  • Swap Result Price: {:.12} SOL/token", swap_effective_price));
                    log(LogTag::System, "INFO", &format!("  • Position Entry Price: {:.12} SOL/token", entry_verification.effective_entry_price));
                    log(LogTag::System, "INFO", &format!("  • Price Difference: {:.2}%", price_diff));
                    
                    if price_diff > 1.0 {
                        log(LogTag::System, "WARNING", &format!("⚠️ Significant price difference detected: {:.2}%", price_diff));
                    } else {
                        log(LogTag::System, "SUCCESS", "✅ Price calculations match within tolerance");
                    }
                }
                
                if let Some(error) = entry_verification.error {
                    log(LogTag::System, "WARNING", &format!("⚠️ Entry verification error: {}", error));
                }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("❌ Position entry verification failed: {}", e));
            }
        }
    }
    
    // Analyze sell transaction using transaction.rs verification
    if let Some(sell_signature) = &sell_result.transaction_signature {
        log(LogTag::System, "INFO", "");
        log(LogTag::System, "INFO", "🔍 ANALYZING SELL TRANSACTION WITH TRANSACTION.RS:");
        
        let tokens_sold = sell_result.input_amount.parse::<u64>().unwrap_or(0);
        match screenerbot::swaps::transaction::verify_position_exit_transaction(
            sell_signature,
            &token.mint,
            tokens_sold
        ).await {
            Ok(exit_verification) => {
                log(LogTag::System, "SUCCESS", "✅ Position Exit Verification Results:");
                log(LogTag::System, "INFO", &format!("  • Transaction: {}", exit_verification.transaction_signature));
                log(LogTag::System, "INFO", &format!("  • Success: {}", exit_verification.success));
                log(LogTag::System, "INFO", &format!("  • Exit Verified: {}", exit_verification.exit_transaction_verified));
                log(LogTag::System, "INFO", &format!("  • Tokens Sold: {} raw", exit_verification.token_amount_sold));
                log(LogTag::System, "INFO", &format!("  • SOL Received: {} lamports ({:.9} SOL)", 
                    exit_verification.sol_received, 
                    screenerbot::rpc::lamports_to_sol(exit_verification.sol_received)));
                log(LogTag::System, "INFO", &format!("  • Effective Exit Price: {:.12} SOL/token", exit_verification.effective_exit_price));
                log(LogTag::System, "INFO", &format!("  • ATA Closed: {}", exit_verification.ata_closed));
                log(LogTag::System, "INFO", &format!("  • ATA Rent Reclaimed: {} lamports ({:.9} SOL)", 
                    exit_verification.ata_rent_reclaimed,
                    screenerbot::rpc::lamports_to_sol(exit_verification.ata_rent_reclaimed)));
                log(LogTag::System, "INFO", &format!("  • Transaction Fee: {} lamports ({:.9} SOL)", 
                    exit_verification.transaction_fee,
                    screenerbot::rpc::lamports_to_sol(exit_verification.transaction_fee)));
                log(LogTag::System, "INFO", &format!("  • Net SOL Received: {:.9} SOL", exit_verification.net_sol_received));
                
                // Compare with swap result
                // Debug: Show sell result effective price details
                log(LogTag::System, "DEBUG", "🔍 SELL RESULT EFFECTIVE PRICE DEBUG:");
                log(LogTag::System, "DEBUG", &format!("  • sell_result.effective_price: {}", fmt_opt_f64(sell_result.effective_price, 12)));
                log(LogTag::System, "DEBUG", &format!("  • sell_result.success: {}", sell_result.success));
                if let Some(error) = &sell_result.error {
                    log(LogTag::System, "DEBUG", &format!("  • sell_result.error: {}", error));
                }
                
                if let Some(swap_effective_price) = sell_result.effective_price {
                    // Extra diagnostics when swap_effective_price is zero or near-zero
                    if swap_effective_price <= 0.0 {
                        log(LogTag::System, "WARNING", "⚠️ Swap effective price is 0.0 — investigating root cause");
                        // Log sell_result breakdown
                        log(LogTag::System, "DEBUG", &format!(
                            "  • SELL RESULT BREAKDOWN: input_amount(raw)={}, output_amount(raw)={}, fee_lamports={}, success={}, err={:?}",
                            sell_result.input_amount,
                            sell_result.output_amount,
                            sell_result.fee_lamports,
                            sell_result.success,
                            sell_result.error
                        ));
                        // Attempt a recompute using exit verification values as a sanity check
                        let token_decimals = screenerbot::tokens::decimals::get_token_decimals_from_chain(&token.mint)
                            .await
                            .unwrap_or(9);
                        let tokens_sold_dec = (exit_verification.token_amount_sold as f64)
                            / (10_f64).powi(token_decimals as i32);
                        let total_sol_received = screenerbot::rpc::lamports_to_sol(
                            exit_verification.sol_received,
                        );
                        let recomputed_exit_price = if tokens_sold_dec > 0.0 {
                            total_sol_received / tokens_sold_dec
                        } else {
                            0.0
                        };
                        log(LogTag::System, "DEBUG", &format!(
                            "  • EXIT VERIFY BREAKDOWN: token_decimals={}, tokens_sold_dec={:.9}, sol_received(SOL)={:.9}, ata_rent_reclaimed(SOL)={:.9}, recomputed_exit_price={:.12}",
                            token_decimals,
                            tokens_sold_dec,
                            screenerbot::rpc::lamports_to_sol(exit_verification.sol_received),
                            screenerbot::rpc::lamports_to_sol(exit_verification.ata_rent_reclaimed),
                            recomputed_exit_price
                        ));
                        log(LogTag::System, "DEBUG", "  • Likely causes: missing SOL received detection in instruction analysis, or quote fallback not applied for sell");
                    }
                    let price_diff = ((exit_verification.effective_exit_price - swap_effective_price) / swap_effective_price * 100.0).abs();
                    log(LogTag::System, "INFO", "");
                    log(LogTag::System, "INFO", "🔍 SELL PRICE COMPARISON:");
                    log(LogTag::System, "INFO", &format!("  • Swap Result Price: {:.12} SOL/token", swap_effective_price));
                    log(LogTag::System, "INFO", &format!("  • Position Exit Price: {:.12} SOL/token", exit_verification.effective_exit_price));
                    log(LogTag::System, "INFO", &format!("  • Price Difference: {:.2}%", price_diff));
                    
                    if price_diff > 1.0 {
                        log(LogTag::System, "WARNING", &format!("⚠️ Significant sell price difference detected: {:.2}%", price_diff));
                    } else {
                        log(LogTag::System, "SUCCESS", "✅ Sell price calculations match within tolerance");
                    }
                } else {
                    // This is the case we're hitting - effective price is None
                    log(LogTag::System, "INFO", "");
                    log(LogTag::System, "INFO", "🔍 SELL PRICE COMPARISON:");
                    log(LogTag::System, "WARNING", "  ❌ Swap Result Price: None (effective price calculation failed)");
                    log(LogTag::System, "INFO", &format!("  • Position Exit Price: {:.12} SOL/token", exit_verification.effective_exit_price));
                    log(LogTag::System, "INFO", "  • Price Difference: inf% (cannot compare with None)");
                    log(LogTag::System, "WARNING", "⚠️ Significant sell price difference detected: inf%");
                    
                    // Add debug info about why effective price is None
                    log(LogTag::System, "DEBUG", "🔍 DEBUGGING WHY EFFECTIVE PRICE IS NONE:");
                    log(LogTag::System, "DEBUG", &format!("  • Sell transaction signature: {:?}", sell_result.transaction_signature));
                    log(LogTag::System, "DEBUG", &format!("  • Sell input amount: {}", sell_result.input_amount));
                    log(LogTag::System, "DEBUG", &format!("  • Sell output amount: {}", sell_result.output_amount));
                    // Re-run a recompute using exit verification context to provide guidance
                    let token_decimals = screenerbot::tokens::decimals::get_token_decimals_from_chain(&token.mint)
                        .await
                        .unwrap_or(9);
                    let tokens_sold_dec = (exit_verification.token_amount_sold as f64)
                        / (10_f64).powi(token_decimals as i32);
                    let total_sol_received = screenerbot::rpc::lamports_to_sol(
                            exit_verification.sol_received,
                    );
                    let recomputed_exit_price = if tokens_sold_dec > 0.0 {
                        total_sol_received / tokens_sold_dec
                    } else {
                        0.0
                    };
                    log(LogTag::System, "DEBUG", &format!(
                        "  • RECOMPUTE (from exit verification): token_decimals={}, tokens_sold_dec={:.9}, total_sol_received(SWAP_ONLY)={:.9} SOL, recomputed_exit_price={:.12}",
                        token_decimals,
                        tokens_sold_dec,
                        total_sol_received,
                        recomputed_exit_price
                    ));
                    log(LogTag::System, "DEBUG", "  • This suggests the transaction verification effective price calculation failed");
                    log(LogTag::System, "DEBUG", "  • Possible reasons: instruction analysis didn't find SOL received, or calculation returned None");
                }
                
                if let Some(error) = exit_verification.error {
                    log(LogTag::System, "WARNING", &format!("⚠️ Exit verification error: {}", error));
                }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("❌ Position exit verification failed: {}", e));
            }
        }
    }
    
    // Test position tracking functions (simulated)
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🔍 TESTING POSITION TRACKING FUNCTIONS:");
    
    if let (Some(buy_signature), Some(sell_signature)) = (&buy_result.transaction_signature, &sell_result.transaction_signature) {
        // Test position entry calculation using positions.rs patterns
        let tokens_received = buy_result.output_amount.parse::<u64>().unwrap_or(0);
        let sol_spent_lamports = screenerbot::rpc::sol_to_lamports(test_amount);
        
        // Simulate position creation
        log(LogTag::System, "INFO", "📊 SIMULATED POSITION CREATION:");
        log(LogTag::System, "INFO", &format!("  • Token: {} ({})", token.symbol, token.mint));
        log(LogTag::System, "INFO", &format!("  • Entry TX: {}", buy_signature));
        log(LogTag::System, "INFO", &format!("  • Tokens Acquired: {} raw", tokens_received));
        log(LogTag::System, "INFO", &format!("  • SOL Invested: {:.9} SOL", test_amount));
        
        // Calculate what the position entry price would be
        if tokens_received > 0 {
            let token_decimals = screenerbot::tokens::decimals::get_token_decimals_from_chain(&token.mint).await.unwrap_or(6);
            let tokens_actual = (tokens_received as f64) / (10_f64).powi(token_decimals as i32);
            let position_entry_price = test_amount / tokens_actual;
            
            log(LogTag::System, "INFO", &format!("  • Tokens (decimal): {:.6}", tokens_actual));
            log(LogTag::System, "INFO", &format!("  • Position Entry Price: {:.12} SOL/token", position_entry_price));
            
            // Compare all price calculations
            log(LogTag::System, "INFO", "");
            log(LogTag::System, "INFO", "🎯 COMPREHENSIVE PRICE VALIDATION:");
            
            if let Some(swap_price) = buy_result.effective_price {
                log(LogTag::System, "INFO", &format!("  • Swap Engine Price: {:.12} SOL/token", swap_price));
                let swap_diff = ((position_entry_price - swap_price) / swap_price * 100.0).abs();
                log(LogTag::System, "INFO", &format!("    └─ vs Position: {:.2}% difference", swap_diff));
            }
            
            if let Some(expected) = expected_price {
                log(LogTag::System, "INFO", &format!("  • Expected Market Price: {:.12} SOL/token", expected));
                let market_diff = ((position_entry_price - expected) / expected * 100.0).abs();
                log(LogTag::System, "INFO", &format!("    └─ vs Position: {:.2}% difference", market_diff));
            }
            
            // Test P&L calculation
            if let Some(current_price) = expected_price {
                let current_value = tokens_actual * current_price;
                let pnl_sol = current_value - test_amount;
                let pnl_percent = (pnl_sol / test_amount) * 100.0;
                
                log(LogTag::System, "INFO", "");
                log(LogTag::System, "INFO", "💰 SIMULATED P&L CALCULATION:");
                log(LogTag::System, "INFO", &format!("  • Entry Value: {:.9} SOL", test_amount));
                log(LogTag::System, "INFO", &format!("  • Current Price: {:.12} SOL/token", current_price));
                log(LogTag::System, "INFO", &format!("  • Current Value: {:.9} SOL", current_value));
                log(LogTag::System, "INFO", &format!("  • P&L: {:.9} SOL ({:+.2}%)", pnl_sol, pnl_percent));
                
                if pnl_sol.abs() > test_amount * 0.5 {
                    log(LogTag::System, "WARNING", "⚠️ P&L calculation shows unrealistic loss/gain (>50%)");
                } else {
                    log(LogTag::System, "SUCCESS", "✅ P&L calculation appears reasonable");
                }
            }
        }
    }
    
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "SUCCESS", "🎉 Advanced transaction and position analysis completed!");

    Ok(analysis_result)
}

/// Comprehensive price comparison structure
#[derive(Debug)]
struct PriceComparison {
    api_price: Option<f64>,
    pool_price: Option<f64>,
    dexscreener_price: Option<f64>,
    gmgn_price: Option<f64>,
    jupiter_price: Option<f64>,
    best_price: Option<f64>,
    price_differences: Vec<(String, String, f64)>, // (source1, source2, diff_percent)
}

/// Get comprehensive price comparison from multiple sources
async fn get_comprehensive_price_comparison(
    token_mint: &str,
    wallet_address: &str,
    test_amount: f64,
) -> PriceComparison {
    log(LogTag::System, "INFO", "📊 Comprehensive Price Analysis:");
    
    // 1. Get price from main API
    let api_price = get_token_price_safe(token_mint).await;
    if let Some(price) = api_price {
        log(LogTag::System, "INFO", &format!("  🟢 API Price: {:.10} SOL", price));
    } else {
        log(LogTag::System, "WARNING", "  🔴 API Price: Not available");
    }
    
    // 2. Get Pool price directly from pool service
    let pool_price = {
        let pool_service = get_pool_service();
        if pool_service.check_token_availability(token_mint).await {
            match pool_service.get_pool_price(token_mint, api_price).await {
                Some(result) => {
                    if let Some(price) = result.price_sol {
                        log(LogTag::System, "INFO", &format!("  🏊 Pool Price: {:.10} SOL (from {} via {})", price, result.pool_address, result.dex_id));
                        Some(price)
                    } else {
                        log(LogTag::System, "WARNING", "  🔴 Pool Price: Calculation failed");
                        None
                    }
                }
                None => {
                    log(LogTag::System, "WARNING", "  🔴 Pool Price: No pool data available");
                    None
                }
            }
        } else {
            log(LogTag::System, "WARNING", "  🔴 Pool Price: Token not available in pool service");
            None
        }
    };
    
    // 3. Get DexScreener price from token object
    let dexscreener_price = None; // Will be set from token object in caller
    
    // 4. Get GMGN quote price
    let gmgn_price = get_gmgn_quote_price(token_mint, wallet_address, test_amount).await;
    if let Some(price) = gmgn_price {
        log(LogTag::System, "INFO", &format!("  🟡 GMGN Quote Price: {:.10} SOL", price));
    } else {
        log(LogTag::System, "WARNING", "  🔴 GMGN Quote Price: Not available");
    }
    
    // 5. Get Jupiter quote price
    let jupiter_price = get_jupiter_quote_price(token_mint, wallet_address, test_amount).await;
    if let Some(price) = jupiter_price {
        log(LogTag::System, "INFO", &format!("  🟠 Jupiter Quote Price: {:.10} SOL", price));
    } else {
        log(LogTag::System, "WARNING", "  🔴 Jupiter Quote Price: Not available");
    }
    
    // Collect all available prices
    let mut prices = Vec::new();
    if let Some(price) = api_price { prices.push(("API", price)); }
    if let Some(price) = pool_price { prices.push(("Pool", price)); }
    if let Some(price) = gmgn_price { prices.push(("GMGN", price)); }
    if let Some(price) = jupiter_price { prices.push(("Jupiter", price)); }
    
    // Calculate price differences
    let mut price_differences = Vec::new();
    for i in 0..prices.len() {
        for j in i+1..prices.len() {
            let (name1, price1) = prices[i];
            let (name2, price2) = prices[j];
            let diff = ((price1 - price2) / price2 * 100.0).abs();
            price_differences.push((name1.to_string(), name2.to_string(), diff));
        }
    }
    
    // Log price differences
    if !price_differences.is_empty() {
        log(LogTag::System, "INFO", "  📈 Price Differences:");
        for (source1, source2, diff) in &price_differences {
            let status = if *diff > 5.0 { "⚠️" } else { "✅" };
            log(LogTag::System, "INFO", &format!("    {} {} vs {}: {:.2}%", status, source1, source2, diff));
        }
    }
    
    // Choose best price (prefer quotes over API/Pool for accuracy, use median if available)
    // API prices may use incorrect decimals or stale data, while quotes are real-time and decimal-aware
    let best_price = if let (Some(gmgn), Some(jupiter)) = (gmgn_price, jupiter_price) {
        // If both quotes available, use the median of the two (more stable than single source)
        let avg = (gmgn + jupiter) / 2.0;
        Some(avg)
    } else {
        // Fallback priority: quotes > pool > API (quotes most accurate, pool real-time, API may be stale)
        jupiter_price
            .or(gmgn_price)
            .or(pool_price)
            .or(api_price)
    };
    
    if let Some(price) = best_price {
        log(LogTag::System, "INFO", &format!("  🎯 Selected Price: {:.10} SOL", price));
    } else {
        log(LogTag::System, "WARNING", "  ❌ No price available from any source");
    }
    
    PriceComparison {
        api_price,
        pool_price,
        dexscreener_price,
        gmgn_price,
        jupiter_price,
        best_price,
        price_differences,
    }
}

/// Get price from GMGN quote
async fn get_gmgn_quote_price(
    token_mint: &str,
    wallet_address: &str,
    test_amount: f64,
) -> Option<f64> {
    match get_gmgn_quote(
        SOL_MINT,
        token_mint,
        screenerbot::rpc::sol_to_lamports(test_amount),
        wallet_address,
        1.0, // 1% slippage
        "ExactIn",
        0.0, // no fee
        false, // no anti-mev
    ).await {
        Ok(quote_data) => {
            let input_lamports = screenerbot::rpc::sol_to_lamports(test_amount);
            let output_amount = quote_data.quote.out_amount.parse::<u64>().unwrap_or(0);
            let output_decimals = quote_data.quote.out_decimals;
            
            if output_amount > 0 {
                let input_sol = screenerbot::rpc::lamports_to_sol(input_lamports);
                let output_tokens = (output_amount as f64) / (10_f64).powi(output_decimals as i32);
                let price = input_sol / output_tokens;
                Some(price)
            } else {
                None
            }
        }
        Err(e) => {
            log(LogTag::System, "DEBUG", &format!("GMGN quote failed: {}", e));
            None
        }
    }
}

/// Get price from Jupiter quote
async fn get_jupiter_quote_price(
    token_mint: &str,
    wallet_address: &str,
    test_amount: f64,
) -> Option<f64> {
    match get_jupiter_quote(
        SOL_MINT,
        token_mint,
        screenerbot::rpc::sol_to_lamports(test_amount),
        wallet_address,
        1.0, // 1% slippage
        "ExactIn",
        0.0, // no fee
        false, // no anti-mev
    ).await {
        Ok(quote_data) => {
            let input_lamports = screenerbot::rpc::sol_to_lamports(test_amount);
            let output_amount = quote_data.quote.out_amount.parse::<u64>().unwrap_or(0);
            let output_decimals = quote_data.quote.out_decimals;
            
            if output_amount > 0 {
                let input_sol = screenerbot::rpc::lamports_to_sol(input_lamports);
                let output_tokens = (output_amount as f64) / (10_f64).powi(output_decimals as i32);
                let price = input_sol / output_tokens;
                Some(price)
            } else {
                None
            }
        }
        Err(e) => {
            log(LogTag::System, "DEBUG", &format!("Jupiter quote failed: {}", e));
            None
        }
    }
}

/// Log detailed swap result information
fn log_swap_result(result: &screenerbot::utils::SwapResult, operation: &str) {
    log(LogTag::System, "INFO", &format!("📋 {} TRANSACTION DETAILS:", operation));
    
    if let Some(tx) = &result.transaction_signature {
        log(LogTag::System, "INFO", &format!("  Signature: {}", tx));
    }
    
    log(LogTag::System, "INFO", &format!("  Success: {}", result.success));
    log(LogTag::System, "INFO", &format!("  Input amount: {}", result.input_amount));
    log(LogTag::System, "INFO", &format!("  Output amount: {}", result.output_amount));
    log(LogTag::System, "INFO", &format!("  Price impact: {}%", result.price_impact));
    log(LogTag::System, "INFO", &format!("  Fee: {} lamports ({:.6} SOL)", result.fee_lamports, lamports_to_sol(result.fee_lamports)));
    log(LogTag::System, "INFO", &format!("  Execution time: {:.3}s", result.execution_time));
    
    if let Some(price) = result.effective_price {
        log(LogTag::System, "INFO", &format!("  Effective price: {:.10} SOL per token", price));
    } else {
        log(LogTag::System, "WARNING", "  Effective price: Not calculated");
    }
}

/// Analyze complete swap cycle and provide comprehensive metrics
async fn analyze_swap_cycle(
    buy_result: &screenerbot::utils::SwapResult,
    sell_result: &screenerbot::utils::SwapResult,
    token: &Token,
    test_amount: f64,
    expected_price: Option<f64>,
) -> String {
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "📊 COMPREHENSIVE SWAP ANALYSIS");
    log(LogTag::System, "INFO", &"=".repeat(80));

    // Basic transaction metrics
    let total_execution_time = buy_result.execution_time + sell_result.execution_time;
    let total_fees_lamports = buy_result.fee_lamports + sell_result.fee_lamports;
    let total_fees_sol = lamports_to_sol(total_fees_lamports);
    
    log(LogTag::System, "INFO", &format!("🔄 TRANSACTION SUMMARY:"));
    log(LogTag::System, "INFO", &format!("  Both transactions successful: {}", buy_result.success && sell_result.success));
    log(LogTag::System, "INFO", &format!("  Total execution time: {:.3}s", total_execution_time));
    log(LogTag::System, "INFO", &format!("  Total fees: {:.6} SOL ({} lamports)", total_fees_sol, total_fees_lamports));

    // Price analysis
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "💰 PRICE ANALYSIS:");
    
    if let Some(buy_price) = buy_result.effective_price {
        log(LogTag::System, "INFO", &format!("  Buy effective price: {:.10} SOL per token", buy_price));
    } else {
        log(LogTag::System, "WARNING", "  Buy effective price: Not available");
    }
    
    if let Some(sell_price) = sell_result.effective_price {
        log(LogTag::System, "INFO", &format!("  Sell effective price: {:.10} SOL per token", sell_price));
    } else {
        log(LogTag::System, "WARNING", "  Sell effective price: Not available");
    }

    // Compare with expected price
    if let Some(expected) = expected_price {
        log(LogTag::System, "INFO", &format!("  Expected price: {:.10} SOL per token", expected));
        
        if let Some(buy_price) = buy_result.effective_price {
            let buy_diff = ((buy_price - expected) / expected * 100.0).abs();
            log(LogTag::System, "INFO", &format!("  Buy vs expected: {:+.2}%", (buy_price - expected) / expected * 100.0));
            
            if buy_diff > MAX_PRICE_SLIPPAGE {
                log(LogTag::System, "WARNING", &format!("⚠️ Buy price difference exceeds tolerance: {:.2}%", buy_diff));
            }
        }
        
        if let Some(sell_price) = sell_result.effective_price {
            let sell_diff = ((sell_price - expected) / expected * 100.0).abs();
            log(LogTag::System, "INFO", &format!("  Sell vs expected: {:+.2}%", (sell_price - expected) / expected * 100.0));
            
            if sell_diff > MAX_PRICE_SLIPPAGE {
                log(LogTag::System, "WARNING", &format!("⚠️ Sell price difference exceeds tolerance: {:.2}%", sell_diff));
            }
        }
    }

    // Round trip analysis
    if let (Some(buy_price), Some(sell_price)) = (buy_result.effective_price, sell_result.effective_price) {
        let price_spread = ((buy_price - sell_price) / buy_price * 100.0).abs();
        log(LogTag::System, "INFO", "");
        log(LogTag::System, "INFO", "🔄 ROUND TRIP ANALYSIS:");
        log(LogTag::System, "INFO", &format!("  Price spread: {:.2}% ({:.10} SOL)", price_spread, (buy_price - sell_price).abs()));
        
        if price_spread > 5.0 {
            log(LogTag::System, "WARNING", &format!("⚠️ High price spread detected: {:.2}%", price_spread));
        }
    }

    // Calculate net result
    let input_sol = test_amount;
    let output_lamports = sell_result.output_amount.parse::<u64>().unwrap_or(0);
    let output_sol = lamports_to_sol(output_lamports);
    let net_result = output_sol - input_sol - total_fees_sol;
    
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "💸 NET RESULT:");
    log(LogTag::System, "INFO", &format!("  Input: {:.6} SOL", input_sol));
    log(LogTag::System, "INFO", &format!("  Output: {:.6} SOL", output_sol));
    log(LogTag::System, "INFO", &format!("  Fees: {:.6} SOL", total_fees_sol));
    log(LogTag::System, "INFO", &format!("  Net: {:.6} SOL ({:+.2}%)", net_result, (net_result / input_sol) * 100.0));

    let status = if buy_result.success && sell_result.success {
        "COMPLETED"
    } else {
        "FAILED"
    };

    // Summary result string
    format!(
        "{} - Net: {:.6} SOL, Fees: {:.6} SOL, Time: {:.3}s",
        status,
        net_result,
        total_fees_sol,
        total_execution_time
    )
}
