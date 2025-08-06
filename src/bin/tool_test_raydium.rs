/// Tool for testing Raydium router integration
/// Tests quote fetching and API connectivity

use screenerbot::swaps::get_raydium_quote;
use screenerbot::swaps::types::SOL_MINT;
use std::env;

const BONK_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <test|quote|help>", args[0]);
        println!();
        println!("Commands:");
        println!("  test     - Test Raydium connectivity with sample quotes");
        println!("  quote    - Get specific token quote (requires token mint address)");
        println!("  help     - Show this help message");
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "test" => {
            test_raydium_connectivity().await?;
        }
        "quote" => {
            if args.len() < 3 {
                println!("Usage: {} quote <TOKEN_MINT>", args[0]);
                println!("Example: {} quote {}", args[0], BONK_MINT);
                return Ok(());
            }
            let token_mint = &args[2];
            get_specific_quote(token_mint).await?;
        }
        "help" => {
            println!("🟣 Raydium Router Test Tool");
            println!();
            println!("This tool tests the Raydium router integration in ScreenerBot.");
            println!();
            println!("Available commands:");
            println!("  test     - Run connectivity tests with known token pairs");
            println!("  quote    - Get a quote for a specific token");
            println!("  help     - Show this help message");
            println!();
            println!("Examples:");
            println!("  {} test", args[0]);
            println!("  {} quote {}", args[0], BONK_MINT);
            println!("  {} quote {}", args[0], USDC_MINT);
        }
        _ => {
            println!("Unknown command: {}", command);
            println!("Use 'help' for available commands");
        }
    }

    Ok(())
}

async fn test_raydium_connectivity() -> Result<(), Box<dyn std::error::Error>> {
    println!("🟣 Testing Raydium Router Integration");
    println!("=====================================");
    println!();

    // Test case 1: SOL to BONK
    println!("Test 1: SOL to BONK quote");
    println!("-------------------------");
    match get_raydium_quote(
        SOL_MINT,
        BONK_MINT,
        10_000_000, // 0.01 SOL
        "11111111111111111111111111111111", // Dummy address
        2.0, // 2% slippage
        0.25, // 0.25% fee
        false,
    ).await {
        Ok(quote_data) => {
            println!("✅ Raydium SOL->BONK quote successful!");
            println!("   Input: {} lamports SOL", quote_data.quote.in_amount);
            println!("   Output: {} BONK tokens", quote_data.quote.out_amount);
            println!("   Price Impact: {}%", quote_data.quote.price_impact_pct);
            println!("   Slippage: {} BPS", quote_data.quote.slippage_bps);
            println!("   Time taken: {:.3}s", quote_data.quote.time_taken);
        }
        Err(e) => {
            println!("❌ Raydium SOL->BONK quote failed: {}", e);
        }
    }

    println!();

    // Test case 2: SOL to USDC
    println!("Test 2: SOL to USDC quote");
    println!("-------------------------");
    match get_raydium_quote(
        SOL_MINT,
        USDC_MINT,
        10_000_000, // 0.01 SOL
        "11111111111111111111111111111111", // Dummy address
        1.0, // 1% slippage
        0.25, // 0.25% fee
        false,
    ).await {
        Ok(quote_data) => {
            println!("✅ Raydium SOL->USDC quote successful!");
            println!("   Input: {} lamports SOL", quote_data.quote.in_amount);
            println!("   Output: {} USDC", quote_data.quote.out_amount);
            println!("   Price Impact: {}%", quote_data.quote.price_impact_pct);
            println!("   Slippage: {} BPS", quote_data.quote.slippage_bps);
            println!("   Time taken: {:.3}s", quote_data.quote.time_taken);
        }
        Err(e) => {
            println!("❌ Raydium SOL->USDC quote failed: {}", e);
        }
    }

    println!();

    // Test case 3: USDC to SOL (reverse)
    println!("Test 3: USDC to SOL quote");
    println!("-------------------------");
    match get_raydium_quote(
        USDC_MINT,
        SOL_MINT,
        1_000_000, // 1 USDC (6 decimals)
        "11111111111111111111111111111111", // Dummy address
        1.0, // 1% slippage
        0.25, // 0.25% fee
        false,
    ).await {
        Ok(quote_data) => {
            println!("✅ Raydium USDC->SOL quote successful!");
            println!("   Input: {} USDC", quote_data.quote.in_amount);
            println!("   Output: {} lamports SOL", quote_data.quote.out_amount);
            println!("   Price Impact: {}%", quote_data.quote.price_impact_pct);
            println!("   Slippage: {} BPS", quote_data.quote.slippage_bps);
            println!("   Time taken: {:.3}s", quote_data.quote.time_taken);
        }
        Err(e) => {
            println!("❌ Raydium USDC->SOL quote failed: {}", e);
        }
    }

    println!();
    println!("🟣 Raydium integration test completed!");
    println!();
    println!("📊 Summary:");
    println!("   • Raydium router is now integrated into ScreenerBot");
    println!("   • The bot will now compare quotes from GMGN, Jupiter, and Raydium");
    println!("   • Best quote (highest output) will be automatically selected");
    println!("   • Router comparison happens on every swap operation");

    Ok(())
}

