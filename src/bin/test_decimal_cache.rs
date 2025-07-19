use std::error::Error;
use screenerbot::decimal_cache::{DecimalCache, fetch_or_cache_decimals};
use screenerbot::global::read_configs;
use screenerbot::logger::{log, LogTag};
use solana_client::rpc_client::RpcClient;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::init();

    // Test with a few known Solana token mints
    let test_mints = vec![
        "So11111111111111111111111111111111111111112".to_string(), // SOL (native, should be 9)
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC (should be 6)
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT (should be 6)
    ];

    println!("Testing decimal cache functionality...");

    // Load configuration
    let configs = read_configs("configs.json")?;
    let rpc_client = RpcClient::new(&configs.rpc_url);

    // Load cache
    let cache_path = Path::new("test_decimal_cache.json");
    let mut decimal_cache = DecimalCache::new();

    // First run - should fetch from chain
    println!("First run - fetching from chain:");
    let decimals_map = fetch_or_cache_decimals(&rpc_client, &test_mints, &mut decimal_cache, cache_path).await?;
    
    for (mint, decimals) in &decimals_map {
        println!("  {}: {} decimals", &mint[..8], decimals);
    }

    // Second run - should use cache
    println!("\nSecond run - using cache:");
    let decimals_map2 = fetch_or_cache_decimals(&rpc_client, &test_mints, &mut decimal_cache, cache_path).await?;
    
    for (mint, decimals) in &decimals_map2 {
        println!("  {}: {} decimals", &mint[..8], decimals);
    }

    // Verify results are the same
    assert_eq!(decimals_map, decimals_map2);
    println!("\n✅ Cache test passed! Results are consistent.");

    Ok(())
}
