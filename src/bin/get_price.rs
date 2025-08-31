/// Universal Price Function Test Binary (NEW ARCHITECTURE)
///
/// This binary tests the new batch-based price system with background monitoring.
/// It demonstrates watchlist management and read-only universal get_price function.
use screenerbot::{
    global::is_debug_pool_prices_enabled,
    logger::init_file_logging,
    tokens::{
        dexscreener::get_global_dexscreener_api,
        initialize_tokens_system,
        pool::{
            // New watchlist management functions
            add_priority_token,
            add_watchlist_token,
            add_watchlist_tokens,
            clear_priority_tokens,
            clear_watchlist_tokens,
            get_pool_service,
            get_price,
            get_priority_tokens,
            get_watchlist_status,
            get_watchlist_tokens,
            PriceOptions,
            PriceResult,
        },
    },
};
use std::time::Instant;
use tokio::time::{sleep, Duration};

/// Test token for demonstrations (A8C3... pump token)
const TEST_TOKEN: &str = "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump";

/// Additional test tokens for comprehensive testing
const TEST_TOKENS: &[(&str, &str)] = &[
    (
        "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump",
        "TANUKI (Pump.fun)",
    ),
    (
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "USDC (Stable)",
    ),
    (
        "So11111111111111111111111111111111111111112",
        "SOL (Native)",
    ),
    (
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "Bonk (Popular)",
    ),
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Universal Price Function Test Suite (NEW ARCHITECTURE)");
    println!("==========================================================");

    // Initialize all systems
    init_file_logging();
    println!("✅ Logging initialized");

    let _tokens_system = initialize_tokens_system().await?;
    println!("✅ Token system initialized");

    // Initialize DexScreener API through the global function
    let _api = get_global_dexscreener_api();
    println!("✅ DexScreener API initialized");

    // Start background monitoring service
    let service = get_pool_service();
    service.start_monitoring().await;
    println!("✅ Background monitoring service started");

    // Enable debug mode is read-only, we can't set it here
    println!("✅ Debug mode status: {}", is_debug_pool_prices_enabled());

    println!("\n📋 Test Menu (NEW ARCHITECTURE):");
    println!("1. Watchlist management tests");
    println!("2. Background batch processing tests");
    println!("3. Read-only get_price tests");
    println!("4. Performance comparison tests");
    println!("5. Real-time monitoring demo");

    // Run all tests
    test_watchlist_management().await?;
    test_batch_processing().await?;
    test_readonly_getprice().await?;
    test_performance_comparison().await?;
    test_realtime_monitoring().await?;

    Ok(())
}

/// Test 1: Watchlist Management
async fn test_watchlist_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Test 1: Watchlist Management");
    println!("================================");

    // Clear existing tokens for clean test
    clear_priority_tokens().await;
    clear_watchlist_tokens().await;

    // Test adding priority tokens
    add_priority_token(TEST_TOKEN).await;
    println!("✅ Added priority token: {}", TEST_TOKEN);

    // Test adding watchlist tokens
    let watchlist_tokens: Vec<String> = TEST_TOKENS
        .iter()
        .map(|(addr, _)| addr.to_string())
        .collect();
    add_watchlist_tokens(&watchlist_tokens).await;
    println!("✅ Added {} tokens to watchlist", watchlist_tokens.len());

    // Check status
    let priority_tokens = get_priority_tokens().await;
    let watchlist = get_watchlist_tokens().await;
    let (total, never_updated, last_update) = get_watchlist_status().await;

    println!("📊 Status:");
    println!("  Priority tokens: {}", priority_tokens.len());
    println!("  Watchlist tokens: {}", watchlist.len());
    println!("  Never updated: {}", never_updated);
    println!("  Last update: {:?}", last_update);

    Ok(())
}

/// Test 2: Background Batch Processing
async fn test_batch_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Test 2: Background Batch Processing");
    println!("======================================");

    println!("⏳ Waiting for background service to process tokens...");

    // Wait for a few update cycles
    for i in 1..=3 {
        sleep(Duration::from_secs(6)).await; // Wait longer than 5s priority interval

        let (total, never_updated, last_update) = get_watchlist_status().await;
        println!(
            "  Cycle {}: {} total, {} never updated, last: {:?}",
            i, total, never_updated, last_update
        );

        if never_updated == 0 {
            println!("✅ All tokens have been updated by background service!");
            break;
        }
    }

    Ok(())
}

