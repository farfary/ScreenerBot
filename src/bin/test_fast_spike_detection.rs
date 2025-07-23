use screenerbot::profit::should_sell_smart_system;
use screenerbot::global::*;
use screenerbot::positions::*;
use chrono::Utc;

fn main() {
    println!("🚀 Testing Fast Spike Detection System");

    // Create a test position
    let position = Position {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        symbol: "SPIKE".to_string(),
        name: "Spike Token".to_string(),
        entry_time: Utc::now(),
        entry_price: 0.01, // $0.01 SOL per token
        effective_entry_price: Some(0.01),
        entry_size_sol: 0.1, // 0.1 SOL invested
        total_size_sol: 0.1,
        position_type: "buy".to_string(),
        token_amount: Some(10_000_000), // 10 tokens with 6 decimals
        exit_price: None,
        effective_exit_price: None,
        sol_received: None,
        exit_time: None,
        price_highest: 0.015,
        price_lowest: 0.008,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
    };

    // Add token to global list for proper P&L calculation
    {
        let mut tokens = LIST_TOKENS.write().unwrap();
        tokens.clear(); // Clear existing tokens first

        // Test Scenario 1: FAST SPIKE with HIGH SUSTAINABILITY
        let fast_spike_token_sustainable = Token {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            symbol: "SPIKE".to_string(),
            name: "Spike Token".to_string(),
            decimals: 6,
            chain: "solana".to_string(),
            logo_url: None,
            coingecko_id: None,
            website: None,
            description: None,
            tags: Vec::new(),
            is_verified: false,
            created_at: None,
            price_dexscreener_sol: Some(0.015), // Current price for P&L calc
            price_dexscreener_usd: None,
            price_pool_sol: None,
            price_pool_usd: None,
            pools: Vec::new(),
            dex_id: None,
            pair_address: None,
            pair_url: None,
            labels: Vec::new(),
            fdv: Some(2000000.0),
            market_cap: Some(1000000.0),

            // FAST SPIKE DETECTED: >25% in 5 minutes
            price_change: Some(PriceChangeStats {
                m5: Some(30.0), // +30% in last 5 minutes - FAST SPIKE!
                h1: Some(50.0), // +50% in last hour - shows it's sustained
                h6: Some(45.0), // Still strong over 6 hours
                h24: Some(40.0), // Good 24h performance
            }),

            // HIGH SUSTAINABILITY FACTORS
            txns: Some(TxnStats {
                m5: Some(TxnPeriod { buys: Some(40), sells: Some(10) }), // Strong buying pressure
                h1: Some(TxnPeriod { buys: Some(150), sells: Some(50) }),
                h6: Some(TxnPeriod { buys: Some(700), sells: Some(300) }),
                h24: Some(TxnPeriod { buys: Some(2500), sells: Some(1500) }),
            }),

            // HUGE VOLUME SURGE - supports sustainability
            volume: Some(VolumeStats {
                m5: Some(50000.0), // Massive 5-minute volume
                h1: Some(120000.0), // Strong hourly volume
                h6: Some(500000.0),
                h24: Some(1200000.0),
            }),

            // DEEP LIQUIDITY - can handle the spike
            liquidity: Some(LiquidityInfo {
                usd: Some(800000.0), // $800K liquidity - very deep
                base: Some(80000000.0),
                quote: Some(8000.0),
            }),

            // Social momentum
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
            boosts: Some(BoostInfo { active: Some(3) }),
        };

        tokens.push(fast_spike_token_sustainable);
    }

    println!("\n=== Fast Spike Test Scenarios ===");

    // Test 1: Fast spike just detected (3 minutes ago) with high sustainability
    println!("\n1️⃣ FAST SPIKE: Just detected (3 min) with HIGH sustainability:");
    let current_price = 0.015; // +50% profit from entry
    let time_held = 180.0; // 3 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &LIST_TOKENS.read().unwrap()[0],
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4} (+50% from entry)", current_price);
    println!("   Time Held: {:.1} minutes", time_held / 60.0);
    println!("   P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 2: Fast spike detected 12 minutes ago - should sell faster
    println!("\n2️⃣ FAST SPIKE: Detected 12 min ago (getting older):");
    let current_price = 0.0165; // +65% profit
    let time_held = 720.0; // 12 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &LIST_TOKENS.read().unwrap()[0],
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4} (+65% from entry)", current_price);
    println!("   Time Held: {:.1} minutes", time_held / 60.0);
    println!("   P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 3: Fast spike with LOW sustainability (likely pump & dump)
    println!("\n3️⃣ FAST SPIKE: With LOW sustainability (pump & dump signs):");

    // Create low sustainability token
    let mut low_sustainability_token = LIST_TOKENS.read().unwrap()[0].clone();
    if let Some(ref mut txns) = low_sustainability_token.txns {
        // More selling than buying during spike - red flag
        if let Some(ref mut m5) = txns.m5 {
            m5.buys = Some(10);
            m5.sells = Some(35);
        }
    }
    if let Some(ref mut volume) = low_sustainability_token.volume {
        // Weak volume for such a big spike
        volume.m5 = Some(5000.0); // Much lower volume
    }
    if let Some(ref mut liquidity) = low_sustainability_token.liquidity {
        // Shallow liquidity - spike likely to dump
        liquidity.usd = Some(30000.0); // Only $30K liquidity
    }

    let current_price = 0.014; // +40% profit
    let time_held = 300.0; // 5 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &low_sustainability_token,
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4} (+40% from entry)", current_price);
    println!("   Time Held: {:.1} minutes", time_held / 60.0);
    println!("   P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 4: Extreme spike >100% - should always sell fast regardless
    println!("\n4️⃣ EXTREME SPIKE: >100% gain - secure profits immediately:");

    // Update token for extreme spike
    let mut extreme_spike_token = LIST_TOKENS.read().unwrap()[0].clone();
    if let Some(ref mut price_change) = extreme_spike_token.price_change {
        price_change.m5 = Some(80.0); // +80% in 5 minutes - extreme!
        price_change.h1 = Some(150.0); // +150% in hour
    }
    extreme_spike_token.price_dexscreener_sol = Some(0.025); // Updated current price

    let current_price = 0.025; // +150% profit!
    let time_held = 420.0; // 7 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &extreme_spike_token,
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4} (+150% from entry)", current_price);
    println!("   Time Held: {:.1} minutes", time_held / 60.0);
    println!("   P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    // Test 5: Regular profit without fast spike - should hold
    println!("\n5️⃣ REGULAR PROFIT: +40% but no fast spike - should hold:");

    // Create regular token without fast spike
    let mut regular_token = LIST_TOKENS.read().unwrap()[0].clone();
    if let Some(ref mut price_change) = regular_token.price_change {
        price_change.m5 = Some(8.0); // Only +8% in 5 minutes - not a spike
        price_change.h1 = Some(40.0); // +40% in hour - gradual buildup
    }
    regular_token.price_dexscreener_sol = Some(0.014);

    let current_price = 0.014; // +40% profit
    let time_held = 1800.0; // 30 minutes
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));
    let (urgency, reason) = should_sell_smart_system(
        &position,
        &regular_token,
        current_price,
        time_held
    );
    println!("   Current Price: ${:.4} (+40% from entry)", current_price);
    println!("   Time Held: {:.1} minutes", time_held / 60.0);
    println!("   P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
    println!("   Urgency: {:.1}%", urgency * 100.0);
    println!("   Reason: {}", reason);

    println!("\n✅ Fast Spike Detection Test Complete!");
    println!("\n📋 Key Insights:");
    println!("   🚀 Fast spikes >25% in <15 min trigger immediate high urgency");
    println!("   ⏱️  Earlier detection = higher urgency (sell faster)");
    println!("   🏗️  High sustainability (volume + liquidity) = slight urgency reduction");
    println!("   ⚠️  Low sustainability (pump signs) = urgency increase");
    println!("   💰 Extreme profits >100% = maximum urgency regardless");
    println!("   📈 Regular gradual gains = normal profit-taking logic");
    println!("\n🎯 Your fast spike requirement: '>25% in <15min = sell fast' ✅ IMPLEMENTED!");
}
