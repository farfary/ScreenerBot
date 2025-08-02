use screenerbot::{
    wallet::{
        buy_token,
        sell_token,
        get_sol_balance,
        get_token_balance,
        get_wallet_address,
        lamports_to_sol,
    },
    tokens::{
        types::Token,
        price_service::get_token_price_safe,
        decimals::get_token_decimals_from_chain,
    },
    global::{ read_configs, set_cmd_args },
    logger::{ log, LogTag, init_file_logging },
};

use std::env;
use tokio;

/// Test configuration
const TEST_SOL_AMOUNT: f64 = 0.001; // 0.001 SOL for testing
const MAX_PRICE_SLIPPAGE: f64 = 10.0; // 10% maximum acceptable slippage

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_file_logging();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <TOKEN_MINT_ADDRESS> [--debug-swap] [--debug-wallet]", args[0]);
        eprintln!(
            "Example: {} EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --debug-swap --debug-wallet",
            args[0]
        );
        std::process::exit(1);
    }

    let token_mint = &args[1];

    // Set up debug flags for global access
    set_cmd_args(args.clone());

    log(LogTag::System, "INFO", "🚀 Starting swap debug tool");
    log(LogTag::System, "INFO", &format!("Target token mint: {}", token_mint));
    log(LogTag::System, "INFO", &format!("Test amount: {:.6} SOL", TEST_SOL_AMOUNT));

    // Validate configuration
    let _configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to read configs: {}", e));
            std::process::exit(1);
        }
    };

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

    // Check initial SOL balance
    log(LogTag::System, "INFO", "💰 Checking initial wallet balance...");
    let initial_sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => {
            log(LogTag::System, "INFO", &format!("Initial SOL balance: {:.6} SOL", balance));
            if balance < TEST_SOL_AMOUNT + 0.002 {
                // Extra for fees
                log(
                    LogTag::System,
                    "ERROR",
                    &format!(
                        "Insufficient SOL balance. Need at least {:.6} SOL, have {:.6} SOL",
                        TEST_SOL_AMOUNT + 0.002,
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
    let token_decimals = match get_token_decimals_from_chain(token_mint).await {
        Ok(decimals) => {
            log(LogTag::System, "INFO", &format!("Token decimals: {}", decimals));
            decimals
        }
        Err(e) => {
            log(
                LogTag::System,
                "WARNING",
                &format!("Failed to get token decimals, using default 9: {}", e)
            );
            9
        }
    };

    let current_price = match get_token_price_safe(token_mint).await {
        Some(price) => {
            log(LogTag::System, "INFO", &format!("Current token price: {:.8} SOL", price));
            price
        }
        None => {
            log(LogTag::System, "WARNING", "Failed to get current price");
            0.0
        }
    };

    // Create Token struct for the functions
    let token = Token {
        mint: token_mint.to_string(),
        symbol: format!("TEST_{}", &token_mint[..8]),
        name: format!("Test Token {}", &token_mint[..8]),
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: Some(current_price),
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
    };

    log(LogTag::System, "INFO", &format!("Using token: {} ({})", token.symbol, token.name));

    // Check initial token balance
    let initial_token_balance = match get_token_balance(&wallet_address, token_mint).await {
        Ok(balance) => {
            log(
                LogTag::System,
                "INFO",
                &format!("Initial {} balance: {} tokens", token.symbol, balance)
            );
            balance
        }
        Err(e) => {
            log(LogTag::System, "INFO", &format!("No initial {} balance ({})", token.symbol, e));
            0
        }
    };

    // STEP 1: Buy tokens with SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 1: Buying tokens with SOL");
    log(LogTag::System, "INFO", "==================================================");

    let expected_price = if current_price > 0.0 { Some(current_price) } else { None };

    let buy_result = match buy_token(&token, TEST_SOL_AMOUNT, expected_price).await {
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
    log(LogTag::System, "INFO", "⏳ Waiting 5 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

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

    // Calculate tokens received
    let tokens_received = token_balance_after_buy - initial_token_balance;
    if tokens_received == 0 {
        log(LogTag::System, "ERROR", "❌ No tokens received from buy transaction!");
        std::process::exit(1);
    }

    log(LogTag::System, "SUCCESS", &format!("✅ Successfully bought {} tokens", tokens_received));

    // STEP 2: Sell tokens back to SOL
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "🎯 STEP 2: Selling tokens back to SOL");
    log(LogTag::System, "INFO", "==================================================");

    // Calculate expected SOL output based on current price
    let expected_sol_output = if current_price > 0.0 {
        let estimated_sol = current_price * (tokens_received as f64);
        log(LogTag::System, "INFO", &format!("Estimated SOL output: {:.6} SOL", estimated_sol));
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
            log(LogTag::System, "WARNING", "Will attempt to continue with partial test...");

            // Still check final balances even if sell failed
            check_final_balances(
                &wallet_address,
                &token,
                initial_sol_balance,
                initial_token_balance
            ).await;
            std::process::exit(1);
        }
    };

    // Wait a moment for transaction to settle
    log(LogTag::System, "INFO", "⏳ Waiting 5 seconds for transaction to settle...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Check final balances and calculate results
    check_final_balances(&wallet_address, &token, initial_sol_balance, initial_token_balance).await;

    // Calculate and display swap metrics
    calculate_swap_metrics(&buy_result, &sell_result, TEST_SOL_AMOUNT, initial_sol_balance).await;

    log(LogTag::System, "INFO", "");
    log(LogTag::System, "SUCCESS", "🎉 Swap debug test completed successfully!");

    Ok(())
}

/// Check final balances after all transactions
async fn check_final_balances(
    wallet_address: &str,
    token: &Token,
    initial_sol_balance: f64,
    initial_token_balance: u64
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

    // Analyze the results
    let sol_loss = initial_sol_balance - final_sol_balance;
    let token_change = (final_token_balance as i64) - (initial_token_balance as i64);

    if sol_loss > 0.0 {
        log(LogTag::System, "INFO", &format!("Net SOL cost: {:.6} SOL", sol_loss));
        let loss_percentage = (sol_loss / TEST_SOL_AMOUNT) * 100.0;
        log(LogTag::System, "INFO", &format!("Loss percentage: {:.2}%", loss_percentage));

        if loss_percentage > MAX_PRICE_SLIPPAGE {
            log(
                LogTag::System,
                "WARNING",
                &format!(
                    "⚠️ High slippage detected: {:.2}% > {:.1}%",
                    loss_percentage,
                    MAX_PRICE_SLIPPAGE
                )
            );
        } else {
            log(
                LogTag::System,
                "SUCCESS",
                &format!(
                    "✅ Acceptable slippage: {:.2}% <= {:.1}%",
                    loss_percentage,
                    MAX_PRICE_SLIPPAGE
                )
            );
        }
    } else {
        log(LogTag::System, "SUCCESS", &format!("✅ Net SOL gain: {:.6} SOL", -sol_loss));
    }

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

/// Calculate and display detailed swap metrics
async fn calculate_swap_metrics(
    buy_result: &screenerbot::wallet::SwapResult,
    sell_result: &screenerbot::wallet::SwapResult,
    _test_amount: f64,
    _initial_sol_balance: f64
) {
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "📈 SWAP METRICS");
    log(LogTag::System, "INFO", "==================================================");

    // Parse amounts from string results
    let buy_input_lamports = buy_result.input_amount.parse::<u64>().unwrap_or(0);
    let buy_output_tokens = buy_result.output_amount.parse::<u64>().unwrap_or(0);
    let sell_input_tokens = sell_result.input_amount.parse::<u64>().unwrap_or(0);
    let sell_output_lamports = sell_result.output_amount.parse::<u64>().unwrap_or(0);

    let buy_input_sol = lamports_to_sol(buy_input_lamports);
    let sell_output_sol = lamports_to_sol(sell_output_lamports);

    log(LogTag::System, "INFO", &format!("Buy transaction:"));
    log(
        LogTag::System,
        "INFO",
        &format!("  Input:  {:.6} SOL ({} lamports)", buy_input_sol, buy_input_lamports)
    );
    log(LogTag::System, "INFO", &format!("  Output: {} tokens", buy_output_tokens));
    log(LogTag::System, "INFO", &format!("  Price impact: {}%", buy_result.price_impact));
    log(LogTag::System, "INFO", &format!("  Fee: {} lamports", buy_result.fee_lamports));

    log(LogTag::System, "INFO", &format!("Sell transaction:"));
    log(LogTag::System, "INFO", &format!("  Input:  {} tokens", sell_input_tokens));
    log(
        LogTag::System,
        "INFO",
        &format!("  Output: {:.6} SOL ({} lamports)", sell_output_sol, sell_output_lamports)
    );
    log(LogTag::System, "INFO", &format!("  Price impact: {}%", sell_result.price_impact));
    log(LogTag::System, "INFO", &format!("  Fee: {} lamports", sell_result.fee_lamports));

    // Calculate effective prices
    if buy_output_tokens > 0 {
        let buy_price_per_token = buy_input_sol / (buy_output_tokens as f64);
        log(
            LogTag::System,
            "INFO",
            &format!("Effective buy price: {:.8} SOL per token", buy_price_per_token)
        );
    }

    if sell_input_tokens > 0 {
        let sell_price_per_token = sell_output_sol / (sell_input_tokens as f64);
        log(
            LogTag::System,
            "INFO",
            &format!("Effective sell price: {:.8} SOL per token", sell_price_per_token)
        );
    }

    // Calculate round-trip efficiency
    let total_fees_lamports = buy_result.fee_lamports + sell_result.fee_lamports;
    let total_fees_sol = lamports_to_sol(total_fees_lamports);
    log(
        LogTag::System,
        "INFO",
        &format!("Total fees: {:.6} SOL ({} lamports)", total_fees_sol, total_fees_lamports)
    );

    let round_trip_efficiency = (sell_output_sol / buy_input_sol) * 100.0;
    log(LogTag::System, "INFO", &format!("Round-trip efficiency: {:.2}%", round_trip_efficiency));

    let total_execution_time = buy_result.execution_time + sell_result.execution_time;
    log(LogTag::System, "INFO", &format!("Total execution time: {:.3}s", total_execution_time));
}
