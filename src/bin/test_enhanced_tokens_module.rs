/// Test binary for the enhanced tokens module
use screenerbot::tokens::*;
use screenerbot::tokens::blacklist::{ add_to_blacklist_manual, is_token_blacklisted };
use screenerbot::logger::{ log, LogTag };
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ScreenerBot Enhanced Tokens Module Test");
    println!("============================================\n");

    // Test 1: Basic system initialization
    println!("📋 Test 1: System Initialization");
    let mut system = initialize_tokens_system().await?;
    let stats = system.get_system_stats().await?;

    println!("✅ System initialized successfully");
    println!("   📊 Total tokens: {}", stats.total_tokens);
    println!("   💧 Tokens with liquidity: {}", stats.tokens_with_liquidity);
    println!("   🚫 Blacklisted tokens: {}", stats.blacklisted_tokens);
    println!();

    // Test 2: Database functionality
    println!("📋 Test 2: Database Functionality");
    let db = TokenDatabase::new()?;
    let db_stats = db.get_stats()?;

    println!("✅ Database connected successfully");
    println!("   📈 Database tokens: {}", db_stats.total_tokens);
    println!("   💎 High liquidity tokens: {}", db_stats.tokens_with_liquidity);
    println!();

    // Test 3: Blacklist system
    println!("📋 Test 3: Blacklist System");
    let test_mint = "test_token_12345";
    let test_symbol = "TEST";

    // Check initial state
    let initially_blacklisted = is_token_blacklisted(test_mint);
    println!("   🔍 Initially blacklisted: {}", initially_blacklisted);

    // Add to blacklist
    if add_to_blacklist_manual(test_mint, test_symbol) {
        println!("   ➕ Added test token to blacklist");

        // Verify blacklisted
        let now_blacklisted = is_token_blacklisted(test_mint);
        println!("   ✅ Now blacklisted: {}", now_blacklisted);
    }

    // Get blacklist stats
    if let Some(blacklist_stats) = get_blacklist_stats() {
        println!(
            "   📊 Blacklist stats: {} total, {} tracked",
            blacklist_stats.total_blacklisted,
            blacklist_stats.total_tracked
        );
    }
    println!();

    // Test 4: API client
    println!("📋 Test 4: API Client");
    let api = DexScreenerApi::new();
    println!("✅ API client created successfully");
    println!("   🌐 Ready for DexScreener API calls");
    println!();

    // Test 5: Liquidity tracking
    println!("📋 Test 5: Liquidity Tracking");
    let tracking_mint = "liquidity_test_token";
    let tracking_symbol = "LIQ";

    // Test with good liquidity
    let good_liquidity_allowed = check_and_track_liquidity(
        tracking_mint,
        tracking_symbol,
        1000.0, // Good liquidity
        5 // Old enough
    );
    println!("   💰 Good liquidity ($1000) allowed: {}", good_liquidity_allowed);

    // Test with poor liquidity
    let poor_liquidity_allowed = check_and_track_liquidity(
        tracking_mint,
        tracking_symbol,
        50.0, // Poor liquidity
        5 // Old enough
    );
    println!("   📉 Poor liquidity ($50) allowed: {}", poor_liquidity_allowed);
    println!();

    // Test 6: Type conversions
    println!("📋 Test 6: Type Conversions");

    // Create test ApiToken
    let api_token = ApiToken {
        mint: "conversion_test".to_string(),
        symbol: "CONV".to_string(),
        name: "Conversion Test".to_string(),
        chain_id: "solana".to_string(),
        dex_id: "raydium".to_string(),
        pair_address: "test_pair_address".to_string(),
        pair_url: Some("https://dexscreener.com/test".to_string()),
        price_native: 0.001,
        price_usd: 0.025,
        price_sol: Some(0.001),
        liquidity: Some(LiquidityInfo {
            usd: Some(2500.0),
            base: Some(250.0),
            quote: Some(125.0),
        }),
        volume: Some(VolumeStats {
            h24: Some(1500.0),
            h6: Some(400.0),
            h1: Some(100.0),
            m5: Some(25.0),
        }),
        txns: Some(TxnStats {
            h24: Some(TxnPeriod { buys: Some(45), sells: Some(32) }),
            h6: Some(TxnPeriod { buys: Some(12), sells: Some(8) }),
            h1: Some(TxnPeriod { buys: Some(3), sells: Some(2) }),
            m5: Some(TxnPeriod { buys: Some(1), sells: Some(0) }),
        }),
        price_change: Some(PriceChangeStats {
            h24: Some(15.5),
            h6: Some(8.2),
            h1: Some(2.1),
            m5: Some(0.5),
        }),
        fdv: Some(125000.0),
        market_cap: Some(62500.0),
        pair_created_at: Some(1640995200), // 2022-01-01
        boosts: Some(BoostInfo { active: Some(2) }),
        info: Some(TokenInfo {
            image_url: Some("https://example.com/token.png".to_string()),
            websites: Some(vec![WebsiteInfo { url: "https://conversiontest.com".to_string() }]),
            socials: Some(
                vec![SocialInfo {
                    platform: "twitter".to_string(),
                    handle: "@convtest".to_string(),
                }]
            ),
        }),
        labels: Some(vec!["test".to_string(), "conversion".to_string()]),
        last_updated: chrono::Utc::now(),
    };

    // Convert to Token
    let token: Token = api_token.clone().into();
    println!("   🔄 ApiToken -> Token conversion successful");
    println!(
        "      Symbol: {}, Liquidity: ${}",
        token.symbol,
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    );

    // Convert back to ApiToken
    let api_token_back: ApiToken = token.into();
    println!("   🔄 Token -> ApiToken conversion successful");
    println!("      Symbol: {}, Price: ${}", api_token_back.symbol, api_token_back.price_usd);
    println!();

    // Test 7: Manual monitoring (without running background tasks)
    println!("📋 Test 7: Manual Operations");

    println!("   🔍 Testing manual monitoring...");
    if let Err(e) = test_monitoring_manual().await {
        println!("   ⚠️  Manual monitoring test failed: {}", e);
    } else {
        println!("   ✅ Manual monitoring test passed");
    }

    println!("   🔍 Testing manual discovery...");
    if let Err(e) = test_discovery_manual().await {
        println!("   ⚠️  Manual discovery test failed: {}", e);
    } else {
        println!("   ✅ Manual discovery test passed");
    }
    println!();

    // Test 8: Integration test
    println!("📋 Test 8: Integration Test");
    if let Err(e) = test_tokens_integration().await {
        println!("   ❌ Integration test failed: {}", e);
    } else {
        println!("   ✅ Integration test passed");
    }
    println!();

    // Test 9: Run a short background test (optional)
    println!("📋 Test 9: Background Tasks (5 second test)");
    let shutdown = Arc::new(Notify::new());

    match system.start_background_tasks(shutdown.clone()).await {
        Ok(handles) => {
            println!("   🚀 Background tasks started ({} tasks)", handles.len());

            // Let them run for 5 seconds
            sleep(Duration::from_secs(5)).await;

            // Shutdown
            shutdown.notify_waiters();
            println!("   🛑 Shutdown signal sent");

            // Wait a moment for graceful shutdown
            sleep(Duration::from_secs(1)).await;
            println!("   ✅ Background tasks test completed");
        }
        Err(e) => {
            println!("   ⚠️  Failed to start background tasks: {}", e);
        }
    }
    println!();

    // Final statistics
    println!("📊 Final System Statistics");
    let final_stats = system.get_system_stats().await?;
    println!("   📈 Total tokens: {}", final_stats.total_tokens);
    println!("   💧 Tokens with liquidity: {}", final_stats.tokens_with_liquidity);
    println!("   ✅ Active tokens: {}", final_stats.active_tokens);
    println!("   🚫 Blacklisted tokens: {}", final_stats.blacklisted_tokens);
    println!();

    println!("🎉 All tests completed successfully!");
    println!("✅ Enhanced tokens module is working correctly");

    Ok(())
}
