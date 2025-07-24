use screenerbot::trader::*;
use screenerbot::global::*;
use screenerbot::positions::*;
use screenerbot::logger::*;
use screenerbot::filtering::POSITION_CLOSE_COOLDOWN_MINUTES;
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Recent Position Close Protection System");
    println!("============================================================");

    // Test token
    let test_mint = "So11111111111111111111111111111111111111112"; // SOL mint for testing
    let test_symbol = "SOL";

    // Create a test token
    let test_token = Token {
        mint: test_mint.to_string(),
        symbol: test_symbol.to_string(),
        name: "Solana".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: Vec::new(),
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(24)),
        price_dexscreener_sol: Some(1.0),
        price_dexscreener_usd: Some(100.0),
        price_pool_sol: None,
        price_pool_usd: None,
        pools: Vec::new(),
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: Vec::new(),
        fdv: None,
        market_cap: None,
        txns: None,
        volume: Some(VolumeStats {
            h24: Some(1000000.0),
            h6: None,
            h1: None,
            m5: None,
        }),
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(500000.0),
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    println!("Test 1: No recent positions (should allow purchase)");

    // Clear any existing positions for clean test
    {
        if let Ok(mut positions) = SAVED_POSITIONS.lock() {
            positions.retain(|p| p.mint != test_mint);
        }
    }

    let result1 = should_buy(&test_token, 0.95, 1.0); // 5% drop
    println!("   Result: {:.3} (should be > 0 if dip detection triggers)", result1);

    println!("Test 2: Recently closed position (should block purchase)");

    // Add a recently closed position (10 minutes ago)
    let recent_close_time = Utc::now() - Duration::minutes(10);
    let test_position = Position {
        mint: test_mint.to_string(),
        symbol: test_symbol.to_string(),
        name: "Solana".to_string(),
        entry_price: 1.0,
        entry_time: Utc::now() - Duration::minutes(30),
        exit_price: Some(1.05),
        exit_time: Some(recent_close_time),
        position_type: "buy".to_string(),
        entry_size_sol: 0.001,
        total_size_sol: 0.001,
        price_highest: 1.05,
        price_lowest: 0.95,
        entry_transaction_signature: Some("test_entry_sig".to_string()),
        exit_transaction_signature: Some("test_exit_sig".to_string()),
        token_amount: Some(1000000),
        effective_entry_price: Some(1.0),
        effective_exit_price: Some(1.05),
        sol_received: Some(0.00105),
    };

    // Add the position to saved positions
    {
        if let Ok(mut positions) = SAVED_POSITIONS.lock() {
            positions.push(test_position);
        }
    }

    let result2 = should_buy(&test_token, 0.95, 1.0); // Same 5% drop
    println!("   Result: {:.3} (should be 0.0 due to recent close)", result2);

    println!("Test 3: Old closed position (should allow purchase)");

    // Clear recent position and add an old one (2 hours ago)
    {
        if let Ok(mut positions) = SAVED_POSITIONS.lock() {
            positions.retain(|p| p.mint != test_mint);

            let old_position = Position {
                mint: test_mint.to_string(),
                symbol: test_symbol.to_string(),
                name: "Solana".to_string(),
                entry_price: 1.0,
                entry_time: Utc::now() - Duration::hours(3),
                exit_price: Some(1.05),
                exit_time: Some(Utc::now() - Duration::hours(2)), // 2 hours ago
                position_type: "buy".to_string(),
                entry_size_sol: 0.001,
                total_size_sol: 0.001,
                price_highest: 1.05,
                price_lowest: 0.95,
                entry_transaction_signature: Some("test_entry_sig_old".to_string()),
                exit_transaction_signature: Some("test_exit_sig_old".to_string()),
                token_amount: Some(1000000),
                effective_entry_price: Some(1.0),
                effective_exit_price: Some(1.05),
                sol_received: Some(0.00105),
            };

            positions.push(old_position);
        }
    }

    let result3 = should_buy(&test_token, 0.95, 1.0); // Same 5% drop
    println!("   Result: {:.3} (should be > 0 if dip detection triggers)", result3);

    println!("Test 4: Open position (no exit_time - should allow purchase)");

    // Clear positions and add an open position (no exit_time)
    {
        if let Ok(mut positions) = SAVED_POSITIONS.lock() {
            positions.retain(|p| p.mint != test_mint);

            let open_position = Position {
                mint: test_mint.to_string(),
                symbol: test_symbol.to_string(),
                name: "Solana".to_string(),
                entry_price: 1.0,
                entry_time: Utc::now() - Duration::minutes(15),
                exit_price: None,
                exit_time: None, // Still open
                position_type: "buy".to_string(),
                entry_size_sol: 0.001,
                total_size_sol: 0.001,
                price_highest: 1.05,
                price_lowest: 0.95,
                entry_transaction_signature: Some("test_entry_sig_open".to_string()),
                exit_transaction_signature: None,
                token_amount: Some(1000000),
                effective_entry_price: Some(1.0),
                effective_exit_price: None,
                sol_received: None,
            };

            positions.push(open_position);
        }
    }

    let result4 = should_buy(&test_token, 0.95, 1.0); // Same 5% drop
    println!("   Result: {:.3} (should be > 0, open positions do not block)", result4);

    println!("Recent Position Close Protection Summary:");
    println!("   - Cooldown period: {} minutes", POSITION_CLOSE_COOLDOWN_MINUTES);
    println!("   - Blocks purchases of tokens with recent closes");
    println!("   - Allows purchases for old closes or open positions");
    println!("   - Helps prevent rapid re-entry into recently sold tokens");

    // Clean up test data
    {
        if let Ok(mut positions) = SAVED_POSITIONS.lock() {
            positions.retain(|p| p.mint != test_mint);
        }
    }

    println!("Recent position close protection test completed!");

    Ok(())
}
