/// Test script to validate enhanced LP lock functionality
/// This script tests the comprehensive LP lock analysis with various tokens

use screenerbot::tokens::lp_lock::{
    check_lp_lock_status,
    check_lp_lock_status_with_retry,
    get_lp_lock_statistics,
    LpLockAnalysis,
    LpLockStatus,
};
use screenerbot::logger::{ init_file_logging, log, LogTag };

/// Test tokens representing different scenarios
const TEST_TOKENS: &[(&str, &str)] = &[
    // Major established tokens
    ("RAY", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"), // Raydium token
    ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"), // USDC
    ("SOL", "So11111111111111111111111111111111111111112"), // Wrapped SOL

    // Pump.fun tokens for testing
    ("TRUMP", "HeLp6NuQkmYB4pYWo2zYs22mESHXPQYzXbB8n4V98jwC"), // Trump token
    ("PEPE", "2zMMhcVQEXDtdE6vsFS7S7D5oUodfJHE8vd1gnBouauv"), // Pepe token

    // Known problematic tokens for edge case testing
    ("TEST1", "11111111111111111111111111111111"), // Invalid token
];

#[tokio::main]
async fn main() {
    // Initialize logging
    init_file_logging();

    log(LogTag::System, "INFO", "🧪 Starting Enhanced LP Lock Testing");

    // Test 1: Individual token analysis
    println!("\n=== Individual Token Analysis ===");
    for (symbol, mint) in TEST_TOKENS {
        println!("\n🔍 Testing {} ({})", symbol, &mint[..8]);

        match check_lp_lock_status(mint).await {
            Ok(analysis) => {
                println!("✅ Analysis successful:");
                print_analysis_details(&analysis);

                // Test validation
                match analysis.validate() {
                    Ok(_) => println!("✅ Analysis validation passed"),
                    Err(e) => println!("❌ Analysis validation failed: {}", e),
                }

                // Test risk assessment
                println!("🎯 Risk Assessment: {}", analysis.risk_assessment());
                println!("🛡️ Safe for Trading: {}", analysis.is_safe_for_trading());
            }
            Err(e) => {
                println!("❌ Analysis failed: {:?}", e);
            }
        }
    }

    // Test 2: Summary function (create manual summary)
    println!("\n=== Manual Summary Testing ===");
    for (symbol, mint) in TEST_TOKENS.iter().take(3) {
        match check_lp_lock_status(mint).await {
            Ok(analysis) => {
                let summary = format!(
                    "{} - {} (Score: {}/100)",
                    analysis.status.risk_level(),
                    analysis.status.description(),
                    analysis.lock_score
                );
                println!("📊 {} Summary: {}", symbol, summary);
            }
            Err(e) => println!("❌ {} Summary failed: {:?}", symbol, e),
        }
    }

    // Test 3: Retry functionality
    println!("\n=== Retry Functionality Testing ===");
    let ray_mint = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
    match check_lp_lock_status_with_retry(ray_mint, 2).await {
        Ok(analysis) => {
            println!("✅ Retry test successful for RAY");
            println!("📊 Score: {}/100", analysis.lock_score);
        }
        Err(e) => println!("❌ Retry test failed: {:?}", e),
    }

    // Test 4: Batch statistics
    println!("\n=== Batch Statistics Testing ===");
    let token_mints: Vec<String> = TEST_TOKENS.iter()
        .take(5) // Test with first 5 tokens
        .map(|(_, mint)| mint.to_string())
        .collect();

    match get_lp_lock_statistics(&token_mints).await {
        Ok(stats) => {
            println!("✅ Statistics analysis successful:");
            println!("📊 Total Analyzed: {}", stats.total_analyzed);
            println!("🔥 Burned Count: {}", stats.burned_count);
            println!("⏰ Time Locked Count: {}", stats.time_locked_count);
            println!("🔒 Program Locked Count: {}", stats.program_locked_count);
            println!("⚠️ Creator Held Count: {}", stats.creator_held_count);
            println!("❓ Unknown Count: {}", stats.unknown_count);
            println!("❌ No Pool Count: {}", stats.no_pool_count);
            println!("💀 Error Count: {}", stats.error_count);
            println!("🛡️ Secure Count (Score >= 70): {}", stats.secure_count);
            println!("📈 Average Score: {}/100", stats.average_score);
        }
        Err(e) => println!("❌ Statistics analysis failed: {:?}", e),
    }

    // Test 5: Edge cases
    println!("\n=== Edge Case Testing ===");

    // Test with empty string
    match check_lp_lock_status("").await {
        Ok(_) => println!("⚠️ Empty string test: Unexpected success"),
        Err(_) => println!("✅ Empty string test: Correctly failed"),
    }

    // Test with invalid address
    match check_lp_lock_status("invalid_address_123").await {
        Ok(_) => println!("⚠️ Invalid address test: Unexpected success"),
        Err(_) => println!("✅ Invalid address test: Correctly failed"),
    }

    log(LogTag::System, "INFO", "🏁 Enhanced LP Lock Testing Complete");
    println!("\n🏁 Enhanced LP Lock Testing Complete!");
}

fn print_analysis_details(analysis: &LpLockAnalysis) {
    println!("  Token: {}", analysis.token_mint);
    if let Some(pool) = &analysis.pool_address {
        println!("  Pool: {}...", &pool[..8]);
    }
    if let Some(lp_mint) = &analysis.lp_mint {
        println!("  LP Mint: {}...", &lp_mint[..8]);
    }
    println!("  Status: {:?}", analysis.status);
    println!("  Lock Score: {}/100", analysis.lock_score);
    println!("  Analysis Time: {:?}", analysis.analyzed_at);

    // Print governance info if available
    if let Some(gov_info) = &analysis.details.governance_info {
        println!("  Governance Program: {}", gov_info.governance_program);
        if let Some(realm) = &gov_info.governance_realm {
            println!("  Governance Realm: {}...", &realm[..8]);
        }
        if let Some(delay) = gov_info.min_governance_delay {
            println!("  Min Governance Delay: {} seconds", delay);
        }
    }

    // Print status-specific details
    match &analysis.status {
        LpLockStatus::TimeLocked { program, unlock_time } => {
            println!("  Lock Program: {}", program);
            println!("  Unlock Time: {}", unlock_time);
        }
        LpLockStatus::ProgramLocked { program, amount } => {
            println!("  Lock Program: {}", program);
            println!("  Locked Amount: {}", amount);
        }
        _ => {}
    }
}
