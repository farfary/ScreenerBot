use screenerbot::global::CMD_ARGS;
use std::env;

/// Simple demonstration of the pool price debug argument system
fn main() {
    // Set up command line arguments
    let args: Vec<String> = env::args().collect();
    if let Ok(mut cmd_args) = CMD_ARGS.lock() {
        *cmd_args = args;
    }

    println!("🧪 Pool Price Debug Argument Demo");
    println!("================================");

    // Check if debug mode is enabled
    let debug_enabled = check_debug_pool_price();

    if debug_enabled {
        println!("✅ DEBUG MODE ENABLED");
        println!("   You would see detailed debug logs like:");
        println!("   🔍 [POOL] DEBUG: Pool type detected: RaydiumCpmm");
        println!("   🔍 [POOL] DEBUG: Cache HIT: Program IDs for token");
        println!("   🔍 [POOL] DEBUG: Received 5 pairs from API");
        println!("   🔍 [POOL] DEBUG: Pool data parsed successfully");
        println!("   🔍 [POOL] DEBUG: LaunchLab pool data length: 317 bytes");
        println!("   🔍 [POOL] DEBUG: First 100 bytes: [1, 2, 3, ...]");
    } else {
        println!("📊 SUMMARY MODE ENABLED");
        println!("   You would see only summary logs like:");
        println!(
            "   📋 [POOL] INFO: Pool Price System - Discovery: 3/5 pools processed (60.0% success)"
        );
        println!(
            "   📋 [POOL] INFO: Pool Price System - Token Analysis: 2/3 pools processed (66.7% success)"
        );
    }

    println!();
    println!("🔧 How to use:");
    println!("   Normal mode:  cargo run --bin screenerbot");
    println!("   Debug mode:   cargo run --bin screenerbot -- --debug-pool-price");
    println!();
    println!("💡 This also works with other binaries:");
    println!("   cargo run --bin test_pool_caching -- --debug-pool-price");
    println!("   cargo run --bin tool_pool_discovery -- --debug-pool-price");

    // Demonstrate the function actually working
    println!();
    println!("🧬 Testing the actual function:");
    demonstrate_conditional_logging();
}

/// Check if debug pool price mode is enabled via command line args
fn check_debug_pool_price() -> bool {
    if let Ok(args) = CMD_ARGS.lock() {
        args.contains(&"--debug-pool-price".to_string())
    } else {
        false
    }
}

/// Helper function for conditional debug logging (same as in pool_price.rs)
fn debug_log(log_type: &str, message: &str) {
    if check_debug_pool_price() {
        println!("🔍 [POOL] {}: {}", log_type, message);
    }
}

/// Helper function for regular pool logging (always visible)
fn pool_log(log_type: &str, message: &str) {
    println!("📋 [POOL] {}: {}", log_type, message);
}

/// Log pool price system summary when debug mode is disabled
fn log_pool_summary(operation: &str, success_count: usize, total_count: usize) {
    if !check_debug_pool_price() && total_count > 0 {
        let success_rate = ((success_count as f64) / (total_count as f64)) * 100.0;
        pool_log(
            "INFO",
            &format!(
                "Pool Price System - {}: {}/{} pools processed ({:.1}% success)",
                operation,
                success_count,
                total_count,
                success_rate
            )
        );
    }
}

/// Demonstrate the conditional logging behavior
fn demonstrate_conditional_logging() {
    pool_log("INFO", "Starting pool discovery simulation...");

    debug_log("DEBUG", "Determining pool type: dex_id='raydium', labels=['CPMM']");
    debug_log("DEBUG", "Cache MISS for token DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263");
    debug_log("DEBUG", "Received 5 pairs from API");

    pool_log("SUCCESS", "Detected: Raydium CPMM pool");

    debug_log("DEBUG", "Pool data parsed successfully");
    debug_log("DEBUG", "Meteora DAMM v2 sqrt_price: 12345");

    pool_log("INFO", "Fetching biggest pool for token");

    debug_log("DEBUG", "Using cached pool for token: BONK");

    // Simulate completion with summary
    if check_debug_pool_price() {
        pool_log("SUCCESS", "Completed price calculation for 3 pools (2 successful)");
    } else {
        log_pool_summary("Simulation", 2, 3);
    }

    println!();
    println!(
        "✅ Demo completed! Notice how debug logs only appear when --debug-pool-price is used."
    );
}
