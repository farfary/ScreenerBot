use screenerbot::{
    utils::{ get_sol_balance, get_token_balance, get_wallet_address },
    swaps::{ buy_token, sell_token },
    rpc::lamports_to_sol,
    tokens::{
        types::Token,
        price::get_token_price_safe,
        decimals::get_token_decimals_from_chain,
        api::{ init_dexscreener_api, get_token_from_mint_global_api },
    },
    global::{ read_configs, set_cmd_args },
    logger::{ log, LogTag, init_file_logging },
    rpc::init_rpc_client,
};

use std::env;
use tokio;

/// Print comprehensive help menu for the Debug Swap Tool
fn print_help() {
    println!("🚀 Debug Swap Tool");
    println!("=====================================");
    println!("Comprehensive testing and debugging tool for swap operations with detailed");
    println!("wallet balance tracking, transaction analysis, and ATA detection validation.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_debug_swap -- <TOKEN_MINT> [OPTIONS]");
    println!("");
    println!("ARGUMENTS:");
    println!("    <TOKEN_MINT>       Token mint address to test swaps with");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h         Show this help message");
    println!("    --debug-swap       Enable detailed swap operation logging");
    println!("    --debug-wallet     Enable detailed wallet balance tracking");
    println!("");
    println!("EXAMPLES:");
    println!("    # Test basic swap with USDC");
    println!("    cargo run --bin tool_debug_swap -- EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
    println!("");
    println!("    # Full debug mode with detailed logging");
    println!(
        "    cargo run --bin tool_debug_swap -- EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --debug-swap --debug-wallet"
    );
    println!("");
    println!("    # Test with a specific token (example: Bonk)");
    println!(
        "    cargo run --bin tool_debug_swap -- DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 --debug-swap"
    );
    println!("");
    println!("TESTING WORKFLOW:");
    println!("    1. Validates token metadata and price data");
    println!("    2. Records initial wallet balances (SOL + tokens)");
    println!("    3. Executes buy transaction with {:.6} SOL", TEST_SOL_AMOUNT);
    println!("    4. Analyzes transaction for ATA detection accuracy");
    println!("    5. Validates post-buy balances and token acquisition");
    println!("    6. Executes sell transaction with acquired tokens");
    println!("    7. Analyzes sell transaction and ATA rent recovery");
    println!("    8. Compares final vs initial balances");
    println!("");
    println!("SAFETY FEATURES:");
    println!("    • Small test amount ({:.6} SOL) to minimize risk", TEST_SOL_AMOUNT);
    println!("    • {}% maximum slippage protection", MAX_PRICE_SLIPPAGE);
    println!("    • Comprehensive balance validation at each step");
    println!("    • Automatic ATA detection and rent calculation");
    println!("    • Transaction failure analysis and recovery");
    println!("");
    println!("DEBUG OUTPUT:");
    println!("    • Token metadata and price information");
    println!("    • Detailed wallet balance changes");
    println!("    • Transaction signatures and confirmation status");
    println!("    • ATA detection confidence scores");
    println!("    • Effective price calculations and slippage analysis");
    println!("    • P&L breakdown including ATA rent recovery");
    println!("");
}

/// Test configuration
const TEST_SOL_AMOUNTS: [f64; 3] = [0.001, 0.002, 0.003]; // Multiple test amounts
const MAX_PRICE_SLIPPAGE: f64 = 10.0; // 10% maximum acceptable slippage

/// Test tokens for comprehensive analysis
const TEST_TOKENS: [&str; 4] = [
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // Bonk
    "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm", // dogwifhat  
    "6p6xGHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN", // Unknown token
    "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H8Dfn",   // Unknown token
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

    // Check for special test modes
    if args.len() >= 2 && args[1] == "test-all" {
        // Run comprehensive tests on all predefined tokens
        return run_comprehensive_tests().await;
    }

    let token_mint = &args[1];

    // Set up debug flags for global access
    set_cmd_args(args.clone());

    log(LogTag::System, "INFO", "🚀 Starting swap debug tool");
    log(LogTag::System, "INFO", &format!("Target token mint: {}", token_mint));
    log(LogTag::System, "INFO", &format!("Test amounts: {:?} SOL", TEST_SOL_AMOUNTS));

    // Validate configuration
    let _configs = match read_configs() {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to read configs: {}", e));
            std::process::exit(1);
        }
    };

    // Initialize API systems
    log(LogTag::System, "INFO", "🔧 Initializing systems...");

    // Initialize RPC client
    if let Err(e) = init_rpc_client() {
        log(LogTag::System, "ERROR", &format!("Failed to initialize RPC client: {}", e));
        std::process::exit(1);
    }

    // Initialize DexScreener API
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize DexScreener API: {}", e));
        std::process::exit(1);
    }

    log(LogTag::System, "SUCCESS", "✅ Systems initialized successfully");

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => {
            log(
                LogTag::System,
                "INFO",
                &format!("Using wallet: {}...{}", &addr[..8], &addr[addr.len() - 8..])
            );
            addr
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            std::process::exit(1);
        }
    };

    // Check initial SOL balance - need enough for largest test + fees
    log(LogTag::System, "INFO", "💰 Checking initial wallet balance...");
    let max_test_amount = TEST_SOL_AMOUNTS.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
    let required_balance = max_test_amount + 0.005; // Extra for fees
    
    let initial_sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => {
            log(LogTag::System, "INFO", &format!("Initial SOL balance: {:.6} SOL", balance));
            if balance < required_balance {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!(
                        "Insufficient SOL balance. Need at least {:.6} SOL, have {:.6} SOL",
                        required_balance,
                        balance
                    )
                );
                std::process::exit(1);
            }
            balance
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get SOL balance: {}", e));
            std::process::exit(1);
        }
    };

    // Get token information
    log(LogTag::System, "INFO", "📊 Fetching token information...");

    // Get token from API (returns complete Token object)
    let token = match get_token_from_mint_global_api(token_mint).await {
        Ok(Some(token)) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("✅ Token found: {} ({})", token.symbol, token.name)
            );
            log(
                LogTag::System,
                "INFO",
                &format!("DEX: {}", token.dex_id.as_ref().unwrap_or(&"Unknown".to_string()))
            );
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "Pair address: {}",
                    token.pair_address.as_ref().unwrap_or(&"None".to_string())
                )
            );
            if let Some(liquidity) = &token.liquidity {
                if let Some(usd) = liquidity.usd {
                    log(LogTag::System, "INFO", &format!("Liquidity: ${:.2}", usd));
                }
            }
            token
        }
        Ok(None) => {
            log(LogTag::System, "ERROR", "❌ Token not found in DexScreener API");
            std::process::exit(1);
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Failed to fetch token info: {}", e));
            std::process::exit(1);
        }
    };

    let _token_decimals = match get_token_decimals_from_chain(token_mint).await {
        Ok(decimals) => {
            log(LogTag::System, "INFO", &format!("Token decimals: {}", decimals));
            Some(decimals)
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Failed to get token decimals: {} - Cannot proceed with analysis", e)
            );
            None
        }
    };

    // Skip further analysis if decimals cannot be determined
    if _token_decimals.is_none() {
        log(LogTag::System, "ERROR", "Cannot analyze swap without token decimals - aborting");
        return Err("Cannot determine token decimals".into());
    }

    let current_price = match get_token_price_safe(token_mint).await {
        Some(price) => {
            log(LogTag::System, "INFO", &format!("Current token price: {:.8} SOL", price));
            price
        }
        None => {
            // Try to get price from the token object
            if let Some(price) = token.price_dexscreener_sol {
                log(LogTag::System, "INFO", &format!("Token price from API: {:.8} SOL", price));
                price
            } else {
                log(LogTag::System, "WARNING", "Failed to get current price");
                0.0
            }
        }
    };

    log(LogTag::System, "INFO", &format!("Using token: {} ({})", token.symbol, token.name));

    // Run single token test with first amount
    run_single_token_test(&token, TEST_SOL_AMOUNTS[0], &wallet_address, initial_sol_balance).await
}

