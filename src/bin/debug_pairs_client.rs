use anyhow::Result;
use screenerbot::pairs::PairsClient;
use screenerbot::trader::database::TraderDatabase;
use screenerbot::config::Config;
use screenerbot::api::dexscreener_rate_limiter::init_dexscreener_rate_limiter;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🔍 ScreenerBot Pairs Client Debug Tool");
    println!("═══════════════════════════════════════");

    // Load config and initialize rate limiter
    let config = Config::load("configs.json")?;
    init_dexscreener_rate_limiter(config.dexscreener).await?;
    println!("✅ DexScreener rate limiter initialized");

    // Create pairs client
    let pairs_client = PairsClient::new()?;

    // Test with tokens from your positions table
    let problematic_tokens = vec![
        "9TnmBWQZx2oRw4XdNyJ77WLpcB2XBhYRubggGLLGBfGPBSa89sN2", // bonk
        "56UDDutoJdGpYFCH5H3fkxUXYdqgdJVZLT9yJ43URktCBhKb62m", // bonk
        "4WvhhstNGhH3vG3Yoe1oGmtGx3EHgRjF5XYkP7LfwB8HBqSr8Tx", // bonk
        "DxfeBp2JW6v3QQqJq6R7Sv5qYqVZqAoXeBtGJ8Ub5iqP2qCaRmt", // pump
        "CxrGV4YZnYfF8KG3aFb8ZRqVKsHxJQzP3dMsHGdLmQ2CtShYPTu", // bonk
        "3dtcLyNgZkLQ8jR2fHnC6pQ9TtVGJXqCRqHrQB7tLQs8CMsQKdW" // bonk
    ];

    for token in &problematic_tokens {
        println!("\n🧪 Testing problematic token: {}", token);
        println!("{}", "─".repeat(60));

        test_pairs_client_methods(&pairs_client, token).await;

        // Add delay between requests
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Test database access
    println!("\n🧪 Testing database access");
    println!("{}", "─".repeat(60));
    test_database_access().await?;

    Ok(())
}

async fn test_pairs_client_methods(pairs_client: &PairsClient, token_address: &str) {
    println!("📡 Testing get_solana_token_pairs...");

    match pairs_client.get_solana_token_pairs(token_address).await {
        Ok(pairs) => {
            println!("✅ get_solana_token_pairs: Found {} pairs", pairs.len());

            if !pairs.is_empty() {
                let first_pair = &pairs[0];
                println!("   First pair: {} on {}", first_pair.pair_address, first_pair.dex_id);

                // Test get_best_pair
                match pairs_client.get_best_pair(pairs.clone()) {
                    Some(best_pair) => {
                        println!("✅ get_best_pair: Found best pair on {}", best_pair.dex_id);

                        // Test quality score
                        let score = pairs_client.calculate_pool_quality_score(&best_pair);
                        println!("✅ Pool quality score: {:.1}", score);

                        // Test price parsing
                        match best_pair.price_usd_float() {
                            Ok(price) => {
                                println!("✅ Price parsing: ${:.10}", price);
                            }
                            Err(e) => {
                                println!("❌ Price parsing failed: {}", e);
                                println!("   Raw price_usd: '{}'", best_pair.price_usd);
                            }
                        }
                    }
                    None => {
                        println!("❌ get_best_pair: No suitable pair found");
                        println!("   Pairs found but filtered out by quality checks");
                        for (i, pair) in pairs.iter().enumerate() {
                            let liquidity_usd = pair.liquidity.as_ref().map_or(0.0, |l| l.usd);
                            let tx_count = pair.total_transactions_24h();
                            println!(
                                "   Pair {}: {} liquidity=${:.2} tx_24h={}",
                                i + 1,
                                pair.dex_id,
                                liquidity_usd,
                                tx_count
                            );
                        }
                    }
                }
            }

            // Test get_best_price
            println!("📡 Testing get_best_price...");
            match pairs_client.get_best_price(token_address).await {
                Ok(Some(price)) => {
                    println!("✅ get_best_price: ${:.10}", price);
                }
                Ok(None) => {
                    println!("⚠️  get_best_price: No price found");
                }
                Err(e) => {
                    println!("❌ get_best_price failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ get_solana_token_pairs failed: {}", e);

            // Check if it's a parsing error
            if e.to_string().contains("Failed to parse JSON response") {
                println!("🔍 This is the JSON parsing error we're looking for!");
                println!("   Token: {}", token_address);
                println!("   Error: {}", e);

                // Try to get more details
                println!("🔍 Let's check this token manually...");
                match
                    reqwest::get(
                        &format!("https://api.dexscreener.com/tokens/v1/solana/{}", token_address)
                    ).await
                {
                    Ok(response) => {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_default();
                        println!("   Direct API status: {}", status);
                        println!("   Direct API response length: {}", text.len());
                        println!(
                            "   Direct API response preview: {}",
                            &text.chars().take(200).collect::<String>()
                        );
                    }
                    Err(e) => {
                        println!("   Direct API call failed: {}", e);
                    }
                }
            }
        }
    }
}

async fn test_database_access() -> Result<()> {
    println!("📊 Testing trader database...");

    match TraderDatabase::new("trader.db") {
        Ok(db) => {
            println!("✅ Database connection successful");

            match db.get_active_positions() {
                Ok(positions) => {
                    println!("✅ Found {} active positions", positions.len());

                    for (id, summary) in positions.iter().take(3) {
                        println!(
                            "   Position {}: {} ({} SOL invested)",
                            id,
                            summary.token_address,
                            summary.total_invested_sol
                        );
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get active positions: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Database connection failed: {}", e);
        }
    }

    Ok(())
}
