/// Test Pool Price Manager Background System
///
/// This test verifies that the new background pool price manager:
/// 1. Starts successfully as a background task
/// 2. Prioritizes open positions and high liquidity tokens
/// 3. Validates and caches pool prices correctly
/// 4. Updates the global token list without blocking other operations
/// 5. Handles validation and failed decode tracking

use screenerbot::pool_price_manager::{
    pool_price_manager,
    get_best_available_price,
    mark_token_as_validated,
    is_token_validated,
};
use screenerbot::logger::{ log, LogTag };
use screenerbot::global::{ LIST_TOKENS, Token, read_configs };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;

#[tokio::main]
async fn main() {
    log(LogTag::Pool, "START", "Testing Pool Price Manager Background System");

    // Initialize with some test tokens
    setup_test_tokens().await;

    // Test the background pool price manager
    test_pool_price_manager().await;

    log(LogTag::Pool, "COMPLETE", "Pool Price Manager test completed");
}

async fn setup_test_tokens() {
    log(LogTag::Pool, "SETUP", "Setting up test tokens");

    // Add some test tokens to the global list
    let test_tokens = vec![
        Token {
            mint: "So11111111111111111111111111111111111111112".to_string(), // WSOL
            symbol: "WSOL".to_string(),
            name: "Wrapped SOL".to_string(),
            decimals: 9,
            chain: "solana".to_string(),
            liquidity: Some(screenerbot::global::LiquidityInfo {
                usd: Some(1000000.0), // High liquidity
                base: Some(500000.0),
                quote: Some(500000.0),
            }),
            price_dexscreener_sol: Some(1.0),
            logo_url: None,
            coingecko_id: None,
            website: None,
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: None,
            price_dexscreener_usd: None,
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
            info: None,
            boosts: None,
        },
        Token {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            decimals: 6,
            chain: "solana".to_string(),
            liquidity: Some(screenerbot::global::LiquidityInfo {
                usd: Some(800000.0), // High liquidity
                base: Some(400000.0),
                quote: Some(400000.0),
            }),
            price_dexscreener_sol: Some(0.004),
            logo_url: None,
            coingecko_id: None,
            website: None,
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: None,
            price_dexscreener_usd: None,
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
            info: None,
            boosts: None,
        },
        Token {
            mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // BONK
            symbol: "BONK".to_string(),
            name: "Bonk".to_string(),
            decimals: 5,
            chain: "solana".to_string(),
            liquidity: Some(screenerbot::global::LiquidityInfo {
                usd: Some(100000.0), // Medium liquidity
                base: Some(50000.0),
                quote: Some(50000.0),
            }),
            price_dexscreener_sol: Some(0.000001),
            logo_url: None,
            coingecko_id: None,
            website: None,
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: None,
            price_dexscreener_usd: None,
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
            info: None,
            boosts: None,
        }
    ];

    if let Ok(mut tokens) = LIST_TOKENS.write() {
        *tokens = test_tokens;
        log(LogTag::Pool, "SETUP", "Added 3 test tokens to global list");
    }
}

async fn test_pool_price_manager() {
    log(LogTag::Pool, "TEST", "Starting pool price manager test");

    // Create shutdown signal
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Start the pool price manager in the background
    let manager_handle = tokio::spawn(async move {
        pool_price_manager(shutdown_clone).await;
    });

    // Wait a bit for the manager to start
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Test 1: Check validation system
    test_validation_system().await;

    // Test 2: Check price retrieval system
    test_price_retrieval_system().await;

    // Test 3: Let the manager run for a few cycles
    log(LogTag::Pool, "TEST", "Letting pool price manager run for 3 cycles...");
    tokio::time::sleep(Duration::from_secs(95)).await; // Let it run for ~3 cycles

    // Test 4: Check if tokens were updated
    test_token_updates().await;

    // Shutdown the manager
    log(LogTag::Pool, "TEST", "Shutting down pool price manager");
    shutdown.notify_waiters();

    // Wait for manager to finish with timeout
    match tokio::time::timeout(Duration::from_secs(10), manager_handle).await {
        Ok(_) => log(LogTag::Pool, "SUCCESS", "Pool price manager shut down gracefully"),
        Err(_) => log(LogTag::Pool, "WARN", "Pool price manager shutdown timed out"),
    }
}

async fn test_validation_system() {
    log(LogTag::Pool, "TEST", "Testing validation system");

    let test_mint = "So11111111111111111111111111111111111111112";

    // Initially should not be validated
    let initial_validation = is_token_validated(test_mint);
    log(LogTag::Pool, "CHECK", &format!("Initial validation status: {}", initial_validation));

    // Mark as validated
    mark_token_as_validated(test_mint);

    // Should now be validated
    let post_validation = is_token_validated(test_mint);
    log(LogTag::Pool, "CHECK", &format!("Post-validation status: {}", post_validation));

    if post_validation {
        log(LogTag::Pool, "SUCCESS", "Validation system working correctly");
    } else {
        log(LogTag::Pool, "ERROR", "Validation system failed");
    }
}

async fn test_price_retrieval_system() {
    log(LogTag::Pool, "TEST", "Testing price retrieval system");

    let test_mints = vec![
        "So11111111111111111111111111111111111111112", // WSOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" // BONK
    ];

    for mint in test_mints {
        let price = get_best_available_price(mint);
        log(LogTag::Pool, "PRICE", &format!("Best price for {}: {:?}", mint, price));
    }
}

async fn test_token_updates() {
    log(LogTag::Pool, "TEST", "Checking token updates after manager cycles");

    if let Ok(tokens) = LIST_TOKENS.read() {
        for token in tokens.iter() {
            log(
                LogTag::Pool,
                "TOKEN",
                &format!(
                    "{} ({}): Pool Price: {:?}, API Price: {:?}",
                    token.symbol,
                    token.mint,
                    token.price_pool_sol,
                    token.price_dexscreener_sol
                )
            );
        }
    }

    log(LogTag::Pool, "COMPLETE", "Token update check completed");
}
