use screenerbot::{
    global::{ read_configs, CACHE_POOL_DIR },
    arguments::{ set_cmd_args, get_cmd_args },
    tokens::{
        pool::{
            init_pool_service,
            get_price_history_for_rl_learning,
            get_detailed_pool_price_history,
            get_pools_with_price_history,
            cleanup_old_price_history_caches,
        },
        dexscreener::init_dexscreener_api,
    },
    rpc::init_rpc_client,
};
use std::env;
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set command args for proper initialization
    set_cmd_args(env::args().collect());

    println!("🔧 Price History Cache Tool");
    println!("===========================");

    let args = get_cmd_args();
    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "test" => {
            if args.len() < 3 {
                println!("❌ Usage: {} test <TOKEN_MINT>", args[0]);
                return Ok(());
            }
            test_price_history_cache(&args[2]).await?;
        }
        "cleanup" => {
            cleanup_cache().await?;
        }
        "stats" => {
            show_cache_stats().await?;
        }
        "help" | "--help" => {
            print_help();
        }
        _ => {
            println!("❌ Unknown command: {}", command);
            print_help();
        }
    }

    Ok(())
}

fn print_help() {
    println!("📖 Price History Cache Tool Help");
    println!("================================");
    println!();
    println!("🎯 Purpose: Test and manage disk-based price history caching system");
    println!();
    println!("📋 Available Commands:");
    println!("  test <TOKEN_MINT>     - Test price history caching for a specific token");
    println!("  cleanup               - Clean up old/expired price history cache files");
    println!("  stats                 - Show cache statistics and file information");
    println!("  help                  - Show this help message");
    println!();
    println!("🔍 Examples:");
    println!(
        "  cargo run --bin tool_price_history_cache -- test EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    );
    println!("  cargo run --bin tool_price_history_cache -- cleanup");
    println!("  cargo run --bin tool_price_history_cache -- stats");
    println!();
    println!("⚠️  Safety Information:");
    println!("  🟢 Low-Risk: All commands are read-only or maintenance operations");
    println!("  💾 Cache Location: {} directory", CACHE_POOL_DIR);
    println!("  🕒 Cache Duration: 2 hours maximum per token");
    println!("  📊 Change Detection: Only records when price changes");
}

async fn test_price_history_cache(token_mint: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Price History Cache for: {}", token_mint);
    println!("===========================================");

    // Initialize required systems
    let _configs = read_configs()?;
    init_rpc_client()?;
    let _ = init_dexscreener_api().await;

    // Initialize pool service
    let pool_service = init_pool_service();

    // Start monitoring to enable caching
    pool_service.start_monitoring().await;

    println!("✅ Pool service initialized and monitoring started");

    // Add token to watch list for active monitoring
    pool_service.add_to_watch_list(token_mint, 10).await;
    println!("📋 Added {} to watch list", token_mint);

    // Try to get existing price history
    println!("\n📊 Checking existing price history...");
    let existing_history = get_price_history_for_rl_learning(token_mint).await;
    println!("📈 Found {} existing price history entries", existing_history.len());

    if !existing_history.is_empty() {
        let latest = existing_history.last().unwrap();
        let oldest = existing_history.first().unwrap();
        println!(
            "   📅 Oldest: {} (price: {:.12})",
            oldest.0.format("%Y-%m-%d %H:%M:%S"),
            oldest.1
        );
        println!(
            "   📅 Latest: {} (price: {:.12})",
            latest.0.format("%Y-%m-%d %H:%M:%S"),
            latest.1
        );
    }

    // Check detailed pool price history
    println!("\n🏊 Checking detailed pool price history...");
    let detailed_history = get_detailed_pool_price_history(token_mint).await;
    println!("🏊 Found {} pools with price history", detailed_history.len());

    for (pool_address, history) in &detailed_history {
        println!(
            "   📍 Pool {}: {} entries",
            &pool_address[..8],
            history.len()
        );
        if let Some(latest) = history.last() {
            println!(
                "       📅 Latest: {} (price: {:.12} SOL, liquidity: ${:.2})",
                latest.0.format("%Y-%m-%d %H:%M:%S"),
                latest.1,
                latest.5
            );
        }
    }

    // Get pools with history
    let pools_with_history = get_pools_with_price_history(token_mint).await;
    println!("\n🗂️  Pools with price history: {}", pools_with_history.len());
    for pool in &pools_with_history {
        println!("   📍 {}", pool);
    }

    // Test price retrieval and caching
    println!("\n🔍 Testing price retrieval and caching...");
    for i in 1..=5 {
        println!("   🔄 Attempt {}/5: Getting pool price...", i);

        match pool_service.get_pool_price(token_mint, None).await {
            Some(result) => {
                if let Some(price) = result.price_sol {
                    println!("   ✅ Got price: {:.12} SOL from {}", price, result.source);
                } else {
                    println!("   ⚠️  Got result but no price value");
                }
            }
            None => {
                println!("   ❌ Failed to get pool price");
            }
        }

        // Wait 6 seconds between attempts (longer than 5-second cache interval)
        if i < 5 {
            println!("   ⏳ Waiting 6 seconds...");
            sleep(Duration::from_secs(6)).await;
        }
    }

    // Check updated price history
    println!("\n📊 Checking updated price history...");
    let updated_history = get_price_history_for_rl_learning(token_mint).await;
    println!("📈 Found {} total price history entries", updated_history.len());

    if updated_history.len() > existing_history.len() {
        let new_entries = updated_history.len() - existing_history.len();
        println!("   🆕 Added {} new entries during test", new_entries);

        // Show the latest entries
        let recent_entries = &updated_history[updated_history.len().saturating_sub(3)..];
        for (i, (timestamp, price)) in recent_entries.iter().enumerate() {
            println!(
                "   📌 Entry {}: {} (price: {:.12})",
                i + 1,
                timestamp.format("%H:%M:%S%.3f"),
                price
            );
        }
    } else {
        println!("   ℹ️  No new entries added (price may not have changed significantly)");
    }

    // Check updated detailed pool history
    println!("\n🏊 Checking updated detailed pool history...");
    let updated_detailed_history = get_detailed_pool_price_history(token_mint).await;
    
    for (pool_address, history) in &updated_detailed_history {
        let previous_count = detailed_history.get(pool_address).map(|h| h.len()).unwrap_or(0);
        if history.len() > previous_count {
            let new_pool_entries = history.len() - previous_count;
            println!(
                "   🆕 Pool {}: Added {} new entries (total: {})",
                &pool_address[..8],
                new_pool_entries,
                history.len()
            );
        }
    }

    // Test RL learning integration
    println!("\n🤖 Testing RL Learning Integration...");
    let rl_history = get_price_history_for_rl_learning(token_mint).await;
    println!("🧠 RL Learning retrieved {} price history entries", rl_history.len());

    if rl_history.len() >= 6 {
        println!("   ✅ Sufficient data for RL analysis (6+ entries required)");

        // Calculate some basic statistics
        let prices: Vec<f64> = rl_history
            .iter()
            .map(|(_, price)| *price)
            .collect();
        let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg_price = prices.iter().sum::<f64>() / (prices.len() as f64);

        println!("   📊 Price Statistics:");
        println!("      Min: {:.12} SOL", min_price);
        println!("      Max: {:.12} SOL", max_price);
        println!("      Avg: {:.12} SOL", avg_price);

        if max_price > min_price {
            let volatility = ((max_price - min_price) / min_price) * 100.0;
            println!("      Volatility: {:.2}%", volatility);
        }
    } else {
        println!("   ⚠️  Insufficient data for RL analysis (need 6+ entries)");
    }

    // Stop monitoring
    pool_service.stop_monitoring().await;
    println!("\n🛑 Stopped pool monitoring");

    println!("\n✅ Price history cache test completed!");

    Ok(())
}

async fn cleanup_cache() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up old price history caches...");
    println!("==========================================");

    match cleanup_old_price_history_caches().await {
        Ok(cleaned_count) => {
            println!("✅ Cleanup completed successfully");
            println!("📊 Processed {} cache files", cleaned_count);
        }
        Err(e) => {
            println!("❌ Cleanup failed: {}", e);
        }
    }

    Ok(())
}

