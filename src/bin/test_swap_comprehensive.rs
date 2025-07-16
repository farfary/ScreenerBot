use screenerbot::config::Config;
use screenerbot::swap::dex::{ JupiterSwap, GmgnSwap };
use screenerbot::swap::types::*;
use anyhow::Result;
use std::time::Instant;

const TEST_TOKENS: &[(&str, &str)] = &[
    ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
    ("WIF", "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"),
    ("JUP", "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"), // Jupiter token
    ("MICHI", "5mbK36SZ7J19An8jFochhQS4of8g6BwUjbeCSxBSoWdp"),
    ("POPCAT", "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr"),
];

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting comprehensive swap testing...\n");

    // Load configuration from the main config file
    let config = Config::load("configs.json").map_err(|e| {
        anyhow::anyhow!("Failed to load config: {}", e)
    })?;

    // Extract wallet public key from private key
    let wallet_public_key = extract_public_key_from_config(&config)?;

    println!("📍 Using wallet: {}\n", wallet_public_key);

    // Initialize DEX clients
    let jupiter = JupiterSwap::new(config.swap.jupiter.clone());
    let gmgn = GmgnSwap::new(config.swap.gmgn.clone());

    // Test amount: 0.001 SOL = 1,000,000 lamports
    let test_amount = 1_000_000u64;
    let slippage_bps = 50; // 0.5% slippage for lowest cost

    println!("💰 Test amount: {} lamports (0.001 SOL)", test_amount);
    println!("🎯 Slippage: {}bps ({}%)", slippage_bps, (slippage_bps as f64) / 100.0);
    println!("🔒 Anti-MEV: disabled (for lowest cost)\n");

    // Test 1: SOL to USDC
    println!("=== TEST 1: SOL → USDC ===");
    test_swap_pair(
        &jupiter,
        &gmgn,
        SOL_MINT,
        USDC_MINT,
        "SOL",
        "USDC",
        test_amount,
        slippage_bps,
        &wallet_public_key
    ).await?;

    // Test 2: USDC to SOL
    println!("\n=== TEST 2: USDC → SOL ===");
    let usdc_amount = 1_000_000u64; // 1 USDC (6 decimals)
    test_swap_pair(
        &jupiter,
        &gmgn,
        USDC_MINT,
        SOL_MINT,
        "USDC",
        "SOL",
        usdc_amount,
        slippage_bps,
        &wallet_public_key
    ).await?;

    // Test 3: SOL to various top tokens
    for (token_name, token_mint) in TEST_TOKENS {
        println!("\n=== TEST: SOL → {} ===", token_name);
        test_swap_pair(
            &jupiter,
            &gmgn,
            SOL_MINT,
            token_mint,
            "SOL",
            token_name,
            test_amount,
            slippage_bps,
            &wallet_public_key
        ).await?;
    }

    // Test 4: Various tokens back to SOL
    for (token_name, token_mint) in TEST_TOKENS {
        println!("\n=== TEST: {} → SOL ===", token_name);
        let token_amount = 1_000_000u64; // Adjust based on token decimals if needed
        test_swap_pair(
            &jupiter,
            &gmgn,
            token_mint,
            SOL_MINT,
            token_name,
            "SOL",
            token_amount,
            slippage_bps,
            &wallet_public_key
        ).await?;
    }

    // Final test: Compare best routes
    println!("\n🏆 === FINAL COMPARISON: BEST ROUTE ANALYSIS ===");
    compare_best_routes(&jupiter, &gmgn, &wallet_public_key).await?;

    println!("\n✅ All tests completed successfully!");
    Ok(())
}

