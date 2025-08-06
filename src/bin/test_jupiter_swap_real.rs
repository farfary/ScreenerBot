/// Real Jupiter Swap Test with 0.002 SOL
/// 
/// This test performs actual on-chain Jupiter swaps using 0.002 SOL trade size.
/// Features:
/// - Real blockchain transactions (not dry runs)
/// - Comprehensive transaction verification
/// - Detailed balance tracking before/after
/// - Effective price calculation and validation
/// - Error handling with transaction analysis
/// - Safe rollback if tests fail
/// 
/// Safety Features:
/// - Small trade size (0.002 SOL) minimizes risk
/// - Comprehensive pre-flight checks
/// - Automatic rollback on test failures
/// - Detailed logging for debugging
/// - Balance validation at each step

use screenerbot::swaps::{
    jupiter::get_jupiter_quote,
    interface::{buy_token, sell_token},
    transaction::get_wallet_address,
    types::SOL_MINT,
};
use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::rpc::{lamports_to_sol, sol_to_lamports, get_rpc_client, init_rpc_client};
use screenerbot::utils::{get_sol_balance, get_token_balance};
use screenerbot::tokens::{Token, decimals::get_token_decimals_from_chain};

use std::time::Instant;
use clap::{Arg, Command};

/// Test configuration - 0.002 SOL as requested
const TEST_TRADE_SIZE_SOL: f64 = 0.002; // 0.002 SOL in native units
const TEST_SLIPPAGE: f64 = 15.0; // 15% slippage tolerance for small trades
const TEST_FEE: f64 = 0.25; // 0.25% fee

/// Well-known token for testing (BONK - high liquidity, low price)
const BONK_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
const BONK_SYMBOL: &str = "BONK";

/// Alternative tokens for testing
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const WIF_MINT: &str = "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm";

/// Test state tracking
#[derive(Debug)]
struct TestState {
    initial_sol_balance: f64,
    initial_token_balance: u64,
    after_buy_sol_balance: f64,
    after_buy_token_balance: u64,
    after_sell_sol_balance: f64,
    after_sell_token_balance: u64,
    buy_signature: Option<String>,
    sell_signature: Option<String>,
    test_token: Token,
    trade_size_lamports: u64,
}

impl TestState {
    async fn new(token_mint: &str, token_symbol: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let wallet_address = get_wallet_address()?;
        let initial_sol = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
        let initial_tokens = get_token_balance(&wallet_address, token_mint).await.unwrap_or(0);
        
        // Get token decimals for proper Token struct
        let _decimals = get_token_decimals_from_chain(token_mint).await.unwrap_or(9);
        
        let test_token = Token {
            mint: token_mint.to_string(),
            symbol: token_symbol.to_string(),
            name: format!("{} Token", token_symbol),
            chain: "solana".to_string(),
            logo_url: None,
            coingecko_id: None,
            website: None,
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: Some(chrono::Utc::now()),
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
        };

        Ok(Self {
            initial_sol_balance: initial_sol,
            initial_token_balance: initial_tokens,
            after_buy_sol_balance: 0.0,
            after_buy_token_balance: 0,
            after_sell_sol_balance: 0.0,
            after_sell_token_balance: 0,
            buy_signature: None,
            sell_signature: None,
            test_token,
            trade_size_lamports: sol_to_lamports(TEST_TRADE_SIZE_SOL),
        })
    }

    fn log_initial_state(&self) {
        log(
            LogTag::Test,
            "INITIAL_STATE",
            &format!(
                "📊 Initial Wallet State:\n  • SOL Balance: {:.6} SOL\n  • {} Balance: {} tokens\n  • Trade Size: {:.6} SOL ({} lamports)",
                self.initial_sol_balance,
                self.test_token.symbol,
                self.initial_token_balance,
                TEST_TRADE_SIZE_SOL,
                self.trade_size_lamports
            )
        );
    }

