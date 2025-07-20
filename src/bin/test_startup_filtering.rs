// Test that trading logic only uses tokens discovered after startup
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag };
use screenerbot::trader::monitor_new_entries;
use std::sync::Arc;
use tokio::sync::Notify;
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Testing startup time-based token filtering");

    // Initialize token database
    initialize_token_database()?;

    // Create some fake tokens with different timestamps
    let old_time = Utc::now() - Duration::hours(2); // 2 hours ago (before startup)
    let new_time = Utc::now() + Duration::minutes(10); // 10 minutes from now (after startup)

    let old_token = Token {
        mint: "OLD_TOKEN_MINT".to_string(),
        symbol: "OLD".to_string(),
        name: "Old Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        created_at: Some(old_time), // This should be filtered out
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        price_dexscreener_sol: Some(0.001),
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(50000.0), // High liquidity to ensure it would be considered
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    let new_token = Token {
        mint: "NEW_TOKEN_MINT".to_string(),
        symbol: "NEW".to_string(),
        name: "New Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        created_at: Some(new_time), // This should be included
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        price_dexscreener_sol: Some(0.002),
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(60000.0), // High liquidity to ensure it would be considered
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    let no_timestamp_token = Token {
        mint: "NO_TIMESTAMP_MINT".to_string(),
        symbol: "NTS".to_string(),
        name: "No Timestamp Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        created_at: None, // This should be included (treated as new)
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        price_dexscreener_sol: Some(0.003),
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(40000.0), // High liquidity to ensure it would be considered
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    // Add tokens to LIST_TOKENS
    {
        let mut tokens = LIST_TOKENS.write().unwrap();
        tokens.push(old_token.clone());
        tokens.push(new_token.clone());
        tokens.push(no_timestamp_token.clone());
    }

    log(LogTag::System, "INFO", "Added test tokens to LIST_TOKENS:");
    log(
        LogTag::System,
        "INFO",
        &format!("  OLD: created {}", old_time.format("%Y-%m-%d %H:%M:%S"))
    );
    log(
        LogTag::System,
        "INFO",
        &format!("  NEW: created {}", new_time.format("%Y-%m-%d %H:%M:%S"))
    );
    log(LogTag::System, "INFO", "  NTS: no timestamp (should be treated as new)");
    log(
        LogTag::System,
        "INFO",
        &format!("  STARTUP: {}", STARTUP_TIME.format("%Y-%m-%d %H:%M:%S"))
    );

    // Cache them to database
    cache_token_to_db(&old_token, "test")?;
    cache_token_to_db(&new_token, "test")?;
    cache_token_to_db(&no_timestamp_token, "test")?;

    // Create a short-lived shutdown signal for testing
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Start the monitor_new_entries task in background with timeout
    let monitor_handle = tokio::spawn(async move { // Give it a moment to start processing
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            monitor_new_entries(shutdown_clone)
        ).await });

    // Wait a bit to let it process
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Signal shutdown
    shutdown.notify_waiters();

    // Wait for the task to complete
    let _ = monitor_handle.await;

    log(LogTag::System, "SUCCESS", "Token filtering test completed");
    log(
        LogTag::System,
        "INFO",
        "Check the logs above to see which tokens were considered for trading"
    );
    log(
        LogTag::System,
        "INFO",
        "OLD token should be filtered out, NEW and NTS should be considered"
    );

    Ok(())
}
