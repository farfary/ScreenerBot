use screenerbot::tokens::pool::{ get_pool_service };
use screenerbot::{ global::read_configs, tokens::api::init_dexscreener_api, rpc::init_rpc_client };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize dependencies
    println!("🔄 Initializing services...");

    let _configs = read_configs()?;
    init_rpc_client()?;
    init_dexscreener_api().await?;

    println!("✅ Services initialized");

    // Get pool service
    let pool_service = get_pool_service();

    // Test a few price requests to populate statistics
    let test_tokens = vec![
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "So11111111111111111111111111111111111111112" // SOL (wrapped)
    ];

    for token in &test_tokens {
        println!("🧪 Testing pool price for {}...", &token[..12]);
        if let Some(result) = pool_service.get_pool_price(token, None).await {
            println!("  ✅ Price: {:?} SOL", result.price_sol);
        } else {
            println!("  ❌ No price calculated");
        }
    }

    // Display enhanced statistics
    println!("\n📊 Enhanced Pool Service Statistics:");
    let enhanced_stats = pool_service.get_enhanced_stats().await;
    println!("  Total Requests: {}", enhanced_stats.total_price_requests);
    println!("  Successful: {}", enhanced_stats.successful_calculations);
    println!("  Failed: {}", enhanced_stats.failed_calculations);
    println!("  Cache Hits: {}", enhanced_stats.cache_hits);
    println!("  Blockchain Calculations: {}", enhanced_stats.blockchain_calculations);
    println!("  API Fallbacks: {}", enhanced_stats.api_fallbacks);
    println!("  Success Rate: {:.1}%", enhanced_stats.get_success_rate());
    println!("  Cache Hit Rate: {:.1}%", enhanced_stats.get_cache_hit_rate());
    println!("  Tokens with Price History: {}", enhanced_stats.tokens_with_price_history);
    println!("  Total Price History Entries: {}", enhanced_stats.total_price_history_entries);

    // Display cache statistics
    println!("\n🏊 Cache Statistics:");
    let (pool_cache, price_cache, availability_cache) = pool_service.get_cache_stats().await;
    println!("  Pool Cache: {} pools", pool_cache);
    println!("  Price Cache: {} prices", price_cache);
    println!("  Availability Cache: {} tokens", availability_cache);

    // Display watch list statistics
    println!("\n👀 Watch List Statistics:");
    let (total, expired, never_checked) = pool_service.get_watch_list_stats().await;
    println!("  Total: {}", total);
    println!("  Expired: {}", expired);
    println!("  Never Checked: {}", never_checked);
    println!("  Active: {}", total.saturating_sub(expired));

    Ok(())
}
