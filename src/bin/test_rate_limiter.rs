use screenerbot::Config;
use screenerbot::api::{
    init_dexscreener_rate_limiter,
    wait_for_dexscreener_rate_limit,
    get_dexscreener_rate_limiter,
};
use std::time::Instant;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 DexScreener Rate Limiter Test");
    println!("═══════════════════════════════════");

    // Load config
    let config = Config::load("configs.json")?;
    println!("📋 Rate limit config:");
    println!("   Requests per minute: {}", config.dexscreener.rate_limit_requests_per_minute);
    println!("   Burst size: {}", config.dexscreener.rate_limit_burst_size);
    println!("   Retry attempts: {}", config.dexscreener.retry_attempts);

    // Initialize the rate limiter
    init_dexscreener_rate_limiter(config.dexscreener.clone()).await?;
    println!("✅ Rate limiter initialized");

    // Test burst allowance
    println!("\n🔥 Testing burst allowance...");
    let start = Instant::now();

    for i in 1..=6 {
        let request_start = Instant::now();
        wait_for_dexscreener_rate_limit().await?;
        let elapsed = request_start.elapsed();

        println!("   Request {}: Waited {:?}", i, elapsed);
    }

    let total_time = start.elapsed();
    println!("   Total time for 6 requests: {:?}", total_time);

    // Get status
    let limiter = get_dexscreener_rate_limiter()?;
    let status = limiter.get_status().await;
    println!("\n📊 Rate limiter status:");
    println!("   Burst allowance remaining: {}", status.burst_allowance_remaining);
    println!("   Time since last request: {:?}", status.time_since_last_request);
    println!("   Requests per minute: {}", status.requests_per_minute);

    // Test normal rate limiting
    println!("\n⏰ Testing normal rate limiting...");
    let start = Instant::now();

    for i in 1..=3 {
        let request_start = Instant::now();
        wait_for_dexscreener_rate_limit().await?;
        let elapsed = request_start.elapsed();

        println!("   Request {}: Waited {:?}", i, elapsed);
    }

    let total_time = start.elapsed();
    println!("   Total time for 3 more requests: {:?}", total_time);

    println!("\n✅ Rate limiter test completed successfully!");
    Ok(())
}
