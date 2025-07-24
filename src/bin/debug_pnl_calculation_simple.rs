use screenerbot::global::*;
use screenerbot::positions::*;
use chrono::Utc;

fn main() {
    println!("🔍 Debug PnL Calculation");

    // Add test token to global list so the decimals lookup works correctly
    {
        let mut tokens = LIST_TOKENS.write().unwrap();
        tokens.clear();
        tokens.push(Token {
            mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            decimals: 6, // 6 decimals (important for calculation!)
            chain: "solana".to_string(),
            price_dexscreener_sol: Some(0.01),
            // Set all other fields to None/default
            logo_url: None,
            coingecko_id: None,
            website: None,
            description: None,
            tags: vec![],
            is_verified: false,
            created_at: None,
            price_dexscreener_usd: None,
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
            price_change: None,
            liquidity: None,
            info: None,
            boosts: None,
        });
    }

    // Create a simple position
    let position = Position {
        mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.01, // $0.01 entry price
        effective_entry_price: Some(0.01),
        entry_time: Utc::now() - chrono::Duration::hours(1),
        position_type: "buy".to_string(),
        entry_size_sol: 0.0005, // Entry size in SOL
        total_size_sol: 0.0005, // Total size in SOL
        token_amount: Some(50000), // 50 tokens with 6 decimals = 50000 raw units
        exit_price: None,
        exit_time: None,
        entry_transaction_signature: Some("test_signature".to_string()),
        exit_transaction_signature: None,
        effective_exit_price: None,
        sol_received: None,
        price_highest: 0.012,
        price_lowest: 0.009,
    };

    // Test different prices
    let test_prices = vec![0.011, 0.015, 0.0085];

    for current_price in test_prices {
        let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(current_price));

        println!("\n--- Price: ${:.4} ---", current_price);
        println!("Entry Price: ${:.4}", position.entry_price);
        println!(
            "Effective Entry: ${:.4}",
            position.effective_entry_price.unwrap_or(position.entry_price)
        );
        println!("P&L SOL: {:.6}", pnl_sol);
        println!("P&L %: {:.2}%", pnl_percent);

        // Manual calculation
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
        let manual_pnl = ((current_price - entry_price) / entry_price) * 100.0;
        println!("Manual P&L %: {:.2}%", manual_pnl);

        // Check values
        println!("Token amount (raw): {:?}", position.token_amount);
        println!("Entry size SOL: {:.6}", position.entry_size_sol);

        if pnl_percent <= -99.0 {
            println!("⚠️  -99% stop loss would trigger!");
        } else if pnl_percent < 0.0 {
            println!("📉 Position at loss - would HOLD");
        } else {
            println!("📈 Position in profit");
        }
    }
}
