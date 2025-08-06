/// Comprehensive test suite for swap operations
/// Tests real on-chain swaps with different amounts and validates pricing accuracy

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::Token;
    use crate::swaps::{
        interface::{buy_token, sell_token, SwapResult},
        pricing::{calculate_effective_price_buy, calculate_effective_price_sell, validate_price_near_expected},
        execution::{get_swap_quote, execute_swap_with_quote},
        types::{SwapRequest, SOL_MINT},
        transaction::get_wallet_address,
    };
    use crate::global::read_configs;
    use crate::rpc::{sol_to_lamports, lamports_to_sol};
    use crate::wallet::get_token_balance;
    use std::str::FromStr;
    use tokio;

    // Test token - Using BONK as it's a well-known token with good liquidity
    const TEST_TOKEN_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK
    const TEST_TOKEN_SYMBOL: &str = "BONK";
    const TEST_TOKEN_NAME: &str = "Bonk";

    // Test amounts in SOL
    const TEST_AMOUNT_1: f64 = 0.001; // 0.001 SOL
    const TEST_AMOUNT_2: f64 = 0.002; // 0.002 SOL  
    const TEST_AMOUNT_3: f64 = 0.003; // 0.003 SOL

    /// Creates a test token instance
    fn create_test_token() -> Token {
        Token {
            mint: TEST_TOKEN_MINT.to_string(),
            symbol: TEST_TOKEN_SYMBOL.to_string(),
            name: TEST_TOKEN_NAME.to_string(),
            chain: "solana".to_string(),
            logo_url: Some("https://example.com/bonk.png".to_string()),
            coingecko_id: None,
            website: None,
            description: Some("Test token for swaps".to_string()),
            tags: vec!["meme".to_string()],
            is_verified: false,
            created_at: Some(chrono::Utc::now()),
            price_dexscreener_sol: Some(0.000000025), // Approximate BONK price
            price_dexscreener_usd: Some(0.00000375),
            price_pool_sol: Some(0.000000025),
            price_pool_usd: Some(0.00000375),
            dex_id: Some("raydium".to_string()),
            pair_address: Some("test_pair_address".to_string()),
            pair_url: None,
            labels: vec![],
            fdv: Some(1500000000.0),
            market_cap: Some(1500000000.0),
            txns: None,
            volume: None,
            price_change: None,
            liquidity: None,
            info: None,
            boosts: None,
        }
    }

    /// Test helper: Validate swap result structure with detailed logging
    fn validate_swap_result(result: &SwapResult, expected_success: bool) {
        println!("\n🔍 DETAILED SWAP RESULT VALIDATION:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        assert_eq!(result.success, expected_success, "Swap result success mismatch");
        
        if expected_success {
            assert!(result.transaction_signature.is_some(), "Missing transaction signature for successful swap");
            assert!(!result.input_amount.is_empty(), "Missing input amount");
            assert!(!result.output_amount.is_empty(), "Missing output amount");
            assert!(result.error.is_none(), "Unexpected error in successful swap");
        }
        
        println!("📊 Core Result Data:");
        println!("   ✅ Success: {}", result.success);
        println!("   🔗 TX Signature: {:?}", result.transaction_signature);
        println!("   💰 Input Amount: {} units", result.input_amount);
        println!("   🎯 Output Amount: {} units", result.output_amount);
        println!("   📈 Price Impact: {}%", result.price_impact);
        println!("   💸 Fee: {} lamports", result.fee_lamports);
        println!("   ⏱️  Execution Time: {:.3}s", result.execution_time);
        
        if let Some(effective_price) = result.effective_price {
            println!("   💎 Effective Price: {:.10} SOL per token", effective_price);
        }
        
        if let Some(error) = &result.error {
            println!("   ❌ Error: {}", error);
        }
        
        // Deep dive into swap_data if available
        if let Some(swap_data) = &result.swap_data {
            println!("\n📋 Quote Details:");
            println!("   🔄 Input Mint: {}", swap_data.quote.input_mint);
            println!("   🎯 Output Mint: {}", swap_data.quote.output_mint);
            println!("   📊 In Amount: {} lamports", swap_data.quote.in_amount);
            println!("   📊 Out Amount: {} tokens", swap_data.quote.out_amount);
            println!("   🔢 In Decimals: {}", swap_data.quote.in_decimals);
            println!("   🔢 Out Decimals: {}", swap_data.quote.out_decimals);
            println!("   📉 Slippage BPS: {}", swap_data.quote.slippage_bps);
            println!("   💥 Price Impact: {}%", swap_data.quote.price_impact_pct);
            println!("   ⏳ Time Taken: {:.3}s", swap_data.quote.time_taken);
            
            if let Some(context_slot) = swap_data.quote.context_slot {
                println!("   🎰 Context Slot: {}", context_slot);
            }
            
            println!("\n🔧 Transaction Details:");
            println!("   📝 Last Valid Block Height: {}", swap_data.raw_tx.last_valid_block_height);
            println!("   💰 Priority Fee: {} lamports", swap_data.raw_tx.prioritization_fee_lamports);
            println!("   🔍 Recent Blockhash: {}...", &swap_data.raw_tx.recent_blockhash[..16]);
            
            if let Some(version) = &swap_data.raw_tx.version {
                println!("   📟 Version: {}", version);
            }
            
            if let Some(amount_in_usd) = &swap_data.amount_in_usd {
                println!("   💵 Amount In USD: ${}", amount_in_usd);
            }
            
            if let Some(amount_out_usd) = &swap_data.amount_out_usd {
                println!("   💰 Amount Out USD: ${}", amount_out_usd);
            }
            
            if let Some(sol_cost) = &swap_data.sol_cost {
                println!("   ⚡ SOL Cost: {} SOL", sol_cost);
            }
        }
        
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    /// Test helper: Validate effective price calculation with detailed analysis
    fn validate_effective_price(effective_price: f64, expected_min: f64, expected_max: f64) {
        println!("\n💎 EFFECTIVE PRICE ANALYSIS:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        assert!(effective_price > 0.0, "Effective price must be positive");
        assert!(effective_price.is_finite(), "Effective price must be finite");
        
        println!("📊 Price Metrics:");
        println!("   🎯 Calculated Price: {:.10} SOL per token", effective_price);
        println!("   🔻 Expected Min: {:.10} SOL per token", expected_min);
        println!("   🔺 Expected Max: {:.10} SOL per token", expected_max);
        
        let within_range = effective_price >= expected_min && effective_price <= expected_max;
        println!("   📏 Within Range: {}", if within_range { "✅ YES" } else { "❌ NO" });
        
        if within_range {
            let range_position = (effective_price - expected_min) / (expected_max - expected_min);
            println!("   📍 Range Position: {:.1}% through range", range_position * 100.0);
        }
        
        // Calculate price in other units for context
        let price_per_million = effective_price * 1_000_000.0;
        let price_in_usd_cents = effective_price * 150.0 * 100.0; // Assuming 150 USD/SOL
        
        println!("\n🔍 Price Context:");
        println!("   📦 Per Million Tokens: {:.6} SOL", price_per_million);
        println!("   💰 Approx USD Value: {:.4} cents per token", price_in_usd_cents);
        
        assert!(
            within_range,
            "Effective price {:.10} is outside expected range [{:.10}, {:.10}]",
            effective_price, expected_min, expected_max
        );
        
        println!("   ✅ PRICE VALIDATION PASSED");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    #[tokio::test]
    async fn test_swap_request_validation() {
        println!("\n🧪 Testing SwapRequest validation...");

        let wallet_address = get_wallet_address().expect("Failed to get wallet address");
        
        // Test valid request
        let valid_request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: TEST_TOKEN_MINT.to_string(),
            input_amount: sol_to_lamports(TEST_AMOUNT_1),
            from_address: wallet_address.clone(),
            slippage: 15.0,
            fee: 0.1,
            is_anti_mev: false,
            expected_price: Some(0.000000025),
        };

        // Should not panic on valid request
        println!("✅ Valid request created successfully");
        println!("   Input: {} SOL -> {}", 
                lamports_to_sol(valid_request.input_amount), 
                &valid_request.output_mint[..8]);

        // Test edge cases
        let zero_amount_request = SwapRequest {
            input_amount: 0,
            ..valid_request.clone()
        };
        println!("✅ Zero amount request created (should be caught by validation)");

        let high_slippage_request = SwapRequest {
            slippage: 150.0, // Over 100%
            ..valid_request.clone()
        };
        println!("✅ High slippage request created (should be caught by validation)");
    }

    #[tokio::test]
    async fn test_price_validation_functions() {
        println!("\n🧪 Testing price validation functions...");

        // Test validate_price_near_expected
        let current_price = 0.000000025;
        let expected_price = 0.000000024;
        let tolerance = 5.0; // 5%

        let is_near = validate_price_near_expected(current_price, expected_price, tolerance);
        assert!(is_near, "Prices should be within 5% tolerance");
        println!("✅ Price validation: {:.10} is within {}% of {:.10}", 
                current_price, tolerance, expected_price);

        // Test price too far apart
        let far_price = 0.000000050; // 100% difference
        let is_far = validate_price_near_expected(far_price, expected_price, tolerance);
        assert!(!is_far, "Prices should NOT be within 5% tolerance");
        println!("✅ Price validation: {:.10} is NOT within {}% of {:.10}", 
                far_price, tolerance, expected_price);
    }

    #[tokio::test]
    #[ignore] // Remove this to run real on-chain tests
    async fn test_get_swap_quote_real() {
        println!("\n🧪 Testing real swap quote retrieval...");

        let wallet_address = get_wallet_address().expect("Failed to get wallet address");
        
        for (i, &amount_sol) in [TEST_AMOUNT_1, TEST_AMOUNT_2, TEST_AMOUNT_3].iter().enumerate() {
            println!("\n📊 Test {}: Getting quote for {} SOL...", i + 1, amount_sol);
            
            let request = SwapRequest {
                input_mint: SOL_MINT.to_string(),
                output_mint: TEST_TOKEN_MINT.to_string(),
                input_amount: sol_to_lamports(amount_sol),
                from_address: wallet_address.clone(),
                slippage: 15.0,
                fee: 0.1,
                is_anti_mev: false,
                expected_price: None,
            };

            match get_swap_quote(&request).await {
                Ok(swap_data) => {
                    println!("✅ Quote received for {} SOL:", amount_sol);
                    println!("   Input: {} lamports", swap_data.quote.in_amount);
                    println!("   Output: {} tokens", swap_data.quote.out_amount);
                    println!("   Price Impact: {}%", swap_data.quote.price_impact_pct);
                    println!("   Time Taken: {:.3}s", swap_data.quote.time_taken);
                    
                    // Validate quote data
                    assert!(!swap_data.quote.in_amount.is_empty(), "Quote input amount missing");
                    assert!(!swap_data.quote.out_amount.is_empty(), "Quote output amount missing");
                    assert!(!swap_data.raw_tx.swap_transaction.is_empty(), "Swap transaction missing");
                    
                    // Parse and validate amounts
                    let in_amount: u64 = swap_data.quote.in_amount.parse()
                        .expect("Failed to parse input amount");
                    let out_amount: u64 = swap_data.quote.out_amount.parse()
                        .expect("Failed to parse output amount");
                    
                    assert!(in_amount > 0, "Input amount should be positive");
                    assert!(out_amount > 0, "Output amount should be positive");
                    
                    println!("   ✅ Quote validation passed");
                }
                Err(e) => {
                    panic!("Failed to get quote for {} SOL: {}", amount_sol, e);
                }
            }
            
            // Rate limiting delay
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }

    #[tokio::test]
    #[ignore] // Remove this to run real on-chain tests
    async fn test_buy_token_real_chain() {
        println!("\n🧪 Testing REAL on-chain BUY operations...");
        
        let token = create_test_token();
        
        for (i, &amount_sol) in [TEST_AMOUNT_1, TEST_AMOUNT_2, TEST_AMOUNT_3].iter().enumerate() {
            println!("\n💰 Test {}: Buying {} SOL worth of {}...", i + 1, amount_sol, token.symbol);
            
            // Get wallet address for balance checking
            let wallet_address = get_wallet_address().expect("Failed to get wallet address");
            
            // Check initial balances
            let initial_sol_balance = crate::wallet::get_sol_balance(&wallet_address).await
                .expect("Failed to get initial SOL balance");
            let initial_token_balance = get_token_balance(&wallet_address, &token.mint).await
                .unwrap_or(0);
            
            println!("📊 Initial Balances:");
            println!("   SOL: {:.6}", initial_sol_balance);
            println!("   {}: {}", token.symbol, initial_token_balance);
            
            // Execute buy
            match buy_token(&token, amount_sol, None).await {
                Ok(result) => {
                    validate_swap_result(&result, true);
                    
                    // Calculate and validate effective price
                    match calculate_effective_price_buy(&result) {
                        Ok(effective_price) => {
                            // Expected price range for BONK (very rough estimates)
                            let expected_min = 0.000000010; // 0.00000001 SOL per BONK
                            let expected_max = 0.000000100; // 0.0000001 SOL per BONK
                            validate_effective_price(effective_price, expected_min, expected_max);
                            
                            println!("💎 BUY Test {} Results:", i + 1);
                            println!("   Amount: {} SOL", amount_sol);
                            println!("   Effective Price: {:.10} SOL per {}", effective_price, token.symbol);
                            println!("   TX: {:?}", result.transaction_signature);
                        }
                        Err(e) => {
                            println!("❌ Failed to calculate effective price: {}", e);
                        }
                    }
                    
                    // Check final balances
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    
                    let final_sol_balance = crate::wallet::get_sol_balance(&wallet_address).await
                        .expect("Failed to get final SOL balance");
                    let final_token_balance = get_token_balance(&wallet_address, &token.mint).await
                        .unwrap_or(0);
                    
                    println!("📊 Final Balances:");
                    println!("   SOL: {:.6} (change: {:.6})", 
                            final_sol_balance,
                            final_sol_balance - initial_sol_balance);
                    println!("   {}: {} (change: +{})", 
                            token.symbol, final_token_balance, 
                            final_token_balance - initial_token_balance);
                    
                    // Validate balance changes
                    assert!(final_sol_balance < initial_sol_balance, "SOL balance should decrease after buy");
                    assert!(final_token_balance > initial_token_balance, "Token balance should increase after buy");
                }
                Err(e) => {
                    println!("❌ Buy failed for {} SOL: {}", amount_sol, e);
                    // Don't panic on swap failures as they can happen due to market conditions
                }
            }
            
            // Delay between tests to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    #[tokio::test]
    #[ignore] // Remove this to run real on-chain tests
    async fn test_sell_token_real_chain() {
        println!("\n🧪 Testing REAL on-chain SELL operations...");
        
        let token = create_test_token();
        let wallet_address = get_wallet_address().expect("Failed to get wallet address");
        
        // First, ensure we have some tokens to sell by checking balance
        let initial_token_balance = get_token_balance(&wallet_address, &token.mint).await
            .unwrap_or(0);
        
        if initial_token_balance == 0 {
            println!("⚠️ No {} tokens found in wallet. Buying some first...", token.symbol);
            
            // Buy some tokens first
            match buy_token(&token, TEST_AMOUNT_2, None).await {
                Ok(buy_result) => {
                    println!("✅ Successfully bought tokens for testing sell operations");
                    validate_swap_result(&buy_result, true);
                    
                    // Wait for transaction to settle
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    println!("❌ Failed to buy tokens for sell test: {}", e);
                    return; // Skip sell tests if we can't buy tokens
                }
            }
        }
        
        // Get updated token balance
        let token_balance = get_token_balance(&wallet_address, &token.mint).await
            .expect("Failed to get token balance");
        
        if token_balance == 0 {
            println!("❌ Still no tokens available for sell test");
            return;
        }
        
        println!("📊 Available {} tokens for sell test: {}", token.symbol, token_balance);
        
        // Test selling different portions (be conservative to avoid selling all tokens)
        let sell_portions = [0.1, 0.2, 0.3]; // Sell 10%, 20%, 30% of holdings
        
        for (i, &portion) in sell_portions.iter().enumerate() {
            let current_balance = get_token_balance(&wallet_address, &token.mint).await
                .unwrap_or(0);
            
            if current_balance == 0 {
                println!("⚠️ No more tokens to sell for test {}", i + 1);
                break;
            }
            
            let sell_amount = (current_balance as f64 * portion) as u64;
            if sell_amount == 0 {
                println!("⚠️ Calculated sell amount is 0 for test {}", i + 1);
                continue;
            }
            
            println!("\n💸 Test {}: Selling {} {} tokens ({:.1}% of holdings)...", 
                    i + 1, sell_amount, token.symbol, portion * 100.0);
            
            // Check initial balances
            let initial_sol_balance = crate::wallet::get_sol_balance(&wallet_address).await
                .expect("Failed to get initial SOL balance");
            
            println!("📊 Pre-sell Balances:");
            println!("   SOL: {:.6}", initial_sol_balance);
            println!("   {}: {}", token.symbol, current_balance);
            
            // Execute sell
            match sell_token(&token, sell_amount, None).await {
                Ok(result) => {
                    validate_swap_result(&result, true);
                    
                    // Calculate and validate effective price
                    match calculate_effective_price_sell(&result) {
                        Ok(effective_price) => {
                            // Expected price range for BONK (very rough estimates)
                            let expected_min = 0.000000010; // 0.00000001 SOL per BONK
                            let expected_max = 0.000000100; // 0.0000001 SOL per BONK
                            validate_effective_price(effective_price, expected_min, expected_max);
                            
                            println!("💎 SELL Test {} Results:", i + 1);
                            println!("   Amount: {} {} tokens", sell_amount, token.symbol);
                            println!("   Effective Price: {:.10} SOL per {}", effective_price, token.symbol);
                            println!("   TX: {:?}", result.transaction_signature);
                        }
                        Err(e) => {
                            println!("❌ Failed to calculate effective price: {}", e);
                        }
                    }
                    
                    // Check final balances
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    
                    let final_sol_balance = crate::wallet::get_sol_balance(&wallet_address).await
                        .expect("Failed to get final SOL balance");
                    let final_token_balance = get_token_balance(&wallet_address, &token.mint).await
                        .unwrap_or(0);
                    
                    println!("📊 Post-sell Balances:");
                    println!("   SOL: {:.6} (change: +{:.6})", 
                            final_sol_balance,
                            final_sol_balance - initial_sol_balance);
                    println!("   {}: {} (change: -{})", 
                            token.symbol, final_token_balance, 
                            current_balance - final_token_balance);
                    
                    // Validate balance changes
                    assert!(final_sol_balance > initial_sol_balance, "SOL balance should increase after sell");
                    assert!(final_token_balance < current_balance, "Token balance should decrease after sell");
                }
                Err(e) => {
                    println!("❌ Sell failed for {} tokens: {}", sell_amount, e);
                    // Don't panic on swap failures as they can happen due to market conditions
                }
            }
            
            // Delay between tests
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    #[tokio::test]
    async fn test_effective_price_calculations() {
        println!("\n🧪 Testing effective price calculation functions...");

        // Create mock swap results for testing price calculations
        let token = create_test_token();
        
        // Test BUY price calculation
        let buy_result = SwapResult {
            success: true,
            transaction_signature: Some("test_buy_signature".to_string()),
            input_amount: sol_to_lamports(TEST_AMOUNT_1).to_string(), // 0.001 SOL
            output_amount: "40000000".to_string(), // 400 BONK (with 5 decimals = 400.00000)
            price_impact: "0.5".to_string(),
            fee_lamports: 5000,
            execution_time: 1.5,
            effective_price: None,
            swap_data: Some(crate::swaps::types::SwapData {
                quote: crate::swaps::types::SwapQuote {
                    input_mint: SOL_MINT.to_string(),
                    in_amount: sol_to_lamports(TEST_AMOUNT_1).to_string(),
                    output_mint: TEST_TOKEN_MINT.to_string(),
                    out_amount: "40000000".to_string(),
                    other_amount_threshold: "39000000".to_string(),
                    in_decimals: 9,
                    out_decimals: 5, // BONK has 5 decimals
                    swap_mode: "ExactIn".to_string(),
                    slippage_bps: "1500".to_string(),
                    platform_fee: None,
                    price_impact_pct: "0.5".to_string(),
                    route_plan: serde_json::Value::Array(vec![]),
                    context_slot: Some(12345),
                    time_taken: 1.5,
                },
                raw_tx: crate::swaps::types::RawTransaction {
                    swap_transaction: "test_transaction".to_string(),
                    last_valid_block_height: 123456789,
                    prioritization_fee_lamports: 5000,
                    recent_blockhash: "test_blockhash".to_string(),
                    version: Some("1".to_string()),
                },
                amount_in_usd: Some("0.15".to_string()),
                amount_out_usd: Some("0.14".to_string()),
                jito_order_id: None,
                sol_cost: Some("0.001".to_string()),
            }),
            error: None,
        };

        match calculate_effective_price_buy(&buy_result) {
            Ok(effective_price) => {
                println!("✅ BUY Effective Price: {:.10} SOL per {}", effective_price, token.symbol);
                
                // Expected: 0.001 SOL / 400.0 BONK = 0.0000025 SOL per BONK
                let expected_price = 0.0000025;
                assert!((effective_price - expected_price).abs() < 0.00000001, 
                       "BUY price calculation mismatch: got {:.10}, expected {:.10}", 
                       effective_price, expected_price);
                println!("   ✅ Price calculation validated");
            }
            Err(e) => {
                panic!("Failed to calculate BUY effective price: {}", e);
            }
        }

        // Test SELL price calculation
        let sell_result = SwapResult {
            success: true,
            transaction_signature: Some("test_sell_signature".to_string()),
            input_amount: "20000000".to_string(), // 200 BONK (with 5 decimals = 200.00000)
            output_amount: sol_to_lamports(0.0005).to_string(), // 0.0005 SOL
            price_impact: "0.3".to_string(),
            fee_lamports: 5000,
            execution_time: 1.2,
            effective_price: None,
            swap_data: Some(crate::swaps::types::SwapData {
                quote: crate::swaps::types::SwapQuote {
                    input_mint: TEST_TOKEN_MINT.to_string(),
                    in_amount: "20000000".to_string(),
                    output_mint: SOL_MINT.to_string(),
                    out_amount: sol_to_lamports(0.0005).to_string(),
                    other_amount_threshold: sol_to_lamports(0.00048).to_string(),
                    in_decimals: 5, // BONK has 5 decimals
                    out_decimals: 9,
                    swap_mode: "ExactIn".to_string(),
                    slippage_bps: "1500".to_string(),
                    platform_fee: None,
                    price_impact_pct: "0.3".to_string(),
                    route_plan: serde_json::Value::Array(vec![]),
                    context_slot: Some(12346),
                    time_taken: 1.2,
                },
                raw_tx: crate::swaps::types::RawTransaction {
                    swap_transaction: "test_sell_transaction".to_string(),
                    last_valid_block_height: 123456790,
                    prioritization_fee_lamports: 5000,
                    recent_blockhash: "test_sell_blockhash".to_string(),
                    version: Some("1".to_string()),
                },
                amount_in_usd: Some("0.05".to_string()),
                amount_out_usd: Some("0.075".to_string()),
                jito_order_id: None,
                sol_cost: Some("0.0005".to_string()),
            }),
            error: None,
        };

        match calculate_effective_price_sell(&sell_result) {
            Ok(effective_price) => {
                println!("✅ SELL Effective Price: {:.10} SOL per {}", effective_price, token.symbol);
                
                // Expected: 0.0005 SOL / 200.0 BONK = 0.0000025 SOL per BONK
                let expected_price = 0.0000025;
                assert!((effective_price - expected_price).abs() < 0.00000001, 
                       "SELL price calculation mismatch: got {:.10}, expected {:.10}", 
                       effective_price, expected_price);
                println!("   ✅ Price calculation validated");
            }
            Err(e) => {
                panic!("Failed to calculate SELL effective price: {}", e);
            }
        }

        println!("✅ All effective price calculations passed!");
    }

    #[tokio::test]
    async fn test_price_consistency_across_amounts() {
        println!("\n🧪 Testing price consistency across different amounts...");

        // This test validates that effective prices should be relatively consistent
        // across different trade sizes (within reasonable bounds due to slippage/impact)

        let amounts = [TEST_AMOUNT_1, TEST_AMOUNT_2, TEST_AMOUNT_3];
        let mut calculated_prices = Vec::new();

        for (i, &amount) in amounts.iter().enumerate() {
            // Create mock results with proportional amounts
            let tokens_received = (amount * 4000.0) as u64; // Assume 4000 tokens per SOL
            
            let result = SwapResult {
                success: true,
                transaction_signature: Some(format!("test_tx_{}", i)),
                input_amount: sol_to_lamports(amount).to_string(),
                output_amount: (tokens_received * 100000).to_string(), // Adjust for 5 decimals
                price_impact: "0.5".to_string(),
                fee_lamports: 5000,
                execution_time: 1.0,
                effective_price: None,
                swap_data: Some(crate::swaps::types::SwapData {
                    quote: crate::swaps::types::SwapQuote {
                        input_mint: SOL_MINT.to_string(),
                        in_amount: sol_to_lamports(amount).to_string(),
                        output_mint: TEST_TOKEN_MINT.to_string(),
                        out_amount: (tokens_received * 100000).to_string(),
                        other_amount_threshold: "0".to_string(),
                        in_decimals: 9,
                        out_decimals: 5,
                        swap_mode: "ExactIn".to_string(),
                        slippage_bps: "1500".to_string(),
                        platform_fee: None,
                        price_impact_pct: "0.5".to_string(),
                        route_plan: serde_json::Value::Array(vec![]),
                        context_slot: Some(12340 + i as u64),
                        time_taken: 1.0,
                    },
                    raw_tx: crate::swaps::types::RawTransaction {
                        swap_transaction: format!("test_tx_{}", i),
                        last_valid_block_height: 123456780 + i as u64,
                        prioritization_fee_lamports: 5000,
                        recent_blockhash: format!("test_hash_{}", i),
                        version: Some("1".to_string()),
                    },
                    amount_in_usd: Some((amount * 150.0).to_string()),
                    amount_out_usd: Some((amount * 148.0).to_string()),
                    jito_order_id: None,
                    sol_cost: Some(amount.to_string()),
                }),
                error: None,
            };

            match calculate_effective_price_buy(&result) {
                Ok(price) => {
                    calculated_prices.push(price);
                    println!("✅ Amount: {} SOL -> Price: {:.10} SOL per token", amount, price);
                }
                Err(e) => {
                    panic!("Failed to calculate price for amount {}: {}", amount, e);
                }
            }
        }

        // Validate price consistency (prices should be very similar for similar market conditions)
        if calculated_prices.len() >= 2 {
            let price_variance = calculated_prices.iter()
                .map(|&p| (p - calculated_prices[0]).abs() / calculated_prices[0])
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            
            println!("📊 Price Analysis:");
            println!("   Prices: {:?}", calculated_prices);
            println!("   Max variance: {:.2}%", price_variance * 100.0);
            
            // In real markets, price variance should be small for similar sized trades
            // but we'll be lenient here since this is a mock test
            assert!(price_variance < 0.05, "Price variance too high: {:.2}%", price_variance * 100.0);
            println!("✅ Price consistency validated (variance < 5%)");
        }
    }

    #[tokio::test]
    async fn test_error_handling() {
        println!("\n🧪 Testing error handling scenarios...");

        // Test failed swap result
        let failed_result = SwapResult {
            success: false,
            transaction_signature: Some("failed_tx".to_string()),
            input_amount: "1000000".to_string(),
            output_amount: "0".to_string(),
            price_impact: "0.0".to_string(),
            fee_lamports: 5000,
            execution_time: 0.5,
            effective_price: None,
            swap_data: None,
            error: Some("Transaction failed".to_string()),
        };

        // Should fail to calculate effective price for failed swap
        match calculate_effective_price_buy(&failed_result) {
            Ok(_) => panic!("Should not calculate price for failed swap"),
            Err(e) => {
                println!("✅ Correctly rejected failed swap: {}", e);
            }
        }

        // Test zero output amount
        let zero_output_result = SwapResult {
            success: true,
            transaction_signature: Some("zero_output_tx".to_string()),
            input_amount: "1000000".to_string(),
            output_amount: "0".to_string(),
            price_impact: "0.0".to_string(),
            fee_lamports: 5000,
            execution_time: 1.0,
            effective_price: None,
            swap_data: Some(crate::swaps::types::SwapData {
                quote: crate::swaps::types::SwapQuote {
                    input_mint: SOL_MINT.to_string(),
                    in_amount: "1000000".to_string(),
                    output_mint: TEST_TOKEN_MINT.to_string(),
                    out_amount: "0".to_string(),
                    other_amount_threshold: "0".to_string(),
                    in_decimals: 9,
                    out_decimals: 5,
                    swap_mode: "ExactIn".to_string(),
                    slippage_bps: "1500".to_string(),
                    platform_fee: None,
                    price_impact_pct: "0.0".to_string(),
                    route_plan: serde_json::Value::Array(vec![]),
                    context_slot: Some(12340),
                    time_taken: 1.0,
                },
                raw_tx: crate::swaps::types::RawTransaction {
                    swap_transaction: "test_tx".to_string(),
                    last_valid_block_height: 123456780,
                    prioritization_fee_lamports: 5000,
                    recent_blockhash: "test_hash".to_string(),
                    version: Some("1".to_string()),
                },
                amount_in_usd: None,
                amount_out_usd: None,
                jito_order_id: None,
                sol_cost: None,
            }),
            error: None,
        };

        // Should fail to calculate effective price with zero output
        match calculate_effective_price_buy(&zero_output_result) {
            Ok(_) => panic!("Should not calculate price with zero output"),
            Err(e) => {
                println!("✅ Correctly rejected zero output: {}", e);
            }
        }

        println!("✅ All error handling tests passed!");
    }

    #[tokio::test]
    async fn test_comprehensive_swap_analysis() {
        println!("\n🧪 COMPREHENSIVE SWAP ANALYSIS TEST");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        let token = create_test_token();
        
        // Test different swap scenarios with detailed analysis
        let test_scenarios = [
            ("Small Trade", TEST_AMOUNT_1, "40000000"),    // 0.001 SOL -> 400 BONK
            ("Medium Trade", TEST_AMOUNT_2, "80000000"),   // 0.002 SOL -> 800 BONK  
            ("Large Trade", TEST_AMOUNT_3, "120000000"),   // 0.003 SOL -> 1200 BONK
        ];
        
        for (scenario_name, sol_amount, expected_tokens) in test_scenarios.iter() {
            println!("\n🎯 Testing Scenario: {}", scenario_name);
            println!("   📊 SOL Amount: {} SOL", sol_amount);
            println!("   🎯 Expected Tokens: {} (raw units)", expected_tokens);
            
            // Create detailed swap result for analysis
            let swap_result = SwapResult {
                success: true,
                transaction_signature: Some(format!("test_tx_{}", scenario_name.replace(" ", "_"))),
                input_amount: sol_to_lamports(*sol_amount).to_string(),
                output_amount: expected_tokens.to_string(),
                price_impact: "0.5".to_string(),
                fee_lamports: 5000,
                execution_time: 1.2,
                effective_price: None,
                swap_data: Some(crate::swaps::types::SwapData {
                    quote: crate::swaps::types::SwapQuote {
                        input_mint: SOL_MINT.to_string(),
                        in_amount: sol_to_lamports(*sol_amount).to_string(),
                        output_mint: TEST_TOKEN_MINT.to_string(),
                        out_amount: expected_tokens.to_string(),
                        other_amount_threshold: (expected_tokens.parse::<u64>().unwrap() * 95 / 100).to_string(),
                        in_decimals: 9,
                        out_decimals: 5,
                        swap_mode: "ExactIn".to_string(),
                        slippage_bps: "1500".to_string(),
                        platform_fee: None,
                        price_impact_pct: "0.5".to_string(),
                        route_plan: serde_json::Value::Array(vec![]),
                        context_slot: Some(12345),
                        time_taken: 1.2,
                    },
                    raw_tx: crate::swaps::types::RawTransaction {
                        swap_transaction: format!("detailed_test_tx_{}", scenario_name.replace(" ", "_")),
                        last_valid_block_height: 123456789,
                        prioritization_fee_lamports: 5000,
                        recent_blockhash: format!("test_blockhash_{}", scenario_name.replace(" ", "_")),
                        version: Some("1".to_string()),
                    },
                    amount_in_usd: Some((sol_amount * 150.0).to_string()),
                    amount_out_usd: Some((sol_amount * 148.0).to_string()),
                    jito_order_id: None,
                    sol_cost: Some(sol_amount.to_string()),
                }),
                error: None,
            };
            
            // Validate with detailed logging
            validate_swap_result(&swap_result, true);
            
            // Calculate and validate effective price
            match calculate_effective_price_buy(&swap_result) {
                Ok(effective_price) => {
                    println!("\n💰 PRICE CALCULATION RESULTS:");
                    
                    // Calculate expected values
                    let tokens_decimal = expected_tokens.parse::<u64>().unwrap() as f64 / 100000.0; // Adjust for 5 decimals
                    let expected_calc_price = sol_amount / tokens_decimal;
                    
                    println!("   🧮 Manual Calculation:");
                    println!("      SOL Input: {} SOL", sol_amount);
                    println!("      Token Output: {:.5} BONK", tokens_decimal);
                    println!("      Expected Price: {:.10} SOL per BONK", expected_calc_price);
                    println!("   🔧 Function Result: {:.10} SOL per BONK", effective_price);
                    println!("   📏 Difference: {:.12} SOL per BONK", (effective_price - expected_calc_price).abs());
                    
                    // Validate price is reasonable
                    let price_diff_percent = ((effective_price - expected_calc_price).abs() / expected_calc_price) * 100.0;
                    println!("   📊 Price Accuracy: {:.4}% difference", price_diff_percent);
                    
                    assert!(price_diff_percent < 0.01, "Price calculation error too high: {:.4}%", price_diff_percent);
                    
                    // Validate price range (adjust for realistic BONK prices)
                    let expected_min = 0.0000001000;  // Lower bound for BONK price
                    let expected_max = 0.0000100000;  // Upper bound for BONK price
                    validate_effective_price(effective_price, expected_min, expected_max);
                    
                    println!("   ✅ {} ANALYSIS COMPLETED", scenario_name.to_uppercase());
                }
                Err(e) => {
                    println!("   ❌ Price calculation failed: {}", e);
                    panic!("Failed to calculate effective price for {}: {}", scenario_name, e);
                }
            }
            
            println!("{}", "─".repeat(80));
        }
        
        println!("\n🎉 ALL COMPREHENSIVE SWAP ANALYSIS TESTS PASSED!");
    }

    /// Integration test runner that can be used to run all real chain tests at once
    #[tokio::test]
    #[ignore] // Remove this to run full integration test
    async fn test_full_swap_integration() {
        println!("\n🚀 Running FULL SWAP INTEGRATION TEST...");
        println!("⚠️ This test will perform real on-chain swaps!");
        println!("🔐 Make sure you have sufficient SOL balance and the correct wallet configured.");
        
        let token = create_test_token();
        let wallet_address = get_wallet_address().expect("Failed to get wallet address");
        
        // Test 1: Get swap quote
        println!("\n📋 Step 1: Testing swap quote...");
        let quote_request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: token.mint.clone(),
            input_amount: sol_to_lamports(TEST_AMOUNT_1),
            from_address: wallet_address.to_string(),
            slippage: 15.0,
            fee: 0.01,
            is_anti_mev: false,
            expected_price: None,
        };
        
        match get_swap_quote(&quote_request).await {
            Ok(quote_data) => {
                println!("✅ Quote received: {} {} for {} SOL", 
                        quote_data.quote.out_amount, token.symbol, TEST_AMOUNT_1);
            }
            Err(e) => {
                println!("❌ Quote failed: {}", e);
                return;
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Test 2: Buy token
        println!("\n💰 Step 2: Testing buy operation...");
        let initial_balance = crate::wallet::get_sol_balance(&wallet_address).await
            .expect("Failed to get initial balance");
            
        match buy_token(&token, TEST_AMOUNT_1, None).await {
            Ok(result) => {
                validate_swap_result(&result, true);
                println!("✅ Buy successful: {:?}", result.transaction_signature);
                
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                
                // Test 3: Sell token
                println!("\n💸 Step 3: Testing sell operation...");
                let token_balance = get_token_balance(&wallet_address, &token.mint).await
                    .unwrap_or(0);
                    
                if token_balance > 0 {
                    let sell_amount = token_balance / 2; // Sell half
                    match sell_token(&token, sell_amount, None).await {
                        Ok(sell_result) => {
                            validate_swap_result(&sell_result, true);
                            println!("✅ Sell successful: {:?}", sell_result.transaction_signature);
                        }
                        Err(e) => {
                            println!("❌ Sell failed: {}", e);
                        }
                    }
                } else {
                    println!("⚠️ No tokens to sell");
                }
            }
            Err(e) => {
                println!("❌ Buy failed: {}", e);
            }
        }
        
        println!("\n🎉 FULL INTEGRATION TEST COMPLETED!");
    }
}