async fn get_specific_quote(token_mint: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🟣 Getting Raydium quote for token: {}", token_mint);
    println!("================================================");
    println!();

    match get_raydium_quote(
        SOL_MINT,
        token_mint,
        10_000_000, // 0.01 SOL
        "11111111111111111111111111111111", // Dummy address
        2.0, // 2% slippage
        0.25, // 0.25% fee
        false,
    ).await {
        Ok(quote_data) => {
            println!("✅ Raydium quote successful!");
            println!();
            println!("Quote Details:");
            println!("  Input Mint: {}", quote_data.quote.input_mint);
            println!("  Output Mint: {}", quote_data.quote.output_mint);
            println!("  Input Amount: {} lamports", quote_data.quote.in_amount);
            println!("  Output Amount: {} tokens", quote_data.quote.out_amount);
            println!("  Price Impact: {}%", quote_data.quote.price_impact_pct);
            println!("  Slippage BPS: {}", quote_data.quote.slippage_bps);
            println!("  Time Taken: {:.3}s", quote_data.quote.time_taken);
            println!("  Input Decimals: {}", quote_data.quote.in_decimals);
            println!("  Output Decimals: {}", quote_data.quote.out_decimals);
            println!("  Swap Mode: {}", quote_data.quote.swap_mode);
            
            // Calculate price per token
            let input_sol = quote_data.quote.in_amount.parse::<f64>().unwrap_or(0.0) / 1_000_000_000.0;
            let output_tokens = quote_data.quote.out_amount.parse::<f64>().unwrap_or(0.0) / 
                (10_f64).powi(quote_data.quote.out_decimals as i32);
            
            if output_tokens > 0.0 {
                let price_per_token = input_sol / output_tokens;
                println!("  Price per Token: {:.12} SOL", price_per_token);
            }
            
            println!();
            println!("Route Information:");
            if let Ok(route_plan) = serde_json::from_value::<Vec<serde_json::Value>>(quote_data.quote.route_plan.clone()) {
                for (i, route) in route_plan.iter().enumerate() {
                    println!("  Route {}: {}", i + 1, serde_json::to_string_pretty(route).unwrap_or_default());
                }
            }
        }
        Err(e) => {
            println!("❌ Raydium quote failed: {}", e);
            println!();
            println!("Possible reasons:");
            println!("  • Token not supported by Raydium");
            println!("  • Network connectivity issues");
            println!("  • Invalid token mint address");
            println!("  • Raydium API temporarily unavailable");
        }
    }

    println!();
    println!("💡 Tip: The bot will automatically try all available routers (GMGN, Jupiter, Raydium)");
    println!("    and select the best quote when making actual trades.");

    Ok(())
}
