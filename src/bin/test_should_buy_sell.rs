use screenerbot::trader::{ should_buy, should_sell };
use screenerbot::global::*;
use screenerbot::positions::Position;
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing should_buy and should_sell functions");

    // Test Token
    let test_token = Token {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        symbol: "USDC".to_string(),
        name: "USD Coin".to_string(),
        decimals: 6,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: true,
        created_at: Some(Utc::now() - Duration::hours(25)), // 25 hours ago
        price_dexscreener_sol: Some(0.000025),
        price_dexscreener_usd: Some(1.0),
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: Some(1000000000.0),
        market_cap: Some(1000000000.0),
        txns: None,
        volume: None,
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(150000.0), // $150k liquidity
            base: Some(0.0),
            quote: Some(0.0),
        }),
        info: None,
        boosts: None,
    };

    // Test should_buy with various scenarios
    println!("\n📊 Testing should_buy function:");

    // Scenario 1: No price drop (should return 0.0)
    let current_price = 0.000025;
    let prev_price = 0.000024; // Price went up
    let buy_urgency = should_buy(&test_token, current_price, prev_price);
    println!(
        "Price increase: {:.6} -> {:.6}, buy_urgency: {:.2}",
        prev_price,
        current_price,
        buy_urgency
    );

    // Scenario 2: Small price drop (below threshold, should return 0.0)
    let current_price = 0.000024;
    let prev_price = 0.000025; // 4% drop
    let buy_urgency = should_buy(&test_token, current_price, prev_price);
    println!(
        "Small drop (4%): {:.6} -> {:.6}, buy_urgency: {:.2}",
        prev_price,
        current_price,
        buy_urgency
    );

    // Scenario 3: Exactly 5% drop (should trigger buy signal)
    let current_price = 0.0000238; // exactly 5% drop
    let prev_price = 0.000025;
    let buy_urgency = should_buy(&test_token, current_price, prev_price);
    println!("5% drop: {:.6} -> {:.6}, buy_urgency: {:.2}", prev_price, current_price, buy_urgency);

    // Scenario 4: Large drop (should trigger higher buy urgency)
    let current_price = 0.00002; // 20% drop
    let prev_price = 0.000025;
    let buy_urgency = should_buy(&test_token, current_price, prev_price);
    println!(
        "20% drop: {:.6} -> {:.6}, buy_urgency: {:.2}",
        prev_price,
        current_price,
        buy_urgency
    );

    // Test should_sell function
    println!("\n📈 Testing should_sell function:");

    let now = Utc::now();

    // Create test position
    let mut test_position = Position {
        mint: test_token.mint.clone(),
        symbol: test_token.symbol.clone(),
        name: test_token.name.clone(),
        entry_price: 0.000025,
        entry_time: now - Duration::minutes(5), // 5 minutes ago
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.025,
        total_size_sol: 0.025,
        price_highest: 0.000025,
        price_lowest: 0.000025,
        entry_transaction_signature: Some("test_sig".to_string()),
        exit_transaction_signature: None,
        token_amount: Some(1000000), // Raw token amount (needs decimals conversion)
        effective_entry_price: Some(0.000025),
        effective_exit_price: None,
        sol_received: None,
    };

    // Scenario 1: Position too young (should return 0.0)
    let current_price = 0.00002; // 20% loss
    let sell_urgency = should_sell(&test_position, current_price, now);
    println!(
        "Position age: {} seconds, 20% loss, sell_urgency: {:.2}",
        (now - test_position.entry_time).num_seconds(),
        sell_urgency
    );

    // Scenario 2: Position old enough with profit
    test_position.entry_time = now - Duration::minutes(3); // 3 minutes ago (above MIN_HOLD_TIME_SECS)
    let current_price = 0.00003; // 20% profit
    let sell_urgency = should_sell(&test_position, current_price, now);
    println!(
        "Position age: {} seconds, 20% profit, sell_urgency: {:.2}",
        (now - test_position.entry_time).num_seconds(),
        sell_urgency
    );

    // Scenario 3: Position with small loss
    let current_price = 0.000024; // 4% loss
    let sell_urgency = should_sell(&test_position, current_price, now);
    println!(
        "Position age: {} seconds, 4% loss, sell_urgency: {:.2}",
        (now - test_position.entry_time).num_seconds(),
        sell_urgency
    );

    // Scenario 4: Old position with time decay
    test_position.entry_time = now - Duration::minutes(35); // 35 minutes ago (time decay active)
    let current_price = 0.000025; // Break even
    let sell_urgency = should_sell(&test_position, current_price, now);
    println!(
        "Position age: {} seconds, break even, sell_urgency: {:.2}",
        (now - test_position.entry_time).num_seconds(),
        sell_urgency
    );

    println!("\n✅ Function tests completed!");

    Ok(())
}
