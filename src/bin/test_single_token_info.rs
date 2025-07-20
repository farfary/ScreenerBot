use screenerbot::discovery::get_single_token_info;
use screenerbot::logger::{log, LogTag};
use std::sync::Arc;
use tokio::sync::Notify;
use colored::Colorize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🧪 Testing Single Token Info Fetcher".bright_blue().bold());
    println!("{}", "=" .repeat(50));

    let shutdown = Arc::new(Notify::new());

    // Test with a few different token mints
    let test_mints = vec![
        // SOL wrapped token (should always work)
        ("So11111111111111111111111111111111111111112", "Wrapped SOL"),
        // USDC (should work)
        ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC"),
        // BONK (popular meme token)
        ("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", "BONK"),
        // Invalid/non-existent mint (should return None)
        ("1111111111111111111111111111111111111111111", "Invalid Token"),
    ];

    for (mint, description) in test_mints {
        println!("\n{} Testing: {} ({})", "🔍".bright_yellow(), description.bright_cyan(), mint.dimmed());
        
        match get_single_token_info(mint, shutdown.clone()).await {
            Ok(Some(token)) => {
                println!("{} {} Found token!", "✅".bright_green(), "SUCCESS:".bright_green().bold());
                println!("   📋 Symbol: {}", token.symbol.bright_white().bold());
                println!("   🏷️  Name: {}", token.name.bright_white());
                println!("   🔢 Decimals: {}", token.decimals.to_string().bright_yellow());
                
                if let Some(price_sol) = token.price_dexscreener_sol {
                    println!("   💰 Price (SOL): {}", format!("{:.8}", price_sol).bright_green());
                }
                
                if let Some(price_usd) = token.price_dexscreener_usd {
                    println!("   💵 Price (USD): ${}", format!("{:.6}", price_usd).bright_green());
                }
                
                if let Some(liquidity) = &token.liquidity {
                    if let Some(usd) = liquidity.usd {
                        println!("   🌊 Liquidity: ${}", format!("{:.2}", usd).bright_blue());
                    }
                }
                
                if let Some(volume) = &token.volume {
                    if let Some(h24) = volume.h24 {
                        println!("   📊 24h Volume: ${}", format!("{:.2}", h24).bright_purple());
                    }
                }
                
                if let Some(market_cap) = token.market_cap {
                    println!("   🏪 Market Cap: ${}", format!("{:.2}", market_cap).bright_magenta());
                }
                
                if let Some(fdv) = token.fdv {
                    println!("   💎 FDV: ${}", format!("{:.2}", fdv).bright_cyan());
                }
                
                if let Some(txns) = &token.txns {
                    if let Some(h24) = &txns.h24 {
                        let buys = h24.buys.unwrap_or(0);
                        let sells = h24.sells.unwrap_or(0);
                        println!("   📈 24h Txns: {} buys, {} sells", 
                            buys.to_string().bright_green(), 
                            sells.to_string().bright_red());
                    }
                }
                
                if !token.labels.is_empty() {
                    println!("   🏷️  Labels: {}", token.labels.join(", ").bright_yellow());
                }
                
                if let Some(info) = &token.info {
                    if !info.websites.is_empty() {
                        println!("   🌐 Website: {}", info.websites[0].url.bright_blue());
                    }
                    if !info.socials.is_empty() {
                        println!("   📱 Socials: {} links", info.socials.len().to_string().bright_purple());
                    }
                }
                
                if let Some(dex_id) = &token.dex_id {
                    println!("   🏦 DEX: {}", dex_id.bright_white());
                }
                
                if let Some(pair_address) = &token.pair_address {
                    println!("   📍 Pair: {}", pair_address.dimmed());
                }
            }
            Ok(None) => {
                println!("{} {} Token not found or no trading pairs available", 
                    "⚠️".bright_yellow(), "NOT FOUND:".bright_yellow().bold());
            }
            Err(e) => {
                println!("{} {} Error fetching token: {}", 
                    "❌".bright_red(), "ERROR:".bright_red().bold(), e);
            }
        }
        
        // Small delay between requests to be API-friendly
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("\n{}", "=" .repeat(50));
    println!("{}", "🎉 Single Token Info Test Complete!".bright_green().bold());
    
    Ok(())
}