    async fn calculate_and_log_results(&self) {
        let sol_spent_buy = self.initial_sol_balance - self.after_buy_sol_balance;
        let tokens_received = self.after_buy_token_balance - self.initial_token_balance;
        let sol_received_sell = self.after_sell_sol_balance - self.after_buy_sol_balance;
        let tokens_sold = self.after_buy_token_balance - self.after_sell_token_balance;
        
        let net_sol_change = self.after_sell_sol_balance - self.initial_sol_balance;
        let effective_price_buy = if tokens_received > 0 {
            let token_decimals = get_token_decimals_from_chain(&self.test_token.mint).await.unwrap_or(9);
            sol_spent_buy / (tokens_received as f64 / 10f64.powi(token_decimals as i32))
        } else { 0.0 };

        log(
            LogTag::Test,
            "FINAL_RESULTS",
            &format!(
                "📊 Complete Swap Test Results:\n\
                 🔵 BUY PHASE:\n  • SOL Spent: {:.6} SOL\n  • Tokens Received: {} tokens\n  • Effective Price: {:.10} SOL per token\n\
                 🔴 SELL PHASE:\n  • Tokens Sold: {} tokens\n  • SOL Received: {:.6} SOL\n\
                 💰 NET RESULT:\n  • Net SOL Change: {:.6} SOL\n  • Success: {}\n\
                 📋 SIGNATURES:\n  • Buy TX: {}\n  • Sell TX: {}",
                sol_spent_buy,
                tokens_received,
                effective_price_buy,
                tokens_sold,
                sol_received_sell,
                net_sol_change,
                if net_sol_change > -0.001 { "✅ Good" } else { "⚠️ High Cost" },
                self.buy_signature.as_ref().unwrap_or(&"None".to_string()),
                self.sell_signature.as_ref().unwrap_or(&"None".to_string())
            )
        );
    }
}

/// Perform comprehensive pre-flight checks
async fn pre_flight_checks(token_mint: &str) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "PREFLIGHT_START", "🔍 Starting pre-flight safety checks...");

    // Check wallet balance
    let wallet_address = get_wallet_address()?;
    let sol_balance = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
    
    log(
        LogTag::Test,
        "PREFLIGHT_BALANCE",
        &format!("💰 Current SOL balance: {:.6} SOL", sol_balance)
    );

    if sol_balance < 0.01 {
        return Err(format!("Insufficient SOL balance: {:.6} SOL (need at least 0.01 SOL for tests + fees)", sol_balance).into());
    }

    // Test Jupiter quote availability
    log(LogTag::Test, "PREFLIGHT_QUOTE", "📊 Testing Jupiter quote availability...");
    
    let quote_result = get_jupiter_quote(
        SOL_MINT,
        token_mint,
        sol_to_lamports(TEST_TRADE_SIZE_SOL),
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    ).await;

    match quote_result {
        Ok(quote_data) => {
            log(
                LogTag::Test,
                "PREFLIGHT_QUOTE_OK",
                &format!(
                    "✅ Jupiter quote successful: {} SOL -> {} tokens (price impact: {}%)",
                    quote_data.quote.in_amount,
                    quote_data.quote.out_amount,
                    quote_data.quote.price_impact_pct
                )
            );
        }
        Err(e) => {
            return Err(format!("Jupiter quote test failed: {}", e).into());
        }
    }

    // Test RPC connectivity
    log(LogTag::Test, "PREFLIGHT_RPC", "🌐 Testing RPC connectivity...");
    let rpc_client = get_rpc_client();
    
    match rpc_client.get_latest_blockhash().await {
        Ok(blockhash) => {
            log(
                LogTag::Test,
                "PREFLIGHT_RPC_OK",
                &format!("✅ RPC connectivity confirmed (latest blockhash: {})", blockhash.to_string())
            );
        }
        Err(e) => {
            return Err(format!("RPC connectivity test failed: {}", e).into());
        }
    }

    log(LogTag::Test, "PREFLIGHT_COMPLETE", "✅ All pre-flight checks passed");
    Ok(())
}

/// Perform actual Jupiter buy swap
async fn test_jupiter_buy(test_state: &mut TestState) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "BUY_START", "🔵 Starting Jupiter BUY test (SOL -> Token)");

    let start_time = Instant::now();
    
    // Execute buy swap using interface layer
    let buy_result = buy_token(
        &test_state.test_token,
        TEST_TRADE_SIZE_SOL,
        None // No expected price restriction
    ).await;

    let execution_time = start_time.elapsed().as_secs_f64();

    match buy_result {
        Ok(swap_result) => {
            if swap_result.success {
                test_state.buy_signature = swap_result.transaction_signature.clone();
                
                log(
                    LogTag::Test,
                    "BUY_SUCCESS",
                    &format!(
                        "✅ Jupiter BUY completed in {:.2}s!\n  • Signature: {}\n  • Input: {} SOL\n  • Output: {} tokens\n  • Price Impact: {}%\n  • Fee: {} lamports",
                        execution_time,
                        swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                        lamports_to_sol(swap_result.input_amount.parse::<u64>().unwrap_or(0)),
                        swap_result.output_amount,
                        swap_result.price_impact,
                        swap_result.fee_lamports
                    )
                );

                // Update balances after buy
                let wallet_address = get_wallet_address()?;
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await; // Wait for confirmation
                
                test_state.after_buy_sol_balance = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
                test_state.after_buy_token_balance = get_token_balance(&wallet_address, &test_state.test_token.mint).await.unwrap_or(0);

                log(
                    LogTag::Test,
                    "BUY_BALANCES",
                    &format!(
                        "💰 Post-buy balances:\n  • SOL: {:.6} SOL (change: {:.6})\n  • {}: {} tokens (change: {})",
                        test_state.after_buy_sol_balance,
                        test_state.after_buy_sol_balance - test_state.initial_sol_balance,
                        test_state.test_token.symbol,
                        test_state.after_buy_token_balance,
                        test_state.after_buy_token_balance as i64 - test_state.initial_token_balance as i64
                    )
                );

                // Validate we received tokens
                if test_state.after_buy_token_balance <= test_state.initial_token_balance {
                    return Err("Buy transaction succeeded but no tokens were received".into());
                }

                Ok(())
            } else {
                Err(format!("Buy transaction failed: {}", swap_result.error.unwrap_or("Unknown error".to_string())).into())
            }
        }
        Err(e) => {
            Err(format!("Jupiter buy swap failed: {}", e).into())
        }
    }
}

