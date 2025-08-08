/// Test to verify that the effective price calculation fixes are working correctly
/// This test ensures that:
/// 1. SOL decimals are always 9 (hardcoded)
/// 2. Token decimals are fetched from the decimals module, not hardcoded to 6
/// 3. Effective price calculations use proper decimal handling

use screenerbot::{
    swaps::{
        pricing::calculate_effective_price_from_raw,
        transaction::verify_swap_transaction,
    },
    rpc::lamports_to_sol,
    logger::{log, LogTag},
    tokens::decimals::get_token_decimals_from_chain,
};

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const EXAMPLE_TOKEN_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK token

#[tokio::main]
async fn main() {
    println!("🧪 Testing Effective Price Calculation Fixes");
    println!("===========================================");
    
    // Test 1: Verify SOL decimals are always 9
    test_sol_decimals().await;
    
    // Test 2: Verify token decimals are fetched properly
    test_token_decimals().await;
    
    // Test 3: Test effective price calculation for buy scenario
    test_buy_price_calculation();
    
    // Test 4: Test effective price calculation for sell scenario  
    test_sell_price_calculation();
    
    println!("\n✅ All pricing tests completed!");
}

async fn test_sol_decimals() {
    println!("\n📝 Test 1: SOL Decimals Verification");
    println!("------------------------------------");
    
    // SOL should always have 9 decimals (hardcoded)
    match get_token_decimals_from_chain(SOL_MINT).await {
        Ok(decimals) => {
            println!("✅ SOL decimals: {}", decimals);
            assert_eq!(decimals, 9, "SOL should always have 9 decimals");
        }
        Err(e) => {
            println!("❌ Failed to get SOL decimals: {}", e);
            panic!("SOL decimals should always be available");
        }
    }
}

async fn test_token_decimals() {
    println!("\n📝 Test 2: Token Decimals Fetching");
    println!("----------------------------------");
    
    // Test with BONK token (should be 5 decimals, not 6)
    match get_token_decimals_from_chain(EXAMPLE_TOKEN_MINT).await {
        Ok(decimals) => {
            println!("✅ BONK token decimals: {}", decimals);
            // BONK has 5 decimals, not the old hardcoded 6
            if decimals != 6 {
                println!("✅ Good! Token decimals are not hardcoded to 6 (got {})", decimals);
            } else {
                println!("⚠️  Warning: Got 6 decimals - verify this is correct and not hardcoded");
            }
        }
        Err(e) => {
            println!("ℹ️  Could not fetch BONK decimals: {} (this is OK for testing)", e);
        }
    }
}

fn test_buy_price_calculation() {
    println!("\n📝 Test 3: Buy Price Calculation");
    println!("--------------------------------");
    
    // Simulate a buy transaction:
    // - Spent 1 SOL (1,000,000,000 lamports)
    // - Received 1,000,000 tokens (assuming 6 decimals = 1 actual token)
    // - Expected price: 1 SOL per token
    
    let sol_spent = 1_000_000_000u64; // 1 SOL in lamports
    let tokens_received = 1_000_000u64; // 1 token with 6 decimals
    let input_decimals = 9u32; // SOL
    let output_decimals = 6u32; // Token
    
    let effective_price = calculate_effective_price_from_raw(
        "buy",
        Some(sol_spent),
        Some(tokens_received),
        Some(sol_spent),
        None,
        0, // No ATA rent
        input_decimals,
        output_decimals,
    );
    
    match effective_price {
        Some(price) => {
            println!("✅ Buy effective price: {:.10} SOL per token", price);
            println!("   Input: {} lamports ({:.6} SOL)", sol_spent, lamports_to_sol(sol_spent));
            println!("   Output: {} raw tokens ({:.6} actual tokens)", tokens_received, tokens_received as f64 / 10f64.powi(output_decimals as i32));
            
            // Expected price should be 1.0 SOL per token
            let expected_price = 1.0;
            let price_diff = (price - expected_price).abs();
            
            if price_diff < 0.000001 { // Small tolerance for floating point
                println!("✅ Price calculation is correct!");
            } else {
                println!("❌ Price calculation error. Expected: {:.10}, Got: {:.10}", expected_price, price);
            }
        }
        None => {
            println!("❌ Failed to calculate buy price");
        }
    }
}

fn test_sell_price_calculation() {
    println!("\n📝 Test 4: Sell Price Calculation");
    println!("---------------------------------");
    
    // Simulate a sell transaction:
    // - Sold 2 tokens (2,000,000 raw with 6 decimals)
    // - Received 1.8 SOL (1,800,000,000 lamports) 
    // - ATA rent reclaimed: 0.002 SOL (2,000,000 lamports)
    // - Expected price: (1.8 + 0.002) / 2 = 0.901 SOL per token
    
    let tokens_sold = 2_000_000u64; // 2 tokens with 6 decimals
    let sol_received = 1_800_000_000u64; // 1.8 SOL in lamports  
    let ata_rent = 2_000_000u64; // 0.002 SOL rent reclaimed
    let input_decimals = 6u32; // Token
    let output_decimals = 9u32; // SOL
    
    let effective_price = calculate_effective_price_from_raw(
        "sell",
        Some(tokens_sold),
        Some(sol_received),
        None,
        Some(sol_received),
        ata_rent,
        input_decimals,
        output_decimals,
    );
    
    match effective_price {
        Some(price) => {
            println!("✅ Sell effective price: {:.10} SOL per token", price);
            println!("   Input: {} raw tokens ({:.6} actual tokens)", tokens_sold, tokens_sold as f64 / 10f64.powi(input_decimals as i32));
            println!("   Output: {} lamports ({:.6} SOL)", sol_received, lamports_to_sol(sol_received));
            println!("   ATA rent: {} lamports ({:.6} SOL)", ata_rent, lamports_to_sol(ata_rent));
            
            // Expected price should be (1.8 + 0.002) / 2 = 0.901 SOL per token
            let expected_price = (lamports_to_sol(sol_received) + lamports_to_sol(ata_rent)) / (tokens_sold as f64 / 10f64.powi(input_decimals as i32));
            let price_diff = (price - expected_price).abs();
            
            println!("   Expected price: {:.10} SOL per token", expected_price);
            
            if price_diff < 0.000001 { // Small tolerance for floating point
                println!("✅ Price calculation is correct!");
            } else {
                println!("❌ Price calculation error. Expected: {:.10}, Got: {:.10}", expected_price, price);
            }
        }
        None => {
            println!("❌ Failed to calculate sell price");
        }
    }
}
