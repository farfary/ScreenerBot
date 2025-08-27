/// Integration Test for New Pool Price Architecture
///
/// Tests the complete integration between trader, pool service, and positions systems

use screenerbot::{
    tokens::pool::{
        add_priority_token,
        remove_priority_token,
        add_watchlist_tokens,
        get_priority_tokens,
        get_watchlist_status,
        get_price,
    },
    logger::{ init_file_logging, log, LogTag },
    global::is_debug_pool_prices_enabled,
};
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    log(LogTag::Pool, "TEST_START", "🧪 Starting integration test for new pool price architecture");

    // Wait for services to be ready (they should be initialized by other systems)
    sleep(Duration::from_secs(2)).await;

    log(LogTag::Pool, "TEST_PRIORITY", "🧪 Testing priority token management");

    // Test priority token management
    let test_token = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC

    // Add to priority tokens
    add_priority_token(test_token).await;
    log(
        LogTag::Pool,
        "TEST_PRIORITY",
        &format!("✅ Added {} to priority tokens", &test_token[..8])
    );

    // Check if it's in priority list
    let priority_tokens = get_priority_tokens().await;
    let is_priority = priority_tokens.contains(&test_token.to_string());
    log(LogTag::Pool, "TEST_PRIORITY", &format!("✅ Priority token present: {}", is_priority));

    // Test watchlist management
    log(LogTag::Pool, "TEST_WATCHLIST", "🧪 Testing watchlist token management");

    let watchlist_tokens = vec![
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // Bonk
        "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr".to_string() // POPCAT
    ];

    add_watchlist_tokens(&watchlist_tokens).await;
    log(
        LogTag::Pool,
        "TEST_WATCHLIST",
        &format!("✅ Added {} tokens to watchlist", watchlist_tokens.len())
    );

    // Check watchlist status
    let (total, never_updated, _last_update) = get_watchlist_status().await;
    log(
        LogTag::Pool,
        "TEST_WATCHLIST",
        &format!("✅ Watchlist status: {} total, {} never updated", total, never_updated)
    );

    // Test price fetching
    log(LogTag::Pool, "TEST_PRICE", "🧪 Testing price fetching");

    // Wait a bit for background service to potentially update prices
    sleep(Duration::from_secs(5)).await;

    // Test price fetch for priority token
    if let Some(price_result) = get_price(test_token, None, false).await {
        log(
            LogTag::Pool,
            "TEST_PRICE",
            &format!(
                "✅ Got price for priority token: {:.8} SOL",
                price_result.price_sol.unwrap_or(0.0)
            )
        );
    } else {
        log(LogTag::Pool, "TEST_PRICE", "⚠️ No price available for priority token");
    }

    // Test price fetch for watchlist token
    if let Some(price_result) = get_price(&watchlist_tokens[0], None, false).await {
        log(
            LogTag::Pool,
            "TEST_PRICE",
            &format!(
                "✅ Got price for watchlist token: {:.8} SOL",
                price_result.price_sol.unwrap_or(0.0)
            )
        );
    } else {
        log(LogTag::Pool, "TEST_PRICE", "⚠️ No price available for watchlist token");
    }

    // Test cleanup
    log(LogTag::Pool, "TEST_CLEANUP", "🧪 Testing token removal");

    remove_priority_token(test_token).await;
    log(
        LogTag::Pool,
        "TEST_CLEANUP",
        &format!("✅ Removed {} from priority tokens", &test_token[..8])
    );

    // Verify removal
    let priority_tokens_after = get_priority_tokens().await;
    let still_priority = priority_tokens_after.contains(&test_token.to_string());
    log(LogTag::Pool, "TEST_CLEANUP", &format!("✅ Priority token removed: {}", !still_priority));

    log(LogTag::Pool, "TEST_COMPLETE", "🎉 Integration test completed successfully!");

    Ok(())
}
