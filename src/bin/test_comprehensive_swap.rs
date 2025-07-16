use anyhow::Result;
use bs58;
use screenerbot::{
    config::Config,
    rpc::RpcManager,
    swap::{ SwapManager, SwapProvider, create_swap_request },
};
use solana_sdk::{ pubkey::Pubkey, signature::Keypair, signer::Signer };
use std::str::FromStr;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("🚀 ScreenerBot Comprehensive Swap Test Starting...\n");

    // Load configuration
    let config = Config::load("configs.json")?;
    println!("✅ Configuration loaded");

    // Create RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone(), config.rpc.clone())?
    );
    println!("✅ RPC Manager initialized");

    // Create swap manager
    let swap_manager = SwapManager::new(config.swap.clone(), rpc_manager.clone());
    println!("✅ Swap Manager initialized");

    // Create wallet from private key
    let private_key_bytes = bs58::decode(&config.main_wallet_private).into_vec()?;
    let keypair = Keypair::try_from(&private_key_bytes[..])?;
    let wallet_pubkey = keypair.pubkey();
    println!("✅ Wallet loaded: {}", wallet_pubkey);

    // Define common token addresses
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?; // SOL
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC
    let bonk_mint = Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263")?; // BONK

    let swap_amount = 1000000; // 0.001 SOL in lamports

    println!("\n📊 Testing provider health checks...");
    let health_status = swap_manager.health_check().await;
    for (provider, healthy) in &health_status {
        println!("   {}: {}", provider, if *healthy { "✅ Healthy" } else { "❌ Unhealthy" });
    }

    println!("\n📊 Getting quotes from all providers...");

    // Test 1: Get all quotes for SOL -> USDC
    let swap_request = create_swap_request(
        sol_mint,
        usdc_mint,
        swap_amount,
        wallet_pubkey,
        Some(100), // 1% slippage
        None
    );

    let all_quotes = swap_manager.get_all_quotes(&swap_request).await;

    println!("\n💱 SOL -> USDC Quotes ({}μ SOL):", (swap_amount as f64) / 1e6);
    for (provider, quote_result) in &all_quotes {
        match quote_result {
            Ok(quote) => {
                println!(
                    "   {}: {} USDC (impact: {:.2}%, routes: {})",
                    provider,
                    (quote.out_amount as f64) / 1e6,
                    quote.price_impact_pct,
                    quote.route_steps
                );
            }
            Err(e) => {
                println!("   {}: ❌ {}", provider, e);
            }
        }
    }

    // Test 2: Get best quote
    println!("\n🏆 Finding best quote...");
    match swap_manager.get_best_quote(&swap_request).await {
        Ok(best_quote) => {
            println!(
                "   Best: {} -> {} USDC (impact: {:.2}%)",
                best_quote.provider,
                (best_quote.out_amount as f64) / 1e6,
                best_quote.price_impact_pct
            );
        }
        Err(e) => {
            println!("   ❌ Failed to get best quote: {}", e);
        }
    }

    // Test 3: Test SOL -> BONK quotes
    println!("\n💱 SOL -> BONK Quotes ({}μ SOL):", (swap_amount as f64) / 1e6);
    let bonk_request = create_swap_request(
        sol_mint,
        bonk_mint,
        swap_amount,
        wallet_pubkey,
        Some(100),
        None
    );

    let bonk_quotes = swap_manager.get_all_quotes(&bonk_request).await;
    for (provider, quote_result) in &bonk_quotes {
        match quote_result {
            Ok(quote) => {
                println!(
                    "   {}: {} BONK (impact: {:.2}%, routes: {})",
                    provider,
                    (quote.out_amount as f64) / 1e5, // BONK has 5 decimals
                    quote.price_impact_pct,
                    quote.route_steps
                );
            }
            Err(e) => {
                println!("   {}: ❌ {}", provider, e);
            }
        }
    }

    // Test 4: Provider-specific tests
    println!("\n🔍 Testing provider-specific features...");

    // Test Jupiter
    if *health_status.get(&SwapProvider::Jupiter).unwrap_or(&false) {
        println!("   Testing Jupiter-specific swap...");
        match swap_manager.swap_with_provider(&swap_request, SwapProvider::Jupiter, &keypair).await {
            Ok(result) => {
                println!("   ✅ Jupiter swap successful: {}", result.signature);
                println!(
                    "      Output: {} USDC, Fee: {} lamports, Time: {}ms",
                    (result.output_amount as f64) / 1e6,
                    result.actual_fee,
                    result.execution_time_ms
                );
            }
            Err(e) => {
                println!("   ❌ Jupiter swap failed: {}", e);
            }
        }
    }

    // Test GMGN
    if *health_status.get(&SwapProvider::Gmgn).unwrap_or(&false) {
        println!("   Testing GMGN-specific swap...");
        match swap_manager.swap_with_provider(&bonk_request, SwapProvider::Gmgn, &keypair).await {
            Ok(result) => {
                println!("   ✅ GMGN swap successful: {}", result.signature);
                println!(
                    "      Output: {} BONK, Fee: {} lamports, Time: {}ms",
                    (result.output_amount as f64) / 1e5,
                    result.actual_fee,
                    result.execution_time_ms
                );
            }
            Err(e) => {
                println!("   ❌ GMGN swap failed: {}", e);
            }
        }
    }

    // Test 5: Auto-swap (best provider selection)
    println!("\n🤖 Testing automatic best provider selection...");
    match swap_manager.swap(&swap_request, &keypair).await {
        Ok(result) => {
            println!("   ✅ Auto swap successful via {}: {}", result.provider, result.signature);
            println!(
                "      Input: {} SOL, Output: {} USDC",
                (result.input_amount as f64) / 1e9,
                (result.output_amount as f64) / 1e6
            );
            println!(
                "      Fee: {} lamports, Time: {}ms",
                result.actual_fee,
                result.execution_time_ms
            );
        }
        Err(e) => {
            println!("   ❌ Auto swap failed: {}", e);
        }
    }

    // Test 6: Statistics
    println!("\n📈 Swap Statistics:");
    let stats = swap_manager.get_stats().await;
    println!("   Total Swaps: {}", stats.total_swaps);
    println!("   Successful: {}", stats.successful_swaps);
    println!("   Failed: {}", stats.failed_swaps);
    println!("   Success Rate: {:.1}%", if stats.total_swaps > 0 {
        ((stats.successful_swaps as f64) / (stats.total_swaps as f64)) * 100.0
    } else {
        0.0
    });
    println!("   Average Execution Time: {}ms", stats.average_execution_time_ms);
    println!("   Total Volume: ${:.2}", stats.total_volume);

    for (provider, provider_stats) in &stats.provider_stats {
        println!("   {} Stats:", provider);
        println!(
            "      Swaps: {}, Success Rate: {:.1}%",
            provider_stats.swaps_count,
            provider_stats.success_rate * 100.0
        );
        println!(
            "      Avg Execution: {}ms, Volume: ${:.2}",
            provider_stats.average_execution_time_ms,
            provider_stats.total_volume
        );
    }

    // Test 7: RPC Manager stats
    println!("\n🌐 RPC Manager Statistics:");
    let rpc_stats = rpc_manager.get_stats().await;
    println!("   Total Requests: {}", rpc_stats.total_requests);
    println!("   Successful: {}", rpc_stats.successful_requests);
    println!("   Failed: {}", rpc_stats.failed_requests);
    println!("   Success Rate: {:.1}%", rpc_stats.success_rate() * 100.0);
    println!("   Average Response Time: {}ms", rpc_stats.average_response_time_ms);

    let endpoint_stats = rpc_manager.get_endpoint_stats().await;
    for (idx, endpoint) in endpoint_stats.iter().enumerate() {
        println!(
            "   Endpoint {}: {} (weight: {}, healthy: {})",
            idx + 1,
            endpoint.url,
            endpoint.weight,
            endpoint.healthy
        );
        println!(
            "      Success Rate: {:.1}%, Response Time: {}ms",
            endpoint.success_rate() * 100.0,
            endpoint.response_time_ms
        );
    }

    println!("\n🎉 Comprehensive swap test completed!");

    Ok(())
}
