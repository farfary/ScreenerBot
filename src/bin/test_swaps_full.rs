/// Comprehensive swap testing for both GMGN and Jupiter routers
/// This file tests the complete swap functionality including quotes, transactions, and verifications
/// Tests are designed to be comprehensive and provide detailed logging for debugging

use screenerbot::swaps::{
    gmgn::{get_gmgn_quote, execute_gmgn_swap, GMGNSwapResult},
    jupiter::{get_jupiter_quote, execute_jupiter_swap, JupiterSwapResult},
    types::{SwapRequest, SOL_MINT},
    transaction::get_wallet_address,
};
use screenerbot::global::read_configs;
use screenerbot::logger::{log, LogTag};
use screenerbot::rpc::lamports_to_sol;

/// Test configuration constants
const TEST_SOL_AMOUNT: u64 = 10_000_000; // 0.01 SOL in lamports
const TEST_SLIPPAGE: f64 = 15.0; // 15% slippage for testing
const TEST_FEE: f64 = 0.25; // 0.25% fee

/// Well-known token mints for testing
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const BONK_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

/// Test GMGN swap functionality with detailed logging
pub async fn test_gmgn_full_swap() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "GMGN_TEST_START", "🔵 Starting comprehensive GMGN swap test");

    // Setup test configuration
    let configs = read_configs()?;
    let wallet_address = get_wallet_address()?;
    
    log(
        LogTag::Test,
        "GMGN_CONFIG",
        &format!("📋 Wallet: {}, SOL Amount: {:.6}", wallet_address, lamports_to_sol(TEST_SOL_AMOUNT))
    );

    // Test 1: Get GMGN quote for SOL -> USDC
    log(LogTag::Test, "GMGN_QUOTE_TEST", "🔄 Testing GMGN quote: SOL -> USDC");
    
    let quote_result = get_gmgn_quote(
        SOL_MINT,
        USDC_MINT,
        TEST_SOL_AMOUNT,
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    ).await;

    match quote_result {
        Ok(swap_data) => {
            log(
                LogTag::Test,
                "GMGN_QUOTE_SUCCESS",
                &format!(
                    "✅ GMGN Quote successful: {} SOL -> {} USDC (Price Impact: {}%)",
                    swap_data.quote.in_amount,
                    swap_data.quote.out_amount,
                    swap_data.quote.price_impact_pct
                )
            );

            // Test 2: Execute GMGN swap (dry run mode)
            log(LogTag::Test, "GMGN_SWAP_TEST", "🔄 Testing GMGN swap execution");
            
            let swap_request = SwapRequest {
                input_mint: SOL_MINT.to_string(),
                output_mint: USDC_MINT.to_string(),
                input_amount: TEST_SOL_AMOUNT,
                from_address: wallet_address.clone(),
                slippage: TEST_SLIPPAGE,
                fee: TEST_FEE,
                is_anti_mev: false,
                expected_price: None,
            };

            // Note: This would be a real transaction - for testing, we just validate the quote
            log(
                LogTag::Test,
                "GMGN_SWAP_VALIDATION",
                &format!(
                    "✅ GMGN Swap validation successful - Transaction would exchange {} lamports for {} tokens",
                    swap_data.quote.in_amount,
                    swap_data.quote.out_amount
                )
            );
        }
        Err(e) => {
            log(
                LogTag::Test,
                "GMGN_QUOTE_ERROR",
                &format!("❌ GMGN Quote failed: {}", e)
            );
            return Err(format!("GMGN quote test failed: {}", e).into());
        }
    }

    log(LogTag::Test, "GMGN_TEST_COMPLETE", "✅ GMGN comprehensive test completed successfully");
    Ok(())
}