/// Run comprehensive tests on all predefined tokens
async fn run_comprehensive_tests() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "🧪 Running comprehensive swap tests on all predefined tokens");
    
    // Initialize everything
    let _configs = match read_configs() {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to read configs: {}", e));
            std::process::exit(1);
        }
    };

    if let Err(e) = init_rpc_client() {
        log(LogTag::System, "ERROR", &format!("Failed to initialize RPC client: {}", e));
        std::process::exit(1);
    }

    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize DexScreener API: {}", e));
        std::process::exit(1);
    }

    let wallet_address = get_wallet_address()?;
    let initial_sol_balance = get_sol_balance(&wallet_address).await?;
    
    log(LogTag::System, "INFO", &format!("Wallet: {}...", &wallet_address[..8]));
    log(LogTag::System, "INFO", &format!("Initial balance: {:.6} SOL", initial_sol_balance));

    // Test each token with different amounts
    for token_mint in TEST_TOKENS.iter() {
        log(LogTag::System, "INFO", "");
        log(LogTag::System, "INFO", &format!("🎯 Testing token: {}", token_mint));
        log(LogTag::System, "INFO", "=".repeat(80));

        // Get token info
        let token = match get_token_from_mint_global_api(token_mint).await {
            Ok(Some(token)) => token,
            Ok(None) => {
                log(LogTag::System, "WARNING", &format!("Token {} not found, skipping", token_mint));
                continue;
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Failed to get token {}: {}", token_mint, e));
                continue;
            }
        };

        // Test with multiple amounts
        for &amount in TEST_SOL_AMOUNTS.iter() {
            log(LogTag::System, "INFO", "");
            log(LogTag::System, "INFO", &format!("💰 Testing {} with {:.6} SOL", token.symbol, amount));
            log(LogTag::System, "INFO", "-".repeat(50));

            match test_single_swap(&token, amount, &wallet_address).await {
                Ok(_) => {
                    log(LogTag::System, "SUCCESS", &format!("✅ {} test with {:.6} SOL completed", token.symbol, amount));
                }
                Err(e) => {
                    log(LogTag::System, "ERROR", &format!("❌ {} test with {:.6} SOL failed: {}", token.symbol, amount, e));
                }
            }

            // Wait between tests
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    log(LogTag::System, "SUCCESS", "🎉 Comprehensive tests completed!");
    Ok(())
}

/// Run test for a single token with specified amount
async fn run_single_token_test(
    token: &screenerbot::tokens::types::Token,
    test_amount: f64,
    wallet_address: &str,
    initial_sol_balance: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", &format!("🎯 Testing {} with {:.6} SOL", token.symbol, test_amount));
    
    // Check balance is sufficient
    if initial_sol_balance < test_amount + 0.005 {
        log(LogTag::System, "ERROR", &format!("Insufficient balance for test amount {:.6} SOL", test_amount));
        return Err("Insufficient balance".into());
    }

    test_single_swap(token, test_amount, wallet_address).await
}

/// Test a single swap cycle with specified token and amount
async fn test_single_swap(
    token: &screenerbot::tokens::types::Token,
    test_amount: f64,
    wallet_address: &str,
) -> Result<(), Box<dyn std::error::Error>> {

/// Test a single swap cycle with specified token and amount
async fn test_single_swap(
    token: &screenerbot::tokens::types::Token,
    test_amount: f64,
    wallet_address: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get token decimals
    let token_decimals = match get_token_decimals_from_chain(&token.mint).await {
        Ok(decimals) => {
            log(LogTag::System, "INFO", &format!("Token decimals: {}", decimals));
            decimals
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token decimals: {}", e));
            return Err(e.into());
        }
    };

    // Get current price from multiple sources for comparison
    let api_price = get_token_price_safe(&token.mint).await;
    let dexscreener_price = token.price_dexscreener_sol;
    
    log(LogTag::System, "INFO", "📊 Price Comparison:");
    if let Some(price) = api_price {
        log(LogTag::System, "INFO", &format!("  API Price: {:.10} SOL", price));
    } else {
        log(LogTag::System, "WARNING", "  API Price: Not available");
    }
    
    if let Some(price) = dexscreener_price {
        log(LogTag::System, "INFO", &format!("  DexScreener Price: {:.10} SOL", price));
    } else {
        log(LogTag::System, "WARNING", "  DexScreener Price: Not available");
    }

    // Use the best available price for validation
    let expected_price = api_price.or(dexscreener_price);

    // Check initial balances
    let initial_sol_balance = get_sol_balance(wallet_address).await.unwrap_or(0.0);
    let initial_token_balance = get_token_balance(wallet_address, &token.mint).await.unwrap_or(0);

    log(LogTag::System, "INFO", &format!("📊 Initial balances:"));
    log(LogTag::System, "INFO", &format!("  SOL: {:.6}", initial_sol_balance));
    log(LogTag::System, "INFO", &format!("  {}: {} tokens", token.symbol, initial_token_balance));

    // STEP 1: Buy tokens with SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 1: Buying tokens with SOL");
    log(LogTag::System, "INFO", "=".repeat(50));

    let buy_result = match buy_token(&token, test_amount, expected_price).await {
        Ok(result) => {
            log(LogTag::System, "SUCCESS", "✅ Buy transaction successful!");
            log_swap_result(&result, "BUY");
            result
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Buy transaction failed: {}", e));
            return Err(e.into());
        }
    };

    // Wait for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 10 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Check balances after buy
    let sol_balance_after_buy = get_sol_balance(wallet_address).await.unwrap_or(0.0);
    let token_balance_after_buy = get_token_balance(wallet_address, &token.mint).await.unwrap_or(0);

    log(LogTag::System, "INFO", &format!("📊 Balances after buy:"));
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
        log(LogTag::System, "ERROR", "❌ No tokens received from buy transaction!");
        return Err("No tokens received".into());
    };

    log(LogTag::System, "SUCCESS", &format!("✅ Successfully bought {} tokens", tokens_to_sell));

    // STEP 2: Sell tokens back to SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 2: Selling tokens back to SOL");
    log(LogTag::System, "INFO", "=".repeat(50));

    // Calculate expected SOL output for validation
    let expected_sol_output = if let Some(price) = expected_price {
        let actual_tokens = (tokens_to_sell as f64) / (10_f64).powi(token_decimals as i32);
        let estimated_sol = price * actual_tokens;
        log(LogTag::System, "INFO", &format!("Expected SOL output: {:.6} SOL", estimated_sol));
        Some(estimated_sol)
    } else {
        None
    };

    let sell_result = match sell_token(&token, tokens_to_sell, expected_sol_output).await {
        Ok(result) => {
            log(LogTag::System, "SUCCESS", "✅ Sell transaction successful!");
            log_swap_result(&result, "SELL");
            result
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Sell transaction failed: {}", e));
            return Err(e.into());
        }
    };

    // Wait for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 5 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Final analysis
    analyze_swap_cycle(&buy_result, &sell_result, token, test_amount, expected_price).await;

    Ok(())
}

/// Log detailed swap result information
fn log_swap_result(result: &screenerbot::utils::SwapResult, operation: &str) {
        Ok(result) => {
            log(LogTag::System, "SUCCESS", &format!("✅ Buy transaction successful!"));
            log(
                LogTag::System,
                "SUCCESS",
                &format!(
                    "Transaction signature: {}",
                    result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
                )
            );
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Input amount: {} lamports", result.input_amount)
            );
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Output amount: {} tokens", result.output_amount)
            );
            log(LogTag::System, "SUCCESS", &format!("Price impact: {}%", result.price_impact));
            log(LogTag::System, "SUCCESS", &format!("Fee: {} lamports", result.fee_lamports));
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Execution time: {:.3}s", result.execution_time)
            );
            result
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Buy transaction failed: {}", e));
            std::process::exit(1);
        }
    };

    // Wait a moment for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 10 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Check balances after buy
    log(LogTag::System, "INFO", "💰 Checking balances after buy...");
    let sol_balance_after_buy = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
    let token_balance_after_buy = get_token_balance(&wallet_address, token_mint).await.unwrap_or(0);

    log(
        LogTag::System,
        "INFO",
        &format!(
            "SOL balance after buy: {:.6} SOL (change: {:.6} SOL)",
            sol_balance_after_buy,
            sol_balance_after_buy - initial_sol_balance
        )
    );
    log(
        LogTag::System,
        "INFO",
        &format!(
            "{} balance after buy: {} tokens (change: {} tokens)",
            token.symbol,
            token_balance_after_buy,
            (token_balance_after_buy as i64) - (initial_token_balance as i64)
        )
    );

    // Calculate tokens received from balance difference
    let tokens_received_balance = token_balance_after_buy - initial_token_balance;

    // Also get tokens received from swap result
    let tokens_received_swap = buy_result.output_amount.parse::<u64>().unwrap_or(0);

    log(
        LogTag::System,
        "INFO",
        &format!("Tokens expected from swap: {} tokens", tokens_received_swap)
    );

    // Use swap result if balance difference is zero (due to timing/caching)
    let tokens_received = if tokens_received_balance > 0 {
        tokens_received_balance
    } else if tokens_received_swap > 0 {
        log(
            LogTag::System,
            "WARNING",
            "⚠️ Balance difference is 0, using swap result data instead"
        );
        tokens_received_swap
    } else {
        log(LogTag::System, "ERROR", "❌ No tokens received from buy transaction!");
        std::process::exit(1);
    };

    log(LogTag::System, "SUCCESS", &format!("✅ Successfully bought {} tokens", tokens_received));

    // STEP 2: Sell tokens back to SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 2: Selling tokens back to SOL");
    log(LogTag::System, "INFO", "==================================================");

    // Calculate expected SOL output based on current price
    let expected_sol_output = if current_price > 0.0 {
        // Convert raw tokens to actual tokens using decimals from the buy result
        let token_decimals = if let Some(swap_data) = &buy_result.swap_data {
            swap_data.quote.out_decimals as u32
        } else {
            match _token_decimals {
                Some(decimals) => decimals as u32,
                None => {
                    log(
                        LogTag::System,
                        "ERROR",
                        "Cannot calculate expected output without decimals"
                    );
                    return Ok(());
                }
            }
        };

        // Convert raw tokens to actual token amount
        let actual_tokens = (tokens_received as f64) / (10_f64).powi(token_decimals as i32);

        // Calculate expected SOL using actual tokens
        let estimated_sol = current_price * actual_tokens;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Expected SOL calculation: {:.6} tokens (raw: {}) * {:.10} SOL/token = {:.6} SOL",
                actual_tokens,
                tokens_received,
                current_price,
                estimated_sol
            )
        );

        Some(estimated_sol)
    } else {
        None
    };

    let sell_result = match sell_token(&token, tokens_received, expected_sol_output).await {
        Ok(result) => {
            log(LogTag::System, "SUCCESS", &format!("✅ Sell transaction successful!"));
            log(
                LogTag::System,
                "SUCCESS",
                &format!(
                    "Transaction signature: {}",
                    result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
                )
            );
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Input amount: {} tokens", result.input_amount)
            );
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Output amount: {} lamports", result.output_amount)
            );
            log(LogTag::System, "SUCCESS", &format!("Price impact: {}%", result.price_impact));
            log(LogTag::System, "SUCCESS", &format!("Fee: {} lamports", result.fee_lamports));
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Execution time: {:.3}s", result.execution_time)
            );
            result
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Sell transaction failed: {}", e));
            log(LogTag::System, "WARNING", "Cannot display complete swap results - sell failed");
            
            // Display only buy results
            log(LogTag::System, "INFO", "");
            log(LogTag::System, "INFO", "📊 PARTIAL RESULTS (BUY ONLY)");
            log(LogTag::System, "INFO", "==================================================");
            log(LogTag::System, "INFO", &format!("Buy transaction success: {}", buy_result.success));
            if let Some(tx) = &buy_result.transaction_signature {
                log(LogTag::System, "INFO", &format!("Buy TX: {}", tx));
            }
            log(LogTag::System, "INFO", &format!("Buy input amount: {} lamports", buy_result.input_amount));
            log(LogTag::System, "INFO", &format!("Buy output amount: {} tokens", buy_result.output_amount));
            
            std::process::exit(1);
        }
    };

    // Wait a moment for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 5 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Check final balances using only swap results
    check_final_balances(&wallet_address, &token, initial_sol_balance, initial_token_balance, &buy_result, &sell_result).await;

    // Display swap metrics using only swap results
    display_swap_metrics(&buy_result, &sell_result).await;

    log(LogTag::System, "INFO", "");
    log(LogTag::System, "SUCCESS", "🎉 Swap debug test completed successfully!");

    Ok(())
}

