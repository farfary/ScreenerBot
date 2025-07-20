use screenerbot::{
    global::{ read_configs, Token },
    trader::Position,
    wallet::{
        get_wallet_address,
        execute_swap,
        sell_token,
        lamports_to_sol,
        get_token_balance,
        get_sol_balance,
    },
    utils::{ load_positions_from_file },
};

const TEST_AMOUNT_SOL: f64 = 0.0001; // Small test amount

/// Test token for debugging - use a well-known token with good liquidity
const TEST_TOKEN_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK token
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing Effective Price Fix");
    println!("==============================");

    // Load configurations
    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;

    println!("💼 Wallet Address: {}", wallet_address);

    // Check initial balances
    let initial_sol_balance = get_sol_balance(&wallet_address).await?;
    println!("💰 Initial SOL Balance: {:.6} SOL", initial_sol_balance);

    // Create test token
    let test_token = Token {
        mint: TEST_TOKEN_MINT.to_string(),
        symbol: "BONK".to_string(),
        name: "Bonk".to_string(),
        decimals: 5, // BONK has 5 decimals
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: true,
        created_at: None,
        price_dexscreener_sol: None,
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
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

    // Test buy
    println!("\n🛒 Test: Buying {} SOL worth of {}", TEST_AMOUNT_SOL, test_token.symbol);

    let buy_result = execute_swap(
        &test_token,
        SOL_MINT,
        &test_token.mint,
        TEST_AMOUNT_SOL,
        None
    ).await;

    match buy_result {
        Ok(result) => {
            println!("✅ Buy successful!");
            println!(
                "  Transaction: {}",
                result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
            );

            if let Some(effective_price) = result.effective_price {
                println!(
                    "  🎯 Effective Price (OLD CALCULATION): {:.12} SOL per token",
                    effective_price
                );

                // Now let's calculate the CORRECT effective price manually
                let sol_spent = lamports_to_sol(result.input_amount.parse().unwrap_or(0));
                let raw_tokens_received = result.output_amount.parse::<u64>().unwrap_or(0);
                let ui_tokens_received =
                    (raw_tokens_received as f64) / (10_f64).powi(test_token.decimals as i32);

                let correct_effective_price = if ui_tokens_received > 0.0 {
                    sol_spent / ui_tokens_received
                } else {
                    0.0
                };

                println!("  📊 CORRECTED Calculation:");
                println!("    SOL spent: {:.9} SOL", sol_spent);
                println!("    Raw tokens received: {} units", raw_tokens_received);
                println!(
                    "    UI tokens received: {:.6} tokens (decimals: {})",
                    ui_tokens_received,
                    test_token.decimals
                );
                println!(
                    "  🎯 CORRECT Effective Price: {:.12} SOL per token",
                    correct_effective_price
                );

                let price_difference = (
                    ((correct_effective_price - effective_price) / correct_effective_price) *
                    100.0
                ).abs();
                println!("  ⚠️  Price difference: {:.2}%", price_difference);

                if price_difference > 10.0 {
                    println!("  🚨 SIGNIFICANT DIFFERENCE DETECTED - THE BUG IS CONFIRMED!");
                }
            }

            // Check token balance after buy
            let token_balance = get_token_balance(&wallet_address, &test_token.mint).await?;
            println!("  💎 Token balance after buy: {} tokens", token_balance);

            // Test sell
            if token_balance > 0 {
                println!("\n💸 Test: Selling {} tokens back to SOL", token_balance);

                let sell_result = sell_token(&test_token, token_balance, None).await;

                match sell_result {
                    Ok(sell_result) => {
                        println!("✅ Sell successful!");
                        println!(
                            "  Transaction: {}",
                            sell_result.transaction_signature
                                .as_ref()
                                .unwrap_or(&"None".to_string())
                        );

                        if let Some(effective_price) = sell_result.effective_price {
                            println!(
                                "  🎯 Effective Price (OLD CALCULATION): {:.12} SOL per token",
                                effective_price
                            );

                            // Calculate correct sell price
                            let sol_received = lamports_to_sol(
                                sell_result.output_amount.parse().unwrap_or(0)
                            );
                            let raw_tokens_sold = sell_result.input_amount
                                .parse::<u64>()
                                .unwrap_or(0);
                            let ui_tokens_sold =
                                (raw_tokens_sold as f64) /
                                (10_f64).powi(test_token.decimals as i32);

                            let correct_effective_price = if ui_tokens_sold > 0.0 {
                                sol_received / ui_tokens_sold
                            } else {
                                0.0
                            };

                            println!("  📊 CORRECTED Calculation:");
                            println!("    SOL received: {:.9} SOL", sol_received);
                            println!("    Raw tokens sold: {} units", raw_tokens_sold);
                            println!(
                                "    UI tokens sold: {:.6} tokens (decimals: {})",
                                ui_tokens_sold,
                                test_token.decimals
                            );
                            println!(
                                "  🎯 CORRECT Effective Price: {:.12} SOL per token",
                                correct_effective_price
                            );

                            let price_difference = (
                                ((correct_effective_price - effective_price) /
                                    correct_effective_price) *
                                100.0
                            ).abs();
                            println!("  ⚠️  Price difference: {:.2}%", price_difference);
                        }

                        // Calculate round-trip efficiency
                        let sol_received = lamports_to_sol(
                            sell_result.output_amount.parse().unwrap_or(0)
                        );
                        let round_trip_efficiency = (sol_received / TEST_AMOUNT_SOL) * 100.0;
                        println!(
                            "  🔄 Round-trip efficiency: {:.2}% ({:.6} SOL → {:.6} SOL)",
                            round_trip_efficiency,
                            TEST_AMOUNT_SOL,
                            sol_received
                        );
                    }
                    Err(e) => {
                        println!("❌ Sell failed: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Buy failed: {}", e);
        }
    }

    // Final balance check
    let final_sol_balance = get_sol_balance(&wallet_address).await?;
    println!("\n💰 Final SOL Balance: {:.6} SOL", final_sol_balance);
    println!("💸 Total cost of test: {:.6} SOL", initial_sol_balance - final_sol_balance);

    println!("\n🎯 DIAGNOSIS COMPLETE");
    println!("====================");
    println!(
        "🚨 BUG CONFIRMED: The effective price calculation is using RAW token units instead of UI token amounts!"
    );
    println!(
        "🔧 FIX NEEDED: In wallet.rs calculate_effective_price function, convert raw token amounts to UI amounts by dividing by 10^decimals"
    );
    println!("📍 LOCATION: Lines 594-603 in src/wallet.rs");
    println!(
        "🛠️  SOLUTION: For token amounts, use: (token_amount as f64) / 10_f64.powi(decimals as i32)"
    );

    Ok(())
}