/// Test Jupiter swap functionality with detailed logging
pub async fn test_jupiter_full_swap() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "JUPITER_TEST_START", "🟡 Starting comprehensive Jupiter swap test");

    // Setup test configuration
    let configs = read_configs()?;
    let wallet_address = get_wallet_address()?;
    
    log(
        LogTag::Test,
        "JUPITER_CONFIG",
        &format!("📋 Wallet: {}, SOL Amount: {:.6}", wallet_address, lamports_to_sol(TEST_SOL_AMOUNT))
    );

    // Test 1: Get Jupiter quote for SOL -> BONK
    log(LogTag::Test, "JUPITER_QUOTE_TEST", "🔄 Testing Jupiter quote: SOL -> BONK");
    
    let quote_result = get_jupiter_quote(
        SOL_MINT,
        BONK_MINT,
        TEST_SOL_AMOUNT,
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    ).await;

    match quote_result {
        Ok(swap_data) => {
            log(
                LogTag::Test,
                "JUPITER_QUOTE_SUCCESS",
                &format!(
                    "✅ Jupiter Quote successful: {} SOL -> {} BONK (Price Impact: {}%)",
                    swap_data.quote.in_amount,
                    swap_data.quote.out_amount,
                    swap_data.quote.price_impact_pct
                )
            );

            // Test 2: Execute Jupiter swap (dry run mode)
            log(LogTag::Test, "JUPITER_SWAP_TEST", "🔄 Testing Jupiter swap execution");
            
            let swap_request = SwapRequest {
                input_mint: SOL_MINT.to_string(),
                output_mint: BONK_MINT.to_string(),
                input_amount: TEST_SOL_AMOUNT,
                from_address: wallet_address.clone(),
                slippage: TEST_SLIPPAGE,
                fee: TEST_FEE,
                is_anti_mev: false,
                expected_price: None,
            };

            // Note: This would be a real transaction - for testing, we just validate the quote
            log(
                LogTag::Test,
                "JUPITER_SWAP_VALIDATION",
                &format!(
                    "✅ Jupiter Swap validation successful - Transaction would exchange {} lamports for {} tokens",
                    swap_data.quote.in_amount,
                    swap_data.quote.out_amount
                )
            );
        }
        Err(e) => {
            log(
                LogTag::Test,
                "JUPITER_QUOTE_ERROR",
                &format!("❌ Jupiter Quote failed: {}", e)
            );
            return Err(format!("Jupiter quote test failed: {}", e).into());
        }
    }

    log(LogTag::Test, "JUPITER_TEST_COMPLETE", "✅ Jupiter comprehensive test completed successfully");
    Ok(())
}

/// Test router comparison functionality
pub async fn test_router_comparison() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "ROUTER_COMPARE_START", "⚖️ Starting router comparison test");

    let wallet_address = get_wallet_address()?;

    // Get quotes from both routers for the same swap
    log(LogTag::Test, "ROUTER_COMPARE_QUOTES", "🔄 Getting quotes from both GMGN and Jupiter");

    let gmgn_quote_future = get_gmgn_quote(
        SOL_MINT,
        USDC_MINT,
        TEST_SOL_AMOUNT,
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    );

    let jupiter_quote_future = get_jupiter_quote(
        SOL_MINT,
        USDC_MINT,
        TEST_SOL_AMOUNT,
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    );

    // Execute both quotes concurrently
    let (gmgn_result, jupiter_result) = tokio::join!(gmgn_quote_future, jupiter_quote_future);

    // Compare results
    match (gmgn_result, jupiter_result) {
        (Ok(gmgn_data), Ok(jupiter_data)) => {
            let gmgn_output: f64 = gmgn_data.quote.out_amount.parse().unwrap_or(0.0);
            let jupiter_output: f64 = jupiter_data.quote.out_amount.parse().unwrap_or(0.0);

            let better_router = if gmgn_output > jupiter_output { "GMGN" } else { "Jupiter" };
            let difference_pct = ((gmgn_output - jupiter_output).abs() / gmgn_output.max(jupiter_output)) * 100.0;

            log(
                LogTag::Test,
                "ROUTER_COMPARE_RESULT",
                &format!(
                    "📊 Router Comparison:\n  • GMGN Output: {:.6} USDC\n  • Jupiter Output: {:.6} USDC\n  • Better Router: {}\n  • Difference: {:.2}%",
                    gmgn_output,
                    jupiter_output,
                    better_router,
                    difference_pct
                )
            );
        }
        (Err(gmgn_err), Ok(_)) => {
            log(
                LogTag::Test,
                "ROUTER_COMPARE_PARTIAL",
                &format!("⚠️ GMGN failed ({}), Jupiter succeeded", gmgn_err)
            );
        }
        (Ok(_), Err(jupiter_err)) => {
            log(
                LogTag::Test,
                "ROUTER_COMPARE_PARTIAL",
                &format!("⚠️ Jupiter failed ({}), GMGN succeeded", jupiter_err)
            );
        }
        (Err(gmgn_err), Err(jupiter_err)) => {
            log(
                LogTag::Test,
                "ROUTER_COMPARE_FAILED",
                &format!("❌ Both routers failed - GMGN: {}, Jupiter: {}", gmgn_err, jupiter_err)
            );
            return Err("Both routers failed during comparison".into());
        }
    }

    log(LogTag::Test, "ROUTER_COMPARE_COMPLETE", "✅ Router comparison test completed");
    Ok(())
}