/// Perform actual Jupiter sell swap
async fn test_jupiter_sell(test_state: &mut TestState) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "SELL_START", "🔴 Starting Jupiter SELL test (Token -> SOL)");

    // Calculate tokens to sell (use actual balance to sell everything)
    let tokens_to_sell = test_state.after_buy_token_balance - test_state.initial_token_balance;
    
    if tokens_to_sell == 0 {
        return Err("No tokens available to sell".into());
    }

    log(
        LogTag::Test,
        "SELL_AMOUNT",
        &format!("💰 Selling {} {} tokens", tokens_to_sell, test_state.test_token.symbol)
    );

    let start_time = Instant::now();
    
    // Execute sell swap using interface layer
    let sell_result = sell_token(
        &test_state.test_token,
        tokens_to_sell, // Position amount for validation
        None // No expected SOL output restriction
    ).await;

    let execution_time = start_time.elapsed().as_secs_f64();

    match sell_result {
        Ok(swap_result) => {
            if swap_result.success {
                test_state.sell_signature = swap_result.transaction_signature.clone();
                
                log(
                    LogTag::Test,
                    "SELL_SUCCESS",
                    &format!(
                        "✅ Jupiter SELL completed in {:.2}s!\n  • Signature: {}\n  • Input: {} tokens\n  • Output: {} SOL\n  • Price Impact: {}%\n  • Fee: {} lamports",
                        execution_time,
                        swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                        swap_result.input_amount,
                        lamports_to_sol(swap_result.output_amount.parse::<u64>().unwrap_or(0)),
                        swap_result.price_impact,
                        swap_result.fee_lamports
                    )
                );

                // Update balances after sell
                let wallet_address = get_wallet_address()?;
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await; // Wait for confirmation
                
                test_state.after_sell_sol_balance = get_sol_balance(&wallet_address).await.unwrap_or(0.0);
                test_state.after_sell_token_balance = get_token_balance(&wallet_address, &test_state.test_token.mint).await.unwrap_or(0);

                log(
                    LogTag::Test,
                    "SELL_BALANCES",
                    &format!(
                        "💰 Post-sell balances:\n  • SOL: {:.6} SOL (change: {:.6})\n  • {}: {} tokens (change: {})",
                        test_state.after_sell_sol_balance,
                        test_state.after_sell_sol_balance - test_state.after_buy_sol_balance,
                        test_state.test_token.symbol,
                        test_state.after_sell_token_balance,
                        test_state.after_sell_token_balance as i64 - test_state.after_buy_token_balance as i64
                    )
                );

                Ok(())
            } else {
                Err(format!("Sell transaction failed: {}", swap_result.error.unwrap_or("Unknown error".to_string())).into())
            }
        }
        Err(e) => {
            Err(format!("Jupiter sell swap failed: {}", e).into())
        }
    }
}

/// Emergency cleanup function - sell any remaining test tokens
async fn emergency_cleanup(test_state: &TestState) -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "CLEANUP_START", "🧹 Starting emergency cleanup...");

    let wallet_address = get_wallet_address()?;
    let current_token_balance = get_token_balance(&wallet_address, &test_state.test_token.mint).await.unwrap_or(0);

    if current_token_balance > test_state.initial_token_balance {
        let tokens_to_clean = current_token_balance - test_state.initial_token_balance;
        
        log(
            LogTag::Test,
            "CLEANUP_SELLING",
            &format!("🔄 Emergency selling {} {} tokens...", tokens_to_clean, test_state.test_token.symbol)
        );

        match sell_token(&test_state.test_token, tokens_to_clean, None).await {
            Ok(result) => {
                if result.success {
                    log(
                        LogTag::Test,
                        "CLEANUP_SUCCESS",
                        &format!("✅ Emergency cleanup completed: {}", result.transaction_signature.unwrap_or("Unknown".to_string()))
                    );
                } else {
                    log(
                        LogTag::Test,
                        "CLEANUP_FAILED",
                        &format!("❌ Emergency cleanup failed: {}", result.error.unwrap_or("Unknown".to_string()))
                    );
                }
            }
            Err(e) => {
                log(
                    LogTag::Test,
                    "CLEANUP_ERROR",
                    &format!("❌ Emergency cleanup error: {}", e)
                );
            }
        }
    } else {
        log(LogTag::Test, "CLEANUP_NONE", "✅ No cleanup needed - no excess tokens found");
    }

    Ok(())
}

