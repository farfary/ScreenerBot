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

    println!("\n🎉 All NEW ARCHITECTURE tests completed!");
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
