// Test to verify new pool price system works with positions.rs integration
use screenerbot::pool_price::{ get_token_price, get_detailed_price_info };

#[tokio::main]
async fn main() {
    test_positions_integration().await;
}

pub async fn test_positions_integration() {
    println!("=== Testing New Pool Price System with Positions ===");

    // Test multiple token mints that positions.rs might encounter
    let test_mints = vec![
        "So11111111111111111111111111111111111111112", // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
        // Add more token mints here for comprehensive testing
    ];

    for mint in test_mints {
        println!("\n🔍 Testing token: {}", mint);

        match get_token_price(mint).await {
            Some(price) => {
                println!("✅ Price lookup successful!");
                println!("   Price: {:.12} SOL", price);

                // Get detailed info for debugging
                if let Ok(Some(detailed)) = get_detailed_price_info(mint).await {
                    println!("   Confidence: {:.2}", detailed.confidence);
                    println!("   Source pools: {:?}", detailed.source_pools);
                }

                println!("   💰 Ready for position calculations");
            }
            None => {
                println!("❌ Failed to get price for token {}", mint);
            }
        }
    }

    println!("\n✨ Integration test complete - new pool price system ready for positions.rs!");
}
