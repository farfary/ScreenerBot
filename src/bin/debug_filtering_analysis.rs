/// Debug filtering analysis tool
/// Analyzes why tokens are being filtered out and provides detailed breakdown

use std::collections::HashMap;
use screenerbot::{
    filtering::{ filter_token_for_trading, FilterResult, FilterReason },
    tokens::{ initialize_tokens_system, get_all_tokens_by_liquidity, Token },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ScreenerBot Filtering Analysis Debug Tool ===");

    // Initialize tokens system
    println!("Initializing tokens system...");
    let _tokens_system = initialize_tokens_system().await?;

    // Get all tokens from database
    let api_tokens = get_all_tokens_by_liquidity().await?;

    println!("Found {} tokens in database", api_tokens.len());

    if api_tokens.is_empty() {
        println!("No tokens found - running discovery first...");
        // Try to use discovery through the system
        if let Err(e) = screenerbot::tokens::discover_tokens_once().await {
            println!("Discovery failed: {}", e);
            return Ok(());
        }

        // Get tokens again after discovery
        let api_tokens = get_all_tokens_by_liquidity().await?;
        println!("After discovery: {} tokens found", api_tokens.len());

        if api_tokens.is_empty() {
            println!("Still no tokens found after discovery");
            return Ok(());
        }
    }

    // Convert ApiToken to Token for filtering analysis
    let analysis_tokens: Vec<Token> = api_tokens
        .into_iter()
        .map(|api_token| api_token.into())
        .collect();

    println!("\n=== FILTERING ANALYSIS ===");

    // Track filtering reasons
    let mut reason_counts: HashMap<String, usize> = HashMap::new();
    let mut approved_count = 0;
    let mut detailed_failures: Vec<(String, String, FilterReason)> = Vec::new();

    // Analyze each token
    for (i, token) in analysis_tokens.iter().enumerate() {
        println!("\n--- Token {}/{}: {} ({}) ---", i + 1, analysis_tokens.len(), token.symbol, if
            token.mint.len() >= 8
        {
            &token.mint[..8]
        } else {
            &token.mint
        });

        // Show token data for debugging
        println!("  Created: {:?}", token.created_at);
        println!("  Price (DexScreener SOL): {:?}", token.price_dexscreener_sol);
        println!(
            "  Liquidity: {:?}",
            token.liquidity.as_ref().map(|l| l.usd)
        );

        // Test filtering
        match filter_token_for_trading(token) {
            FilterResult::Approved => {
                approved_count += 1;
                println!("  ✅ APPROVED - Token passed all filters");
            }
            FilterResult::Rejected(reason) => {
                let reason_str = format!("{:?}", reason);
                *reason_counts.entry(reason_str.clone()).or_insert(0) += 1;
                detailed_failures.push((token.symbol.clone(), token.mint.clone(), reason.clone()));
                println!("  ❌ REJECTED - Reason: {:?}", reason);
            }
        }
    }

    // Summary statistics
    println!("\n=== FILTERING SUMMARY ===");
    println!("Total tokens analyzed: {}", analysis_tokens.len());
    println!(
        "Approved tokens: {} ({:.1}%)",
        approved_count,
        ((approved_count as f64) / (analysis_tokens.len() as f64)) * 100.0
    );
    println!(
        "Rejected tokens: {} ({:.1}%)",
        analysis_tokens.len() - approved_count,
        (((analysis_tokens.len() - approved_count) as f64) / (analysis_tokens.len() as f64)) * 100.0
    );

    // Breakdown by rejection reason
    println!("\n=== REJECTION REASONS ===");
    for (reason, count) in reason_counts.iter() {
        let percentage = ((*count as f64) / (analysis_tokens.len() as f64)) * 100.0;
        println!("{}: {} tokens ({:.1}%)", reason, count, percentage);
    }

    // Show detailed failures for top rejection reasons
    println!("\n=== DETAILED FAILURE EXAMPLES ===");
    let mut reason_examples: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for (symbol, mint, reason) in detailed_failures {
        let reason_str = format!("{:?}", reason);
        reason_examples.entry(reason_str).or_insert_with(Vec::new).push((symbol, mint));
    }

    for (reason, examples) in reason_examples.iter() {
        println!("\n{} examples:", reason);
        for (i, (symbol, mint)) in examples.iter().take(3).enumerate() {
            println!("  {}. {} ({}...)", i + 1, symbol, if mint.len() >= 8 {
                &mint[..8]
            } else {
                mint
            });
        }
        if examples.len() > 3 {
            println!("  ... and {} more", examples.len() - 3);
        }
    }

    // Check specific constraints
    println!("\n=== CONSTRAINT ANALYSIS ===");

    // Age analysis
    let mut age_stats = Vec::new();
    for token in &analysis_tokens {
        if let Some(created_at) = token.created_at {
            let age_hours = (chrono::Utc::now() - created_at).num_hours();
            age_stats.push(age_hours);
        }
    }

    if !age_stats.is_empty() {
        age_stats.sort();
        let min_age = age_stats[0];
        let max_age = age_stats[age_stats.len() - 1];
        let median_age = age_stats[age_stats.len() / 2];

        println!("Token ages - Min: {}h, Max: {}h, Median: {}h", min_age, max_age, median_age);
        println!("Minimum required age: {}h", screenerbot::filtering::MIN_TOKEN_AGE_HOURS);

        let too_young = age_stats
            .iter()
            .filter(|&&age| age < screenerbot::filtering::MIN_TOKEN_AGE_HOURS)
            .count();
        println!(
            "Tokens too young: {} ({:.1}%)",
            too_young,
            ((too_young as f64) / (age_stats.len() as f64)) * 100.0
        );
    }

    // Liquidity analysis
    let mut liquidity_stats = Vec::new();
    for token in &analysis_tokens {
        if let Some(liquidity) = &token.liquidity {
            if let Some(usd) = liquidity.usd {
                liquidity_stats.push(usd);
            }
        }
    }

    if !liquidity_stats.is_empty() {
        liquidity_stats.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min_liq = liquidity_stats[0];
        let max_liq = liquidity_stats[liquidity_stats.len() - 1];
        let median_liq = liquidity_stats[liquidity_stats.len() / 2];

        println!(
            "Liquidity (USD) - Min: ${:.2}, Max: ${:.2}, Median: ${:.2}",
            min_liq,
            max_liq,
            median_liq
        );

        let zero_liquidity = liquidity_stats
            .iter()
            .filter(|&&liq| liq <= 0.0)
            .count();
        println!(
            "Tokens with zero/negative liquidity: {} ({:.1}%)",
            zero_liquidity,
            ((zero_liquidity as f64) / (liquidity_stats.len() as f64)) * 100.0
        );
    }

    // Price analysis
    let mut price_stats = Vec::new();
    for token in &analysis_tokens {
        if let Some(price) = token.price_dexscreener_sol {
            if price > 0.0 {
                price_stats.push(price);
            }
        }
    }

    println!(
        "Tokens with valid DexScreener SOL prices: {} ({:.1}%)",
        price_stats.len(),
        ((price_stats.len() as f64) / (analysis_tokens.len() as f64)) * 100.0
    );

    println!("\n=== ANALYSIS COMPLETE ===");

    Ok(())
}
