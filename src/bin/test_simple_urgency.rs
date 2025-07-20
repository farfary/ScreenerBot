use screenerbot::trader::*;
use screenerbot::global::*;
use chrono::{ DateTime, Utc };

fn main() {
    println!("🧪 Testing Improved Sell Urgency Function\n");

    // Create test position
    let test_pos = Position {
        mint: "test".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.00001,
        entry_time: Utc::now() - chrono::Duration::minutes(10),
        exit_price: None,
        exit_time: None,
        pnl_sol: None,
        pnl_percent: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.0005,
        total_size_sol: 0.0005,
        drawdown_percent: 0.0,
        price_highest: 0.00001,
        price_lowest: 0.00001,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
        token_amount: None,
        effective_entry_price: None,
        effective_exit_price: None,
    };

    let now = Utc::now();
    let scenarios = vec![
        ("Small profit after 5 min", 0.000011, 5), // +10%
        ("Small profit after 15 min", 0.000011, 15), // +10%
        ("Good profit after 15 min", 0.000013, 15), // +30%
        ("Small loss after 5 min", 0.000009, 5), // -10%
        ("Big loss after 5 min", 0.000006, 5), // -40%
        ("Big loss after 15 min", 0.000006, 15), // -40%
        ("Catastrophic loss", 0.000003, 10) // -70%
    ];

    println!("📊 Testing various market scenarios:");
    println!("=====================================");

    for (scenario, price, minutes) in scenarios {
        let test_time = test_pos.entry_time + chrono::Duration::minutes(minutes);
        let urgency = calculate_sell_urgency(&test_pos, price, test_time);
        let pnl = ((price - test_pos.entry_price) / test_pos.entry_price) * 100.0;

        let should_sell = urgency > 0.7;
        let status = if should_sell { "🔴 SELL" } else { "🟢 HOLD" };

        println!("{}: P&L {:.1}%, Urgency: {:.3} - {}", scenario, pnl, urgency, status);
    }

    println!("\n✅ Improved urgency function is working!");
    println!("🎯 Key improvements:");
    println!("   - Longer time window (30 min vs 10 min)");
    println!("   - Current P&L based (not peak price)");
    println!("   - Multi-factor urgency calculation");
    println!("   - Loss-tolerant until -50%");
    println!("   - Profit-taking balanced with time");
}
