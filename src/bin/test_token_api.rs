use screenerbot::discovery::update_tokens_from_mints;
use screenerbot::global::{ LIST_MINTS, LIST_TOKENS };
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Token API Response Parsing");

    // Add some test mints
    {
        let mut mints = LIST_MINTS.write().unwrap();
        mints.insert("So11111111111111111111111111111111111111112".to_string()); // SOL
        mints.insert("Cdq1WR1d4i2hMrqKUWgZeUbRpkhamGHSvm1f6ATpuray".to_string()); // ALT with full metadata
        mints.insert("726MUA2D5tyUfgWuByU7hzccX3CjixKbBv6NTpDXeBEV".to_string()); // FlopCat
    }

    println!("📋 Added test mints to LIST_MINTS");

    // Create shutdown signal (won't be used)
    let shutdown = Arc::new(Notify::new());

    // Call the API
    println!("🌐 Calling update_tokens_from_mints API...");
    match update_tokens_from_mints(shutdown).await {
        Ok(_) => println!("✅ API call successful"),
        Err(e) => {
            println!("❌ API call failed: {}", e);
            return Err(e);
        }
    }

    // Display the results
    println!("\n📊 Results:");
    if let Ok(tokens) = LIST_TOKENS.read() {
        for (i, token) in tokens.iter().enumerate() {
            println!("\n--- Token {} ---", i + 1);
            println!("Mint: {}", token.mint);
            println!("Symbol: {}", token.symbol);
            println!("Name: {}", token.name);
            println!("Price SOL: {:?}", token.price_dexscreener_sol);
            println!("Price USD: {:?}", token.price_dexscreener_usd);

            // New DexScreener fields
            println!("DEX ID: {:?}", token.dex_id);
            println!("Pair Address: {:?}", token.pair_address);
            println!("Pair URL: {:?}", token.pair_url);
            println!("Labels: {:?}", token.labels);
            println!("FDV: {:?}", token.fdv);
            println!("Market Cap: {:?}", token.market_cap);

            if let Some(liquidity) = &token.liquidity {
                println!("Liquidity USD: {:?}", liquidity.usd);
                println!("Liquidity Base: {:?}", liquidity.base);
                println!("Liquidity Quote: {:?}", liquidity.quote);
            }

            if let Some(volume) = &token.volume {
                println!("Volume 24h: {:?}", volume.h24);
                println!("Volume 1h: {:?}", volume.h1);
            }

            if let Some(txns) = &token.txns {
                if let Some(h24) = &txns.h24 {
                    println!("Txns 24h - Buys: {:?}, Sells: {:?}", h24.buys, h24.sells);
                }
            }

            if let Some(price_change) = &token.price_change {
                println!("Price Change 24h: {:?}%", price_change.h24);
            }

            if let Some(info) = &token.info {
                println!("Image URL: {:?}", info.image_url);
                println!("Websites: {:?}", info.websites);
                println!("Socials: {:?}", info.socials);
            }

            if let Some(boosts) = &token.boosts {
                println!("Active Boosts: {:?}", boosts.active);
            }
        }

        println!("\n🎯 Summary: {} tokens parsed successfully", tokens.len());

        // Test liquidity sorting
        let mut sorted_tokens = tokens.clone();
        sorted_tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        println!("\n📈 Tokens sorted by liquidity (highest first):");
        for (i, token) in sorted_tokens.iter().enumerate() {
            let liquidity_usd = token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            println!(
                "{}. {} ({}) - Liquidity: ${:.2}",
                i + 1,
                token.symbol,
                token.mint,
                liquidity_usd
            );
        }
    } else {
        println!("❌ Failed to read LIST_TOKENS");
    }

    Ok(())
}
