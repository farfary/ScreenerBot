// test_token_monitoring_system.rs - Test the new token monitoring system with position exclusion
use screenerbot::*;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing New Token Monitoring System");
    println!("=====================================");

    // Initialize systems
    global::initialize_token_database()?;

    // Create shutdown signal
    let shutdown = Arc::new(Notify::new());

    // Start the monitoring systems
    let monitor_shutdown = shutdown.clone();
    let monitor_handle = tokio::spawn(async move {
        monitor::monitor(monitor_shutdown).await;
    });

    println!("✅ Started monitoring systems:");
    println!("   - Discovery Manager (mint discovery)");
    println!("   - Token Monitor (main watch list, excludes open positions)");
    println!("   - Position Monitor (fast monitoring for open positions)");
    println!("   - Cleanup tasks");

    // Let it run for a bit to see the monitoring in action
    println!("\n🕐 Running monitoring for 30 seconds...");
    sleep(Duration::from_secs(30)).await;

    // Test position exclusion by simulating an open position
    println!("\n🧪 Testing position exclusion logic...");

    // Get current open position mints
    let open_mints = position_monitor::get_open_position_mints();
    println!("📊 Current open position mints: {}", open_mints.len());

    for mint in &open_mints {
        println!("   - {}", mint);
    }

    // Get tokens from LIST_TOKENS
    let token_count = if let Ok(tokens) = global::LIST_TOKENS.read() {
        println!("📈 Current tokens in LIST_TOKENS: {}", tokens.len());

        // Count how many are position tokens
        let position_tokens = tokens
            .iter()
            .filter(|t| open_mints.contains(&t.mint))
            .count();

        println!("   - Position tokens in list: {}", position_tokens);
        println!("   - Non-position tokens: {}", tokens.len() - position_tokens);

        tokens.len()
    } else {
        0
    };

    // Display monitoring statistics
    println!("\n📊 Monitoring System Statistics:");
    println!("================================");
    println!("Total tokens being tracked: {}", token_count);
    println!("Open position tokens (fast monitoring): {}", open_mints.len());
    println!(
        "Regular tokens (standard monitoring): {}",
        token_count.saturating_sub(open_mints.len())
    );

    // Get blacklist statistics
    let (blacklisted_count, tracked_count) = token_blacklist::get_blacklist_stats();
    println!("Blacklisted tokens: {}", blacklisted_count);
    println!("Tracked for liquidity: {}", tracked_count);

    println!("\n✅ Test completed. Shutting down monitoring systems...");

    // Shutdown
    shutdown.notify_waiters();
    monitor_handle.abort();

    sleep(Duration::from_secs(2)).await;
    println!("🏁 Test finished successfully!");

    Ok(())
}