/// Test 3: Read-only get_price Function
async fn test_readonly_getprice() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Test 3: Read-only get_price Function");
    println!("=======================================");

    let options = PriceOptions::default();

    for (token_addr, token_name) in TEST_TOKENS {
        let start_time = Instant::now();
        let result = get_price(token_addr, Some(options.clone()), false).await;
        let duration = start_time.elapsed();

        match result {
            Some(price_result) => {
                println!(
                    "✅ {} ({}): {:.8} SOL [cached: {}] ({:?})",
                    token_name,
                    &token_addr[..8],
                    price_result.price_sol.unwrap_or(0.0),
                    price_result.is_cached,
                    duration
                );
            }
            None => {
                println!(
                    "❌ {} ({}): No price available ({:?})",
                    token_name,
                    &token_addr[..8],
                    duration
                );
            }
        }
    }

    Ok(())
}

/// Test 4: Performance Comparison
async fn test_performance_comparison() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Test 4: Performance Comparison");
    println!("==================================");

    let test_token = TEST_TOKEN;
    let iterations = 10;

    println!("⚡ Testing {} iterations of get_price calls...", iterations);

    let mut total_duration = Duration::default();
    let mut successful_calls = 0;

    for i in 1..=iterations {
        let start_time = Instant::now();
        let result = get_price(test_token, Some(PriceOptions::default()), false).await;
        let duration = start_time.elapsed();

        total_duration += duration;

        if result.is_some() {
            successful_calls += 1;
        }

        if i % 3 == 0 {
            println!("  Completed {} calls, avg: {:?}", i, total_duration / i);
        }
    }

    println!("📊 Performance Results:");
    println!("  Total calls: {}", iterations);
    println!("  Successful: {}", successful_calls);
    println!("  Average duration: {:?}", total_duration / iterations);
    println!(
        "  Success rate: {:.1}%",
        ((successful_calls as f64) / (iterations as f64)) * 100.0
    );

    Ok(())
}

/// Test 5: Real-time Monitoring Demo
async fn test_realtime_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Test 5: Real-time Monitoring Demo");
    println!("====================================");

    println!("⏳ Monitoring price updates for 30 seconds...");

    let monitor_token = TEST_TOKEN;
    let mut last_price: Option<f64> = None;
    let start_time = Instant::now();

    while start_time.elapsed() < Duration::from_secs(30) {
        if let Some(result) = get_price(monitor_token, Some(PriceOptions::default()), false).await {
            if let Some(current_price) = result.price_sol {
                if last_price.is_none() || (last_price.unwrap() - current_price).abs() > 0.0001 {
                    println!(
                        "  🔄 Price update: {:.8} SOL [{}] at {:?}",
                        current_price,
                        result.source,
                        chrono::Utc::now().format("%H:%M:%S")
                    );
                    last_price = Some(current_price);
                }
            }
        }

        sleep(Duration::from_secs(2)).await;
    }

    println!("✅ Real-time monitoring completed");

    Ok(())
}

/// Test 2: Compare different price options
async fn run_price_options_tests() {
    println!("\n🧪 TEST 2: Price Options Comparison");
    println!("───────────────────────────────────");

    let options_tests = vec![
        ("Simple", PriceOptions::simple()),
        ("Comprehensive", PriceOptions::comprehensive()),
        ("API Only", PriceOptions::api_only()),
        ("Pool Only", PriceOptions::pool_only()),
        ("Fresh (Force Refresh)", PriceOptions::fresh()),
        (
            "Custom High Liquidity",
            PriceOptions {
                include_pool: true,
                include_api: true,
                allow_cache: false,
                force_refresh: true,
                timeout_secs: Some(30),
                min_liquidity_usd: Some(1_000_000.0), // 1M USD minimum
            },
        ),
    ];

    for (test_name, options) in options_tests {
        println!("\n🔬 Testing: {}", test_name);

        let start = Instant::now();
        let result = get_price(TEST_TOKEN, Some(options), false).await;
        let duration = start.elapsed();

        match result {
            Some(price_result) => {
                display_price_result(
                    &price_result,
                    &format!("{} ({}ms)", test_name, duration.as_millis()),
                );
            }
            None => {
                println!("   ❌ No result for {} - possibly filtered out", test_name);
            }
        }
    }
}

