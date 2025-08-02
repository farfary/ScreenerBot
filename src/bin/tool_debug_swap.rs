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
        api::{ init_dexscreener_api, get_token_from_mint_global_api },
    },
    global::{ read_configs, set_cmd_args },
    logger::{ log, LogTag, init_file_logging },
    rpc::init_rpc_client,
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
            _token_decimals as u32
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

    // Display effective prices from SwapResult (calculated with correct decimals)
    if let Some(buy_effective_price) = buy_result.effective_price {
        log(
            LogTag::System,
            "INFO",
            &format!("Effective buy price: {:.10} SOL per token", buy_effective_price)
        );
    } else {
        log(LogTag::System, "WARNING", "Buy effective price not available");
    }

    if let Some(sell_effective_price) = sell_result.effective_price {
        log(
            LogTag::System,
            "INFO",
            &format!("Effective sell price: {:.10} SOL per token", sell_effective_price)
        );

        // Compare buy and sell prices if both are available
        if let Some(buy_effective_price) = buy_result.effective_price {
            let price_difference_percent = if buy_effective_price > 0.0 {
                ((sell_effective_price - buy_effective_price) / buy_effective_price) * 100.0
            } else {
                0.0
            };

            log(
                LogTag::System,
                "INFO",
                &format!(
                    "📈 Price difference: {:.2}% (buy: {:.10}, sell: {:.10})",
                    price_difference_percent,
                    buy_effective_price,
                    sell_effective_price
                )
            );
        }
    } else {
        log(LogTag::System, "WARNING", "Sell effective price not available");

        // Fallback: Calculate manually with proper decimal handling
        if buy_output_tokens > 0 && sell_input_tokens > 0 {
            // Get token decimals from buy result
            if let Some(swap_data) = &buy_result.swap_data {
                let token_decimals = swap_data.quote.out_decimals as u32;

                // Calculate effective prices manually with correct decimals
                let buy_tokens_actual =
                    (buy_output_tokens as f64) / (10_f64).powi(token_decimals as i32);
                let sell_tokens_actual =
                    (sell_input_tokens as f64) / (10_f64).powi(token_decimals as i32);

                let buy_price_manual = buy_input_sol / buy_tokens_actual;
                let sell_price_manual = sell_output_sol / sell_tokens_actual;

                log(
                    LogTag::System,
                    "INFO",
                    &format!(
                        "Manual effective buy price: {:.10} SOL per token (with {} decimals)",
                        buy_price_manual,
                        token_decimals
                    )
                );
                log(
                    LogTag::System,
                    "INFO",
                    &format!(
                        "Manual effective sell price: {:.10} SOL per token (with {} decimals)",
                        sell_price_manual,
                        token_decimals
                    )
                );

                let manual_price_diff = if buy_price_manual > 0.0 {
                    ((sell_price_manual - buy_price_manual) / buy_price_manual) * 100.0
                } else {
                    0.0
                };

                log(
                    LogTag::System,
                    "INFO",
                    &format!(
                        "📈 Manual price difference: {:.2}% (buy: {:.10}, sell: {:.10})",
                        manual_price_diff,
                        buy_price_manual,
                        sell_price_manual
                    )
                );
            }
        }
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
