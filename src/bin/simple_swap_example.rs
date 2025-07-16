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

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    println!("🚀 Simple Swap Example\n");

    // Load configuration
    let config = Config::load("configs.json")?;

    // Create RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone(), config.rpc.clone())?
    );

    // Create swap manager
    let swap_manager = SwapManager::new(config.swap.clone(), rpc_manager);

    // Create wallet
    let private_key_bytes = bs58::decode(&config.main_wallet_private).into_vec()?;
    let keypair = Keypair::try_from(&private_key_bytes[..])?;

    // Define swap parameters
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
    let amount = 1000000; // 0.001 SOL

    // Create swap request
    let swap_request = create_swap_request(
        sol_mint,
        usdc_mint,
        amount,
        keypair.pubkey(),
        Some(100), // 1% slippage
        None // Auto-select best provider
    );

    println!("📊 Getting best quote...");

    // Get the best quote
    match swap_manager.get_best_quote(&swap_request).await {
        Ok(quote) => {
            println!("✅ Best quote from {}:", quote.provider);
            println!("   Input: {} SOL", (quote.in_amount as f64) / 1e9);
            println!("   Output: {} USDC", (quote.out_amount as f64) / 1e6);
            println!("   Price Impact: {:.2}%", quote.price_impact_pct);
            println!("   Route Steps: {}", quote.route_steps);

            // Uncomment to execute the swap
            /*
            println!("\n🔄 Executing swap...");
            match swap_manager.execute_swap(&swap_request, &quote, &keypair).await {
                Ok(result) => {
                    println!("✅ Swap successful!");
                    println!("   Transaction: {}", result.signature);
                    println!("   Provider: {}", result.provider);
                    println!("   Execution Time: {}ms", result.execution_time_ms);
                }
                Err(e) => {
                    println!("❌ Swap failed: {}", e);
                }
            }
            */
        }
        Err(e) => {
            println!("❌ Failed to get quote: {}", e);
        }
    }

    // Example of provider-specific swap
    println!("\n🎯 Testing Jupiter-specific quote...");
    match swap_manager.swap_with_provider(&swap_request, SwapProvider::Jupiter, &keypair).await {
        Ok(result) => {
            println!("✅ Jupiter swap would be successful");
            println!("   Expected output: {} USDC", (result.output_amount as f64) / 1e6);
        }
        Err(e) => {
            println!("❌ Jupiter swap failed: {}", e);
        }
    }

    Ok(())
}
