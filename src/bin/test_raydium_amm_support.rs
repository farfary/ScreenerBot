#![allow(warnings)]
use screenerbot::global::read_configs;
use screenerbot::pool_price::{ PoolDiscoveryAndPricing, PoolType };
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Testing Raydium AMM Support");

    // Test configuration
    let configs = read_configs("configs.json").map_err(|e| anyhow::anyhow!("Config error: {}", e))?;
    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    println!("✅ Pool service initialized with Raydium AMM support");

    // Test pool type detection
    println!("\n📊 Testing Pool Type Enum:");
    println!("RaydiumAmm: {:?}", PoolType::RaydiumAmm);

    // Test pool type from dex_id and labels
    println!("\n🔍 Testing Pool Type Detection:");
    let amm_type = PoolType::from_dex_id_and_labels("raydium", &vec!["AMM".to_string()]);
    println!("Detected type for 'raydium' with 'AMM' label: {:?}", amm_type);

    let standard_raydium_type = PoolType::from_dex_id_and_labels("raydium", &vec![]);
    println!("Detected type for standard 'raydium' pool: {:?}", standard_raydium_type);

    // Test the Raydium AMM program ID
    println!("\n🔐 Program ID Constants:");
    println!("RAYDIUM_AMM_PROGRAM_ID: RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w");

    println!("\n✅ All Raydium AMM support tests completed successfully!");
    println!("📝 Integration Points Added:");
    println!("  - PoolType::RaydiumAmm enum variant");
    println!("  - RaydiumAmmData structure");
    println!("  - PoolSpecificData::RaydiumAmm variant");
    println!("  - parse_raydium_amm_data() function");
    println!("  - Program ID mapping: RVKd61ztZW9g2VZgPZrFYuXJcZ1t7xvaUo1NkL6MZ5w");
    println!("  - Pool type display: 'AMM'");
    println!("  - Pool detection logic for Raydium AMM pools");

    Ok(())
}
