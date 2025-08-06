use screenerbot::logger::{ log, LogTag };
use screenerbot::global::{ read_configs, set_cmd_args };
use screenerbot::tokens::api::init_dexscreener_api;
use screenerbot::rpc::init_rpc_client;
use screenerbot::tokens::price::initialize_price_service;
use screenerbot::tokens::pool::get_pool_service;
use screenerbot::rl_learning::{
    get_rl_entry_score,
    get_simple_entry_score,
    is_rl_entry_recommended,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set command arguments for debug flags
    let args: Vec<String> = env::args().collect();
    set_cmd_args(args.clone());

    log(LogTag::RlLearn, "TOOL_START", "🔬 Starting RL Entry Analysis Tool");

    // Check for token mint argument
    if args.len() < 2 {
        println!("Usage: cargo run --bin tool_rl_entry_analysis -- <TOKEN_MINT>");
        println!(
            "Example: cargo run --bin tool_rl_entry_analysis -- EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
        return Ok(());
    }

    let token_mint = &args[1];

    // Initialize required services
    log(LogTag::RlLearn, "INIT", "Initializing services for RL analysis...");

    let configs = read_configs()?;
    init_dexscreener_api().await?;
    init_rpc_client()?;
    initialize_price_service().await?;

    // Get pool service (it's already initialized)
    let _pool_service = get_pool_service();

    // Mock token data for testing (in real usage, this would come from token discovery)
    let current_price = 0.000123456789; // Example SOL price
    let liquidity_usd = 25000.0;
    let volume_24h = 150000.0;
    let market_cap = Some(500000.0);
    let rugcheck_score = Some(45.0); // Medium risk

    log(
        LogTag::RlLearn,
        "ANALYSIS_START",
        &format!("🎯 Analyzing entry for token: {}", token_mint)
    );
    log(
        LogTag::RlLearn,
        "TEST_DATA",
        &format!(
            "📊 Test data - Price: {:.12} SOL, Liquidity: ${:.0}, Volume: ${:.0}, Risk: {:.0}",
            current_price,
            liquidity_usd,
            volume_24h,
            rugcheck_score.unwrap_or(0.0)
        )
    );

    // Test 1: Get comprehensive entry analysis
    println!("\n=== COMPREHENSIVE RL ENTRY ANALYSIS ===");
    match
        get_rl_entry_score(
            token_mint,
            current_price,
            liquidity_usd,
            volume_24h,
            market_cap,
            rugcheck_score
        ).await
    {
        Ok(analysis) => {
            println!("🎯 Token: {}", token_mint);
            println!(
                "📈 Recommendation: {} {}",
                analysis.recommendation.emoji(),
                analysis.recommendation.to_string()
            );
            println!("🔢 Combined Score: {:.1}%", analysis.combined_score * 100.0);
            println!("🤖 RL Score: {:.1}%", analysis.rl_score * 100.0);
            println!("⏰ Timing Score: {:.1}%", analysis.timing_score * 100.0);
            println!("🛡️ Risk Score: {:.1}%", analysis.risk_score * 100.0);
            println!("🎯 Confidence: {:.1}%", analysis.confidence * 100.0);

            println!("\n--- Price Analysis Details ---");
            println!("💰 Current Price: {:.12} SOL", analysis.price_analysis.current_price);
            println!("📉 Drop from High: {:.1}%", analysis.price_analysis.drop_percentage);
            println!("📊 Range Position: {:.1}%", analysis.price_analysis.range_position * 100.0);
            println!("⚡ Momentum Score: {:.2}", analysis.price_analysis.momentum_score);
            println!("📈 5min Change: {:.2}%", analysis.price_analysis.price_change_5min);
            println!("📈 10min Change: {:.2}%", analysis.price_analysis.price_change_10min);
            println!("📈 30min Change: {:.2}%", analysis.price_analysis.price_change_30min);
            println!("🌊 Volatility: {:.3}", analysis.price_analysis.volatility);
        }
        Err(e) => {
            println!("❌ Failed to get RL analysis: {}", e);
        }
    }

    // Test 2: Simple score
    println!("\n=== SIMPLE ENTRY SCORE ===");
    let simple_score = get_simple_entry_score(
        token_mint,
        current_price,
        liquidity_usd,
        volume_24h,
        market_cap,
        rugcheck_score
    ).await;
    println!("📊 Simple Entry Score: {:.1}%", simple_score * 100.0);

    // Test 3: Recommendation check with different thresholds
    println!("\n=== ENTRY RECOMMENDATIONS ===");
    let thresholds = vec![0.5, 0.6, 0.7, 0.8];
    for threshold in thresholds {
        let recommended = is_rl_entry_recommended(
            token_mint,
            current_price,
            liquidity_usd,
            volume_24h,
            market_cap,
            rugcheck_score,
            threshold
        ).await;
        println!("🎯 Entry recommended at {:.0}% threshold: {}", threshold * 100.0, if recommended {
            "✅ YES"
        } else {
            "❌ NO"
        });
    }

    // Test 4: Multiple price scenarios
    println!("\n=== PRICE SCENARIO ANALYSIS ===");
    let price_scenarios = vec![
        (current_price * 0.8, "20% Drop"),
        (current_price * 0.9, "10% Drop"),
        (current_price, "Current Price"),
        (current_price * 1.1, "10% Pump"),
        (current_price * 1.2, "20% Pump")
    ];

    for (test_price, scenario) in price_scenarios {
        let score = get_simple_entry_score(
            token_mint,
            test_price,
            liquidity_usd,
            volume_24h,
            market_cap,
            rugcheck_score
        ).await;
        println!("📈 {}: Score {:.1}%", scenario, score * 100.0);
    }

    log(LogTag::RlLearn, "TOOL_COMPLETE", "✅ RL Entry Analysis Tool completed successfully");
    println!("\n🎉 Analysis complete! Check logs for detailed debug information.");

    Ok(())
}
