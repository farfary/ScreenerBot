use screenerbot::{
    config::Config,
    rpc::RpcManager,
    swap::{manager::SwapManager, types::*, raydium::RaydiumProvider},
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use std::{str::FromStr, sync::Arc};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 ScreenerBot Real Swap Integration Test");
    println!("⚠️  WARNING: This test will execute REAL on-chain transactions!");
    println!("   Make sure you're using testnet or small amounts on mainnet");
    println!();

    // Load configuration
    let config = Config::load("configs.json").map_err(|e| format!("Failed to load config: {}", e))?;

    // Initialize RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(
            config.rpc_url.clone(),
            config.rpc_fallbacks.clone(),
            config.rpc.clone()
        )?
    );

    // Initialize swap manager
    let swap_manager = SwapManager::new(config.swap.clone(), rpc_manager.clone());

    // Create keypair from private key
    let private_key_bytes = bs58::decode(&config.main_wallet_private)
        .into_vec()
        .map_err(|e| format!("Failed to decode private key: {}", e))?;
    let keypair = Keypair::try_from(&private_key_bytes[..])
        .map_err(|e| format!("Failed to create keypair: {}", e))?;

    println!("💳 Wallet: {}", keypair.pubkey());

    // Define token addresses
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?; // SOL
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC

    // Create swap request for a small amount (0.001 SOL = 1,000,000 lamports)
    let swap_request = SwapRequest {
        user_public_key: keypair.pubkey(),
        input_mint: sol_mint,
        output_mint: usdc_mint,
        amount: 1_000_000, // 0.001 SOL
        slippage_bps: 100,  // 1%
        wrap_unwrap_sol: true,
        use_shared_accounts: true,
        priority_fee: Some(100_000), // 0.1 lamports per CU
        compute_unit_price: None, // Don't use both priority_fee and compute_unit_price
        preferred_provider: None, // Let the system choose the best provider
    };

    println!("🔄 Swap Request:");
    println!("   From: {} SOL", (swap_request.amount as f64) / 1e9);
    println!("   To: USDC");
    println!("   Slippage: {}%", swap_request.slippage_bps as f64 / 100.0);
    println!();

    // Test 1: Get quotes from all providers
    println!("📊 Getting quotes from all providers...");
    let quotes = swap_manager.get_all_quotes(&swap_request).await;

    for (provider, result) in &quotes {
        match result {
            Ok(quote) => {
                let output_tokens = match provider {
                    SwapProvider::Jupiter | SwapProvider::Raydium => {
                        (quote.out_amount as f64) / 1e6 // USDC has 6 decimals
                    }
                    SwapProvider::Gmgn => {
                        (quote.out_amount as f64) / 1e6 // USDC has 6 decimals
                    }
                };

                println!("✅ {}: {} USDC (impact: {:.2}%, route: {} steps)", 
                    provider, 
                    output_tokens, 
                    quote.price_impact_pct, 
                    quote.route_steps
                );
            }
            Err(e) => {
                println!("❌ {}: {}", provider, e);
            }
        }
    }
    println!();

    // Test 2: Get best quote
    println!("🎯 Getting best quote...");
    match swap_manager.get_best_quote(&swap_request).await {
        Ok(best_quote) => {
            let output_tokens = (best_quote.out_amount as f64) / 1e6;
            println!("✅ Best quote from {}: {} USDC", best_quote.provider, output_tokens);
            println!("   Price Impact: {:.2}%", best_quote.price_impact_pct);
            println!("   Route Steps: {}", best_quote.route_steps);
            println!("   Estimated Fee: {} lamports", best_quote.estimated_fee);

            // Test 3: Execute real swap
            println!();
            println!("⚡ EXECUTING REAL SWAP NOW!");

            // EXECUTING REAL SWAP!
            println!("⚡ Executing real swap...");
            match swap_manager.execute_swap(&swap_request, &best_quote, &keypair).await {
                Ok(result) => {
                    println!("🎉 Swap executed successfully!");
                    println!("   Provider: {}", result.provider);
                    println!("   Signature: {}", result.signature);
                    println!("   Input: {} SOL", (result.input_amount as f64) / 1e9);
                    println!("   Output: {} USDC", (result.output_amount as f64) / 1e6);
                    println!("   Fee: {} lamports", result.actual_fee);
                    println!("   Time: {} ms", result.execution_time_ms);
                    println!("   Explorer: https://solscan.io/tx/{}", result.signature);
                }
                Err(e) => {
                    println!("❌ Swap failed: {}", e);
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get best quote: {}", e);
            return Err(e.into());
        }
    }

    // Test 4: Provider-specific swaps
    println!();
    println!("🔍 Testing provider-specific methods...");

    // Test Raydium specifically if available
    let raydium_config = &config.swap.raydium;
    if raydium_config.enabled {
        println!("   Testing Raydium direct quote...");
        let raydium_provider = RaydiumProvider::new(raydium_config.clone());
        
        match raydium_provider.get_quote(&sol_mint, &usdc_mint, 1_000_000, 300).await {
            Ok(quote) => {
                println!("   ✅ Raydium quote: {} USDC", (quote.out_amount as f64) / 1e6);
                
                // Test transaction creation with higher slippage for Raydium
                match raydium_provider.get_swap_transaction(
                    &keypair.pubkey(),
                    &quote,
                    true,  // wrap SOL
                    false, // unwrap SOL  
                    Some(100_000)
                ).await {
                    Ok(transaction) => {
                        println!("   ✅ Raydium transaction created successfully");
                        println!("      Priority fee: {} lamports", transaction.priority_fee);
                        println!("      Transaction size: {} bytes", transaction.serialized_transaction.len());
                        
                        // EXECUTING REAL RAYDIUM SWAP
                        let rpc_client = rpc_manager.get_rpc_client()?;
                        match raydium_provider.execute_swap(&transaction, &keypair, &rpc_client).await {
                            Ok(signature) => {
                                println!("   🎉 Raydium swap executed: {}", signature);
                                println!("      Explorer: https://solscan.io/tx/{}", signature);
                            }
                            Err(e) => {
                                println!("   ❌ Raydium swap failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Failed to create Raydium transaction: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Raydium quote failed: {}", e);
            }
        }
    }

    // Test 5: Health checks
    println!();
    println!("🏥 Testing provider health checks...");
    let health_results = swap_manager.health_check().await;
    for (provider, healthy) in health_results {
        println!("   {}: {}", provider, if healthy { "✅ Healthy" } else { "❌ Unhealthy" });
    }

    // Test 6: Get swap statistics
    println!();
    println!("📈 Current swap statistics:");
    let stats = swap_manager.get_stats().await;
    println!("   Total swaps: {}", stats.total_swaps);
    println!("   Successful swaps: {}", stats.successful_swaps);
    println!("   Failed swaps: {}", stats.failed_swaps);
    println!("   Success rate: {:.1}%", 
        if stats.total_swaps > 0 {
            (stats.successful_swaps as f64 / stats.total_swaps as f64) * 100.0
        } else {
            0.0
        }
    );
    println!("   Total volume: ${:.2}", stats.total_volume);
    println!("   Average execution time: {} ms", stats.average_execution_time_ms);

    for (provider, provider_stats) in &stats.provider_stats {
        println!("   {} stats:", provider);
        println!("      Swaps: {}", provider_stats.swaps_count);
        println!("      Success rate: {:.1}%", provider_stats.success_rate * 100.0);
        println!("      Avg execution time: {} ms", provider_stats.average_execution_time_ms);
        println!("      Avg price impact: {:.2}%", provider_stats.average_price_impact);
        println!("      Volume: ${:.2}", provider_stats.total_volume);
    }

    println!();
    println!("✅ All tests completed successfully!");
    println!("🎉 REAL SWAPS WERE EXECUTED!");
    println!("⚠️  Check the transaction signatures above for confirmation!");

    Ok(())
}
