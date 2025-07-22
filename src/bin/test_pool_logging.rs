use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::logger::{ log, LogTag };
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Pool Logging System");
    println!("===============================");
    println!();

    // Test the new Pool log tag
    log(LogTag::Pool, "INFO", "Testing new POOL log tag functionality");
    log(LogTag::Pool, "SUCCESS", "Pool logging system initialized successfully");
    log(
        LogTag::Pool,
        "DEBUG",
        "This is a debug message (should only appear if ENABLE_POOL_DEBUG_LOGS = true)"
    );
    log(LogTag::Pool, "WARN", "This is a warning message");
    log(LogTag::Pool, "ERROR", "This is an error message");

    println!();
    log(LogTag::Pool, "INFO", "Debug configuration test completed");
    log(
        LogTag::Pool,
        "INFO",
        "Note: Debug logs are controlled by ENABLE_POOL_DEBUG_LOGS constant in pool_price.rs"
    );

    // Test different log types
    println!();
    log(LogTag::Pool, "PRICE", "Price calculation: 0.000123 SOL per token");
    log(LogTag::Pool, "BUY", "Pool buy operation simulated");
    log(LogTag::Pool, "SELL", "Pool sell operation simulated");
    log(LogTag::Pool, "BALANCE", "Pool balance checked: 1000 tokens");

    println!();
    log(LogTag::Pool, "SUCCESS", "All pool logging tests completed successfully! ✅");

    Ok(())
}
