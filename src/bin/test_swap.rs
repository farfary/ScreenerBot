use screenerbot::wallet::{
    buy_token,
    sell_token,
    get_token_price_sol,
    get_sol_balance,
    get_token_balance,
    get_wallet_address,
    validate_price_near_expected,
};
use screenerbot::global::Token;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing Swap Function with Balance & Price Validation");
    println!("=========================================================");

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => {
            println!("👛 Wallet Address: {}", addr);
            addr
        }
        Err(e) => {
            println!("❌ Failed to get wallet address: {}", e);
            return Ok(());
        }
    };

    // Check initial balances
    println!("\n💰 Checking initial balances...");
    match get_sol_balance(&wallet_address).await {
        Ok(balance) => println!("   SOL Balance: {:.6} SOL", balance),
        Err(e) => println!("   ❌ Failed to get SOL balance: {}", e),
    }

    // Example token (BONK)
    let test_token = Token {
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(),
        symbol: "BONK".to_string(),
        name: "Bonk".to_string(),
        decimals: 5,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: true,
        created_at: None,
        price_dexscreener_sol: Some(0.000001),
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],

        // New DexScreener fields
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

    match get_token_balance(&wallet_address, &test_token.mint).await {
        Ok(balance) => println!("   {} Balance: {} tokens", test_token.symbol, balance),
        Err(e) => println!("   ❌ Failed to get {} balance: {}", test_token.symbol, e),
    }

    println!("\n📈 Getting current token price...");
    let current_price = match get_token_price_sol(&test_token.mint).await {
        Ok(price) => {
            println!("✅ Current BONK price: {:.12} SOL", price);
            price
        }
        Err(e) => {
            println!("❌ Failed to get price: {}", e);
            return Ok(());
        }
    };

    // Test price validation
    println!("\n🎯 Testing price validation...");
    let expected_price = current_price;
    let near_price = current_price * 1.02; // 2% higher
    let far_price = current_price * 1.1; // 10% higher

    println!("   Current price: {:.12} SOL", current_price);
    println!("   Expected price: {:.12} SOL", expected_price);
    println!("   Near price (+2%): {:.12} SOL", near_price);
    println!("   Far price (+10%): {:.12} SOL", far_price);

    println!(
        "   ✅ Current vs Expected (same): {}",
        validate_price_near_expected(current_price, expected_price, 5.0)
    );
    println!(
        "   ✅ Current vs Near (+2%): {}",
        validate_price_near_expected(current_price, near_price, 5.0)
    );
    println!(
        "   ❌ Current vs Far (+10%): {}",
        validate_price_near_expected(current_price, far_price, 5.0)
    );

    println!("\n💰 Testing buy operation with price validation (0.001 SOL -> BONK)...");
    match buy_token(&test_token, 0.001, Some(current_price)).await {
        Ok(result) => {
            println!("✅ Buy operation successful!");
            println!("   Input: {} lamports", result.input_amount);
            println!("   Output: {} tokens", result.output_amount);
            println!("   Price Impact: {}%", result.price_impact);
            println!("   Fee: {} lamports", result.fee_lamports);
            println!("   Execution Time: {:.3}s", result.execution_time);
            if let Some(effective_price) = result.effective_price {
                println!("   🎯 Effective Price: {:.12} SOL per token", effective_price);
            }
            if
                let (Some(actual_in), Some(actual_out)) = (
                    result.actual_input_change,
                    result.actual_output_change,
                )
            {
                println!(
                    "   📊 Actual Changes: {} lamports SOL -> {} tokens",
                    actual_in,
                    actual_out
                );
            }
        }
        Err(e) => println!("❌ Buy failed: {}", e),
    }

    println!("\n💸 Testing buy with unrealistic expected price (should fail)...");
    let unrealistic_price = current_price * 0.5; // 50% lower than current
    println!("   Trying to buy with expected price: {:.12} SOL (50% lower)", unrealistic_price);
    match buy_token(&test_token, 0.001, Some(unrealistic_price)).await {
        Ok(_) => println!("   ⚠️  Unexpected success - price validation might be off"),
        Err(e) => println!("   ✅ Expected failure: {}", e),
    }

    println!("\n💸 Testing sell operation with validation...");
    // First check if we have BONK tokens to sell
    match get_token_balance(&wallet_address, &test_token.mint).await {
        Ok(token_balance) => {
            if token_balance > 0 {
                let sell_amount = std::cmp::min(token_balance, 100000000u64); // Sell up to 100M BONK
                println!("   Selling {} BONK tokens (of {} available)", sell_amount, token_balance);

                let expected_sol_output = current_price * (sell_amount as f64);
                println!("   Expected SOL output: {:.6} SOL", expected_sol_output);

                match sell_token(&test_token, sell_amount, Some(expected_sol_output)).await {
                    Ok(result) => {
                        println!("✅ Sell operation successful!");
                        println!("   Input: {} tokens", result.input_amount);
                        println!("   Output: {} lamports", result.output_amount);
                        println!(
                            "   Output SOL: {:.6} SOL",
                            screenerbot::wallet::lamports_to_sol(
                                result.output_amount.parse().unwrap_or(0)
                            )
                        );
                        println!("   Price Impact: {}%", result.price_impact);
                        println!("   Fee: {} lamports", result.fee_lamports);
                        println!("   Execution Time: {:.3}s", result.execution_time);
                        if let Some(effective_price) = result.effective_price {
                            println!(
                                "   🎯 Effective Price: {:.12} SOL per token",
                                effective_price
                            );
                        }
                        if
                            let (Some(actual_in), Some(actual_out)) = (
                                result.actual_input_change,
                                result.actual_output_change,
                            )
                        {
                            println!(
                                "   📊 Actual Changes: {} tokens -> {} lamports SOL",
                                actual_in,
                                actual_out
                            );
                        }
                    }
                    Err(e) => println!("❌ Sell failed: {}", e),
                }
            } else {
                println!("   ⚠️  No BONK tokens available to sell");
            }
        }
        Err(e) => println!("   ❌ Failed to check token balance: {}", e),
    }

    println!("\n✨ Swap function testing completed!");

    Ok(())
}
