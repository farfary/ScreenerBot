use std::sync::Arc;
use anyhow::Result;
use screenerbot::{ Config, Discovery, pairs::PairsClient };

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Debug Market Data Update");

    // Test with known working tokens first
    let test_tokens = ["So11111111111111111111111111111111111111112"];

    println!("🧪 Testing with known tokens: {:?}", test_tokens);

    // Test pairs client directly
    println!("� Creating PairsClient...");
    let pairs_client = PairsClient::new()?;
    println!("✅ PairsClient created");

    println!("📡 Testing API call...");
    println!("⏰ Starting get_multiple_token_pairs...");

    match pairs_client.get_multiple_token_pairs(&test_tokens).await {
        Ok(pairs) => {
            println!("✅ Got {} pairs from API", pairs.len());
            for pair in &pairs {
                println!(
                    "  - {} ({}) vol: {} SOL",
                    pair.base_token.symbol,
                    &pair.base_token.address[..8],
                    pair.volume.h24
                );
            }
        }
        Err(e) => {
            println!("❌ API call failed: {}", e);
        }
    }

    println!("🏁 Done");
    Ok(())
}
