use screenerbot::global::*;
use screenerbot::positions::*;
use screenerbot::profit::*;
use chrono::{ Utc, Duration };

// Simple P&L calculation for testing (without global dependencies)
fn calculate_test_pnl(entry_price: f64, current_price: f64, entry_size_sol: f64) -> (f64, f64) {
    let price_change_percent = ((current_price - entry_price) / entry_price) * 100.0;
    let fees = 2.0 * 0.000003; // Very small fees for testing
    let fee_percent = (fees / entry_size_sol) * 100.0;
    let net_pnl_percent = price_change_percent - fee_percent;
    let net_pnl_sol = (net_pnl_percent / 100.0) * entry_size_sol;
    (net_pnl_sol, net_pnl_percent)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing New Dynamic Profit System ===\n");

    // Create a test position
    let test_position = Position {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.001, // 0.001 SOL
        entry_time: Utc::now() - Duration::seconds(1800), // 30 minutes ago
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.1, // Invested 0.1 SOL (larger amount for more realistic fees)
        total_size_sol: 0.1,
        price_highest: 0.0012, // Token peaked at +20%
        price_lowest: 0.0008, // Token dipped to -20%
        entry_transaction_signature: Some("test_signature".to_string()),
        exit_transaction_signature: None,
        token_amount: Some(1000), // 1000 raw units (will be converted with decimals)
        effective_entry_price: Some(0.001),
        effective_exit_price: None,
        sol_received: None,
    };

    // Create a test token with some volatility data
    let test_token = Token {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 6,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: Vec::new(),
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(2)),
        price_dexscreener_sol: Some(0.0011),
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: Vec::new(),
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: Vec::new(),
        fdv: None,
        market_cap: None,
        // Add some volatility data for testing
        txns: Some(TxnStats {
            m5: Some(TxnPeriod { buys: Some(15), sells: Some(5) }), // Bullish 5m
            h1: Some(TxnPeriod { buys: Some(80), sells: Some(20) }), // Bullish 1h
            h6: Some(TxnPeriod { buys: Some(200), sells: Some(100) }), // Moderately bullish 6h
            h24: None,
        }),
        volume: Some(VolumeStats {
            m5: Some(1000.0),
            h1: Some(8000.0), // Recent volume spike
            h6: Some(30000.0),
            h24: Some(100000.0),
        }),
        price_change: Some(PriceChangeStats {
            m5: Some(5.0), // +5% in last 5 minutes
            h1: Some(-10.0), // -10% in last hour
            h6: Some(15.0), // +15% in last 6 hours
            h24: Some(-5.0), // -5% in last 24 hours
        }),
        liquidity: Some(LiquidityInfo {
            usd: Some(50000.0),
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    println!("📊 Test Position Details:");
    println!("  • Symbol: {}", test_position.symbol);
    println!("  • Entry Price: {:.6} SOL", test_position.entry_price);
    println!(
        "  • Entry Time: {} minutes ago",
        (Utc::now() - test_position.entry_time).num_minutes()
    );
    println!(
        "  • Price Range: {:.6} - {:.6} SOL",
        test_position.price_lowest,
        test_position.price_highest
    );
    println!();

    // Test different current prices and scenarios
    let test_scenarios = vec![
        (0.0015, "🚀 Moon scenario (+50%)"),
        (0.0012, "📈 Good profit (+20%)"),
        (0.0011, "💰 Small profit (+10%)"),
        (0.001, "➡️ Break even (0%)"),
        (0.0009, "📉 Small loss (-10%)"),
        (0.0008, "🔴 Significant loss (-20%)"),
        (0.0005, "💀 Major loss (-50%)")
    ];

    println!("🧠 Dynamic Profit Analysis Results:\n");

    for (current_price, scenario) in test_scenarios {
        println!("{}", scenario);
        println!("  Current Price: {:.6} SOL", current_price);

        // Calculate P&L using simple test function
        let (pnl_sol, pnl_percent) = calculate_test_pnl(
            test_position.entry_price,
            current_price,
            test_position.entry_size_sol
        );
        println!("  P&L: {:.6} SOL ({:.1}%)", pnl_sol, pnl_percent);

        // Analyze price decline
        let decline = analyze_price_decline(&test_position, current_price);
        println!("  Decline from entry: {:.1}%", decline.decline_from_entry_percent);
        println!("  Decline from peak: {:.1}%", decline.decline_from_peak_percent);

        // Analyze token volatility
        let volatility = analyze_token_volatility(&test_token);
        println!("  Recovery probability: {:.1}%", volatility.recovery_probability * 100.0);
        println!("  Momentum score: {:.1}%", volatility.momentum_score * 100.0);

        // Calculate dynamic profit target
        let time_held = 1800.0; // 30 minutes
        let profit_target = calculate_dynamic_profit_target(
            &test_position,
            &test_token,
            current_price,
            time_held
        );
        println!("  Dynamic profit target: {:.1}%", profit_target.final_target_percent);
        println!("  Time decay factor: {:.3}", profit_target.time_decay_multiplier);

        // Get sell decision
        let (urgency, reason) = should_sell_dynamic(
            &test_position,
            &test_token,
            current_price,
            time_held
        );
        println!("  📊 Sell urgency: {:.1}% - {}", urgency * 100.0, reason);

        println!();
    }

    println!("🕐 Time Decay Analysis (for break-even price):");
    let test_price = 0.001; // Break-even price
    let time_intervals = vec![
        (300.0, "5 minutes"),
        (900.0, "15 minutes"),
        (1800.0, "30 minutes"),
        (3600.0, "1 hour"),
        (7200.0, "2 hours"),
        (10800.0, "3 hours")
    ];

    for (time_held, label) in time_intervals {
        let profit_target = calculate_dynamic_profit_target(
            &test_position,
            &test_token,
            test_price,
            time_held
        );
        let (urgency, reason) = should_sell_dynamic(
            &test_position,
            &test_token,
            test_price,
            time_held
        );

        println!(
            "  {} - Target: {:.1}% | Urgency: {:.1}% | {}",
            label,
            profit_target.final_target_percent,
            urgency * 100.0,
            reason
        );
    }

    println!("\n✅ Dynamic Profit System Test Complete!");
    println!("\n📋 Key Features Demonstrated:");
    println!("  🎯 Dynamic profit targets that decay over time");
    println!("  📊 Volatility analysis using token transaction data");
    println!("  🔄 Recovery probability based on price movements");
    println!("  ⏰ Time-based urgency calculations");
    println!("  🧠 Smart sell decisions combining multiple factors");

    Ok(())
}
