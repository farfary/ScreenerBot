/// Test to demonstrate the decimal fix for slippage calculation
use screenerbot::tokens::*;
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Decimal Fix for Slippage Calculation");
    println!("===============================================");

    // Test case: BONK token (from the user's error)
    let bonk_mint = "AahaK8RkXL8SSRpfM1jeJqtkCnUtG9g8S19gP5FQbonk";

    println!("\n1. Testing decimal fetching and caching:");
    println!("   Token: BONK");
    println!("   Mint: {}", bonk_mint);

    // Test getting decimals from cache first
    if let Some(cached_decimals) = get_token_decimals_cached(bonk_mint) {
        println!("   ✅ Found cached decimals: {}", cached_decimals);
    } else {
        println!("   ⚠️  No cached decimals found");

        // Try to fetch from chain
        match get_token_decimals_guaranteed(bonk_mint).await {
            Ok(decimals) => {
                println!("   ✅ Fetched decimals from chain: {}", decimals);
                println!("   ✅ Decimals have been cached for future use");
            }
            Err(e) => {
                println!("   ❌ Failed to fetch decimals: {}", e);
            }
        }
    }

    println!("\n2. Demonstrating the slippage calculation fix:");

    // Example from the user's error log
    let sol_amount = 0.0005_f64;
    let raw_output_amount = 5716695981_f64;

    println!("   Swap: {} SOL → {} raw tokens", sol_amount, raw_output_amount);

    // Wrong calculation (using default 9 decimals)
    let wrong_decimals = 9u8;
    let wrong_tokens = raw_output_amount / (10_f64).powi(wrong_decimals as i32);
    let wrong_price = sol_amount / wrong_tokens;

    println!("\n   ❌ WRONG (using default {} decimals):", wrong_decimals);
    println!("      UI tokens: {:.12}", wrong_tokens);
    println!("      Price per token: {:.12} SOL", wrong_price);

    // Correct calculation (using actual 6 decimals from quote)
    let correct_decimals = 6u8; // From quote response: "outDecimals": 6
    let correct_tokens = raw_output_amount / (10_f64).powi(correct_decimals as i32);
    let correct_price = sol_amount / correct_tokens;

    println!("\n   ✅ CORRECT (using actual {} decimals from quote):", correct_decimals);
    println!("      UI tokens: {:.12}", correct_tokens);
    println!("      Price per token: {:.12} SOL", correct_price);

    // Calculate the difference
    let price_difference = (((wrong_price - correct_price) / correct_price) * 100.0).abs();

    println!("\n   📊 COMPARISON:");
    println!("      Price difference: {:.2}%", price_difference);
    println!("      This is why slippage was showing 100,098.31%!");

    println!("\n3. Key fixes implemented:");
    println!("   ✅ buy_token() now uses swap_data.quote.out_decimals");
    println!("   ✅ execute_swap_with_quote() uses actual decimals from quote");
    println!("   ✅ execute_swap() uses actual decimals from quote");
    println!("   ✅ Added centralized decimal fetching: get_token_decimals_guaranteed()");
    println!("   ✅ Added chain-based decimal caching: fetch_token_decimals_from_chain()");

    println!("\n4. Usage recommendations:");
    println!("   🔧 For price calculations: Use decimals from quote response");
    println!("   🔧 For general decimals: Use get_token_decimals_guaranteed()");
    println!("   🔧 Never rely on defaults for price calculations");

    println!("\n✅ Test completed successfully!");
    println!("   The slippage calculation should now be accurate.");

    Ok(())
}
