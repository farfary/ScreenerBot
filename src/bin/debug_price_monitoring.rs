/// Debug tool to analyze price monitoring system
/// 
/// This tool helps debug why tokens show identical prices by analyzing:
/// 1. How often tokens are being monitored and updated
/// 2. Which tokens have fresh vs stale prices
/// 3. The monitoring priority system effectiveness

use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::tokens::{ initialize_tokens_system };
use screenerbot::tokens::price_service::{ initialize_price_service, get_priority_tokens_safe };
use screenerbot::trader::get_tokens_from_safe_system;
use screenerbot::global::{ read_configs };
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();
    
    log(LogTag::System, "START", "Price monitoring debug tool started");

    // Read configs first
    let _configs = read_configs("configs.json").map_err(|e| format!("Config error: {}", e))?;

    // Initialize tokens system
    log(LogTag::System, "INIT", "Initializing tokens system...");
    let mut _tokens_system = initialize_tokens_system().await?;
    
    // Initialize price service
    log(LogTag::System, "INIT", "Initializing price service...");
    initialize_price_service().await?;
    
    // Analyze current token state
    log(LogTag::System, "ANALYSIS", "Analyzing current token database state...");
    analyze_token_prices().await?;
    
    // Check monitoring priority system
    log(LogTag::System, "ANALYSIS", "Analyzing monitoring priority system...");
    analyze_monitoring_priorities().await?;
    
    // Monitor for 2 cycles to see updates
    log(LogTag::System, "MONITOR", "Starting 2-cycle monitoring test...");
    monitor_price_updates().await?;
    
    log(LogTag::System, "COMPLETE", "Price monitoring debug analysis complete");
    Ok(())
}

/// Analyze current token prices and freshness
async fn analyze_token_prices() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = get_tokens_from_safe_system().await;
    
    log(LogTag::System, "TOKENS", &format!("Total tokens in database: {}", tokens.len()));
    
    let mut tokens_with_prices = 0;
    let mut tokens_without_prices = 0;
    let mut recent_samples = Vec::new();
    
    for (i, token) in tokens.iter().enumerate() {
        if let Some(price) = token.price_dexscreener_sol {
            tokens_with_prices += 1;
            
            // Collect samples for analysis
            if recent_samples.len() < 10 {
                recent_samples.push((token.symbol.clone(), token.mint.clone(), price));
            }
        } else {
            tokens_without_prices += 1;
        }
    }
    
    log(LogTag::System, "PRICE_ANALYSIS", 
        &format!("Tokens with prices: {}, without prices: {}", 
            tokens_with_prices, tokens_without_prices));
    
    // Show sample tokens
    log(LogTag::System, "SAMPLES", "Sample tokens with prices:");
    for (symbol, mint, price) in recent_samples {
        log(LogTag::System, "SAMPLE", 
            &format!("  {} ({}): {:.10} SOL", symbol, &mint[..8], price));
    }
    
    Ok(())
}

/// Analyze what tokens are being prioritized for monitoring
async fn analyze_monitoring_priorities() -> Result<(), Box<dyn std::error::Error>> {
    let priority_tokens = get_priority_tokens_safe().await;
    
    log(LogTag::System, "PRIORITY", 
        &format!("Priority tokens for monitoring: {}", priority_tokens.len()));
    
    if priority_tokens.is_empty() {
        log(LogTag::System, "PRIORITY_ISSUE", 
            "⚠️  NO PRIORITY TOKENS - This explains why prices aren't updating!");
        log(LogTag::System, "PRIORITY_ISSUE", 
            "The monitoring system has no tokens to monitor, so no price updates occur");
    } else {
        log(LogTag::System, "PRIORITY_TOKENS", "Priority tokens being monitored:");
        for (i, mint) in priority_tokens.iter().take(5).enumerate() {
            log(LogTag::System, "PRIORITY_TOKEN", 
                &format!("  {}: {}", i + 1, &mint[..8]));
        }
    }
    
    Ok(())
}

/// Monitor price updates for 2 cycles
async fn monitor_price_updates() -> Result<(), Box<dyn std::error::Error>> {
    // Take snapshot before
    let tokens_before = get_tokens_from_safe_system().await;
    let sample_tokens: Vec<_> = tokens_before.iter().take(5)
        .map(|t| (t.symbol.clone(), t.mint.clone(), t.price_dexscreener_sol))
        .collect();
    
    log(LogTag::System, "BEFORE_MONITOR", "Token prices before monitoring:");
    for (symbol, mint, price) in &sample_tokens {
        log(LogTag::System, "BEFORE", 
            &format!("  {} ({}): {:?}", symbol, &mint[..8], price));
    }
    
    // Wait for monitoring cycles
    log(LogTag::System, "WAITING", "Waiting 90 seconds for monitoring cycles...");
    sleep(tokio::time::Duration::from_secs(90)).await;
    
    // Take snapshot after
    let tokens_after = get_tokens_from_safe_system().await;
    
    log(LogTag::System, "AFTER_MONITOR", "Token prices after monitoring:");
    for (symbol, mint, _) in &sample_tokens {
        if let Some(token_after) = tokens_after.iter().find(|t| t.mint == *mint) {
            log(LogTag::System, "AFTER", 
                &format!("  {} ({}): {:?}", symbol, &mint[..8], token_after.price_dexscreener_sol));
        }
    }
    
    // Compare changes
    log(LogTag::System, "CHANGES", "Price change analysis:");
    for (symbol, mint, price_before) in &sample_tokens {
        if let Some(token_after) = tokens_after.iter().find(|t| t.mint == *mint) {
            let price_after = token_after.price_dexscreener_sol;
            
            match (price_before, price_after) {
                (Some(before), Some(after)) => {
                    if (before - after).abs() > 0.000000000001 {
                        log(LogTag::System, "PRICE_CHANGED", 
                            &format!("  ✅ {} price changed: {:.10} → {:.10}", 
                                symbol, before, after));
                    } else {
                        log(LogTag::System, "PRICE_UNCHANGED", 
                            &format!("  ❌ {} price unchanged: {:.10}", symbol, before));
                    }
                }
                (None, Some(after)) => {
                    log(LogTag::System, "PRICE_ADDED", 
                        &format!("  ✅ {} got new price: {:.10}", symbol, after));
                }
                (Some(before), None) => {
                    log(LogTag::System, "PRICE_LOST", 
                        &format!("  ❌ {} lost price: {:.10}", symbol, before));
                }
                (None, None) => {
                    log(LogTag::System, "PRICE_STILL_NONE", 
                        &format!("  ❌ {} still has no price", symbol));
                }
            }
        }
    }
    
    Ok(())
}
