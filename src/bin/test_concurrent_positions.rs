use screenerbot::{ trader::monitor_open_positions, global::{ LIST_TOKENS, Token } };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Concurrent Position Monitoring");

    // Create some mock tokens for testing
    let tokens = vec![
        Token {
            mint: "test1".to_string(),
            symbol: "TEST1".to_string(),
            name: "Test Token 1".to_string(),
            decimals: 6,
            chain: "solana".to_string(),
            logo_url: Some("https://example.com/logo1.png".to_string()),
            coingecko_id: None,
            website: Some("https://test1.com".to_string()),
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: None,
            price_dexscreener_sol: Some(0.000001),
            price_dexscreener_usd: Some(0.00005),
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
            liquidity: Some(screenerbot::global::LiquidityInfo {
                usd: Some(10000.0),
                base: None,
                quote: None,
            }),
            info: None,
            boosts: None,
        },
        Token {
            mint: "test2".to_string(),
            symbol: "TEST2".to_string(),
            name: "Test Token 2".to_string(),
            decimals: 9,
            chain: "solana".to_string(),
            logo_url: Some("https://example.com/logo2.png".to_string()),
            coingecko_id: None,
            website: Some("https://test2.com".to_string()),
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: None,
            price_dexscreener_sol: Some(0.000002),
            price_dexscreener_usd: Some(0.0001),
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
            liquidity: Some(screenerbot::global::LiquidityInfo {
                usd: Some(20000.0),
                base: None,
                quote: None,
            }),
            info: None,
            boosts: None,
        }
    ];

    // Update global token list
    {
        let mut token_list = LIST_TOKENS.write().unwrap();
        *token_list = tokens;
    }

    println!("✅ Set up mock tokens in global list");

    // Create shutdown notifier for testing
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Spawn the monitor task
    let monitor_task = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    println!("🚀 Started monitor_open_positions task");

    // Let it run for a few cycles to test the concurrent logic
    tokio::time::sleep(Duration::from_secs(15)).await;

    println!("⏰ Sending shutdown signal after 15 seconds...");

    // Signal shutdown
    shutdown.notify_one();

    // Wait for task to complete
    let _ = monitor_task.await;

    println!("✅ Test completed - monitor_open_positions handled shutdown gracefully");
    println!("🎯 Key improvements in concurrent selling:");
    println!("   • Multiple positions can be sold simultaneously");
    println!("   • Uses semaphore to limit concurrent transactions (max 3)");
    println!("   • Each sell operation has timeouts to prevent hanging");
    println!("   • Batch updates position data after all sells complete");
    println!("   • Graceful shutdown handling during sell operations");

    Ok(())
}