/// Check final balances using only swap results (no calculations in debug tool)
async fn check_final_balances(
    wallet_address: &str,
    token: &Token,
    initial_sol_balance: f64,
    initial_token_balance: u64,
    buy_result: &screenerbot::utils::SwapResult,
    sell_result: &screenerbot::utils::SwapResult
) {
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "📊 FINAL BALANCE CHECK");
    log(LogTag::System, "INFO", "==================================================");

    let final_sol_balance = get_sol_balance(wallet_address).await.unwrap_or(0.0);
    let final_token_balance = get_token_balance(wallet_address, &token.mint).await.unwrap_or(0);

    log(LogTag::System, "INFO", &format!("Initial SOL balance: {:.6} SOL", initial_sol_balance));
    log(LogTag::System, "INFO", &format!("Final SOL balance:   {:.6} SOL", final_sol_balance));
    log(
        LogTag::System,
        "INFO",
        &format!("SOL difference:      {:.6} SOL", final_sol_balance - initial_sol_balance)
    );

    log(
        LogTag::System,
        "INFO",
        &format!("Initial {} balance: {} tokens", token.symbol, initial_token_balance)
    );
    log(
        LogTag::System,
        "INFO",
        &format!("Final {} balance:   {} tokens", token.symbol, final_token_balance)
    );
    log(
        LogTag::System,
        "INFO",
        &format!(
            "{} difference:      {} tokens",
            token.symbol,
            (final_token_balance as i64) - (initial_token_balance as i64)
        )
    );

    // Use swap results directly - no custom calculations
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "💱 SWAP RESULTS SUMMARY");
    log(LogTag::System, "INFO", "==================================================");
    
    // Buy transaction results
    log(LogTag::System, "INFO", &format!("Buy transaction success: {}", buy_result.success));
    if let Some(tx) = &buy_result.transaction_signature {
        log(LogTag::System, "INFO", &format!("Buy TX: {}", tx));
    }
    log(LogTag::System, "INFO", &format!("Buy input amount: {} lamports", buy_result.input_amount));
    log(LogTag::System, "INFO", &format!("Buy output amount: {} tokens", buy_result.output_amount));
    if let Some(price) = buy_result.effective_price {
        log(LogTag::System, "INFO", &format!("Buy effective price: {:.10} SOL per token", price));
    }
    
    // Sell transaction results
    log(LogTag::System, "INFO", &format!("Sell transaction success: {}", sell_result.success));
    if let Some(tx) = &sell_result.transaction_signature {
        log(LogTag::System, "INFO", &format!("Sell TX: {}", tx));
    }
    log(LogTag::System, "INFO", &format!("Sell input amount: {} tokens", sell_result.input_amount));
    log(LogTag::System, "INFO", &format!("Sell output amount: {} lamports", sell_result.output_amount));
    if let Some(price) = sell_result.effective_price {
        log(LogTag::System, "INFO", &format!("Sell effective price: {:.10} SOL per token", price));
    }

    let token_change = (final_token_balance as i64) - (initial_token_balance as i64);
    if token_change == 0 {
        log(
            LogTag::System,
            "SUCCESS",
            &format!("✅ No remaining {} tokens (clean round trip)", token.symbol)
        );
    } else {
        log(
            LogTag::System,
            "INFO",
            &format!("Remaining {} tokens: {}", token.symbol, token_change)
        );
    }
}