/// Test 3: Multiple tokens comparison
async fn run_multiple_token_tests() {
    println!("\n🧪 TEST 3: Multiple Token Comparison");
    println!("────────────────────────────────────");

    for (token_address, token_name) in TEST_TOKENS {
        println!("\n🪙 Testing: {}", token_name);

        let start = Instant::now();
        let result = get_price(token_address, Some(PriceOptions::comprehensive()), false).await;
        let duration = start.elapsed();

        match result {
            Some(price_result) => {
                display_price_result(
                    &price_result,
                    &format!("{} ({}ms)", token_name, duration.as_millis()),
                );
            }
            None => {
                println!("   ❌ Failed to get price for {}", token_name);
            }
        }
    }
}

/// Test 4: Performance benchmarks
async fn run_performance_tests() {
    println!("\n🧪 TEST 4: Performance Benchmarks");
    println!("──────────────────────────────────");

    let iterations = 5;
    let mut total_duration = Duration::from_millis(0);
    let mut success_count = 0;

    println!(
        "🏃 Running {} iterations for performance testing...",
        iterations
    );

    for i in 1..=iterations {
        let start = Instant::now();
        let result = get_price(TEST_TOKEN, Some(PriceOptions::fresh()), false).await;
        let duration = start.elapsed();
        total_duration += duration;

        if result.is_some() {
            success_count += 1;
            println!("   ✅ Iteration {}: {}ms", i, duration.as_millis());
        } else {
            println!("   ❌ Iteration {}: failed", i);
        }

        // Small delay between iterations
        sleep(Duration::from_millis(1000)).await;
    }

    let avg_duration = total_duration / (iterations as u32);
    println!("\n📊 Performance Summary:");
    println!("   • Total iterations: {}", iterations);
    println!("   • Successful: {}", success_count);
    println!(
        "   • Success rate: {:.1}%",
        ((success_count as f64) / (iterations as f64)) * 100.0
    );
    println!("   • Average duration: {}ms", avg_duration.as_millis());
    println!("   • Total duration: {}ms", total_duration.as_millis());
}

/// Test 5: Error handling and edge cases
async fn run_error_handling_tests() {
    println!("\n🧪 TEST 5: Error Handling Tests");
    println!("───────────────────────────────");

    let error_tests = vec![
        ("Invalid Token", "InvalidTokenAddress123"),
        (
            "Non-existent Token",
            "11111111111111111111111111111111111111111",
        ),
        ("Empty String", ""),
        ("Too Short", "ABC"),
    ];

    for (test_name, token_address) in error_tests {
        println!("\n🚨 Testing: {}", test_name);

        let start = Instant::now();
        let result = get_price(token_address, Some(PriceOptions::simple()), false).await;
        let duration = start.elapsed();

        match result {
            Some(price_result) => {
                println!(
                    "   ⚠️  Unexpected success for {}: {:.6} SOL",
                    test_name,
                    price_result.price_sol.unwrap_or(0.0)
                );
            }
            None => {
                println!(
                    "   ✅ Correctly handled error for {} ({}ms)",
                    test_name,
                    duration.as_millis()
                );
            }
        }
    }
}

