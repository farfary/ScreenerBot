use screenerbot::discovery::update_tokens_from_mints;
use screenerbot::global::{ LIST_MINTS, LIST_TOKENS };
use screenerbot::trader::monitor_new_entries;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Updated Trader Logic (Liquidity Sorting + Sequential Checking)");

    // Add some test mints with different liquidity levels
    {
        let mut mints = LIST_MINTS.write().unwrap();
        mints.insert("So11111111111111111111111111111111111111112".to_string()); // SOL (highest liquidity)
        mints.insert("Cdq1WR1d4i2hMrqKUWgZeUbRpkhamGHSvm1f6ATpuray".to_string()); // ALT (medium liquidity)
        mints.insert("726MUA2D5tyUfgWuByU7hzccX3CjixKbBv6NTpDXeBEV".to_string()); // FlopCat (low/no liquidity)
        mints.insert("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string()); // BONK (for variety)
    }

    println!("📋 Added test mints to LIST_MINTS");

    // Create shutdown signal
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // First, populate tokens from API
    println!("🌐 Fetching token data from API...");
    match update_tokens_from_mints(shutdown_clone).await {
        Ok(_) => println!("✅ Token data fetched successfully"),
        Err(e) => {
            println!("❌ Failed to fetch token data: {}", e);
            return Err(e);
        }
    }

    // Display the tokens that will be processed
    println!("\n📊 Tokens available for trading analysis:");
    if let Ok(tokens) = LIST_TOKENS.read() {
        let mut sorted_tokens = tokens.clone();
        sorted_tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        for (i, token) in sorted_tokens.iter().enumerate() {
            let liquidity_usd = token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let price_sol = token.price_dexscreener_sol.unwrap_or(0.0);
            println!(
                "{}. {} ({}) - Price: {:.12} SOL, Liquidity: ${:.2}",
                i + 1,
                token.symbol,
                token.mint,
                price_sol,
                liquidity_usd
            );
        }
    }

    println!("\n🔄 Starting trader monitor for 10 seconds to observe behavior...");
    println!("   (Watch for liquidity-based sorting and sequential token checking)");

    // Start the monitor in a separate task
    let monitor_shutdown = shutdown.clone();
    let monitor_handle = tokio::spawn(async move {
        monitor_new_entries(monitor_shutdown).await;
    });

    // Let it run for 10 seconds
    sleep(Duration::from_secs(10)).await;

    // Signal shutdown
    println!("\n🛑 Stopping trader monitor...");
    shutdown.notify_waiters();

    // Wait for the monitor to finish
    let _ = monitor_handle.await;

    println!("✅ Test completed successfully");
    println!("\n📝 What to observe in the logs:");
    println!("   - Tokens should be checked in liquidity order (highest first)");
    println!("   - Processing should be sequential, not parallel");
    println!("   - Debug logs should show token index, liquidity values");
    println!("   - Entry opportunities detected based on price drops");

    Ok(())
}