/// Display swap metrics using only swap results (no calculations in debug tool)
async fn display_swap_metrics(
    buy_result: &screenerbot::utils::SwapResult,
    sell_result: &screenerbot::utils::SwapResult
) {
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "📈 SWAP METRICS FROM RESULTS");
    log(LogTag::System, "INFO", "==================================================");

    // Display buy transaction results directly
    log(LogTag::System, "INFO", &format!("Buy transaction:"));
    log(LogTag::System, "INFO", &format!("  Success: {}", buy_result.success));
    log(LogTag::System, "INFO", &format!("  Input amount: {} (from swap result)", buy_result.input_amount));
    log(LogTag::System, "INFO", &format!("  Output amount: {} (from swap result)", buy_result.output_amount));
    log(LogTag::System, "INFO", &format!("  Price impact: {}%", buy_result.price_impact));
    log(LogTag::System, "INFO", &format!("  Fee: {} lamports", buy_result.fee_lamports));
    log(LogTag::System, "INFO", &format!("  Execution time: {:.3}s", buy_result.execution_time));
    if let Some(price) = buy_result.effective_price {
        log(LogTag::System, "INFO", &format!("  Effective price: {:.10} SOL per token", price));
    } else {
        if buy_result.success {
            log(LogTag::System, "WARNING", "  Effective price: Not calculated (unexpected for successful transaction)");
        } else {
            log(LogTag::System, "INFO", "  Effective price: Not available (transaction failed validation)");
        }
    }

    // Display sell transaction results directly
    log(LogTag::System, "INFO", &format!("Sell transaction:"));
    log(LogTag::System, "INFO", &format!("  Success: {}", sell_result.success));
    log(LogTag::System, "INFO", &format!("  Input amount: {} (from swap result)", sell_result.input_amount));
    log(LogTag::System, "INFO", &format!("  Output amount: {} (from swap result)", sell_result.output_amount));
    log(LogTag::System, "INFO", &format!("  Price impact: {}%", sell_result.price_impact));
    log(LogTag::System, "INFO", &format!("  Fee: {} lamports", sell_result.fee_lamports));
    log(LogTag::System, "INFO", &format!("  Execution time: {:.3}s", sell_result.execution_time));
    if let Some(price) = sell_result.effective_price {
        log(LogTag::System, "INFO", &format!("  Effective price: {:.10} SOL per token", price));
    } else {
        if sell_result.success {
            log(LogTag::System, "WARNING", "  Effective price: Not calculated (unexpected for successful transaction)");
        } else {
            log(LogTag::System, "INFO", "  Effective price: Not available (transaction failed validation)");
        }
    }

    // Summary using swap results
    let total_execution_time = buy_result.execution_time + sell_result.execution_time;
    let total_fees_lamports = buy_result.fee_lamports + sell_result.fee_lamports;
    let total_fees_sol = lamports_to_sol(total_fees_lamports);
    
    log(LogTag::System, "INFO", &format!("Total fees: {:.6} SOL ({} lamports)", total_fees_sol, total_fees_lamports));
    log(LogTag::System, "INFO", &format!("Total execution time: {:.3}s", total_execution_time));
}
