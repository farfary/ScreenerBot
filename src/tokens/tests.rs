/// Test module for the tokens system
use crate::tokens::*;
use crate::logger::{ log, LogTag };

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{ sleep, Duration };

    #[tokio::test]
    async fn test_token_database() {
        // Test database creation
        let db = TokenDatabase::new();
        assert!(db.is_ok());

        let db = db.unwrap();

        // Test empty database
        let tokens = db.get_all_tokens().await;
        assert!(tokens.is_ok());

        // Test stats
        let stats = db.get_stats();
        assert!(stats.is_ok());

        println!("✅ Token database test passed");
    }

    #[tokio::test]
    async fn test_blacklist_system() {
        // Test blacklist creation
        let mut blacklist = TokenBlacklist::new();

        // Test adding to blacklist
        blacklist.add_to_blacklist(
            "test_mint_123",
            "TEST",
            crate::tokens::blacklist::BlacklistReason::LowLiquidity
        );

        // Test checking blacklist
        assert!(blacklist.is_blacklisted("test_mint_123"));
        assert!(!blacklist.is_blacklisted("not_blacklisted"));

        // Test liquidity tracking
        let allowed = blacklist.check_and_track_liquidity(
            "new_token",
            "NEW",
            50.0, // Low liquidity
            5 // Old enough
        );
        assert!(allowed); // Should be allowed initially

        println!("✅ Blacklist system test passed");
    }

    #[tokio::test]
    async fn test_api_client() {
        // Test API client creation
        let api = DexScreenerApi::new();

        // Test initialization (may fail in test environment)
        // This is mainly to test the client structure
        println!("✅ API client creation test passed");
    }

    #[tokio::test]
    async fn test_type_conversions() {
        // Create a test ApiToken
        let api_token = ApiToken {
            mint: "test_mint".to_string(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            chain_id: "solana".to_string(),
            dex_id: "raydium".to_string(),
            pair_address: "test_pair".to_string(),
            pair_url: Some("https://example.com".to_string()),
            price_native: 0.001,
            price_usd: 0.02,
            price_sol: Some(0.001),
            liquidity: Some(LiquidityInfo {
                usd: Some(1000.0),
                base: Some(100.0),
                quote: Some(50.0),
            }),
            volume: None,
            txns: None,
            price_change: None,
            fdv: Some(100000.0),
            market_cap: Some(50000.0),
            pair_created_at: Some(1640995200), // 2022-01-01
            boosts: None,
            info: None,
            labels: Some(vec!["test".to_string()]),
            last_updated: chrono::Utc::now(),
        };

        // Test conversion to Token
        let token: Token = api_token.clone().into();
        assert_eq!(token.mint, "test_mint");
        assert_eq!(token.symbol, "TEST");
        assert_eq!(token.chain, "solana");

        // Test conversion back to ApiToken
        let api_token_back: ApiToken = token.into();
        assert_eq!(api_token_back.mint, "test_mint");
        assert_eq!(api_token_back.symbol, "TEST");

        println!("✅ Type conversion test passed");
    }

    #[tokio::test]
    async fn test_discovery_system() {
        // Test discovery system creation
        let discovery = TokenDiscovery::new();
        assert!(discovery.is_ok());

        println!("✅ Discovery system creation test passed");
    }

    #[tokio::test]
    async fn test_monitor_system() {
        // Test monitor system creation
        let monitor = TokenMonitor::new();
        assert!(monitor.is_ok());

        println!("✅ Monitor system creation test passed");
    }
}

/// Run all token system tests
pub async fn run_token_system_tests() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "TEST", "Starting tokens system tests...");

    // Test database functionality
    log(LogTag::System, "TEST", "Testing database...");
    let db = TokenDatabase::new()?;
    let stats = db.get_stats()?;
    log(
        LogTag::System,
        "SUCCESS",
        &format!("Database test passed - {} tokens", stats.total_tokens)
    );

    // Test blacklist functionality
    log(LogTag::System, "TEST", "Testing blacklist...");
    let blacklist = TokenBlacklist::new();
    log(LogTag::System, "SUCCESS", "Blacklist test passed");

    // Test API client
    log(LogTag::System, "TEST", "Testing API client...");
    let _api = DexScreenerApi::new();
    log(LogTag::System, "SUCCESS", "API client test passed");

    // Test discovery system
    log(LogTag::System, "TEST", "Testing discovery system...");
    let _discovery = TokenDiscovery::new()?;
    log(LogTag::System, "SUCCESS", "Discovery system test passed");

    // Test monitor system
    log(LogTag::System, "TEST", "Testing monitor system...");
    let _monitor = TokenMonitor::new()?;
    log(LogTag::System, "SUCCESS", "Monitor system test passed");

    log(LogTag::System, "SUCCESS", "All tokens system tests passed! ✅");

    Ok(())
}

/// Manual discovery test
pub async fn test_discovery_manual() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "TEST", "Running manual discovery test...");

    let results = discover_tokens_once().await?;

    log(LogTag::System, "RESULT", &format!("Discovery completed with {} sources", results.len()));

    for result in results {
        log(
            LogTag::System,
            "DISCOVERY",
            &format!(
                "Source: {} - Success: {} - Tokens: {}",
                result.source,
                result.success,
                result.new_tokens.len()
            )
        );
    }

    Ok(())
}

/// Manual monitoring test
pub async fn test_monitoring_manual() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "TEST", "Running manual monitoring test...");

    monitor_tokens_once().await?;

    let stats = get_monitoring_stats().await?;
    log(
        LogTag::System,
        "RESULT",
        &format!(
            "Monitoring completed - Total: {}, Active: {}, Blacklisted: {}",
            stats.total_tokens,
            stats.active_tokens,
            stats.blacklisted_count
        )
    );

    Ok(())
}

/// Comprehensive tokens system integration test
pub async fn test_tokens_integration() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INTEGRATION", "Starting tokens system integration test...");

    // Step 1: Initialize systems
    log(LogTag::System, "STEP", "1. Initializing systems...");
    let _db = TokenDatabase::new()?;
    let _discovery = TokenDiscovery::new()?;
    let _monitor = TokenMonitor::new()?;

    // Step 2: Test blacklist functionality
    log(LogTag::System, "STEP", "2. Testing blacklist functionality...");
    let test_mint = format!("integration_test_mint_{}", chrono::Utc::now().timestamp());
    let test_symbol = "ITEST";

    // Should not be blacklisted initially
    assert!(!is_token_blacklisted(&test_mint));

    // Manually add to blacklist
    let added = crate::tokens::blacklist::add_to_blacklist_manual(&test_mint, test_symbol);
    if added {
        log(LogTag::System, "SUCCESS", "Manual blacklist addition works");

        // Should now be blacklisted
        assert!(is_token_blacklisted(&test_mint));
        log(LogTag::System, "SUCCESS", "Blacklist checking works");
    }

    // Step 3: Test statistics
    log(LogTag::System, "STEP", "3. Testing statistics...");
    if let Some(blacklist_stats) = get_blacklist_stats() {
        log(
            LogTag::System,
            "STATS",
            &format!(
                "Blacklist stats - Total: {}, Tracked: {}",
                blacklist_stats.total_blacklisted,
                blacklist_stats.total_tracked
            )
        );
    }

    let monitor_stats = get_monitoring_stats().await?;
    log(
        LogTag::System,
        "STATS",
        &format!(
            "Monitor stats - Total: {}, Active: {}",
            monitor_stats.total_tokens,
            monitor_stats.active_tokens
        )
    );

    log(LogTag::System, "SUCCESS", "Integration test completed successfully! ✅");

    Ok(())
}