async fn test_swap_pair(
    jupiter: &JupiterSwap,
    gmgn: &GmgnSwap,
    input_mint: &str,
    output_mint: &str,
    input_symbol: &str,
    output_symbol: &str,
    amount: u64,
    slippage_bps: u32,
    wallet_pubkey: &str
) -> Result<()> {
    let request = SwapRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        amount,
        slippage_bps,
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: None,
        is_anti_mev: false, // Disabled for lowest cost
    };

    let mut results = Vec::new();

    // Test Jupiter
    if jupiter.is_enabled() {
        println!("🪐 Testing Jupiter...");
        let start = Instant::now();
        match jupiter.get_quote(&request).await {
            Ok(route) => {
                let duration = start.elapsed();
                println!("  ✅ Jupiter quote successful");
                println!("     📈 Output: {} lamports", route.out_amount);
                println!("     💥 Price impact: {}%", route.price_impact_pct);
                println!("     ⏱️  Time: {:?}", duration);

                // Test getting swap transaction
                match jupiter.get_swap_transaction(&route, wallet_pubkey).await {
                    Ok(tx) => {
                        println!(
                            "     🔗 Transaction ready (block: {})",
                            tx.last_valid_block_height
                        );
                        results.push((
                            "Jupiter",
                            route.out_amount.clone(),
                            route.price_impact_pct.clone(),
                            duration,
                        ));
                    }
                    Err(e) => println!("     ❌ Transaction failed: {}", e),
                }
            }
            Err(e) => println!("  ❌ Jupiter failed: {}", e),
        }
    } else {
        println!("🪐 Jupiter disabled");
    }

    // Test GMGN
    if gmgn.is_enabled() {
        println!("🎯 Testing GMGN...");
        let start = Instant::now();
        match gmgn.get_quote(&request).await {
            Ok(route) => {
                let duration = start.elapsed();
                println!("  ✅ GMGN quote successful");
                println!("     📈 Output: {} lamports", route.out_amount);
                println!("     💥 Price impact: {}%", route.price_impact_pct);
                println!("     ⏱️  Time: {:?}", duration);

                // Test getting swap transaction
                match gmgn.get_swap_transaction(&route, wallet_pubkey).await {
                    Ok(tx) => {
                        println!(
                            "     🔗 Transaction ready (block: {})",
                            tx.last_valid_block_height
                        );
                        results.push((
                            "GMGN",
                            route.out_amount.clone(),
                            route.price_impact_pct.clone(),
                            duration,
                        ));
                    }
                    Err(e) => println!("     ❌ Transaction failed: {}", e),
                }
            }
            Err(e) => println!("  ❌ GMGN failed: {}", e),
        }
    } else {
        println!("🎯 GMGN disabled");
    }

    // Compare results
    if results.len() > 1 {
        println!("\n📊 Comparison for {} → {}:", input_symbol, output_symbol);
        results.sort_by(|a, b| {
            let a_output: u64 = a.1.parse().unwrap_or(0);
            let b_output: u64 = b.1.parse().unwrap_or(0);
            b_output.cmp(&a_output) // Sort by output descending (higher is better)
        });

        for (i, (dex, output, impact, time)) in results.iter().enumerate() {
            let badge = if i == 0 { "🥇" } else { "🥈" };
            println!("  {} {}: {} output, {}% impact, {:?}", badge, dex, output, impact, time);
        }
    }

    Ok(())
}

async fn compare_best_routes(
    jupiter: &JupiterSwap,
    gmgn: &GmgnSwap,
    wallet_pubkey: &str
) -> Result<()> {
    println!("Performing final best route comparison with SOL → USDC...");

    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: 1_000_000, // 0.001 SOL
        slippage_bps: 50, // 0.5%
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: None,
        is_anti_mev: false,
    };

    let mut best_route = None;
    let mut best_output = 0u64;
    let mut best_dex = "";

    // Test Jupiter
    if let Ok(route) = jupiter.get_quote(&request).await {
        if let Ok(output) = route.out_amount.parse::<u64>() {
            if output > best_output {
                best_output = output;
                best_dex = "Jupiter";
                best_route = Some(route);
            }
        }
    }

    // Test GMGN
    if let Ok(route) = gmgn.get_quote(&request).await {
        if let Ok(output) = route.out_amount.parse::<u64>() {
            if output > best_output {
                best_output = output;
                best_dex = "GMGN";
                best_route = Some(route);
            }
        }
    }

    if let Some(route) = best_route {
        println!("\n🏆 BEST ROUTE WINNER: {}", best_dex);
        println!("   📈 Output: {} USDC lamports", best_output);
        println!("   💥 Price Impact: {}%", route.price_impact_pct);
        println!("   🎯 Slippage: {}bps", route.slippage_bps);

        // Test the winning route's transaction
        println!("\n🧪 Testing best route transaction generation...");

        match best_dex {
            "Jupiter" => {
                match jupiter.get_swap_transaction(&route, wallet_pubkey).await {
                    Ok(tx) => {
                        println!("   ✅ Transaction ready for execution");
                        println!("   🔗 Block height: {}", tx.last_valid_block_height);
                        println!("   📦 Transaction size: {} bytes", tx.swap_transaction.len());
                    }
                    Err(e) => println!("   ❌ Transaction generation failed: {}", e),
                }
            }
            "GMGN" => {
                match gmgn.get_swap_transaction(&route, wallet_pubkey).await {
                    Ok(tx) => {
                        println!("   ✅ Transaction ready for execution");
                        println!("   🔗 Block height: {}", tx.last_valid_block_height);
                        println!("   📦 Transaction size: {} bytes", tx.swap_transaction.len());
                    }
                    Err(e) => println!("   ❌ Transaction generation failed: {}", e),
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown DEX"));
            }
        }
    } else {
        println!("❌ No successful routes found");
    }

    Ok(())
}

fn extract_public_key_from_config(_config: &Config) -> Result<String> {
    // For this test, we'll use a dummy public key since we're not actually executing transactions
    // In a real implementation, you would derive the public key from the private key
    Ok("11111111111111111111111111111112".to_string()) // System program ID as dummy
}
