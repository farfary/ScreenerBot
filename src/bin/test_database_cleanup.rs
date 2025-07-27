/// Test binary for database cleanup functionality
use screenerbot::tokens::cache::TokenDatabase;
use screenerbot::tokens::types::*;
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "TEST", "Starting database cleanup test");

    // Create database instance
    let db = TokenDatabase::new()?;

    // Get some statistics before cleanup
    let stats_before = db.get_stats()?;
    println!("Database stats before cleanup:");
    println!("  Total tokens: {}", stats_before.total_tokens);
    println!("  Tokens with liquidity (>100): {}", stats_before.tokens_with_liquidity);

    // Create a test token with zero liquidity
    let test_token = ApiToken {
        mint: "TEST_ZERO_LIQUIDITY_TOKEN".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Zero Liquidity Token".to_string(),
        decimals: 9,
        chain_id: "solana".to_string(),
        dex_id: "raydium".to_string(),
        pair_address: "test_pair".to_string(),
        pair_url: Some("https://test.com".to_string()),
        price_native: 0.0,
        price_usd: 0.0,
        price_sol: Some(0.0),
        liquidity: Some(LiquidityInfo {
            usd: Some(0.0), // Zero liquidity
            base: Some(0.0),
            quote: Some(0.0),
        }),
        volume: None,
        txns: None,
        price_change: None,
        fdv: None,
        market_cap: None,
        pair_created_at: Some(chrono::Utc::now().timestamp()),
        boosts: None,
        info: None,
        labels: None,
        last_updated: chrono::Utc::now(),
    };

    // Add the test token to database
    log(LogTag::System, "TEST", "Adding test token with zero liquidity");
    let tokens_to_update = vec![test_token];
    db.update_tokens(&tokens_to_update).await?;

    // Get stats after adding test token
    let stats_after_add = db.get_stats()?;
    println!("\nDatabase stats after adding test token:");
    println!("  Total tokens: {}", stats_after_add.total_tokens);
    println!("  Tokens with liquidity (>100): {}", stats_after_add.tokens_with_liquidity);

    // Run cleanup
    log(LogTag::System, "TEST", "Running database cleanup");
    let deleted_count = db.cleanup_zero_liquidity_tokens().await?;
    println!("\nCleanup results:");
    println!("  Tokens deleted: {}", deleted_count);

    // Get final stats
    let stats_final = db.get_stats()?;
    println!("\nDatabase stats after cleanup:");
    println!("  Total tokens: {}", stats_final.total_tokens);
    println!("  Tokens with liquidity (>100): {}", stats_final.tokens_with_liquidity);

    log(LogTag::System, "TEST", "Database cleanup test completed");

    Ok(())
}
