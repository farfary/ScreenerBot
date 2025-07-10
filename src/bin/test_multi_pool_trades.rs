// Test script to verify multi-pool trades integration
use screenerbot::prelude::*;
use screenerbot::trades::{ add_tokens_to_monitor, TOKENS_TO_MONITOR };

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing multi-pool trades integration...");

    // Example token (replace with an actual token mint from your system)
    let test_token = Token {
        mint: "So11111111111111111111111111111111111111112".to_string(), // SOL
        balance: "0".to_string(),
        ata_pubkey: "".to_string(),
        program_id: "".to_string(),
        symbol: "SOL".to_string(),
        pair_address: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(),
        name: "Wrapped SOL".to_string(),
        dex_id: "raydium".to_string(),
        url: "".to_string(),
        labels: Vec::new(),
        quote_address: "".to_string(),
        quote_name: "".to_string(),
        quote_symbol: "".to_string(),
        price_native: "1.0".to_string(),
        price_usd: "100.0".to_string(),
        last_price_usd: "100.0".to_string(),
        volume_usd: "1000000.0".to_string(),
        fdv_usd: "50000000000.0".to_string(),
        image_url: "".to_string(),
        txns: crate::Txns {
            m5: crate::TxnCount { buys: 10, sells: 5 },
            h1: crate::TxnCount { buys: 100, sells: 50 },
            h6: crate::TxnCount { buys: 600, sells: 300 },
            h24: crate::TxnCount { buys: 2400, sells: 1200 },
        },
        volume: crate::Volume {
            m5: 10000.0,
            h1: 100000.0,
            h6: 600000.0,
            h24: 2400000.0,
        },
        price_change: crate::PriceChange {
            m5: 1.5,
            h1: 3.2,
            h6: 5.8,
            h24: 10.2,
        },
        liquidity: crate::Liquidity {
            usd: 5000000.0,
            base: 1000000.0,
            quote: 4000000.0,
        },
        pair_created_at: 1640995200,
        rug_check: RugCheckData::default(),
    };

    let tokens = vec![&test_token];

    // Test adding tokens to monitor
    println!("📊 Adding tokens to monitor...");
    add_tokens_to_monitor(&tokens).await;

    // Check the monitoring list
    let monitor_list = TOKENS_TO_MONITOR.read().await;
    if let Some(pools) = monitor_list.get(&test_token.mint) {
        println!("✅ Token {} is now monitored with {} pools:", test_token.symbol, pools.len());
        for (i, pool) in pools.iter().enumerate() {
            println!("  Pool {}: {}", i + 1, pool);
        }
    } else {
        println!("❌ Token not found in monitoring list");
    }

    println!("🧪 Test completed!");
    Ok(())
}