/// Test 6: Real-time price monitoring
async fn run_realtime_monitoring() {
    println!("\n🧪 TEST 6: Real-time Price Monitoring");
    println!("─────────────────────────────────────");

    let monitoring_duration = 60; // seconds
    let check_interval = 10; // seconds
    let iterations = monitoring_duration / check_interval;

    println!(
        "📡 Monitoring {} for {} seconds (checking every {}s)",
        TEST_TOKEN, monitoring_duration, check_interval
    );

    let mut previous_price: Option<f64> = None;
    let mut price_changes = 0;
    let mut min_price = f64::MAX;
    let mut max_price = f64::MIN;

    for i in 1..=iterations {
        println!("\n⏰ Check #{}/{}", i, iterations);

        let start = Instant::now();
        let result = get_price(TEST_TOKEN, Some(PriceOptions::fresh()), false).await;
        let duration = start.elapsed();

        match result {
            Some(price_result) => {
                if let Some(current_price) =
                    price_result.pool_price_sol.or(price_result.api_price_sol)
                {
                    min_price = min_price.min(current_price);
                    max_price = max_price.max(current_price);

                    let change_indicator = if let Some(prev) = previous_price {
                        if current_price > prev {
                            price_changes += 1;
                            "📈 +".to_string()
                        } else if current_price < prev {
                            price_changes += 1;
                            "📉 -".to_string()
                        } else {
                            "➡️ =".to_string()
                        }
                    } else {
                        "🎯".to_string()
                    };

                    println!(
                        "   {} Price: {:.12} SOL ({}ms)",
                        change_indicator,
                        current_price,
                        duration.as_millis()
                    );
                    println!(
                        "   📊 Source: {} | Cached: {}",
                        price_result.source, price_result.is_cached
                    );

                    if let Some(pool_addr) = &price_result.pool_address {
                        println!(
                            "   🏊 Pool: {}...{}",
                            &pool_addr[..8],
                            &pool_addr[pool_addr.len() - 8..]
                        );
                    }

                    if let Some(liquidity) = price_result.liquidity_usd {
                        println!("   💧 Liquidity: ${:.0}", liquidity);
                    }

                    previous_price = Some(current_price);
                } else {
                    println!("   ❌ No price data available");
                }
            }
            None => {
                println!("   ❌ Failed to get price");
            }
        }

        if i < iterations {
            sleep(Duration::from_secs(check_interval as u64)).await;
        }
    }

    println!("\n📈 Monitoring Summary:");
    println!("   • Total checks: {}", iterations);
    println!("   • Price changes: {}", price_changes);
    if min_price != f64::MAX && max_price != f64::MIN {
        println!(
            "   • Price range: {:.12} - {:.12} SOL",
            min_price, max_price
        );
        println!(
            "   • Price spread: {:.12} SOL ({:.2}%)",
            max_price - min_price,
            ((max_price - min_price) / min_price) * 100.0
        );
    }
}

/// Display a comprehensive price result
fn display_price_result(result: &PriceResult, title: &str) {
    println!("💰 {}", title);
    println!("   ├─ Token: {}", &result.token_address[..8]);
    println!(
        "   ├─ Primary Price: {:.12} SOL",
        result.price_sol.unwrap_or(0.0)
    );

    if let Some(api_price) = result.api_price_sol {
        println!("   ├─ API Price: {:.12} SOL", api_price);
    }

    if let Some(pool_price) = result.pool_price_sol {
        println!("   ├─ Pool Price: {:.12} SOL", pool_price);

        // Show price difference if both API and pool prices exist
        if let Some(api_price) = result.api_price_sol {
            let diff = pool_price - api_price;
            let diff_percent = (diff / api_price) * 100.0;
            println!(
                "   ├─ Pool vs API: {:+.12} SOL ({:+.2}%)",
                diff, diff_percent
            );
        }
    }

    if let Some(usd_price) = result.price_usd {
        println!("   ├─ USD Price: ${:.6}", usd_price);
    }

    println!("   ├─ Source: {}", result.source);
    println!("   ├─ Cached: {}", result.is_cached);
    println!(
        "   ├─ Calculated: {}",
        result.calculated_at.format("%H:%M:%S")
    );

    if let Some(pool_addr) = &result.pool_address {
        println!(
            "   ├─ Pool: {}...{}",
            &pool_addr[..8],
            &pool_addr[pool_addr.len() - 8..]
        );
    }

    if let Some(dex_id) = &result.dex_id {
        println!("   ├─ DEX: {}", dex_id);
    }

    if let Some(pool_type) = &result.pool_type {
        println!("   ├─ Pool Type: {}", pool_type);
    }

    if let Some(liquidity) = result.liquidity_usd {
        println!("   ├─ Liquidity: ${:.0}", liquidity);
    }

    if let Some(volume) = result.volume_24h {
        println!("   ├─ Volume 24h: ${:.0}", volume);
    }

    println!("   ├─ Has Pool Data: {}", result.has_pool_data());
    println!("   └─ Has API Data: {}", result.has_api_data());
}