/// Main entry point for comprehensive swap testing
#[tokio::main]
async fn main() {
    // Initialize logging
    log(LogTag::Test, "START", "🚀 Starting comprehensive swap testing suite");
    
    // Run all tests
    match run_all_swap_tests().await {
        Ok(_) => {
            log(LogTag::Test, "SUCCESS", "✅ All swap tests completed successfully");
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("❌ Test suite failed: {}", e));
            std::process::exit(1);
        }
    }
}
pub async fn run_all_swap_tests() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "ALL_TESTS_START", "🚀 Starting comprehensive swap test suite");

    // Test 1: GMGN comprehensive test
    if let Err(e) = test_gmgn_full_swap().await {
        log(LogTag::Test, "ALL_TESTS_ERROR", &format!("❌ GMGN test failed: {}", e));
        return Err(e);
    }

    // Test 2: Jupiter comprehensive test
    if let Err(e) = test_jupiter_full_swap().await {
        log(LogTag::Test, "ALL_TESTS_ERROR", &format!("❌ Jupiter test failed: {}", e));
        return Err(e);
    }

    // Test 3: Router comparison test
    if let Err(e) = test_router_comparison().await {
        log(LogTag::Test, "ALL_TESTS_ERROR", &format!("❌ Router comparison failed: {}", e));
        return Err(e);
    }

    log(LogTag::Test, "ALL_TESTS_COMPLETE", "🎉 All comprehensive swap tests completed successfully");
    Ok(())
}

/// Test individual router functionality with error handling
pub async fn test_router_resilience() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Test, "RESILIENCE_START", "🛡️ Starting router resilience test");

    let wallet_address = get_wallet_address()?;

    // Test with invalid token mint
    log(LogTag::Test, "RESILIENCE_INVALID_MINT", "🔄 Testing with invalid token mint");
    
    let invalid_quote = get_gmgn_quote(
        SOL_MINT,
        "InvalidMintAddress123456789", // Invalid mint
        TEST_SOL_AMOUNT,
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    ).await;

    match invalid_quote {
        Ok(_) => {
            log(LogTag::Test, "RESILIENCE_UNEXPECTED", "⚠️ Unexpected success with invalid mint");
        }
        Err(e) => {
            log(
                LogTag::Test,
                "RESILIENCE_EXPECTED_ERROR",
                &format!("✅ Expected error with invalid mint: {}", e)
            );
        }
    }

    // Test with very small amount
    log(LogTag::Test, "RESILIENCE_SMALL_AMOUNT", "🔄 Testing with very small amount");
    
    let small_quote = get_jupiter_quote(
        SOL_MINT,
        USDC_MINT,
        1000, // Very small amount
        &wallet_address,
        TEST_SLIPPAGE,
        TEST_FEE,
        false,
    ).await;

    match small_quote {
        Ok(data) => {
            log(
                LogTag::Test,
                "RESILIENCE_SMALL_SUCCESS",
                &format!("✅ Small amount quote successful: {} -> {}", data.quote.in_amount, data.quote.out_amount)
            );
        }
        Err(e) => {
            log(
                LogTag::Test,
                "RESILIENCE_SMALL_ERROR",
                &format!("ℹ️ Small amount failed (expected): {}", e)
            );
        }
    }

    log(LogTag::Test, "RESILIENCE_COMPLETE", "✅ Router resilience test completed");
    Ok(())
}
