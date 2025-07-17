use screenerbot::marketdata::GeckoTerminalClient;
use reqwest::Client;
use anyhow::Result;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing GeckoTerminal API...");

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("ScreenerBot/1.0")
        .build()?;

    let gecko_client = GeckoTerminalClient::new(client);

    // Test with a known Solana token (wrapped SOL)
    let test_token = "So11111111111111111111111111111111111111112";

    println!("Fetching data for token: {}", test_token);

    match gecko_client.fetch_token_data(test_token).await {
        Ok(Some((token_data, pools))) => {
            println!("Token Data:");
            println!("  Symbol: {}", token_data.symbol);
            println!("  Name: {}", token_data.name);
            println!("  Price USD: ${:.6}", token_data.price_usd);
            println!("  Price Change 24h: {:.2}%", token_data.price_change_24h);
            println!("  Volume 24h: ${:.2}", token_data.volume_24h);
            println!("  Market Cap: ${:.2}", token_data.market_cap);
            println!("  Liquidity: ${:.2}", token_data.liquidity_usd);

            if let Some(pool_addr) = &token_data.top_pool_address {
                println!("  Top Pool: {}", pool_addr);
            }

            println!("\nPools found: {}", pools.len());
            for (i, pool) in pools.iter().enumerate().take(3) {
                println!("  Pool {}: {}", i + 1, pool.pool_address);
                println!("    Liquidity: ${:.2}", pool.liquidity_usd);
                println!("    Volume 24h: ${:.2}", pool.volume_24h);
            }
        }
        Ok(None) => {
            println!("No data found for token: {}", test_token);
        }
        Err(e) => {
            println!("Error fetching token data: {}", e);
        }
    }

    println!("GeckoTerminal API test completed!");

    Ok(())
}
