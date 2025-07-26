/// Test 5-Second Price Updates
///
/// This binary tests the new 5-second price update system to ensure
/// it's working correctly with the updated configuration.

use screenerbot::logger::{ log, LogTag };
use screenerbot::tokens::{
    initialize_tokens_system,
    get_token_price_safe,
    get_price_cache_stats,
    update_open_positions_safe,
    start_enhanced_monitoring,
};
use std::sync::Arc;
use tokio::time::{ sleep, Duration };
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing 5-Second Price Update System");
    println!("=====================================");

    // Initialize the tokens system
    log(LogTag::System, "INIT", "Initializing tokens system for 5-second price updates test");

    let _system = initialize_tokens_system().await?;
    log(LogTag::System, "SUCCESS", "Tokens system initialized successfully");

    // Set up some test tokens (common ones that should have prices)
    let test_mints = vec![
        "So11111111111111111111111111111111111111112".to_string(), // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string() // USDC
    ];

    // Update open positions to prioritize these tokens
    update_open_positions_safe(test_mints.clone()).await;
    log(LogTag::System, "TEST", "Set test tokens as open positions for priority monitoring");

    // Start the enhanced monitoring
    let shutdown = Arc::new(Notify::new());
    let _monitor_handle = start_enhanced_monitoring(shutdown.clone()).await?;

    log(LogTag::System, "START", "Enhanced monitoring started with 5-second intervals");

    // Monitor for 30 seconds to see the updates
    println!("\n⏱️  Monitoring price updates for 30 seconds...");
    println!("Expected: Updates every ~5 seconds");
    println!();

    for i in 1..=6 {
        sleep(Duration::from_secs(5)).await;

        println!("📊 Update #{} ({}s elapsed):", i, i * 5);

        // Check cache stats
        let cache_stats = get_price_cache_stats().await;
        println!("   {}", cache_stats);

        // Check prices for test tokens
        for mint in &test_mints {
            let price = get_token_price_safe(mint).await;
            let symbol = if mint.contains("So111") { "SOL" } else { "USDC" };

            match price {
                Some(p) => println!("   💰 {}: {:.6} SOL", symbol, p),
                None => println!("   ❌ {}: No price available", symbol),
            }
        }

        println!();
    }

    // Shutdown
    shutdown.notify_waiters();
    sleep(Duration::from_millis(100)).await; // Give time for cleanup

    println!("✅ 5-Second Price Update Test Complete!");
    println!("   - Monitoring ran for 30 seconds");
    println!("   - Expected 6 update cycles");
    println!("   - Check logs above for actual update frequency");

    Ok(())
}