async fn show_cache_stats() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Pool Price History Cache Statistics");
    println!("=====================================");

    let cache_dir = std::path::Path::new(CACHE_POOL_DIR);

    if !cache_dir.exists() {
        println!("📁 Cache directory does not exist");
        return Ok(());
    }

    let mut total_tokens = 0;
    let mut total_pools = 0;
    let mut total_entries = 0;
    let mut total_size = 0;

    match std::fs::read_dir(cache_dir) {
        Ok(token_entries) => {
            for token_entry in token_entries {
                if let Ok(token_entry) = token_entry {
                    let token_path = token_entry.path();
                    if token_path.is_dir() {
                        if let Some(token_mint) = token_path.file_name().and_then(|s| s.to_str()) {
                            total_tokens += 1;
                            let mut token_pools = 0;
                            let mut token_entries = 0;
                            let mut token_size = 0;

                            match std::fs::read_dir(&token_path) {
                                Ok(pool_entries) => {
                                    for pool_entry in pool_entries {
                                        if let Ok(pool_entry) = pool_entry {
                                            let pool_path = pool_entry.path();
                                            if pool_path.extension().and_then(|s| s.to_str()) == Some("json") {
                                                if let Some(_pool_address) = pool_path.file_stem().and_then(|s| s.to_str()) {
                                                    token_pools += 1;

                                                    // Get file size
                                                    if let Ok(metadata) = std::fs::metadata(&pool_path) {
                                                        token_size += metadata.len();
                                                    }

                                                    // Try to load and count entries
                                                    if let Ok(contents) = std::fs::read_to_string(&pool_path) {
                                                        if
                                                            let Ok(pool_cache) =
                                                                serde_json::from_str::<screenerbot::tokens::pool::PoolPriceHistoryCache>(
                                                                    &contents
                                                                )
                                                        {
                                                            token_entries += pool_cache.entries.len();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Failed to read token directory {}: {}", token_mint, e);
                                }
                            }

                            total_pools += token_pools;
                            total_entries += token_entries;
                            total_size += token_size;

                            println!(
                                "📄 {}: {} entries from {} pools ({:.2} KB)",
                                token_mint,
                                token_entries,
                                token_pools,
                                (token_size as f64) / 1024.0
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to read cache directory: {}", e);
            return Ok(());
        }
    }

    println!("\n📋 Summary:");
    println!("   🗂️  Total tokens: {}", total_tokens);
    println!("   🏊 Total pools: {}", total_pools);
    println!("   📊 Total price entries: {}", total_entries);
    println!(
        "   💾 Total cache size: {} bytes ({:.2} KB)",
        total_size,
        (total_size as f64) / 1024.0
    );

    if total_tokens > 0 {
        let avg_pools = (total_pools as f64) / (total_tokens as f64);
        let avg_entries = (total_entries as f64) / (total_tokens as f64);
        println!("   📈 Average pools per token: {:.1}", avg_pools);
        println!("   📈 Average entries per token: {:.1}", avg_entries);
    }

    Ok(())
}
