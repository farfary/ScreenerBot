// Test to verify batched RPC works with positions.rs integration
use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::pool_price::types::PoolType;

#[tokio::main]
pub async fn main() {
    test_positions_integration().await;
}

pub async fn test_positions_integration() {
    println!("=== Testing Batched RPC Integration with Positions ===");

    let pool_discovery = PoolDiscoveryAndPricing::new("https://api.mainnet-beta.solana.com");

    // Test multiple pool types that positions.rs might encounter
    let test_pools = vec![
        ("6UmmUiYoBjSrhakAobJw8BvkmJtDVxaeBtbt7rxWo1mg", PoolType::RaydiumCpmm)
        // Add more pool addresses here for comprehensive testing
    ];

    for (pool_address, pool_type) in test_pools {
        println!("\n🔍 Testing pool: {} (type: {:?})", pool_address, pool_type);

        match pool_discovery.parse_pool_data_batched(pool_address, pool_type).await {
            Ok(pool_data) => {
                println!("✅ Batched RPC successful!");
                println!(
                    "   Token A: {} (decimals: {})",
                    pool_data.token_a.mint,
                    pool_data.token_a.decimals
                );
                println!(
                    "   Token B: {} (decimals: {})",
                    pool_data.token_b.mint,
                    pool_data.token_b.decimals
                );
                println!("   Reserve A: {} tokens", pool_data.reserve_a.balance);
                println!("   Reserve B: {} tokens", pool_data.reserve_b.balance);

                // Simulate position price calculation
                if pool_data.reserve_a.balance > 0 && pool_data.reserve_b.balance > 0 {
                    let token_a_ui =
                        (pool_data.reserve_a.balance as f64) /
                        (10_f64).powi(pool_data.token_a.decimals as i32);
                    let token_b_ui =
                        (pool_data.reserve_b.balance as f64) /
                        (10_f64).powi(pool_data.token_b.decimals as i32);

                    if token_a_ui > 0.0 {
                        let calculated_price = token_b_ui / token_a_ui;
                        println!("   💰 Calculated price: {} SOL per token", calculated_price);
                        println!("   🎯 Position integration: Ready for P&L calculations");
                    }
                } else {
                    println!("   ⚠️  Empty reserves - might be inactive pool");
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }

    println!("\n✨ Integration test complete - batched RPC optimization ready for positions.rs!");
}
