use screenerbot::marketdata::{ MarketDatabase, MarketData };
use screenerbot::discovery::DiscoveryDatabase;
use anyhow::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Market Data Module...");

    // Create discovery database and add some test tokens
    let discovery_db = Arc::new(DiscoveryDatabase::new()?);
    discovery_db.save_token("So11111111111111111111111111111111111111112")?; // SOL
    discovery_db.save_token("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?; // USDC

    println!("Created discovery database with test tokens");

    // Create market data module
    let market_data = Arc::new(MarketData::new(discovery_db)?);

    println!("Starting market data module...");
    market_data.start().await?;

    // Wait for a few update cycles
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Check stats
    let stats = market_data.get_stats().await;
    println!("Market Data Stats:");
    println!("  Total tokens tracked: {}", stats.total_tokens_tracked);
    println!("  Active tokens: {}", stats.active_tokens);
    println!("  Total pools: {}", stats.total_pools);
    println!("  Update rate per hour: {:.2}", stats.update_rate_per_hour);

    // Check if we got any token data
    let all_tokens = market_data.get_all_tokens().await?;
    println!("\nTokens with market data: {}", all_tokens.len());

    for token in all_tokens.iter().take(2) {
        println!("Token: {} ({})", token.symbol, token.name);
        println!("  Price: ${:.6}", token.price_usd);
        println!("  Volume 24h: ${:.2}", token.volume_24h);
        println!("  Market Cap: ${:.2}", token.market_cap);
        println!("  Liquidity: ${:.2}", token.liquidity_usd);

        // Check pools for this token
        let pools = market_data.get_token_pools(&token.mint).await?;
        println!("  Pools: {}", pools.len());
        for pool in pools.iter().take(2) {
            println!("    Pool: {} (${:.2} liquidity)", pool.pool_address, pool.liquidity_usd);
        }
        println!();
    }

    // Stop the module
    market_data.stop().await;
    println!("Market data module stopped");

    println!("Market Data Module test completed successfully!");

    Ok(())
}
