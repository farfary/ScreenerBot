use anyhow::Result;
use chrono::Utc;
use screenerbot::{ Config, trader::position::Position, trader::strategy::TradingStrategy };

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Sell Signal Generation in Dry Run Mode");
    println!("═══════════════════════════════════════════════════");

    // Load config
    let config = Config::load("configs.json")?;

    // Create strategy
    let strategy = TradingStrategy::new(config.trader.clone());

    // Create test positions with different profit levels
    let test_cases = vec![
        ("Token with 50.74% profit", 0.00000116, 0.00000077), // Like your real position
        ("Token with 13.91% profit", 0.00000033, 0.00000029), // Like your real position
        ("Token with 3.09% profit", 0.0000022, 0.00000213), // Like your real position
        ("Token with 4.58% profit", 0.00000284, 0.00000272), // Like your real position
        ("Token with -2.98% loss", 0.00000069, 0.00000071), // Should not trigger sell
        ("Token with exactly 5% profit", 0.000001, 0.00000095) // Exactly at trigger
    ];

    for (description, current_price, buy_price) in test_cases {
        println!("\n📊 Testing: {}", description);

        // Create position
        let mut position = Position::new("test_token".to_string(), "TEST".to_string());

        // Set up position as if we bought at buy_price
        position.total_tokens = 1000.0;
        position.total_invested_sol = buy_price * 1000.0;
        position.average_buy_price = buy_price;
        position.original_entry_price = buy_price;
        position.update_price(current_price);

        // Calculate P&L
        let profit_percent = ((current_price - buy_price) / buy_price) * 100.0;

        println!("   Buy Price: {:.10} SOL", buy_price);
        println!("   Current Price: {:.10} SOL", current_price);
        println!("   Profit/Loss: {:.2}%", profit_percent);
        println!("   Sell Trigger: {:.1}%", config.trader.sell_trigger_percent);

        // Analyze position for signals
        let signals = strategy.analyze_position(&position, current_price);

        // Check results
        let has_sell_signal = signals
            .iter()
            .any(|s| matches!(s.signal_type, screenerbot::trader::types::TradeSignalType::Sell));
        let has_stop_loss_signal = signals
            .iter()
            .any(|s|
                matches!(s.signal_type, screenerbot::trader::types::TradeSignalType::StopLoss)
            );

        if profit_percent >= config.trader.sell_trigger_percent {
            if has_sell_signal {
                println!("   ✅ SELL signal generated (Expected: YES)");
            } else {
                println!("   ❌ NO SELL signal generated (Expected: YES) - BUG!");
            }
        } else if profit_percent <= config.trader.stop_loss_percent {
            if has_stop_loss_signal {
                println!("   ✅ STOP LOSS signal generated (Expected: YES)");
            } else {
                println!("   ❌ NO STOP LOSS signal generated (Expected: YES) - BUG!");
            }
        } else {
            if !has_sell_signal && !has_stop_loss_signal {
                println!("   ✅ No signals generated (Expected: NO)");
            } else {
                println!("   ❌ Unexpected signal generated (Expected: NO) - BUG!");
            }
        }

        // Show all signals
        if !signals.is_empty() {
            println!("   Signals generated: {}", signals.len());
            for signal in &signals {
                println!("     - {:?} at price {:.10}", signal.signal_type, signal.current_price);
            }
        }
    }

    println!("\n🎯 Summary:");
    println!("   - Sell trigger is set to {}%", config.trader.sell_trigger_percent);
    println!("   - Stop loss is set to {}%", config.trader.stop_loss_percent);
    println!("   - Dry run mode: {}", config.trader.dry_run);

    if config.trader.dry_run {
        println!("   - In dry run mode, both BUY and SELL should be simulated");
    }

    println!("\n✅ Test Complete");
    Ok(())
}
