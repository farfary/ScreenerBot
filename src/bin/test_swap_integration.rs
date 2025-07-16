use screenerbot::swap::{ manager::SwapManager, types::SwapRequest };
use screenerbot::config::Config;
use screenerbot::rpc::RpcManager;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("🚀 Testing ScreenerBot Swap Module Integration");

    // Load configuration
    let config = Config::load("configs.json").expect("Failed to load config");

    // Initialize RPC Manager
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone(), config.rpc)?
    );

    // Initialize Swap Manager
    let swap_manager = SwapManager::new(config.swap, rpc_manager);

    // Test provider health check
    println!("🔍 Checking provider health...");
    let health_status = swap_manager.health_check().await;

    for (provider, status) in health_status {
        let status_icon = if status { "✅" } else { "❌" };
        println!("  {} {}: {}", status_icon, format!("{:?}", provider), if status {
            "Healthy"
        } else {
            "Unhealthy"
        });
    }

    // Create a test swap request (SOL to USDC)
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?; // SOL
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC
    let user_pubkey = Pubkey::from_str("11111111111111111111111111111112")?; // Dummy user

    let swap_request = SwapRequest {
        input_mint: sol_mint,
        output_mint: usdc_mint,
        amount: 1_000_000_000, // 1 SOL in lamports
        slippage_bps: 100, // 1% slippage
        user_public_key: user_pubkey,
        preferred_provider: None,
        priority_fee: None,
        compute_unit_price: None,
        wrap_unwrap_sol: true,
        use_shared_accounts: false,
    };

    println!("💱 Testing quote fetching...");
    println!("  Input: 1 SOL ({})", sol_mint);
    println!("  Output: USDC ({})", usdc_mint);
    println!("  Slippage: 1%");

    // Get quotes from all providers
    let quotes = swap_manager.get_all_quotes(&swap_request).await;

    if quotes.is_empty() {
        println!("❌ No quotes available from any provider");
        return Ok(());
    }

    println!("📊 Quote Results:");
    for (provider, quote_result) in &quotes {
        match quote_result {
            Ok(quote) => {
                println!(
                    "  ✅ {}: {} output tokens (rate: {:.6})",
                    format!("{:?}", provider),
                    quote.out_amount,
                    (quote.out_amount as f64) / (swap_request.amount as f64)
                );
            }
            Err(e) => {
                println!("  ❌ {}: Error - {}", format!("{:?}", provider), e);
            }
        }
    }

    // Try to get the best quote
    match swap_manager.get_best_quote(&swap_request).await {
        Ok(best_quote) => {
            println!("🏆 Best Quote:");
            println!("  Provider: {:?}", best_quote.provider);
            println!("  Output Amount: {} tokens", best_quote.out_amount);
            println!("  Price Impact: {:.2}%", best_quote.price_impact_pct);
            println!("  Route Steps: {}", best_quote.route_steps);
        }
        Err(e) => {
            println!("❌ Could not determine best quote: {}", e);
        }
    }

    // Display swap statistics
    let stats = swap_manager.get_stats().await;
    println!("📈 Swap Manager Statistics:");
    println!("  Total Swaps: {}", stats.total_swaps);
    println!("  Successful Swaps: {}", stats.successful_swaps);
    println!("  Failed Swaps: {}", stats.failed_swaps);
    println!("  Total Volume: {:.2}", stats.total_volume);
    println!("  Average Execution Time: {}ms", stats.average_execution_time_ms);

    println!("✅ Integration test completed successfully!");

    Ok(())
}
