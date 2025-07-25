// Example demonstrating the new batched RPC functionality
// This file shows how the new parse_pool_data_batched method
// reduces RPC calls from 5 to 1 for safer RPC usage

use screenerbot::pool_price::{ get_token_price, get_detailed_price_info };
use screenerbot::pool_price::types::PoolType;

#[tokio::main]
pub async fn main() {
    demo_batched_rpc_optimization().await;
}

pub async fn demo_batched_rpc_optimization() {
    // Example token mint (SOL)
    let mint = "So11111111111111111111111111111111111111112";

    println!("=== New Pool Price System Demo ===");
    println!("Token mint: {}", mint);

    // Use the new get_token_price function
    match get_token_price(mint).await {
        Some(price) => {
            println!("✅ SUCCESS: Got token price");
            println!("Price: {:.12} SOL", price);

            // Get detailed info for debugging
            if let Ok(Some(detailed)) = get_detailed_price_info(mint).await {
                println!("Confidence: {:.2}", detailed.confidence);
                println!("Source pools: {:?}", detailed.source_pools);
            }
        }
        None => {
            println!("❌ FAILED: Could not get token price");
        }
    }
}
