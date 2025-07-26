/// Test script to verify blacklist logging improvements
///
/// This test verifies:
/// 1. Blacklist tag is added to logger correctly
/// 2. Only logs when actual blacklisting occurs
/// 3. No excessive "Saved blacklist with 0 entries" messages

use screenerbot::tokens::blacklist::*;
use screenerbot::logger::{ log, LogTag, init_file_logging };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Blacklist Logging Improvements");
    println!("=========================================");

    // Initialize logging
    init_file_logging();

    // Test 1: Verify Blacklist log tag works
    println!("\n1. Testing Blacklist log tag...");
    log(LogTag::Blacklist, "TEST", "Blacklist log tag is working correctly");

    // Test 2: Track liquidity without blacklisting (should NOT save every time)
    println!("\n2. Testing liquidity tracking (should not spam save messages)...");
    for i in 1..=5 {
        let result = check_and_track_liquidity(
            "test_mint_123",
            "TEST",
            150.0, // Above threshold, should not blacklist
            24 // 24 hours old
        );
        println!("   Track #{}: Allowed = {}", i, result);
    }

    // Test 3: Track low liquidity to trigger blacklisting
    println!("\n3. Testing low liquidity tracking (should blacklist after 5 occurrences)...");
    for i in 1..=6 {
        let result = check_and_track_liquidity(
            "low_liquidity_mint",
            "LOWLIQ",
            50.0, // Below threshold
            24 // 24 hours old
        );
        println!("   Low liquidity track #{}: Allowed = {}", i, result);

        if i == 5 {
            println!("   ^ Should trigger blacklisting here");
        }
    }

    // Test 4: Check if token is blacklisted
    println!("\n4. Testing blacklist check...");
    let is_blacklisted = is_token_blacklisted("low_liquidity_mint");
    println!("   Is 'low_liquidity_mint' blacklisted? {}", is_blacklisted);

    // Test 5: Manual blacklisting
    println!("\n5. Testing manual blacklisting...");
    let success = add_to_blacklist_manual("manual_test_mint", "MANUAL");
    println!("   Manual blacklist success: {}", success);

    // Test 6: Get stats
    println!("\n6. Testing blacklist stats...");
    if let Some(stats) = get_blacklist_stats() {
        println!("   Total blacklisted: {}", stats.total_blacklisted);
        println!("   Total tracked: {}", stats.total_tracked);
        println!("   Reason breakdown: {:?}", stats.reason_breakdown);
    }

    println!("\n✅ Blacklist logging test completed!");
    println!("📝 Check the log output above - you should see:");
    println!("   - Blacklist logs with [BLACKLIST] tag in red");
    println!("   - Only 'SAVED' logs when actual blacklisting occurs");
    println!("   - No spam of 'Saved blacklist with 0 entries'");

    Ok(())
}