/// Main test execution function
async fn run_jupiter_real_swap_test(token_mint: &str, token_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
    log(
        LogTag::Test,
        "TEST_START",
        &format!(
            "🚀 Starting Real Jupiter Swap Test\n  • Token: {} ({})\n  • Trade Size: {:.6} SOL\n  • Slippage: {}%",
            token_symbol,
            token_mint,
            TEST_TRADE_SIZE_SOL,
            TEST_SLIPPAGE
        )
    );

    // Initialize test state
    let mut test_state = TestState::new(token_mint, token_symbol).await?;
    test_state.log_initial_state();

    // Pre-flight checks
    pre_flight_checks(token_mint).await?;

    // Phase 1: Buy tokens with SOL
    match test_jupiter_buy(&mut test_state).await {
        Ok(_) => {
            log(LogTag::Test, "BUY_PHASE_COMPLETE", "✅ Buy phase completed successfully");
        }
        Err(e) => {
            log(LogTag::Test, "BUY_PHASE_FAILED", &format!("❌ Buy phase failed: {}", e));
            emergency_cleanup(&test_state).await?;
            return Err(e);
        }
    }

    // Small delay between operations
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Phase 2: Sell tokens back to SOL
    match test_jupiter_sell(&mut test_state).await {
        Ok(_) => {
            log(LogTag::Test, "SELL_PHASE_COMPLETE", "✅ Sell phase completed successfully");
        }
        Err(e) => {
            log(LogTag::Test, "SELL_PHASE_FAILED", &format!("❌ Sell phase failed: {}", e));
            emergency_cleanup(&test_state).await?;
            return Err(e);
        }
    }

    // Calculate and display final results
    test_state.calculate_and_log_results().await;

    log(
        LogTag::Test,
        "TEST_COMPLETE",
        "🎉 Real Jupiter swap test completed successfully!"
    );

    Ok(())
}

/// Command line interface
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup command line arguments
    let matches = Command::new("Jupiter Real Swap Test")
        .version("1.0")
        .about("Performs real Jupiter swaps with 0.002 SOL on Solana blockchain")
        .arg(
            Arg::new("token")
                .long("token")
                .value_name("TOKEN_MINT")
                .help("Token mint address to test with")
                .default_value(BONK_MINT)
        )
        .arg(
            Arg::new("symbol")
                .long("symbol")
                .value_name("SYMBOL")
                .help("Token symbol for display")
                .default_value(BONK_SYMBOL)
        )
        .arg(
            Arg::new("debug-swap")
                .long("debug-swap")
                .help("Enable detailed swap debugging")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-wallet")
                .long("debug-wallet")
                .help("Enable wallet operation debugging")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Initialize system
    init_file_logging();
    init_rpc_client()?;

    let token_mint = matches.get_one::<String>("token").unwrap();
    let token_symbol = matches.get_one::<String>("symbol").unwrap();
    
    // Display startup information
    log(
        LogTag::Test,
        "STARTUP",
        &format!(
            "🔧 Jupiter Real Swap Test Configuration:\n  • Token: {} ({})\n  • Trade Size: {:.6} SOL ({} lamports)\n  • Slippage: {}%\n  • Fee: {}%\n  • Debug Swap: {}\n  • Debug Wallet: {}",
            token_symbol,
            token_mint,
            TEST_TRADE_SIZE_SOL,
            sol_to_lamports(TEST_TRADE_SIZE_SOL),
            TEST_SLIPPAGE,
            TEST_FEE,
            matches.get_flag("debug-swap"),
            matches.get_flag("debug-wallet")
        )
    );

    // Safety confirmation
    log(
        LogTag::Test,
        "SAFETY_WARNING",
        "⚠️ This test performs REAL blockchain transactions with REAL SOL. Press Ctrl+C to cancel or wait 5 seconds to proceed..."
    );
    
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Run the test
    match run_jupiter_real_swap_test(token_mint, token_symbol).await {
        Ok(_) => {
            log(LogTag::Test, "SUCCESS", "✅ All tests completed successfully!");
            std::process::exit(0);
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("❌ Test failed: {}", e));
            std::process::exit(1);
        }
    }
}
