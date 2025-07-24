use screenerbot::global::*;
use screenerbot::profit::*;
use screenerbot::positions::*;
use chrono::Utc;

#[tokio::main]
async fn main() {
    println!("🔍 Testing Debug Profit System");
    println!("===============================================");

    // Simulate command line args for debugging
    {
        let mut args = CMD_ARGS.lock().unwrap();
        args.clear();
        args.push("test_debug_profit".to_string());
        args.push("--debug-profit".to_string());
    }

    // Verify debug mode is enabled
    println!("Debug profit enabled: {}", is_debug_profit_enabled());

    // Create a test token
    let token = Token {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 6,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: Some(0.0001),
        price_dexscreener_usd: Some(0.01),
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: Some(PriceChangeStats {
            m5: Some(3.5), // 3.5% in 5 minutes (will trigger fading momentum)
            h1: Some(15.0), // 15% in 1 hour (strong momentum)
            h6: Some(25.0),
            h24: Some(50.0),
        }),
        liquidity: None,
        info: None,
        boosts: None,
    };

    // Create test position
    let mut position = Position {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        entry_price: 0.0001,
        entry_time: Utc::now() - chrono::Duration::minutes(10), // 10 minutes old
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.1,
        total_size_sol: 0.1,
        price_highest: 0.0001,
        price_lowest: 0.0001,
        entry_transaction_signature: Some("test-sig".to_string()),
        exit_transaction_signature: None,
        token_amount: Some(1000000),
        effective_entry_price: Some(0.0001),
        effective_exit_price: None,
        sol_received: None,
    };

    println!("\n🧪 TEST SCENARIO 1: 10% profit in 10 minutes (should hold)");
    println!("=======================================================");

    let current_price = 0.00011; // 10% profit
    let mut price_tracker = PriceTracker::new(position.effective_entry_price.unwrap());
    price_tracker.update(current_price);

    let time_held_seconds = 10.0 * 60.0; // 10 minutes
    let (urgency, reason) = should_sell_next_gen(
        &position,
        &token,
        current_price,
        time_held_seconds,
        &price_tracker
    );

    println!("Result: Urgency {:.2}, Reason: {}", urgency, reason);

    println!("\n🧪 TEST SCENARIO 2: 100% profit in 10 minutes (should sell - fast target hit)");
    println!("=======================================================================");

    let current_price = 0.0002; // 100% profit
    price_tracker.update(current_price);

    let (urgency, reason) = should_sell_next_gen(
        &position,
        &token,
        current_price,
        time_held_seconds,
        &price_tracker
    );

    println!("Result: Urgency {:.2}, Reason: {}", urgency, reason);

    println!("\n🧪 TEST SCENARIO 3: 50% profit but dipping 15% from peak");
    println!("=======================================================");

    let current_price = 0.00015; // 50% profit
    let mut price_tracker = PriceTracker::new(position.effective_entry_price.unwrap());

    // Simulate peak at 60% profit, then dip to 50%
    price_tracker.update(0.00016); // Peak at 60%
    price_tracker.update(current_price); // Dip to 50%

    let (urgency, reason) = should_sell_next_gen(
        &position,
        &token,
        current_price,
        time_held_seconds,
        &price_tracker
    );

    println!("Result: Urgency {:.2}, Reason: {}", urgency, reason);

    println!("\n🧪 TEST SCENARIO 4: Loss scenario (should hold - zero loss protection)");
    println!("====================================================================");

    let current_price = 0.00008; // -20% loss
    let mut price_tracker = PriceTracker::new(position.effective_entry_price.unwrap());
    price_tracker.update(current_price);

    let (urgency, reason) = should_sell_next_gen(
        &position,
        &token,
        current_price,
        time_held_seconds,
        &price_tracker
    );

    println!("Result: Urgency {:.2}, Reason: {}", urgency, reason);

    println!("\n🧪 TEST SCENARIO 5: Force sell after 61 minutes with 6% profit");
    println!("==============================================================");

    position.entry_time = Utc::now() - chrono::Duration::minutes(61); // 61 minutes old
    let current_price = 0.000106; // 6% profit
    let mut price_tracker = PriceTracker::new(position.effective_entry_price.unwrap());
    price_tracker.update(current_price);

    let time_held_seconds = 61.0 * 60.0; // 61 minutes
    let (urgency, reason) = should_sell_next_gen(
        &position,
        &token,
        current_price,
        time_held_seconds,
        &price_tracker
    );

    println!("Result: Urgency {:.2}, Reason: {}", urgency, reason);

    println!("\n✅ Debug profit testing completed!");
    println!("Run any binary with --debug-profit to see detailed profit decision logs");
}
