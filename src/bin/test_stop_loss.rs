use screenerbot::profit::should_sell;
use screenerbot::positions::Position;
use chrono::Utc;

fn main() {
    println!("🧪 Testing Stop Loss Logic");

    // Create a test position
    let position = Position {
        mint: "test_mint".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 1.0,
        entry_time: Utc::now(),
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.1,
        total_size_sol: 0.1,
        price_highest: 1.0,
        price_lowest: 1.0,
        entry_transaction_signature: None,
        exit_transaction_signature: None,
        token_amount: Some(1000000),
        effective_entry_price: Some(1.0),
        effective_exit_price: None,
        sol_received: None,
    };

    println!("\n📊 Testing different loss scenarios:");

    // Test scenarios
    let test_cases = vec![
        (-10.0, "10% loss - should hold"),
        (-50.0, "50% loss - should hold"),
        (-90.0, "90% loss - should hold"),
        (-99.0, "99% loss - should trigger emergency exit"),
        (-99.5, "99.5% loss - should trigger emergency exit"),
        (10.0, "10% profit - should check other conditions")
    ];

    for (loss_percent, description) in test_cases {
        let test_price = position.entry_price * (1.0 + loss_percent / 100.0);
        let (urgency, reason) = should_sell(&position, test_price);

        println!(
            "  {} (price: {:.8}): Urgency {:.2} - {}",
            description,
            test_price,
            urgency,
            reason
        );
    }

    println!("\n✅ Stop loss test completed");
}
