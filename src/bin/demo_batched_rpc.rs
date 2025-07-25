// Example demonstrating the new batched RPC functionality
// This file shows how the new parse_pool_data_batched method
// reduces RPC calls from 5 to 1 for safer RPC usage

use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::pool_price::types::PoolType;

#[tokio::main]
pub async fn main() {
    demo_batched_rpc_optimization().await;
}

pub async fn demo_batched_rpc_optimization() {
    let pool_discovery = PoolDiscoveryAndPricing::new("https://api.mainnet-beta.solana.com");

    // Example Raydium CPMM pool address
    let pool_address = "6UmmUiYoBjSrhakAobJw8BvkmJtDVxaeBtbt7rxWo1mg";

    println!("=== Batched RPC Optimization Demo ===");
    println!("Pool address: {}", pool_address);

    // Use the new batched method - reduces from 5 RPC calls to 1
    match pool_discovery.parse_pool_data_batched(pool_address, PoolType::RaydiumCpmm).await {
        Ok(pool_data) => {
            println!("✅ SUCCESS: Batched RPC call completed");
            println!(
                "Token A: {} (decimals: {})",
                pool_data.token_a.mint,
                pool_data.token_a.decimals
            );
            println!(
                "Token B: {} (decimals: {})",
                pool_data.token_b.mint,
                pool_data.token_b.decimals
            );
            println!("Reserve A: {} tokens", pool_data.reserve_a.balance);
            println!("Reserve B: {} tokens", pool_data.reserve_b.balance);
            println!("📈 Optimization: 1 RPC call instead of 5 (80% reduction)");
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }
}
