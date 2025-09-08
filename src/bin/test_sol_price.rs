/// Test SOL Price Service
///
/// Simple test binary to verify SOL price service functionality

use screenerbot::sol_price::{
    start_sol_price_service,
    get_sol_price,
    get_sol_price_stats,
    force_refresh_sol_price,
};
use screenerbot::logger::{ log, LogTag, init_file_logging };
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    log(LogTag::Test, "INFO", "🚀 Starting SOL price service test");

    // Set up shutdown notification
    let shutdown = Arc::new(Notify::new());

    // Start the SOL price service
    start_sol_price_service(shutdown.clone()).await?;

    log(LogTag::Test, "INFO", "✅ SOL price service started");

    // Wait a bit for initial price fetch
    log(LogTag::Test, "INFO", "⏳ Waiting for initial price fetch...");
    sleep(Duration::from_secs(5)).await;

    // Test getting SOL price
    let price = get_sol_price();
    log(LogTag::Test, "INFO", &format!("💰 Current SOL price: ${:.4}", price));

    // Test getting detailed stats
    let stats = get_sol_price_stats();
    log(LogTag::Test, "INFO", &format!("📊 {}", stats));

    // Test force refresh
    log(LogTag::Test, "INFO", "🔄 Testing force refresh...");
    match force_refresh_sol_price().await {
        Ok(new_price) => {
            log(
                LogTag::Test,
                "SUCCESS",
                &format!("✅ Force refresh successful: ${:.4}", new_price)
            );
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("❌ Force refresh failed: {}", e));
        }
    }

    // Get updated stats
    let updated_stats = get_sol_price_stats();
    log(LogTag::Test, "INFO", &format!("📊 Updated: {}", updated_stats));

    // Run for a minute to see multiple updates
    log(LogTag::Test, "INFO", "⏱️ Running for 60 seconds to monitor price updates...");

    for i in 1..=12 {
        sleep(Duration::from_secs(5)).await;
        let current_price = get_sol_price();
        log(LogTag::Test, "INFO", &format!("📈 Check {}: SOL = ${:.4}", i, current_price));
    }

    // Shutdown the service
    log(LogTag::Test, "INFO", "🛑 Shutting down SOL price service");
    shutdown.notify_waiters();

    // Give it time to clean up
    sleep(Duration::from_secs(2)).await;

    log(LogTag::Test, "SUCCESS", "✅ SOL price service test completed successfully");

    Ok(())
}
