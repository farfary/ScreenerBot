use screenerbot::profit::should_sell_smart_system;
use screenerbot::positions::calculate_position_pnl;
use screenerbot::global::*;
use screenerbot::positions::*;
use chrono::Utc;

fn main() {
    println!("🧠 Debugging P&L Calculation");

    // Create a test position
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
        token_amount: Some(10000000), // 10 tokens with 6 decimals = 10000000 raw units
        exit_price: None,
        effective_exit_price: None,
        sol_received: None,
        exit_time: None,
        price_highest: 0.012,
        price_lowest: 0.008,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
    };

    // Test P&L calculation with different prices
    let test_prices = [
        (0.011, "Profit +10%"),
        (0.015, "Profit +50%"),
        (0.0085, "Loss -15%"),
        (0.0065, "Loss -35%"),
        (0.0035, "Loss -65%"),
    ];

    println!("\n=== P&L Calculation Debug ===");
    for (price, description) in test_prices.iter() {
        let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, Some(*price));
        println!("Price: ${:.4} ({})", price, description);
        println!(
            "  Expected P&L: {:.1}%",
            ((price - position.entry_price) / position.entry_price) * 100.0
        );
        println!("  Calculated P&L: {:.4} SOL ({:.1}%)", pnl_sol, pnl_percent);
        println!();
    }

    println!("=== Position Details ===");
    println!("Entry price: ${:.4}", position.entry_price);
    println!("Entry size: {:.4} SOL", position.entry_size_sol);
    println!("Token amount (raw): {}", position.token_amount.unwrap_or(0));
    println!("Token amount (UI): {:.2}", (position.token_amount.unwrap_or(0) as f64) / 1_000_000.0);
    println!(
        "Effective entry: ${:.4}",
        position.effective_entry_price.unwrap_or(position.entry_price)
    );
}
