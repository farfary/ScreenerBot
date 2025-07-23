use screenerbot::profit::should_sell_smart_system;
use screenerbot::global::*;
use screenerbot::positions::*;
use chrono::Utc;

fn main() {
    println!("🧠 Testing Smart Profit System");

    // Create a test token with rich data
    let token = Token {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 6, // IMPORTANT: 6 decimals to match our test position
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: Vec::new(),
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: Some(0.011),
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: Vec::new(),
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: Vec::new(),
        fdv: Some(1000000.0),
        market_cap: Some(500000.0),

        // Rich transaction data
        txns: Some(TxnStats {
            m5: Some(TxnPeriod { buys: Some(25), sells: Some(10) }), // Recent buying pressure
            h1: Some(TxnPeriod { buys: Some(120), sells: Some(80) }),
            h6: Some(TxnPeriod { buys: Some(600), sells: Some(400) }),
            h24: Some(TxnPeriod { buys: Some(2000), sells: Some(1800) }),
        }),

        // Volume data
        volume: Some(VolumeStats {
            m5: Some(10000.0), // Recent volume surge
            h1: Some(50000.0),
            h6: Some(200000.0),
            h24: Some(600000.0),
        }),

        // Price change data showing momentum
        price_change: Some(PriceChangeStats {
            m5: Some(5.0), // +5% in last 5 minutes
            h1: Some(10.0), // +10% in last hour
            h6: Some(8.0), // +8% in last 6 hours
            h24: Some(15.0), // +15% in 24 hours
        }),

        // Good liquidity
        liquidity: Some(LiquidityInfo {
            usd: Some(150000.0), // $150K liquidity
            base: Some(15000000.0),
            quote: Some(1500.0),
        }),

        // Social activity
        info: Some(TokenInfo {
            image_url: None,
            header: None,
            open_graph: None,
            websites: vec![WebsiteLink {
                label: Some("Official".to_string()),
                url: "https://example.com".to_string(),
            }],
            socials: vec![SocialLink {
                link_type: "twitter".to_string(),
                url: "https://twitter.com/example".to_string(),
            }],
        }),

        // Active boosts
        boosts: Some(BoostInfo { active: Some(2) }),
    };

    // Add token to global list for proper P&L calculation
    {
        let mut tokens = LIST_TOKENS.write().unwrap();
        tokens.push(token.clone());
    }

    // Create a test position - with entry amount that makes sense
    // If we bought 10 UI tokens at $0.01 each, we spent 0.1 SOL
    // So token_amount should be 10 * 10^6 = 10,000,000 raw units
    let position = Position {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_time: Utc::now(),
        entry_price: 0.01, // $0.01 SOL per token
        effective_entry_price: Some(0.01),
        entry_size_sol: 0.1, // 0.1 SOL invested
        total_size_sol: 0.1,
        position_type: "buy".to_string(),
        token_amount: Some(10_000_000), // 10 tokens with 6 decimals = 10,000,000 raw units
        exit_price: None,
        effective_exit_price: None,
        sol_received: None,
        exit_time: None,
        price_highest: 0.012,
        price_lowest: 0.008,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
    };

    println!("\n=== Test Scenarios ===");

    // Test 1: Strong profit with good momentum - should hold
    println!("\n1️⃣ Strong profit (+10%) with good momentum:");
    let current_price = 0.011; // +10% profit
    let time_held = 1800.0; // 30 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(&position, &token, current_price, time_held);
    println!("   Current Price: ${:.4}", current_price);
    println!(
        "   Expected P&L: +{:.1}%",
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    );
    println!("   Calculated P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 2: High profit but momentum fading
    println!("\n2️⃣ High profit (+50%) but momentum might be fading:");
    let mut fading_token = token.clone();
    if let Some(ref mut price_change) = fading_token.price_change {
        price_change.m5 = Some(-2.0); // Recent decline
        price_change.h1 = Some(8.0); // But still positive overall
    }
    let current_price = 0.015; // +50% profit
    let time_held = 3600.0; // 1 hour
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &fading_token,
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4}", current_price);
    println!(
        "   Expected P&L: +{:.1}%",
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    );
    println!("   Calculated P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 3: Moderate loss (-15%) with good recovery signals
    println!("\n3️⃣ Moderate loss (-15%) with good recovery signals:");
    let current_price = 0.0085; // -15% loss
    let time_held = 2400.0; // 40 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(&position, &token, current_price, time_held);
    println!("   Current Price: ${:.4}", current_price);
    println!(
        "   Expected P&L: {:.1}%",
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    );
    println!("   Calculated P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 4: Deep loss (-35%) with low recovery probability
    println!("\n4️⃣ Deep loss (-35%) with low recovery probability:");
    let mut bad_token = token.clone();
    if let Some(ref mut txns) = bad_token.txns {
        // Heavy selling pressure
        if let Some(ref mut m5) = txns.m5 {
            m5.buys = Some(5);
            m5.sells = Some(30);
        }
    }
    if let Some(ref mut volume) = bad_token.volume {
        volume.m5 = Some(2000.0); // Declining volume
    }
    if let Some(ref mut liquidity) = bad_token.liquidity {
        liquidity.usd = Some(25000.0); // Lower liquidity
    }
    let current_price = 0.0065; // -35% loss
    let time_held = 5400.0; // 1.5 hours
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &bad_token,
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4}", current_price);
    println!(
        "   Expected P&L: {:.1}%",
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    );
    println!("   Calculated P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 5: Catastrophic loss (-65%) - emergency exit
    println!("\n5️⃣ Catastrophic loss (-65%) - emergency exit:");
    let current_price = 0.0035; // -65% loss
    let time_held = 7200.0; // 2 hours
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &bad_token,
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4}", current_price);
    println!(
        "   Expected P&L: {:.1}%",
        ((current_price - position.entry_price) / position.entry_price) * 100.0
    );
    println!("   Calculated P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    println!("\n✅ Smart Profit System Test Complete!");
    println!("\n📋 Summary:");
    println!("   • Profitable positions with strong momentum: Hold for more gains");
    println!("   • Profitable positions with fading momentum: Sell to secure profits");
    println!("   • Moderate losses with recovery signals: Hold and monitor");
    println!("   • Deep losses with poor recovery: Exit to minimize damage");
    println!("   • Catastrophic losses: Emergency exit immediately");
}
